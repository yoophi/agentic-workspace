//! `exchange.requested.v1`·`exchange.status.v1` 본문(040). 요청 본문은 core `AgentExchangeRequestedEvent`의 미러,
//! 상태 본문은 `operations::exchange::AgentExchangeDto`다.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::operations::exchange::{AgentExchangeDeliveryDto, AgentExchangeEndpointRefDto};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRequestedDto {
    pub request_id: String,
    pub source: AgentExchangeEndpointRefDto,
    pub target: AgentExchangeEndpointRefDto,
    pub message: String,
    pub delivery: AgentExchangeDeliveryDto,
    pub created_at: String,
}
