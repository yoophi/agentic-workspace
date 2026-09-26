//! `Workbench` 구현. 정본 `docs/client-server-architecture-research.md`의 1단계 첫 세로 slice(037).
//!
//! 계층은 AW Tauri 백엔드와 같은 hexagonal 구조를 따른다:
//! - `domain`: 순수 모델(`Project`, `ProjectError`) — Tauri·파일시스템·rusqlite 의존 없음
//! - `ports`: 저장소·ledger·lock 인터페이스와 시그니처 타입만
//! - `application`: `project_service`, authorization, idempotency, registry, `WorkbenchRuntime`
//! - `infrastructure`: JSON 저장, SQLite ledger, `StorageCoordinator`, `DataPaths`

// `WorkbenchFault`는 wire DTO라 Box로 감싸지 않는다. 동기 함수가 이를 Err로 돌려줄 때 나는 lint를 crate 단위로 끈다.
#![allow(clippy::result_large_err)]

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod ports;

pub use workbench_protocol as protocol;
