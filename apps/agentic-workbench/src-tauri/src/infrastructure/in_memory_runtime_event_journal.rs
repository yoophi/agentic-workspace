//! Bounded in-memory runtime event journal.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use serde_json::Value;

use crate::ports::runtime_event_journal::{
    RuntimeEventJournal, RuntimeEventSnapshot, SequencedRuntimeEvent,
};

const DEFAULT_RUNTIME_EVENT_CAPACITY: usize = 512;

#[derive(Debug, Default)]
struct RunJournal {
    last_sequence: u64,
    terminal: bool,
    events: VecDeque<SequencedRuntimeEvent>,
}

#[derive(Debug, Clone)]
pub struct InMemoryRuntimeEventJournal {
    capacity: usize,
    runs: Arc<Mutex<HashMap<String, RunJournal>>>,
}

impl Default for InMemoryRuntimeEventJournal {
    fn default() -> Self {
        Self::new(DEFAULT_RUNTIME_EVENT_CAPACITY)
    }
}

impl InMemoryRuntimeEventJournal {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RuntimeEventJournal for InMemoryRuntimeEventJournal {
    fn append(&self, run_id: &str, event: Value, terminal: bool) -> SequencedRuntimeEvent {
        let mut runs = self.runs.lock().expect("runtime journal lock");
        let journal = runs.entry(run_id.to_owned()).or_default();
        journal.last_sequence += 1;
        journal.terminal |= terminal;
        let sequenced = SequencedRuntimeEvent {
            run_id: run_id.to_owned(),
            sequence: journal.last_sequence,
            event,
            terminal,
        };
        journal.events.push_back(sequenced.clone());
        while journal.events.len() > self.capacity {
            journal.events.pop_front();
        }
        sequenced
    }

    fn replay(&self, run_id: &str, after_sequence: u64) -> RuntimeEventSnapshot {
        let runs = self.runs.lock().expect("runtime journal lock");
        let Some(journal) = runs.get(run_id) else {
            return RuntimeEventSnapshot {
                run_id: run_id.to_owned(),
                events: Vec::new(),
                last_sequence: 0,
                terminal: false,
                gap_detected: after_sequence > 0,
            };
        };
        let first_sequence = journal
            .events
            .front()
            .map(|event| event.sequence)
            .unwrap_or(journal.last_sequence.saturating_add(1));
        RuntimeEventSnapshot {
            run_id: run_id.to_owned(),
            events: journal
                .events
                .iter()
                .filter(|event| event.sequence > after_sequence)
                .cloned()
                .collect(),
            last_sequence: journal.last_sequence,
            terminal: journal.terminal,
            gap_detected: after_sequence.saturating_add(1) < first_sequence,
        }
    }

    fn remove(&self, run_id: &str) {
        self.runs
            .lock()
            .expect("runtime journal lock")
            .remove(run_id);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn replays_events_after_cursor_with_per_run_sequences() {
        let journal = InMemoryRuntimeEventJournal::new(4);
        journal.append("run-a", json!({"kind": "started"}), false);
        journal.append("run-b", json!({"kind": "started"}), false);
        journal.append("run-a", json!({"kind": "completed"}), true);

        let snapshot = journal.replay("run-a", 1);
        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.events[0].sequence, 2);
        assert!(snapshot.terminal);
        assert!(!snapshot.gap_detected);
    }

    #[test]
    fn reports_a_gap_when_bounded_events_were_evicted() {
        let journal = InMemoryRuntimeEventJournal::new(2);
        for index in 0..4 {
            journal.append("run-a", json!({"index": index}), false);
        }
        let snapshot = journal.replay("run-a", 0);
        assert_eq!(snapshot.events.len(), 2);
        assert_eq!(snapshot.events[0].sequence, 3);
        assert!(snapshot.gap_detected);
        assert_eq!(snapshot.last_sequence, 4);
    }

    /// FR-043: the default retention must be at least 512 events per run, because the spec
    /// promises that range is rehydratable after promotion.
    #[test]
    fn default_retention_is_the_specified_minimum() {
        assert_eq!(DEFAULT_RUNTIME_EVENT_CAPACITY, 512);
        assert_eq!(
            InMemoryRuntimeEventJournal::default().capacity,
            DEFAULT_RUNTIME_EVENT_CAPACITY
        );
    }

    /// FR-043 boundary: 511 and 512 events stay fully replayable with no gap, and the 513th
    /// event evicts the oldest one and must be reported as a gap rather than a clean replay.
    #[test]
    fn retains_up_to_the_bound_and_reports_a_gap_only_past_it() {
        for (appended, expected_len, expect_gap) in [(511usize, 511usize, false), (512, 512, false), (513, 512, true)]
        {
            let journal = InMemoryRuntimeEventJournal::default();
            for index in 0..appended {
                journal.append("run-a", json!({"index": index}), false);
            }

            let snapshot = journal.replay("run-a", 0);

            assert_eq!(
                snapshot.events.len(),
                expected_len,
                "{appended} appended events should replay {expected_len}"
            );
            assert_eq!(snapshot.last_sequence, appended as u64);
            assert_eq!(
                snapshot.gap_detected, expect_gap,
                "{appended} appended events should report gap_detected={expect_gap}"
            );
            assert_eq!(
                snapshot.events.first().map(|event| event.sequence),
                Some(appended as u64 - expected_len as u64 + 1)
            );
        }
    }

    /// FR-034/FR-043: "no events yet" must be distinguishable from "events were evicted".
    /// An empty run replayed from the start is not a gap; a cursor ahead of an unknown run is.
    #[test]
    fn separates_no_events_yet_from_an_evicted_range() {
        let journal = InMemoryRuntimeEventJournal::default();

        let untouched = journal.replay("run-missing", 0);
        assert!(untouched.events.is_empty());
        assert!(!untouched.gap_detected);
        assert_eq!(untouched.last_sequence, 0);
        assert!(!untouched.terminal);

        let ahead_of_unknown_run = journal.replay("run-missing", 7);
        assert!(ahead_of_unknown_run.events.is_empty());
        assert!(ahead_of_unknown_run.gap_detected);
    }
}
