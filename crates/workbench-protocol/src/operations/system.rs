//! `system.*` operation의 input 타입. output은 `descriptor::DescribeOutput`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// `system.describe` input. 필드가 없고 추가 필드는 거절한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SystemDescribeInput {}
