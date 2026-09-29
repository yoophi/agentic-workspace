//! Live-first recovery orchestration. Async completion tokens never confer an ACK.
pub mod session;
use crate::{
    domain::{
        events::{ConsumerId, Delivery, EventReducer, Reset},
        limits::{Limits, Resource},
    },
    ports::{ClientError, EventConsumer, Snapshot, SnapshotPort},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use workbench_protocol::{
    workbench::{EventEnvelope, GapNotice, GapReason, StreamCursor},
    PROTOCOL_VERSION,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPhase {
    Connecting,
    LiveBuffering,
    SnapshotLoading,
    ConsumerReset,
    Live,
    Terminal,
    Exhausted,
    Idle,
}
pub enum RecoveryAction {
    Connect(StreamCursor),
    Load(LoadRequest),
    Exhausted,
    ListenerExhausted(ConsumerId),
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadScope {
    Stream,
    Listener(ConsumerId),
}
pub struct LoadRequest {
    scope: LoadScope,
    owner: Arc<()>,
    generation: u64,
    applied: StreamCursor,
    live: StreamCursor,
}
impl LoadRequest {
    pub fn live_cursor(&self) -> &StreamCursor {
        &self.live
    }
    pub fn applied_cursor(&self) -> &StreamCursor {
        &self.applied
    }
}
impl std::fmt::Debug for LoadRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LoadRequest([redacted])")
    }
}
pub struct ResetWork {
    scope: LoadScope,
    owner: Arc<()>,
    generation: u64,
    consumer: ConsumerId,
    reset: Reset,
    applied: StreamCursor,
    snapshot: Arc<Snapshot>,
}
impl ResetWork {
    pub fn applied_cursor(&self) -> &StreamCursor {
        &self.applied
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
    pub fn consumer(&self) -> ConsumerId {
        self.consumer
    }
}
impl std::fmt::Debug for ResetWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResetWork([redacted])")
    }
}
struct Round {
    generation: u64,
    boundary: StreamCursor,
    terminal: bool,
    hello: bool,
    load_issued: bool,
    snapshot: Option<Arc<Snapshot>>,
    pending: BTreeSet<ConsumerId>,
    resets: BTreeMap<ConsumerId, Reset>,
}
struct ListenerRound {
    generation: u64,
    attempts: u32,
    reset: Option<Reset>,
    exhausted: bool,
}
pub struct EventRecovery {
    owner: Arc<()>,
    reducer: EventReducer,
    limits: Limits,
    phase: RecoveryPhase,
    round: Option<Round>,
    generation: u64,
    attempts: u32,
    listeners: BTreeMap<ConsumerId, ListenerRound>,
}
impl EventRecovery {
    pub fn new(cursor: StreamCursor, limits: Limits) -> Result<Self, ClientError> {
        Ok(Self {
            owner: Arc::new(()),
            reducer: EventReducer::new(cursor, limits.clone())?,
            limits,
            phase: RecoveryPhase::Connecting,
            round: None,
            generation: 0,
            attempts: 0,
            listeners: BTreeMap::new(),
        })
    }
    pub fn phase(&self) -> RecoveryPhase {
        self.phase
    }
    pub fn is_live(&self) -> bool {
        self.phase == RecoveryPhase::Live
    }
    pub fn cursor(&self) -> StreamCursor {
        self.reducer.cursor()
    }
    pub fn consumer_cursor(&self, id: ConsumerId) -> Result<StreamCursor, ClientError> {
        self.reducer.consumer_cursor(id)
    }
    pub fn reconnect_cursor(&self) -> StreamCursor {
        self.round
            .as_ref()
            .map(|r| r.boundary.clone())
            .unwrap_or_else(|| self.cursor())
    }
    /// An earlier listener can require a fresh ticket even while the old socket is live.
    /// A recovery snapshot already covers that listener, so it supersedes replay.
    pub fn take_replay_requirement(&mut self) -> Option<StreamCursor> {
        let requested = self.reducer.take_replay_requirement();
        (requested && self.round.is_none()).then(|| self.cursor())
    }
    pub fn disconnected(&mut self) -> Result<RecoveryAction, ClientError> {
        self.check_active_completion()?;
        if self.round.is_some() {
            return Ok(RecoveryAction::Connect(self.reconnect_cursor()));
        }
        if self.reducer.snapshot_on_reconnect() {
            let boundary = StreamCursor {
                after_sequence: self.reducer.received(),
                ..self.cursor()
            };
            self.begin_round(boundary, false)
        } else {
            self.phase = RecoveryPhase::Connecting;
            Ok(RecoveryAction::Connect(self.cursor()))
        }
    }
    pub fn register(&mut self, after: u64) -> Result<ConsumerId, ClientError> {
        let id = self.reducer.register(after)?;
        if let Some(round) = &mut self.round {
            let reset = self.reducer.begin_reset(id)?;
            round.pending.insert(id);
            round.resets.insert(id, reset);
        }
        Ok(id)
    }
    pub fn unregister(&mut self, id: ConsumerId) -> Result<(), ClientError> {
        self.reducer.unregister(id)?;
        self.listeners.remove(&id);
        if let Some(round) = &mut self.round {
            round.pending.remove(&id);
            round.resets.remove(&id);
        }
        if self.reducer.consumers().is_empty() {
            self.round = None;
            self.phase = RecoveryPhase::Idle;
        } else {
            self.finish_round_if_applied();
        }
        Ok(())
    }
    pub fn start_gap(&mut self, gap: GapNotice) -> Result<RecoveryAction, ClientError> {
        if matches!(
            self.phase,
            RecoveryPhase::Terminal | RecoveryPhase::Exhausted
        ) {
            return Err(ClientError::Unavailable);
        }
        let current = self.cursor();
        if gap.stream_id != current.stream_id
            || gap.epoch.is_empty()
            || gap
                .first_sequence
                .zip(gap.last_sequence)
                .is_some_and(|(first, last)| first > last)
        {
            return Err(ClientError::Protocol);
        }
        if gap.epoch != current.epoch {
            self.reducer.rebind(StreamCursor {
                epoch: gap.epoch.clone(),
                after_sequence: 0,
                ..current
            })?;
            // A new epoch invalidates every round before reason-specific shortcuts.
            // Cursor zero obtains a new-epoch retention gap if live history already moved.
            return self.begin_round(self.cursor(), gap.reason == GapReason::Evicted);
        }
        if matches!(
            gap.reason,
            GapReason::SubscriberLagged | GapReason::Shutdown
        ) {
            if self.reducer.snapshot_on_reconnect() {
                // Notification subscriptions are live-only. A reconnect cannot replay
                // the lost signals; obtain fresh state after hello and reset consumers.
                let boundary = StreamCursor {
                    after_sequence: gap.last_sequence.unwrap_or(0).max(self.reducer.received()),
                    ..self.cursor()
                };
                return self.begin_round(boundary, false);
            }
            return Ok(RecoveryAction::Connect(self.reconnect_cursor()));
        }
        let after = match gap.reason {
            GapReason::EpochChanged => 0,
            GapReason::Evicted => self.reducer.received(),
            _ => gap.last_sequence.unwrap_or(0),
        };
        let boundary = StreamCursor {
            after_sequence: after,
            ..self.cursor()
        };
        self.begin_round(boundary, gap.reason == GapReason::Evicted)
    }
    fn begin_round(
        &mut self,
        boundary: StreamCursor,
        terminal: bool,
    ) -> Result<RecoveryAction, ClientError> {
        if self.attempts >= self.limits.config().recovery_attempts {
            self.round = None;
            self.listeners.clear();
            self.phase = RecoveryPhase::Exhausted;
            self.reducer.invalidate_pending()?;
            return Ok(RecoveryAction::Exhausted);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(ClientError::StaleGeneration)?;
        let ids = self.reducer.consumers();
        let mut resets = BTreeMap::new();
        for id in &ids {
            resets.insert(*id, self.reducer.begin_reset(*id)?);
        }
        self.listeners.clear();
        self.attempts += 1;
        self.generation = generation;
        self.round = Some(Round {
            generation,
            boundary: boundary.clone(),
            terminal,
            hello: false,
            load_issued: false,
            snapshot: None,
            pending: ids.into_iter().collect(),
            resets,
        });
        self.phase = RecoveryPhase::LiveBuffering;
        if terminal {
            let load = self.issue_load()?;
            Ok(RecoveryAction::Load(load))
        } else {
            Ok(RecoveryAction::Connect(boundary))
        }
    }
    fn issue_load(&mut self) -> Result<LoadRequest, ClientError> {
        let applied = self.cursor();
        let round = self.round.as_mut().ok_or(ClientError::StaleGeneration)?;
        if round.load_issued {
            return Err(ClientError::StaleGeneration);
        }
        round.load_issued = true;
        self.phase = RecoveryPhase::SnapshotLoading;
        Ok(LoadRequest {
            scope: LoadScope::Stream,
            owner: self.owner.clone(),
            generation: round.generation,
            live: round.boundary.clone(),
            applied,
        })
    }
    pub fn hello(
        &mut self,
        protocol: u16,
        epoch: &str,
    ) -> Result<Option<LoadRequest>, ClientError> {
        if matches!(
            self.phase,
            RecoveryPhase::Terminal | RecoveryPhase::Exhausted | RecoveryPhase::Idle
        ) {
            return Err(ClientError::Unavailable);
        }
        if protocol != PROTOCOL_VERSION || epoch != self.cursor().epoch {
            return Err(ClientError::Incompatible);
        }
        if let Some(round) = &mut self.round {
            if round.terminal {
                return Err(ClientError::Protocol);
            }
            round.hello = true;
            if !round.load_issued {
                return self.issue_load().map(Some);
            }
            Ok(None)
        } else {
            self.phase = RecoveryPhase::Live;
            Ok(None)
        }
    }
    pub fn receive(&mut self, event: EventEnvelope) -> Result<(), ClientError> {
        if matches!(
            self.phase,
            RecoveryPhase::Terminal | RecoveryPhase::Exhausted | RecoveryPhase::Idle
        ) {
            return Err(ClientError::Unavailable);
        }
        self.reducer.receive(event)
    }
    pub fn next(&mut self, id: ConsumerId) -> Result<Option<Delivery>, ClientError> {
        if self.reducer.snapshot_on_reconnect() && self.round.is_some() {
            // For live-only streams, every reset must complete before delivery resumes.
            // Retained streams keep their independent consumer replay/reset behavior.
            self.reducer.consumer_cursor(id)?;
            return Ok(None);
        }
        self.reducer.next(id)
    }
    pub fn ack(&mut self, delivery: Delivery) -> Result<(), ClientError> {
        self.check_active_completion()?;
        self.reducer.ack(delivery)
    }
    fn check_active_completion(&self) -> Result<(), ClientError> {
        if matches!(
            self.phase,
            RecoveryPhase::Terminal | RecoveryPhase::Exhausted | RecoveryPhase::Idle
        ) {
            Err(ClientError::StaleGeneration)
        } else {
            Ok(())
        }
    }
    pub fn close(&mut self) -> Result<(), ClientError> {
        self.round = None;
        self.listeners.clear();
        self.phase = RecoveryPhase::Terminal;
        self.reducer.invalidate_pending()
    }
    fn check_load(&self, load: &LoadRequest) -> Result<(), ClientError> {
        self.check_active_completion()?;
        if !Arc::ptr_eq(&self.owner, &load.owner) {
            return Err(ClientError::StaleGeneration);
        }
        if let LoadScope::Listener(id) = load.scope {
            let listener = self
                .listeners
                .get(&id)
                .ok_or(ClientError::StaleGeneration)?;
            return if listener.generation == load.generation
                && !listener.exhausted
                && listener.reset.is_some()
            {
                Ok(())
            } else {
                Err(ClientError::StaleGeneration)
            };
        }

        let round = self.round.as_ref().ok_or(ClientError::StaleGeneration)?;
        if !Arc::ptr_eq(&self.owner, &load.owner)
            || round.generation != load.generation
            || !round.load_issued
            || (!round.terminal && !round.hello)
            || round.snapshot.is_some()
        {
            return Err(ClientError::StaleGeneration);
        }
        Ok(())
    }
    pub fn snapshot_loaded(
        &mut self,
        load: LoadRequest,
        snapshot: Snapshot,
    ) -> Result<Vec<ResetWork>, ClientError> {
        self.check_load(&load)?;
        self.limits.check_cursor(&snapshot.cursor)?;
        self.limits.check_add(
            Resource::Snapshot,
            0,
            serde_json::to_vec(&snapshot.value)
                .map_err(|_| ClientError::Protocol)?
                .len(),
        )?;
        if let LoadScope::Listener(id) = load.scope {
            let applied = self.consumer_cursor(id)?;
            if snapshot.cursor.stream_id != applied.stream_id
                || snapshot.cursor.epoch != applied.epoch
                || snapshot.cursor.after_sequence < applied.after_sequence
            {
                return Err(ClientError::Protocol);
            }
            let listener = self
                .listeners
                .get_mut(&id)
                .ok_or(ClientError::StaleGeneration)?;
            let reset = listener.reset.take().ok_or(ClientError::StaleGeneration)?;
            return Ok(vec![ResetWork {
                scope: load.scope,
                owner: self.owner.clone(),
                generation: load.generation,
                consumer: id,
                reset,
                applied,
                snapshot: Arc::new(snapshot),
            }]);
        }
        let round = self.round.as_mut().ok_or(ClientError::StaleGeneration)?;
        if snapshot.cursor.stream_id != round.boundary.stream_id
            || snapshot.cursor.epoch != round.boundary.epoch
            || snapshot.cursor.after_sequence < round.boundary.after_sequence
        {
            return Err(ClientError::Protocol);
        }
        round.snapshot = Some(Arc::new(snapshot));
        self.phase = RecoveryPhase::ConsumerReset;
        self.pending_resets()
    }
    pub fn pending_resets(&mut self) -> Result<Vec<ResetWork>, ClientError> {
        let round = self.round.as_mut().ok_or(ClientError::StaleGeneration)?;
        let snapshot = round
            .snapshot
            .as_ref()
            .ok_or(ClientError::StaleGeneration)?
            .clone();
        let mut work = Vec::new();
        for (id, reset) in std::mem::take(&mut round.resets) {
            work.push(ResetWork {
                scope: LoadScope::Stream,
                owner: self.owner.clone(),
                generation: round.generation,
                consumer: id,
                reset,
                applied: self.reducer.consumer_cursor(id)?,
                snapshot: snapshot.clone(),
            });
        }
        Ok(work)
    }
    pub fn reset_applied(&mut self, work: ResetWork) -> Result<(), ClientError> {
        if let LoadScope::Listener(id) = work.scope {
            self.check_listener_work(&work)?;
            self.reducer
                .finish_reset(work.reset, &work.snapshot.cursor)?;
            self.listeners.remove(&id);
            if let Some(round) = &mut self.round {
                round.pending.remove(&id);
                round.resets.remove(&id);
            }
            self.finish_round_if_applied();
            return Ok(());
        }

        let round = self.round.as_ref().ok_or(ClientError::StaleGeneration)?;
        if !Arc::ptr_eq(&self.owner, &work.owner)
            || round.generation != work.generation
            || !round.pending.contains(&work.consumer)
        {
            return Err(ClientError::StaleGeneration);
        }
        self.reducer
            .finish_reset(work.reset, &work.snapshot.cursor)?;
        self.round
            .as_mut()
            .ok_or(ClientError::StaleGeneration)?
            .pending
            .remove(&work.consumer);
        self.finish_round_if_applied();
        Ok(())
    }
    fn finish_round_if_applied(&mut self) {
        if self
            .round
            .as_ref()
            .is_some_and(|r| r.snapshot.is_some() && r.pending.is_empty())
        {
            let terminal = self.round.take().is_some_and(|r| r.terminal);
            self.phase = if terminal {
                RecoveryPhase::Terminal
            } else {
                RecoveryPhase::Live
            };
            if terminal {
                self.listeners.clear();
                let _ = self.reducer.invalidate_pending(); // terminal guard also rejects every completion
            }
            self.attempts = 0;
        }
    }
    pub fn snapshot_failed(&mut self, load: LoadRequest) -> Result<RecoveryAction, ClientError> {
        self.check_load(&load)?;
        if let LoadScope::Listener(id) = load.scope {
            return self.begin_listener_reset(id);
        }
        let round = self.round.as_ref().ok_or(ClientError::StaleGeneration)?;
        let (boundary, terminal) = (round.boundary.clone(), round.terminal);
        self.begin_round(boundary, terminal)
    }

    fn check_listener_work(&self, work: &ResetWork) -> Result<(), ClientError> {
        self.check_active_completion()?;
        let listener = self
            .listeners
            .get(&work.consumer)
            .ok_or(ClientError::StaleGeneration)?;
        if !Arc::ptr_eq(&self.owner, &work.owner)
            || listener.generation != work.generation
            || listener.exhausted
            || listener.reset.is_some()
        {
            return Err(ClientError::StaleGeneration);
        }
        Ok(())
    }
    fn begin_listener_reset(&mut self, id: ConsumerId) -> Result<RecoveryAction, ClientError> {
        let applied = self.consumer_cursor(id)?;
        let attempts = self.listeners.get(&id).map_or(0, |r| r.attempts);
        if attempts >= self.limits.config().recovery_attempts {
            if let Some(listener) = self.listeners.get_mut(&id) {
                listener.exhausted = true;
                listener.reset = None;
            }
            return Ok(RecoveryAction::ListenerExhausted(id));
        }
        let reset = self.reducer.begin_reset(id)?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(ClientError::StaleGeneration)?;
        self.listeners.insert(
            id,
            ListenerRound {
                generation: self.generation,
                attempts: attempts + 1,
                reset: Some(reset),
                exhausted: false,
            },
        );
        Ok(RecoveryAction::Load(LoadRequest {
            scope: LoadScope::Listener(id),
            owner: self.owner.clone(),
            generation: self.generation,
            live: applied.clone(),
            applied,
        }))
    }
    pub fn consumer_failed(&mut self, delivery: Delivery) -> Result<RecoveryAction, ClientError> {
        self.check_active_completion()?;
        let id = delivery.consumer();
        self.reducer.fail(delivery)?;
        self.begin_listener_reset(id)
    }
    pub fn reset_failed(&mut self, work: ResetWork) -> Result<RecoveryAction, ClientError> {
        match work.scope {
            LoadScope::Listener(_) => self.check_listener_work(&work)?,
            LoadScope::Stream => {
                let round = self.round.as_ref().ok_or(ClientError::StaleGeneration)?;
                if !Arc::ptr_eq(&self.owner, &work.owner)
                    || round.generation != work.generation
                    || !round.pending.contains(&work.consumer)
                {
                    return Err(ClientError::StaleGeneration);
                }
            }
        }
        self.begin_listener_reset(work.consumer)
    }
    pub fn complete_snapshot(
        &mut self,
        completion: SnapshotCompletion,
    ) -> Result<RecoveryOutcome, ClientError> {
        // Validate ownership before classifying even terminal errors: a late old
        // HTTP failure cannot terminate a newer recovery/live generation.
        self.check_load(&completion.request)?;
        match completion.result {
            Ok(snapshot) => self
                .snapshot_loaded(completion.request, snapshot)
                .map(RecoveryOutcome::Reset),
            Err(error) if transient_recovery(&error) => self
                .snapshot_failed(completion.request)
                .map(RecoveryOutcome::Action),
            Err(error) => {
                self.close()?;
                Err(error)
            }
        }
    }
    pub fn complete_reset(
        &mut self,
        completion: ResetCompletion,
    ) -> Result<Option<RecoveryAction>, ClientError> {
        match completion.result {
            Ok(()) => {
                self.reset_applied(completion.work)?;
                Ok(None)
            }
            Err(_) => self.reset_failed(completion.work).map(Some),
        }
    }
    pub fn complete_delivery(
        &mut self,
        completion: DeliveryCompletion,
    ) -> Result<Option<RecoveryAction>, ClientError> {
        match completion.result {
            Ok(()) => {
                self.ack(completion.delivery)?;
                Ok(None)
            }
            Err(_) => self.consumer_failed(completion.delivery).map(Some),
        }
    }
    pub fn backoff(&self, jitter: u16) -> Result<Duration, ClientError> {
        self.backoff_for(self.attempts, jitter)
    }
    pub(crate) fn backoff_for(&self, attempts: u32, jitter: u16) -> Result<Duration, ClientError> {
        if jitter > 1000 {
            return Err(ClientError::InvalidInput);
        }
        let config = self.limits.config();
        let factor = 1u32
            .checked_shl(attempts.saturating_sub(1))
            .unwrap_or(u32::MAX);
        let base = config
            .backoff_min
            .saturating_mul(factor)
            .min(config.backoff_max);
        let millis = base
            .as_millis()
            .saturating_mul(800 + u128::from(jitter) * 400 / 1000)
            / 1000;
        Ok(Duration::from_millis(millis.clamp(
            config.backoff_min.as_millis(),
            config.backoff_max.as_millis(),
        ) as u64))
    }
}

pub enum RecoveryOutcome {
    Reset(Vec<ResetWork>),
    Action(RecoveryAction),
}
pub struct SnapshotCompletion {
    request: LoadRequest,
    result: Result<Snapshot, ClientError>,
}
pub struct ResetCompletion {
    work: ResetWork,
    result: Result<(), ClientError>,
}
pub struct DeliveryCompletion {
    delivery: Delivery,
    result: Result<(), ClientError>,
}
/// Owns every asynchronous callback. Dropping the wait aborts the task; explicit
/// cancel also waits for its destructor. No task may outlive its caller intentionally.
pub struct OwnedJob<T> {
    handle: tokio::task::JoinHandle<T>,
    deadline: Duration,
}
impl<T: Send + 'static> OwnedJob<T> {
    fn spawn(
        future: impl std::future::Future<Output = T> + Send + 'static,
        deadline: Duration,
    ) -> Self {
        Self {
            handle: tokio::spawn(future),
            deadline,
        }
    }
    pub async fn join(mut self) -> Result<T, ClientError> {
        match tokio::time::timeout(self.deadline, &mut self.handle).await {
            Ok(result) => result.map_err(|_| ClientError::Protocol),
            Err(_) => {
                self.handle.abort();
                let _ = tokio::time::timeout(self.deadline, &mut self.handle).await;
                Err(ClientError::Deadline)
            }
        }
    }
    pub async fn cancel(mut self) -> Result<(), ClientError> {
        self.handle.abort();
        match tokio::time::timeout(self.deadline, &mut self.handle).await {
            Ok(Err(error)) if error.is_cancelled() => Ok(()),
            Ok(Ok(_)) => Ok(()),
            Ok(Err(_)) => Err(ClientError::Protocol),
            Err(_) => Err(ClientError::Deadline),
        }
    }
}
impl<T> Drop for OwnedJob<T> {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn bounded_callback<T>(
    future: impl std::future::Future<Output = Result<T, ClientError>>,
    deadline: Duration,
) -> Result<T, ClientError> {
    use futures_util::FutureExt;
    match tokio::time::timeout(
        deadline,
        std::panic::AssertUnwindSafe(future).catch_unwind(),
    )
    .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(ClientError::Protocol),
        Err(_) => Err(ClientError::Deadline),
    }
}
pub fn spawn_snapshot(
    request: LoadRequest,
    mut source: Box<dyn SnapshotPort>,
    limits: &Limits,
) -> OwnedJob<SnapshotCompletion> {
    let deadline = limits.config().request_timeout;
    OwnedJob::spawn(
        async move {
            let result = bounded_callback(
                async { source.snapshot_after(&request.applied, &request.live).await },
                deadline,
            )
            .await;
            SnapshotCompletion { request, result }
        },
        deadline.saturating_add(deadline),
    )
}
pub fn spawn_reset(
    work: ResetWork,
    mut consumer: Box<dyn EventConsumer>,
    limits: &Limits,
) -> OwnedJob<(Box<dyn EventConsumer>, ResetCompletion)> {
    let deadline = limits.config().request_timeout;
    OwnedJob::spawn(
        async move {
            let result = bounded_callback(
                async { consumer.reset_from(&work.snapshot, &work.applied).await },
                deadline,
            )
            .await;
            (consumer, ResetCompletion { work, result })
        },
        deadline.saturating_add(deadline),
    )
}
pub fn spawn_delivery(
    delivery: Delivery,
    mut consumer: Box<dyn EventConsumer>,
    limits: &Limits,
) -> OwnedJob<(Box<dyn EventConsumer>, DeliveryCompletion)> {
    let deadline = limits.config().request_timeout;
    OwnedJob::spawn(
        async move {
            let result =
                bounded_callback(async { consumer.consume(&delivery.event).await }, deadline).await;
            (consumer, DeliveryCompletion { delivery, result })
        },
        deadline.saturating_add(deadline),
    )
}

fn transient_recovery(error: &ClientError) -> bool {
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
