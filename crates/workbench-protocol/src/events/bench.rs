//! `bench.titleRequested.v1` 본문(040, ADR 0007). 창 제목은 데스크톱 표현 상태라 서버는 요청만 알린다.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TitleRequestedDto {
    pub title: String,
}
