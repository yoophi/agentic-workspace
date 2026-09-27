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
    /// 한 줄. 요청 id·operation은 클라이언트가 보낸 값(인증 전 포함)이라 따옴표로 감싸고 escape한다 — 개행·공백으로
    /// 기록 줄이나 필드를 위조하지 못한다.
    pub fn line(&self) -> String {
        format!(
            "requestId={:?} operation={:?} principal={} status={} latencyMs={}",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_supplied_values_cannot_forge_lines_or_fields() {
        let entry = AccessEntry {
            request_id: Some("req\nrequestId=forged status=200".into()),
            operation: "project.list principal=desktop".into(),
            principal_kind: None,
            status: 401,
            latency_ms: 0,
        };
        let line = entry.line();
        assert!(!line.contains('\n'), "{line}");
        assert_eq!(
            line.matches("status=").count(),
            2,
            "only inside the quoted value and the real field: {line}"
        );
        assert!(line.ends_with("status=401 latencyMs=0"), "{line}");
        assert!(
            line.contains(r#"requestId="req\nrequestId=forged status=200""#),
            "{line}"
        );
    }
}
