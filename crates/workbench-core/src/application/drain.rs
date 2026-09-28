//! 비우기(drain) 입구 분류(044, research R7·R14, `specs/044-standalone-server/contracts/drain-classification.md`).
//!
//! `draining` 상태의 호출 입구는 operation마다 네 가지로 처리한다. 분류는 **빠짐없는 match**라, 새 operation을 추가하면
//! 분류하지 않고는 컴파일되지 않는다. 문서 표와 이 match는 `tests/drain_classification.rs`가 대조한다.

use workbench_protocol::{FaultCode, OperationId, RequestId, WorkbenchFault};

use crate::application::work_gate::{GateState, WorkGate};

pub const MESSAGE_DRAINING: &str = "server is draining; new work is not accepted.";
pub const MESSAGE_STOPPING: &str = "server is stopping; no new calls are accepted.";

/// 비우기 중 새 작업 거절(`draining`, 적용 안 됨).
pub fn draining_fault(request_id: &RequestId) -> WorkbenchFault {
    WorkbenchFault::new(FaultCode::Draining, request_id.clone(), MESSAGE_DRAINING)
}

/// 관문이 비우기 중인가(`stopping`은 서버가 503으로 막는다).
pub fn is_draining(gate: Option<&std::sync::Arc<WorkGate>>) -> bool {
    gate.is_some_and(|gate| matches!(gate.state(), GateState::Draining(_)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainClass {
    /// 조회. 받는다.
    Query,
    /// 끝내는·해제하는 제어. 받는다(활동 작업을 줄이거나 이미 있는 작업을 마무리).
    Control,
    /// 이어 가기(조건부). 입력이 이미 있는 대기 항목을 가리키고 서버가 확인하면 받는다. 아니면 새 작업과 같다.
    /// 조건 판정은 입구가 operation별로 한다(교환 전달 T037, 대기 task 배정 T038).
    Continuation,
    /// 새 작업·새 데이터 변경. `draining` fault(`notApplied`)로 거절한다.
    NewWork,
}

/// operation의 정적 분류. `Continuation`은 "조건을 입구가 확인한다"는 뜻이다.
pub fn drain_class(operation: OperationId) -> DrainClass {
    match operation {
        OperationId::AgentList
        | OperationId::AgentListProviderSessions
        | OperationId::AgentRunSettingsGet
        | OperationId::BenchList
        | OperationId::ExchangeGetForRun
        | OperationId::ExchangeList
        | OperationId::ExchangeListPeers
        | OperationId::GitListBranches
        | OperationId::GitListRemotes
        | OperationId::GitListWorktrees
        | OperationId::GoalGet
        | OperationId::OrchestrationCollectReports
        | OperationId::OrchestrationGet
        | OperationId::OrchestrationGetAgentRole
        | OperationId::OrchestrationGetOwnTask
        | OperationId::OrchestrationListChildTasks
        | OperationId::OrchestrationListRecoverable
        | OperationId::OrchestrationListTasks
        | OperationId::OrchestrationWaitChildTasks
        | OperationId::ProjectList
        | OperationId::RunListToolCandidates
        | OperationId::RunReplay
        | OperationId::SavedPromptList
        | OperationId::ServerStatus
        | OperationId::SystemDescribe
        | OperationId::WorktreeGetChanges
        | OperationId::WorktreeGetCommitDetail
        | OperationId::WorktreeGetCommitFileDiff
        | OperationId::WorktreeGetFileDiff
        | OperationId::WorktreeGetGraph
        | OperationId::WorktreeListChanges
        | OperationId::WorktreeListFiles
        | OperationId::WorktreeListHistory
        | OperationId::WorktreeReadTextFile => DrainClass::Query,
        OperationId::BenchClose
        | OperationId::BenchRequestTitle
        | OperationId::DesktopIssueWindowToken
        | OperationId::DesktopRetireWindow
        | OperationId::ExchangeAcknowledge
        | OperationId::ExchangeDiscardDelivery
        | OperationId::GoalRecordProgress
        | OperationId::LeaseAcquire
        | OperationId::LeaseRelease
        | OperationId::LeaseRenew
        | OperationId::OrchestrationCancelChildTask
        | OperationId::OrchestrationCancelTask
        | OperationId::OrchestrationCollectChildResults
        | OperationId::OrchestrationInterruptChildTask
        | OperationId::OrchestrationReportBlocked
        | OperationId::OrchestrationReportProgress
        | OperationId::OrchestrationReportResult
        | OperationId::OrchestrationRequestParentInput
        | OperationId::OrchestrationRespondInput
        | OperationId::OrchestrationSendParentMessage
        | OperationId::OrchestrationSetPresentation
        | OperationId::RunCancel
        | OperationId::RunRespondPermission
        | OperationId::RunSetPermissionMode
        | OperationId::ServerStop => DrainClass::Control,
        OperationId::OrchestrationAssignChildTask | OperationId::RunSendPrompt => {
            DrainClass::Continuation
        }
        OperationId::AgentRunSettingsSave
        | OperationId::BenchOpen
        | OperationId::ExchangeSend
        | OperationId::ExchangeSendFromRun
        | OperationId::ExchangeSyncWorkspace
        | OperationId::GitCreateWorktree
        | OperationId::GitDeleteWorktree
        | OperationId::GoalClear
        | OperationId::GoalCreate
        | OperationId::GoalUpdate
        | OperationId::OrchestrationAdoptManualChild
        | OperationId::OrchestrationBindCoordinator
        | OperationId::OrchestrationBootstrap
        | OperationId::OrchestrationCreateChildTask
        | OperationId::OrchestrationDelegateGoal
        | OperationId::OrchestrationDispatchPrompt
        | OperationId::OrchestrationHandoffCoordinator
        | OperationId::OrchestrationReassignChildTask
        | OperationId::OrchestrationReassignTask
        | OperationId::OrchestrationRecover
        | OperationId::OrchestrationRetryChildTask
        | OperationId::OrchestrationRetryTask
        | OperationId::OrchestrationSendChildCommand
        | OperationId::OrchestrationSendChildMessage
        | OperationId::ProjectCreate
        | OperationId::ProjectDelete
        | OperationId::ProjectUpdate
        | OperationId::RunCancelAndSend
        | OperationId::RunStart
        | OperationId::RunSteer
        | OperationId::SavedPromptCreate
        | OperationId::SavedPromptDelete
        | OperationId::SavedPromptUpdate => DrainClass::NewWork,
    }
}
