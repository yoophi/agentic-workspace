//! Per-caller event state. Receipt and applied positions have independent lifetimes.
use crate::{
    domain::limits::{Limits, Resource},
    ports::ClientError,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use workbench_protocol::{
    events::{parse_stream_id, EVENT_SCHEMAS},
    workbench::{EventEnvelope, StreamCursor},
};
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ConsumerId {
    owner: u64,
    ordinal: u64,
}
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
#[derive(Clone)]
struct Pending {
    event: Arc<EventEnvelope>,
    bytes: usize,
}
struct Consumer {
    generation: u64,
    applied: u64,
    queue: VecDeque<Pending>,
    inflight: Option<u64>,
    resetting: bool,
}
pub struct Delivery {
    pub event: Arc<EventEnvelope>,
    owner: Arc<()>,
    consumer: ConsumerId,
    consumer_generation: u64,
    stream_generation: u64,
}
impl std::fmt::Debug for Delivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Delivery([redacted])")
    }
}
pub struct Reset {
    owner: Arc<()>,
    consumer: ConsumerId,
    consumer_generation: u64,
    stream_generation: u64,
}
impl std::fmt::Debug for Reset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Reset([redacted])")
    }
}
pub struct EventReducer {
    owner_id: u64,
    owner: Arc<()>,
    cursor: StreamCursor,
    received: u64,
    generation: u64,
    next_consumer: u64,
    consumers: BTreeMap<ConsumerId, Consumer>,
    backlog: VecDeque<Pending>,
    limits: Limits,
    replay: bool,
}
impl EventReducer {
    pub fn new(cursor: StreamCursor, limits: Limits) -> Result<Self, ClientError> {
        validate_cursor(&cursor)?;
        let received = cursor.after_sequence;
        let owner_id = NEXT_OWNER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| ClientError::StaleGeneration)?;
        Ok(Self {
            owner_id,
            owner: Arc::new(()),
            cursor,
            received,
            generation: 1,
            next_consumer: 1,
            consumers: BTreeMap::new(),
            backlog: VecDeque::new(),
            limits,
            replay: false,
        })
    }
    pub fn snapshot_on_reconnect(&self) -> bool {
        parse_stream_id(&self.cursor.stream_id).is_some_and(|(kind, _)| {
            kind.class() == workbench_protocol::events::EventClass::Notification
        })
    }
    pub fn received(&self) -> u64 {
        self.received
    }
    pub fn cursor(&self) -> StreamCursor {
        let after = self
            .consumers
            .values()
            .map(|s| s.applied)
            .min()
            .unwrap_or(self.cursor.after_sequence);
        StreamCursor {
            after_sequence: after,
            ..self.cursor.clone()
        }
    }
    pub fn requires_replay(&self) -> bool {
        self.replay
    }
    pub fn take_replay_requirement(&mut self) -> bool {
        std::mem::take(&mut self.replay)
    }
    pub fn register(&mut self, after: u64) -> Result<ConsumerId, ClientError> {
        self.limits
            .check_add(Resource::QueueItems, self.consumers.len(), 1)?;
        let id = ConsumerId {
            owner: self.owner_id,
            ordinal: self.next_consumer,
        };
        let next = self
            .next_consumer
            .checked_add(1)
            .ok_or(ClientError::StaleGeneration)?;
        let queue: VecDeque<_> = self
            .backlog
            .iter()
            .filter(|p| p.event.sequence > after)
            .cloned()
            .collect();
        let first = self.consumers.is_empty();
        if !first {
            let (items, bytes) = self.usage();
            self.limits
                .check_add(Resource::QueueItems, items, queue.len())?;
            self.limits.check_add(
                Resource::QueueBytes,
                bytes,
                queue.iter().map(|p| p.bytes).sum(),
            )?;
        }
        self.next_consumer = next;
        self.replay |= after < self.received
            && queue
                .front()
                .is_none_or(|p| p.event.sequence > after.saturating_add(1));
        self.consumers.insert(
            id,
            Consumer {
                generation: 1,
                applied: after,
                queue,
                inflight: None,
                resetting: false,
            },
        );
        if first {
            self.backlog.clear();
        }
        Ok(id)
    }
    pub fn unregister(&mut self, id: ConsumerId) -> Result<(), ClientError> {
        let before = self.cursor().after_sequence;
        let removed = self
            .consumers
            .remove(&id)
            .ok_or(ClientError::StaleGeneration)?;
        if self.consumers.is_empty() {
            self.cursor.after_sequence = before;
            self.backlog = removed
                .queue
                .into_iter()
                .filter(|p| p.event.sequence > before)
                .collect();
        }
        Ok(())
    }
    fn usage(&self) -> (usize, usize) {
        let backlog = self.backlog.iter();
        let consumers = self.consumers.values().flat_map(|s| s.queue.iter());
        backlog
            .chain(consumers)
            .fold((0, 0), |(n, b), p| (n + 1, b + p.bytes))
    }
    pub fn receive(&mut self, event: EventEnvelope) -> Result<(), ClientError> {
        let (kind, _) = parse_stream_id(&event.stream_id).ok_or(ClientError::Protocol)?;
        if event.stream_id != self.cursor.stream_id
            || event.epoch != self.cursor.epoch
            || event.sequence == 0
            || event.event_id.is_empty()
            || !EVENT_SCHEMAS
                .iter()
                .any(|s| s.stream_kind == kind && s.schema == event.schema)
        {
            return Err(ClientError::Protocol);
        }
        let bytes = serde_json::to_vec(&event)
            .map_err(|_| ClientError::Protocol)?
            .len();
        self.limits.check_add(Resource::Message, 0, bytes)?;
        let pending = Pending {
            event: Arc::new(event),
            bytes,
        };
        let seq = pending.event.sequence;
        let destinations = if self.consumers.is_empty() {
            usize::from(should_queue(&self.backlog, self.cursor.after_sequence, seq))
        } else {
            self.consumers
                .values()
                .filter(|s| should_queue(&s.queue, s.applied, seq))
                .count()
        };
        let (items, retained) = self.usage();
        self.limits
            .check_add(Resource::QueueItems, items, destinations)?;
        self.limits.check_add(
            Resource::QueueBytes,
            retained,
            bytes
                .checked_mul(destinations)
                .ok_or(ClientError::Protocol)?,
        )?;
        // All checks precede any consumer mutation: overflow does not partially enqueue.
        self.received = self.received.max(seq);
        if self.consumers.is_empty() {
            if destinations != 0 {
                enqueue_ordered(&mut self.backlog, pending);
            }
        } else {
            for s in self.consumers.values_mut() {
                if should_queue(&s.queue, s.applied, seq) {
                    enqueue_ordered(&mut s.queue, pending.clone());
                }
            }
        }
        Ok(())
    }
    pub fn next(&mut self, id: ConsumerId) -> Result<Option<Delivery>, ClientError> {
        let retained = !self.snapshot_on_reconnect();
        let s = self
            .consumers
            .get_mut(&id)
            .ok_or(ClientError::StaleGeneration)?;
        if s.inflight.is_some() || s.resetting {
            return Ok(None);
        }
        let Some(pending) = s.queue.front() else {
            return Ok(None);
        };
        if retained && s.applied.checked_add(1) != Some(pending.event.sequence) {
            return Ok(None); // live ahead of replay: never acknowledge across a hole
        }
        s.inflight = Some(pending.event.sequence);
        Ok(Some(Delivery {
            owner: self.owner.clone(),
            event: pending.event.clone(),
            consumer: id,
            consumer_generation: s.generation,
            stream_generation: self.generation,
        }))
    }
    fn check_delivery(&self, d: &Delivery) -> Result<(), ClientError> {
        let s = self
            .consumers
            .get(&d.consumer)
            .ok_or(ClientError::StaleGeneration)?;
        if !Arc::ptr_eq(&d.owner, &self.owner)
            || d.stream_generation != self.generation
            || d.consumer_generation != s.generation
            || s.inflight != Some(d.event.sequence)
        {
            return Err(ClientError::StaleGeneration);
        }
        Ok(())
    }
    pub fn ack(&mut self, d: Delivery) -> Result<(), ClientError> {
        self.check_delivery(&d)?;
        let s = self
            .consumers
            .get_mut(&d.consumer)
            .ok_or(ClientError::StaleGeneration)?;
        s.queue.pop_front();
        s.applied = s.applied.max(d.event.sequence);
        s.inflight = None;
        Ok(())
    }
    pub fn fail(&mut self, d: Delivery) -> Result<Reset, ClientError> {
        self.check_delivery(&d)?;
        self.begin_reset(d.consumer)
    }
    pub fn begin_reset(&mut self, id: ConsumerId) -> Result<Reset, ClientError> {
        let s = self
            .consumers
            .get_mut(&id)
            .ok_or(ClientError::StaleGeneration)?;
        s.generation = s
            .generation
            .checked_add(1)
            .ok_or(ClientError::StaleGeneration)?;
        s.inflight = None;
        s.resetting = true;
        Ok(Reset {
            owner: self.owner.clone(),
            consumer: id,
            consumer_generation: s.generation,
            stream_generation: self.generation,
        })
    }
    pub fn finish_reset(&mut self, r: Reset, snapshot: &StreamCursor) -> Result<(), ClientError> {
        let s = self
            .consumers
            .get_mut(&r.consumer)
            .ok_or(ClientError::StaleGeneration)?;
        if !Arc::ptr_eq(&r.owner, &self.owner)
            || r.stream_generation != self.generation
            || r.consumer_generation != s.generation
        {
            return Err(ClientError::StaleGeneration);
        }
        if snapshot.stream_id != self.cursor.stream_id || snapshot.epoch != self.cursor.epoch {
            return Err(ClientError::Protocol);
        }
        s.applied = s.applied.max(snapshot.after_sequence);
        s.queue.retain(|p| p.event.sequence > s.applied);
        s.resetting = false;
        s.inflight = None;
        Ok(())
    }
    pub fn rebind(&mut self, cursor: StreamCursor) -> Result<(), ClientError> {
        validate_cursor(&cursor)?;
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(ClientError::StaleGeneration)?;
        if self.consumers.values().any(|s| s.generation == u64::MAX) {
            return Err(ClientError::StaleGeneration);
        }
        self.generation = generation;
        self.received = cursor.after_sequence;
        self.cursor = cursor;
        self.backlog.clear();
        self.replay = false;
        for s in self.consumers.values_mut() {
            s.generation += 1;
            s.applied = self.cursor.after_sequence;
            s.queue.clear();
            s.inflight = None;
            s.resetting = false;
        }
        Ok(())
    }
}
fn validate_cursor(cursor: &StreamCursor) -> Result<(), ClientError> {
    if parse_stream_id(&cursor.stream_id).is_none() || cursor.epoch.is_empty() {
        return Err(ClientError::InvalidInput);
    }
    Ok(())
}

fn should_queue(queue: &VecDeque<Pending>, applied: u64, sequence: u64) -> bool {
    sequence > applied && !queue.iter().any(|p| p.event.sequence == sequence)
}
fn enqueue_ordered(queue: &mut VecDeque<Pending>, pending: Pending) {
    let position = queue
        .iter()
        .position(|p| p.event.sequence > pending.event.sequence)
        .unwrap_or(queue.len());
    queue.insert(position, pending);
}
