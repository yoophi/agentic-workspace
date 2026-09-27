//! `lease.*` operation(044): 붙은 클라이언트의 서버 유지 요청. 임대가 없고 활동 작업도 없으면 유휴 시간 뒤 서버가 멈춘다
//! (research R9). 소유자 주체만 부른다.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum LeaseClientKindDto {
    Desktop,
    Cli,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseAcquireInput {
    pub client_kind: LeaseClientKindDto,
    /// 클라이언트가 정한 식별자(데스크톱: 앱 인스턴스 uuid).
    pub client_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaseAcquireOutput {
    pub lease_id: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseRenewInput {
    pub lease_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaseRenewOutput {
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseReleaseInput {
    pub lease_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LeaseReleaseOutput {
    /// 이 호출이 임대를 풀었으면 true(없던 임대도 성공, false).
    pub released: bool,
}
