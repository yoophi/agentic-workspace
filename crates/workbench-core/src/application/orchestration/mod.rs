//! orchestration(041): AW에서 옮긴 도메인 서비스. 작업 영역 ↔ 작업대 묶임·역할·저장 경계는 이 모듈이 소유한다.

pub mod binding;
pub mod command_service;
pub mod notification_dispatcher;
pub mod runtime;
pub mod scheduler;
pub mod service;
