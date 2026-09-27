//! Persistence port for orchestration workspace aggregates.
//!
//! 041(research R1): 모든 작업 영역이 한 저장 단위에 있으므로 읽기-수정-쓰기는 **저장소 전체 경계** 하나로
//! 직렬화되어야 한다. 이 포트는 그 계약만 정의한다 — 쓰기는 `begin`이 연 transaction 안에서만 하고 `commit`으로
//! 끝내며(commit 없이 버리면 저장하지 않는다), 읽기는 `snapshot`(경계 안의 일관된 사본)으로 한다.
//! transaction은 경계를 쥐고 있으므로 await를 사이에 두고 들고 있으면 안 된다(구현은 `Send`가 아니다).
//! 경계의 구현(lock·파일 입출력)은 `infrastructure::orchestration`에 있다.

use crate::domain::agent_orchestration::{OrchestrationError, OrchestrationSession};

/// 저장소 전체 경계를 쥔 쓰기 transaction.
pub trait OrchestrationTransaction {
    /// 경계 안에서 읽은 전체 작업 영역. 수정은 `commit` 때 한 번에 저장된다.
    fn sessions(&mut self) -> &mut Vec<OrchestrationSession>;

    /// 수정한 전체를 저장하고 경계를 푼다.
    fn commit(self) -> Result<(), OrchestrationError>;
}

pub trait OrchestrationRepository: Send + Sync {
    type Tx<'a>: OrchestrationTransaction
    where
        Self: 'a;

    /// 경계를 잡고 전체를 읽은 transaction을 연다. 같은 저장 단위를 가리키는 모든 인스턴스는 같은 경계를 쓴다.
    fn begin(&self) -> Result<Self::Tx<'_>, OrchestrationError>;

    /// 경계 안에서 한 번 읽은 일관된 사본.
    fn snapshot(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError>;
}
