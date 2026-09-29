//! Owns one stream's socket reader, snapshot loads and independent consumer callbacks.
use super::{
    bounded_callback, DeliveryCompletion, EventRecovery, LoadRequest, OwnedJob, RecoveryAction,
    RecoveryOutcome, RecoveryPhase, ResetCompletion, ResetWork, SnapshotCompletion,
};
use crate::{
    domain::{
        events::ConsumerId,
        limits::{Limits, Resource},
    },
    ports::{ClientError, EventConsumer, EventSource},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{mpsc, Mutex as AsyncMutex},
    task::JoinSet,
};
use workbench_protocol::{events::EventFrame, workbench::StreamCursor, PROTOCOL_VERSION};
type Consumer = Arc<AsyncMutex<Box<dyn EventConsumer>>>;
enum Completion {
    Opened(ConsumerId, Result<(), ClientError>),
    Snapshot(SnapshotCompletion),
    Reset(ConsumerId, ResetCompletion, bool),
    Delivery(ConsumerId, DeliveryCompletion, bool),
}
/// Progress is published only by the actor after applied completion; readers cannot ACK.
#[derive(Clone)]
pub struct SessionProgress(Arc<Mutex<Progress>>);
struct Progress {
    cursor: StreamCursor,
    phase: RecoveryPhase,
    exhausted: BTreeSet<ConsumerId>,
    budget: Arc<Mutex<QueueBudget>>,
}
impl SessionProgress {
    pub fn queue_usage(&self) -> Result<(usize, usize), ClientError> {
        let budget = self
            .0
            .lock()
            .map_err(|_| ClientError::Protocol)?
            .budget
            .clone();
        let state = budget.lock().map_err(|_| ClientError::Protocol)?;
        Ok((
            state.model_items + state.pending_items,
            state.model_bytes + state.pending_bytes,
        ))
    }
    pub fn queue_peaks(&self) -> Result<(usize, usize), ClientError> {
        let budget = self
            .0
            .lock()
            .map_err(|_| ClientError::Protocol)?
            .budget
            .clone();
        let state = budget.lock().map_err(|_| ClientError::Protocol)?;
        Ok((state.peak_items, state.peak_bytes))
    }
    pub fn cursor(&self) -> Result<StreamCursor, ClientError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| ClientError::Protocol)?
            .cursor
            .clone())
    }
    pub fn phase(&self) -> Result<RecoveryPhase, ClientError> {
        Ok(self.0.lock().map_err(|_| ClientError::Protocol)?.phase)
    }
    pub fn exhausted_consumers(&self) -> Result<Vec<ConsumerId>, ClientError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| ClientError::Protocol)?
            .exhausted
            .iter()
            .copied()
            .collect())
    }
}
pub struct SessionResult {
    pub cursor: StreamCursor,
    pub result: Result<(), ClientError>,
    pub cleanup_error: Option<ClientError>,
}
struct Reader {
    job: OwnedJob<()>,
    frames: mpsc::Receiver<Result<QueuedFrame, ClientError>>,
}
pub struct EventSession {
    model: EventRecovery,
    limits: Limits,
    source: Arc<dyn EventSource>,
    consumers: BTreeMap<ConsumerId, Consumer>,
    busy: BTreeSet<ConsumerId>,
    jobs: JoinSet<Completion>,
    opens: JoinSet<Completion>,
    opened: BTreeSet<ConsumerId>,
    opening: BTreeSet<ConsumerId>,
    pending_resets: BTreeMap<ConsumerId, ResetWork>,
    reader: Option<Reader>,
    progress: SessionProgress,
    reconnects: u32,
    budget: Arc<Mutex<QueueBudget>>,
}
impl Drop for EventSession {
    fn drop(&mut self) {
        self.jobs.abort_all();
        self.opens.abort_all();
        if let Some(reader) = &self.reader {
            reader.job.handle.abort();
        }
        let _ = self.model.close();
        if let Ok(mut progress) = self.progress.0.lock() {
            progress.cursor = self.model.cursor();
            progress.phase = RecoveryPhase::Terminal;
        }
        if let Ok(mut state) = self.budget.lock() {
            let (items, bytes) = self.model.reducer.usage();
            state.model_items = items;
            state.model_bytes = bytes;
        }
    }
}
impl EventSession {
    pub fn new(
        cursor: StreamCursor,
        source: Arc<dyn EventSource>,
        limits: Limits,
    ) -> Result<Self, ClientError> {
        let model = EventRecovery::new(cursor.clone(), limits.clone())?;
        let budget = Arc::new(Mutex::new(QueueBudget {
            limits: limits.clone(),
            model_items: 0,
            model_bytes: 0,
            pending_items: 0,
            pending_bytes: 0,
            peak_items: 0,
            peak_bytes: 0,
        }));
        Ok(Self {
            model,
            budget: budget.clone(),
            limits,
            source,
            consumers: BTreeMap::new(),
            busy: BTreeSet::new(),
            jobs: JoinSet::new(),
            opens: JoinSet::new(),
            opened: BTreeSet::new(),
            opening: BTreeSet::new(),
            pending_resets: BTreeMap::new(),
            reader: None,
            progress: SessionProgress(Arc::new(Mutex::new(Progress {
                cursor,
                phase: RecoveryPhase::Connecting,
                exhausted: BTreeSet::new(),
                budget,
            }))),
            reconnects: 0,
        })
    }
    pub fn progress(&self) -> SessionProgress {
        self.progress.clone()
    }
    /// Subscribe before running; each consumer owns an independent applied/reset generation.
    pub fn subscribe(
        &mut self,
        after: u64,
        consumer: Box<dyn EventConsumer>,
    ) -> Result<ConsumerId, ClientError> {
        let id = self.model.register(after)?;
        self.consumers
            .insert(id, Arc::new(AsyncMutex::new(consumer)));
        Ok(id)
    }
    pub async fn run(mut self, stop: impl Future<Output = ()>) -> SessionResult {
        let result = tokio::select! {biased; _=stop=>Err(ClientError::Cancelled),result=self.drive()=>result};
        let cleanup_error = self.cleanup().await.err();
        let cursor = self.model.cursor();
        SessionResult {
            cursor,
            result,
            cleanup_error,
        }
    }
    async fn drive(&mut self) -> Result<(), ClientError> {
        if self.consumers.is_empty() {
            return Err(ClientError::InvalidInput);
        }
        self.connect(self.model.cursor()).await?;
        for (&id, port) in &self.consumers {
            let port = port.clone();
            let cursor = self.model.consumer_cursor(id)?;
            let deadline = self.limits.config().request_timeout;
            self.busy.insert(id);
            self.opening.insert(id);
            self.opens.spawn(async move {
                let result =
                    bounded_callback(async { port.lock().await.opened(&cursor).await }, deadline)
                        .await;
                Completion::Opened(id, result)
            });
        }
        loop {
            self.publish()?;
            if self.model.phase() == RecoveryPhase::Terminal {
                return Ok(());
            }
            if self.model.phase() == RecoveryPhase::Exhausted {
                return Err(ClientError::Unavailable);
            }
            self.schedule_deliveries()?;
            enum Wake {
                Frame(Option<Result<QueuedFrame, ClientError>>),
                Job(Option<Result<Completion, tokio::task::JoinError>>),
            }
            let wake = tokio::select! {
             frame=async {match self.reader.as_mut() {Some(reader)=>reader.frames.recv().await,None=>std::future::pending().await}}=>Wake::Frame(frame),
             completion=self.jobs.join_next(),if !self.jobs.is_empty()=>Wake::Job(completion),
             completion=self.opens.join_next(),if !self.opens.is_empty()=>Wake::Job(completion),
            };
            match wake {
                Wake::Frame(Some(Ok(mut queued))) => {
                    let mut frame = Some(queued.frame);
                    {
                        let budget = self.budget.clone();
                        let mut state = budget.lock().map_err(|_| ClientError::Protocol)?;
                        queued.lease.release(&mut state);
                        frame = match frame.take().ok_or(ClientError::Protocol)? {
                            EventFrame::Event { event } => {
                                self.model.reducer.receive_reserved(
                                    event,
                                    state.pending_items,
                                    state.pending_bytes,
                                )?;
                                None
                            }
                            control => Some(control),
                        };
                        let (items, bytes) = self.model.reducer.usage();
                        state.model_items = items;
                        state.model_bytes = bytes;
                        state.changed();
                    }
                    match frame {
                        None => {}
                        Some(EventFrame::Gap { gap }) => {
                            let before = self.model.generation;
                            let action = {
                                let budget = self.budget.clone();
                                let mut state = budget.lock().map_err(|_| ClientError::Protocol)?;
                                let action = self.model.start_gap(gap)?;
                                let (items, bytes) = self.model.reducer.usage();
                                state.model_items = items;
                                state.model_bytes = bytes;
                                state.changed();
                                action
                            };
                            if before != self.model.generation
                                || matches!(action, RecoveryAction::Exhausted)
                            {
                                self.cancel_callbacks().await?;
                            }
                            self.action(action).await?;
                        }
                        Some(EventFrame::Fault { fault }) => return Err(ClientError::Fault(fault)),
                        _ => return Err(ClientError::Protocol),
                    }
                }
                Wake::Frame(Some(Err(ClientError::Unavailable))) | Wake::Frame(None) => {
                    let action = self.model.disconnected()?;
                    self.action(action).await?;
                }
                Wake::Frame(Some(Err(error))) => return Err(error),
                Wake::Job(Some(Ok(completion))) => self.complete(completion).await?,
                Wake::Job(Some(Err(_))) => return Err(ClientError::Protocol),
                Wake::Job(None) => {}
            }
        }
    }
    fn publish(&self) -> Result<(), ClientError> {
        let mut p = self.progress.0.lock().map_err(|_| ClientError::Protocol)?;
        p.cursor = self.model.cursor();
        p.phase = self.model.phase();
        Ok(())
    }
    async fn connect(&mut self, cursor: StreamCursor) -> Result<(), ClientError> {
        self.cancel_reader().await?;
        let source = self.source.clone();
        let deadline = self.limits.config().request_timeout;
        let mut socket =
            bounded_callback(async { source.connect(&cursor).await }, deadline).await?;
        if socket.identity().epoch() != cursor.epoch {
            return Err(ClientError::Incompatible);
        }
        let load = self
            .model
            .hello(PROTOCOL_VERSION, socket.identity().epoch())?;
        // Reader handoff and every reducer queue share a single aggregate budget.
        // A permit survives channel buffering and is transferred atomically into the reducer.
        let (tx, frames) = mpsc::channel(1);
        let budget = self.budget.clone();
        let job = OwnedJob::spawn(
            async move {
                loop {
                    let frame = socket.next().await;
                    match frame {
                        Ok(Some(frame)) => {
                            let lease = QueueLease::acquire(budget.clone(), &frame);
                            match lease {
                                Ok(lease) => {
                                    if tx.send(Ok(QueuedFrame { frame, lease })).await.is_err() {
                                        break;
                                    }
                                }
                                Err(error) => {
                                    drop(frame);
                                    let _ = tx.send(Err(error)).await;
                                    break;
                                }
                            }
                        }
                        Ok(None) => break,
                        Err(error) => {
                            let _ = tx.send(Err(error)).await;
                            break;
                        }
                    }
                }
                let _ = tokio::time::timeout(deadline, socket.close()).await;
            },
            self.limits.config().connect_timeout,
        );
        self.reader = Some(Reader { job, frames });
        if let Some(load) = load {
            self.load(load)
        }
        Ok(())
    }
    fn load(&mut self, request: LoadRequest) {
        let source = self.source.clone();
        let deadline = self.limits.config().request_timeout;
        self.jobs.spawn(async move {
            let result = bounded_callback(
                async {
                    let mut source = source.snapshot_port();
                    source.snapshot_after(&request.applied, &request.live).await
                },
                deadline,
            )
            .await;
            Completion::Snapshot(SnapshotCompletion { request, result })
        });
    }
    fn resets(&mut self, work: Vec<ResetWork>) -> Result<(), ClientError> {
        for work in work {
            let id = work.consumer();
            if !self.opened.contains(&id) {
                self.pending_resets.insert(id, work);
                continue;
            }
            let port = self
                .consumers
                .get(&id)
                .ok_or(ClientError::StaleGeneration)?
                .clone();
            let deadline = self.limits.config().request_timeout;
            self.busy.insert(id);
            self.jobs.spawn(async move {
                let mut terminal = false;
                let result = bounded_callback(
                    async {
                        let mut port = port.lock().await;
                        terminal = !port.resync_on_failure();
                        port.reset_from(&work.snapshot, &work.applied).await
                    },
                    deadline,
                )
                .await;
                Completion::Reset(id, ResetCompletion { work, result }, terminal)
            });
        }
        Ok(())
    }
    fn schedule_deliveries(&mut self) -> Result<(), ClientError> {
        for (&id, port) in &self.consumers {
            if !self.opened.contains(&id) || self.busy.contains(&id) {
                continue;
            }
            if let Some(delivery) = self.model.next(id)? {
                let port = port.clone();
                let deadline = self.limits.config().request_timeout;
                self.busy.insert(id);
                self.jobs.spawn(async move {
                    let mut terminal = false;
                    let result = bounded_callback(
                        async {
                            let mut port = port.lock().await;
                            terminal = !port.resync_on_failure();
                            port.consume(&delivery.event).await
                        },
                        deadline,
                    )
                    .await;
                    Completion::Delivery(id, DeliveryCompletion { delivery, result }, terminal)
                });
            }
        }
        Ok(())
    }
    async fn complete(&mut self, completion: Completion) -> Result<(), ClientError> {
        let action = {
            let budget = self.budget.clone();
            let mut state = budget.lock().map_err(|_| ClientError::Protocol)?;
            let action = match completion {
                Completion::Opened(id, result) => {
                    self.busy.remove(&id);
                    self.opening.remove(&id);
                    result?;
                    self.opened.insert(id);
                    if let Some(work) = self.pending_resets.remove(&id) {
                        self.resets(vec![work])?;
                    }
                    None
                }
                Completion::Snapshot(completion) => {
                    match self.model.complete_snapshot(completion) {
                        Ok(RecoveryOutcome::Reset(work)) => {
                            self.resets(work)?;
                            None
                        }
                        Ok(RecoveryOutcome::Action(action)) => Some(action),
                        Err(ClientError::StaleGeneration) => None,
                        Err(error) => return Err(error),
                    }
                }
                Completion::Reset(id, completion, terminal) => {
                    if terminal && completion.result.is_err() {
                        return completion.result;
                    }
                    self.busy.remove(&id);
                    let recovering = self.model.round.is_some();
                    let action = self.model.complete_reset(completion)?;
                    if recovering && action.is_none() && self.model.is_live() {
                        self.reconnects = 0;
                    }
                    action
                }
                Completion::Delivery(id, completion, terminal) => {
                    if terminal && completion.result.is_err() {
                        return completion.result;
                    }
                    self.busy.remove(&id);
                    let action = self.model.complete_delivery(completion)?;
                    if action.is_none() && self.model.is_live() {
                        self.reconnects = 0;
                    }
                    action
                }
            };
            let (items, bytes) = self.model.reducer.usage();
            state.model_items = items;
            state.model_bytes = bytes;
            state.changed();
            action
        };
        if let Some(action) = action {
            self.action(action).await?
        }
        Ok(())
    }
    async fn action(&mut self, action: RecoveryAction) -> Result<(), ClientError> {
        match action {
            RecoveryAction::Connect(cursor) => {
                // Retire the old socket before backoff. A hello alone never resets this budget.
                self.cancel_reader().await?;
                loop {
                    if self.reconnects >= self.limits.config().recovery_attempts {
                        return Err(ClientError::Unavailable);
                    }
                    self.reconnects += 1;
                    let random = uuid::Uuid::new_v4();
                    let bytes = random.as_bytes();
                    let jitter = u16::from_be_bytes([bytes[14], bytes[15]]) % 1001;
                    let delay = self
                        .model
                        .backoff_for(self.reconnects.max(self.model.attempts), jitter)?;
                    tokio::time::sleep(delay).await;
                    match self.connect(cursor.clone()).await {
                        Ok(()) => return Ok(()),
                        Err(error) if transient_connect(&error) => {}
                        Err(error) => return Err(error),
                    }
                }
            }
            RecoveryAction::Load(load) => {
                if self.model.round.as_ref().is_some_and(|r| r.terminal) {
                    self.cancel_reader().await?;
                }
                self.load(load);
                Ok(())
            }
            RecoveryAction::Exhausted => Err(ClientError::Unavailable),
            RecoveryAction::ListenerExhausted(id) => {
                self.progress
                    .0
                    .lock()
                    .map_err(|_| ClientError::Protocol)?
                    .exhausted
                    .insert(id);
                if self.consumers.len() == 1 {
                    Err(ClientError::Unavailable)
                } else {
                    Ok(())
                }
            }
        }
    }
    async fn cancel_reader(&mut self) -> Result<(), ClientError> {
        if let Some(reader) = self.reader.take() {
            reader.job.cancel().await?;
        }
        Ok(())
    }
    async fn cancel_callbacks(&mut self) -> Result<(), ClientError> {
        self.jobs.abort_all();
        tokio::time::timeout(self.limits.config().connect_timeout, async {
            while self.jobs.join_next().await.is_some() {}
        })
        .await
        .map_err(|_| ClientError::Deadline)?;
        self.busy = self.opening.clone();
        self.pending_resets.clear();
        Ok(())
    }
    async fn cleanup(&mut self) -> Result<(), ClientError> {
        // Stop new IO first, then preserve already successful callback completions.
        // Aborted/pending callbacks never become ACKs; stale generations stay stale.
        self.jobs.abort_all();
        self.opens.abort_all();
        let reader = self.cancel_reader().await;
        let mut settled = Vec::new();
        let jobs = tokio::time::timeout(self.limits.config().connect_timeout, async {
            while let Some(result) = self.jobs.join_next().await {
                if let Ok(completion) = result {
                    settled.push(completion);
                }
            }
        })
        .await
        .map_err(|_| ClientError::Deadline);
        let mut applied = Ok(());
        for completion in settled {
            let result = match completion {
                Completion::Delivery(_, completion, _) if completion.result.is_ok() => {
                    self.model.ack(completion.delivery)
                }
                Completion::Reset(_, completion, _) if completion.result.is_ok() => {
                    self.model.complete_reset(completion).map(|_| ())
                }
                _ => Ok(()),
            };
            if !matches!(result, Err(ClientError::StaleGeneration)) {
                applied = applied.and(result);
            }
        }
        let opens = tokio::time::timeout(self.limits.config().connect_timeout, async {
            while self.opens.join_next().await.is_some() {}
        })
        .await
        .map_err(|_| ClientError::Deadline);
        let closed = {
            let budget = self.budget.clone();
            let mut state = budget.lock().map_err(|_| ClientError::Protocol)?;
            let result = self.model.close();
            let (items, bytes) = self.model.reducer.usage();
            state.model_items = items;
            state.model_bytes = bytes;
            result
        };
        let published = self.publish();
        reader
            .and(jobs)
            .and(applied)
            .and(opens)
            .and(closed)
            .and(published)
    }
}

struct QueueBudget {
    limits: Limits,
    model_items: usize,
    model_bytes: usize,
    pending_items: usize,
    pending_bytes: usize,
    peak_items: usize,
    peak_bytes: usize,
}
impl QueueBudget {
    fn changed(&mut self) {
        self.peak_items = self.peak_items.max(self.model_items + self.pending_items);
        self.peak_bytes = self.peak_bytes.max(self.model_bytes + self.pending_bytes);
    }
}
struct QueuedFrame {
    frame: EventFrame,
    lease: QueueLease,
}
struct QueueLease {
    budget: Arc<Mutex<QueueBudget>>,
    bytes: usize,
    active: bool,
}
impl QueueLease {
    fn acquire(budget: Arc<Mutex<QueueBudget>>, frame: &EventFrame) -> Result<Self, ClientError> {
        let bytes = match frame {
            EventFrame::Event { event } => serde_json::to_vec(event),
            _ => serde_json::to_vec(frame),
        }
        .map_err(|_| ClientError::Protocol)?
        .len();
        {
            let mut state = budget.lock().map_err(|_| ClientError::Protocol)?;
            let items = state.limits.check_add(
                Resource::QueueItems,
                state.model_items,
                state.pending_items,
            )?;
            let retained = state.limits.check_add(
                Resource::QueueBytes,
                state.model_bytes,
                state.pending_bytes,
            )?;
            state.limits.check_add(Resource::QueueItems, items, 1)?;
            state
                .limits
                .check_add(Resource::QueueBytes, retained, bytes)?;
            state.pending_items += 1;
            state.pending_bytes += bytes;
            state.changed();
        }
        Ok(Self {
            budget,
            bytes,
            active: true,
        })
    }
    fn release(&mut self, state: &mut QueueBudget) {
        if self.active {
            state.pending_items -= 1;
            state.pending_bytes -= self.bytes;
            self.active = false;
        }
    }
}
impl Drop for QueueLease {
    fn drop(&mut self) {
        if self.active {
            let budget = self.budget.clone();
            if let Ok(mut state) = budget.lock() {
                self.release(&mut state);
            };
        }
    }
}

fn transient_connect(error: &ClientError) -> bool {
    match error {
        ClientError::Unavailable | ClientError::Deadline | ClientError::TransportUnknown => true,
        ClientError::Fault(fault) => {
            fault.retryable
                && matches!(
                    fault.code,
                    workbench_protocol::FaultCode::Unavailable
                        | workbench_protocol::FaultCode::Draining
                        | workbench_protocol::FaultCode::RateLimited
                        | workbench_protocol::FaultCode::DeadlineExceeded
                )
        }
        _ => false,
    }
}
