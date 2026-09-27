//! `desktop.*` operation(044): 데스크톱이 소유자 자격 증명으로 창 토큰을 받고 창을 폐기한다(research R6).
//! incarnation은 데스크톱이 발급하고, 서버는 폐기한 창 주체를 세대 동안 tombstone으로 기억한다(Codex 설계 리뷰 C4).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopIssueWindowTokenInput {
    pub label: String,
    pub incarnation: String,
    /// 창 WebView 출처. 허용 목록만.
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIssueWindowTokenOutput {
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopRetireWindowInput {
    pub label: String,
    pub incarnation: String,
    /// 사용자가 창을 닫았으면 true(그 창 주체가 연 작업대를 모두 닫는다). 앱 종료가 창을 걷어 내면 false.
    pub close_bench: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRetireWindowOutput {
    pub revoked_tokens: u64,
    pub closed_benches: Vec<String>,
}
