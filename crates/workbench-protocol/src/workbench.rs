//! 외부 Seam: `Workbench` trait. 모든 inbound Adapter(Tauri compat, HTTP, in-memory)는 이 두 동작만 안다.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    call::{CallReply, CallRequest, RequestId},
    fault::WorkbenchFault,
    principal::AuthenticatedPrincipal,
};

/// stream별 재연결 cursor. 정본 §WebSocket, replay 계약. 037은 시그니처만 둔다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StreamCursor {
    pub stream_id: String,
    pub epoch: String,
    pub after_sequence: u64,
}

/// 이벤트 구독 요청. 037은 `events`가 항상 `unsupportedSchema`를 돌려준다.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    #[serde(default)]
    pub cursors: Vec<StreamCursor>,
}

/// 공통 이벤트 봉투. 2단계에서 채운다.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub event_id: String,
    pub stream_id: String,
    pub epoch: String,
    pub sequence: u64,
    pub schema: String,
    pub occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<RequestId>,
    #[schema(value_type = Value)]
    pub body: serde_json::Value,
}

/// 이벤트 스트림 핸들. 2단계에서 실제 stream 타입으로 바뀐다.
#[derive(Debug)]
pub struct EventStream {
    _private: (),
}

/// Seam. `call`은 유한 명령·조회, `events`는 순서 있는 이벤트와 replay.
#[async_trait]
pub trait Workbench: Send + Sync {
    async fn call(
        &self,
        principal: AuthenticatedPrincipal,
        request: CallRequest,
    ) -> Result<CallReply, WorkbenchFault>;

    fn events(
        &self,
        principal: AuthenticatedPrincipal,
        request: Subscription,
    ) -> Result<EventStream, WorkbenchFault>;
}

impl WorkbenchFault {
    /// 037의 `events` 응답. 2단계 이벤트 통합 전까지 구독을 제공하지 않는다.
    pub fn events_unsupported(request_id: RequestId) -> Self {
        Self::unsupported_schema(request_id, "이벤트 구독은 2단계에서 제공됩니다.")
    }
}
