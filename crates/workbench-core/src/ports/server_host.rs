//! 서버 조립이 제공하는 것(044 research R6, contracts/server-lifecycle.md §4). core는 HTTP 어댑터(`workbench-server`)에
//! 의존하지 않는다 — 창 토큰 발급기·이벤트 표·인스턴스 식별자는 조립(`workbench-host`)이 이 port로 넣는다.

use workbench_protocol::{AuthenticatedPrincipal, PrincipalSubject};

/// 발급한 창 토큰. `token`은 호출자에게 한 번만 건넨다.
#[derive(Debug, Clone)]
pub struct WindowToken {
    pub token: String,
    /// RFC 3339.
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WindowTokenError {
    /// 창 WebView 허용 출처가 아님.
    #[error("this window cannot connect to the Workbench server.")]
    OriginNotAllowed,
    /// 이 세대에 폐기된 창 주체(tombstone, Codex 설계 리뷰 C4).
    #[error("window is retired.")]
    Retired,
}

pub trait ServerHost: Send + Sync {
    /// 서버 인스턴스 식별자(안내 파일·신원 증명과 같은 값). embedded·시험 조립은 `None`.
    fn instance_id(&self) -> Option<String>;

    /// 창 주체·출처에 묶인 짧은 토큰. 폐기 tombstone 확인과 발급은 발급기의 같은 잠금 아래에서 한다.
    fn issue_window_token(
        &self,
        principal: AuthenticatedPrincipal,
        origin: &str,
    ) -> Result<WindowToken, WindowTokenError>;

    /// 창 주체의 토큰·이벤트 표를 폐기하고 tombstone을 세운다(세대 동안). 폐기한 토큰·표 수.
    fn retire_window(&self, subject: &PrincipalSubject) -> u64;

    /// 받아들인 분리 호출 수(HTTP·MCP). 활동 작업 표시용.
    fn accepted_calls(&self) -> u64;
}
