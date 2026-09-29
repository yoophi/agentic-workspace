use workbench_client::application::admission::{
    admit, classification, AdmissionError, CallerProfile, OperationClass,
};
use workbench_protocol::{operations::OPERATIONS, OperationId};

#[test]
fn catalog_is_closed_and_every_operation_is_classified() {
    assert_eq!(
        OPERATIONS.len(),
        94,
        "review new catalog entries before activation"
    );
    for spec in OPERATIONS {
        let result = admit(spec.id.as_str(), CallerProfile::Owner);
        assert_eq!(
            result.is_ok(),
            classification(spec.id) != OperationClass::PrerequisiteRequired,
            "{}",
            spec.id.as_str()
        );
    }
    assert_eq!(
        admit("future.query", CallerProfile::Owner),
        Err(AdmissionError::UnsupportedOperation)
    );
}

#[test]
fn owner_nonexecuting_operations_are_available() {
    for op in [
        OperationId::ProjectList,
        OperationId::ProjectCreate,
        OperationId::ProjectUpdate,
        OperationId::ProjectDelete,
        OperationId::SystemDescribe,
        OperationId::ServerStatus,
        OperationId::BenchOpen,
        OperationId::BenchList,
        OperationId::OrchestrationBootstrap,
        OperationId::OrchestrationGet,
    ] {
        assert_eq!(admit(op.as_str(), CallerProfile::Owner), Ok(op));
    }
}

#[test]
fn missing_prerequisites_cannot_be_bypassed_by_generic_calls() {
    for op in [
        OperationId::RunStart,
        OperationId::RunCancel,
        OperationId::RunRespondPermission,
        OperationId::OrchestrationRecover,
        OperationId::OrchestrationDelegateGoal,
        OperationId::BenchClose,
        OperationId::GitListBranches,
        OperationId::AgentListProviderSessions,
        OperationId::ServerStop,
        OperationId::LeaseAcquire,
        OperationId::DesktopIssueWindowToken,
    ] {
        assert_eq!(
            admit(op.as_str(), CallerProfile::Owner),
            Err(AdmissionError::PrerequisiteUnavailable)
        );
    }
}

#[test]
fn agent_profile_never_falls_back_to_owner_even_for_reads() {
    for spec in OPERATIONS {
        assert_eq!(
            admit(spec.id.as_str(), CallerProfile::AgentScoped),
            Err(AdmissionError::AgentIdentityUnavailable)
        );
    }
}

#[test]
fn alias_and_generic_operations_share_one_admission_policy() {
    for op in [
        OperationId::ProjectList,
        OperationId::RunStart,
        OperationId::RunCancel,
        OperationId::ServerStatus,
    ] {
        let explicit = admit(op.as_str(), CallerProfile::Owner);
        let generic = admit(
            serde_json::to_value(op).unwrap().as_str().unwrap(),
            CallerProfile::Owner,
        );
        assert_eq!(explicit, generic);
    }
}
