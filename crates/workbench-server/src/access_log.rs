//! 접근 기록(042 research R9, contracts §6). 요청 id·operation·주체 종류·상태·지연만 남긴다 — URI query(구독 표),
//! 헤더(bearer), 본문(prompt)은 기록하지 않는다.

use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessEntry {
    pub request_id: Option<String>,
    /// `POST /v1/calls`면 operation, 그 외는 경로 이름(`handshake`, `events` 등).
    pub operation: String,
    pub principal_kind: Option<&'static str>,
    pub status: u16,
    pub latency_ms: u128,
}

impl AccessEntry {
    pub fn line(&self) -> String {
        format!(
            "requestId={} operation={} principal={} status={} latencyMs={}",
            self.request_id.as_deref().unwrap_or("-"),
            self.operation,
            self.principal_kind.unwrap_or("-"),
            self.status,
            self.latency_ms
        )
    }
}

pub trait AccessLog: Send + Sync {
    fn record(&self, entry: &AccessEntry);
}

/// stderr 한 줄(`[workbench-http] …`).
#[derive(Debug, Default, Clone, Copy)]
pub struct StderrAccessLog;

impl AccessLog for StderrAccessLog {
    fn record(&self, entry: &AccessEntry) {
        eprintln!("[workbench-http] {}", entry.line());
    }
}

/// 시험용 수집기.
#[derive(Debug, Default)]
pub struct CollectingAccessLog {
    lines: Mutex<Vec<String>>,
}

impl CollectingAccessLog {
    pub fn lines(&self) -> Vec<String> {
        crate::auth::lock(&self.lines).clone()
    }
}

impl AccessLog for CollectingAccessLog {
    fn record(&self, entry: &AccessEntry) {
        crate::auth::lock(&self.lines).push(entry.line());
    }
}
