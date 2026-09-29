//! Closed existing-instance activation policy. Server authorization still applies.
use workbench_protocol::OperationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerProfile {
    Owner,
    /// Trusted agent identity issuance is not supported by the current wire.
    AgentScoped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    StoredQuery,
    NonExecutingMutation,
    RuntimeQuery,
    PrerequisiteRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AdmissionError {
    #[error("operation is unsupported")]
    UnsupportedOperation,
    #[error("operation requires unavailable containment and lifecycle readiness proof")]
    PrerequisiteUnavailable,
    #[error("trusted agent endpoint identity is unavailable; owner fallback is forbidden")]
    AgentIdentityUnavailable,
}

/// Explicit names only: a newly added query is not automatically safe (Git queries spawn helpers).
pub fn classification(operation: OperationId) -> OperationClass {
    use OperationId::*;
    match operation {
        ProjectList | SavedPromptList | GoalGet | AgentRunSettingsGet => {
            OperationClass::StoredQuery
        }
        ProjectCreate | ProjectUpdate | ProjectDelete | BenchOpen | OrchestrationBootstrap => {
            OperationClass::NonExecutingMutation
        }
        SystemDescribe
        | ServerStatus
        | BenchList
        | OrchestrationGet
        | OrchestrationListTasks
        | OrchestrationCollectReports => OperationClass::RuntimeQuery,
        _ => OperationClass::PrerequisiteRequired,
    }
}

/// Both explicit CLI aliases and generic calls enter here. No caller-supplied readiness boolean.
pub fn admit(operation: &str, profile: CallerProfile) -> Result<OperationId, AdmissionError> {
    let operation = OperationId::parse(operation).ok_or(AdmissionError::UnsupportedOperation)?;
    if profile == CallerProfile::AgentScoped {
        return Err(AdmissionError::AgentIdentityUnavailable);
    }
    match classification(operation) {
        OperationClass::PrerequisiteRequired => Err(AdmissionError::PrerequisiteUnavailable),
        _ => Ok(operation),
    }
}

/// Watching existing non-executing workspace/bench state does not activate process operations.
pub fn admit_stream(stream: &str, profile: CallerProfile) -> Result<(), crate::ports::ClientError> {
    use workbench_protocol::events::{parse_stream_id, StreamKind};
    let (kind, _) = parse_stream_id(stream).ok_or(crate::ports::ClientError::InvalidInput)?;
    if profile != CallerProfile::Owner
        || !matches!(kind, StreamKind::Orchestration | StreamKind::Bench)
    {
        return Err(crate::ports::ClientError::PrerequisiteUnavailable);
    }
    Ok(())
}
