//! Replay port for bounded per-run runtime events.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequencedRuntimeEvent {
    pub run_id: String,
    pub sequence: u64,
    pub event: Value,
    pub terminal: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEventSnapshot {
    pub run_id: String,
    pub events: Vec<SequencedRuntimeEvent>,
    pub last_sequence: u64,
    pub terminal: bool,
    pub gap_detected: bool,
}

pub trait RuntimeEventJournal: Send + Sync {
    fn append(&self, run_id: &str, event: Value, terminal: bool) -> SequencedRuntimeEvent;

    fn replay(&self, run_id: &str, after_sequence: u64) -> RuntimeEventSnapshot;

    fn remove(&self, run_id: &str);
}
