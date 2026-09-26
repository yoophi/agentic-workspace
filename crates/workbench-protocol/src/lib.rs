//! Agentic Workbench 서버-클라이언트 Seam의 wire 계약.
//!
//! 이 crate는 Tauri·저장·프로세스에 의존하지 않는다. `Workbench` trait의 두 동작(`call`, `events`)과
//! 그 위를 오가는 요청·응답·오류 타입, operation descriptor, OpenAPI 문서 조립만 담는다.
//! 구현은 `workbench-core`에 있다. 설계 근거: `docs/client-server-architecture-research.md`,
//! `specs/037-workbench-seam/`.

// `WorkbenchFault`는 wire DTO라 Box로 감싸지 않는다. 동기 함수가 이를 Err로 돌려줄 때 나는 lint를 crate 단위로 끈다.
#![allow(clippy::result_large_err)]

pub mod call;
pub mod descriptor;
pub mod fault;
pub mod openapi;
pub mod operations;
pub mod principal;
pub mod workbench;

pub use call::{
    CallReply, CallRequest, IdempotencyKey, InvalidIdentifier, OperationId, RequestId,
    CONTRACT_REVISION, PROTOCOL_VERSION,
};
pub use descriptor::{DescribeOutput, Effect, OperationDescriptor, OperationKind};
pub use fault::{FaultCode, Outcome, WorkbenchFault};
pub use principal::{AuthenticatedPrincipal, PrincipalKind, Scope};
pub use workbench::{EventEnvelope, EventStream, StreamCursor, Subscription, Workbench};
