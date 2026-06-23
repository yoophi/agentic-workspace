use std::path::PathBuf;

use serde::Serialize;

/// 우리가 지원하는 ACP provider가 로컬에 남기는 네이티브 세션의 종류.
/// agent 카탈로그의 `agent_id`와 매핑된다.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderKind {
    Claude,
    Codex,
    Pi,
}

/// 카탈로그의 `agent_id`를 provider 종류로 매핑한다. 네이티브 세션 조회를
/// 지원하지 않는 provider(예: `opencode`)는 `None`을 돌려준다.
pub fn provider_kind_for(agent_id: &str) -> Option<ProviderKind> {
    match agent_id {
        "claude-code" => Some(ProviderKind::Claude),
        "codex" => Some(ProviderKind::Codex),
        "pi-coding-agent" => Some(ProviderKind::Pi),
        _ => None,
    }
}

/// provider가 디스크에 저장한 세션 하나를 요약한 메타데이터.
/// 프론트로 그대로 직렬화되므로 camelCase를 사용한다.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSession {
    pub agent_id: String,
    pub id: String,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub file: String,
    pub message_count: usize,
    /// RFC3339 문자열. 추출 실패 시 `None`.
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub model: Option<String>,
    pub branch: Option<String>,
    pub source: Option<String>,
}

/// 세션 조회 범위. 특정 작업 디렉터리(워크트리)로 한정하거나 전체를 본다.
#[derive(Clone, Debug)]
pub enum SessionScope {
    All,
    Path(PathBuf),
}
