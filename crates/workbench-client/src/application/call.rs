//! Shared request policy and explicit attempt orchestration.
use crate::{
    application::admission::{admit, CallerProfile},
    domain::attempt::Attempt,
    ports::{CallTransport, ClientError},
};
use serde::de::DeserializeOwned;
use workbench_protocol::{
    operations::*, CallRequest, OperationId, OperationKind, PROTOCOL_VERSION,
};
fn input<T: DeserializeOwned>(value: &serde_json::Value) -> Result<(), ClientError> {
    serde_json::from_value::<T>(value.clone())
        .map(|_| ())
        .map_err(|_| ClientError::InvalidInput)
}
pub fn validate_request(request: &CallRequest) -> Result<OperationId, ClientError> {
    let operation = admit(&request.operation, CallerProfile::Owner)
        .map_err(|_| ClientError::PrerequisiteUnavailable)?;
    if request.protocol_version != PROTOCOL_VERSION
        || (spec_for(operation).kind == OperationKind::Command && request.idempotency_key.is_none())
        || request.timeout_ms == Some(0)
    {
        return Err(ClientError::InvalidInput);
    }
    use OperationId::*;
    match operation {
        ProjectList => input::<project::ProjectListInput>(&request.input),
        ProjectCreate => input::<project::ProjectCreateInput>(&request.input),
        ProjectUpdate => input::<project::ProjectUpdateInput>(&request.input),
        ProjectDelete => input::<project::ProjectDeleteInput>(&request.input),
        SavedPromptList => input::<saved_prompt::SavedPromptListInput>(&request.input),
        GoalGet => input::<goal::GoalGetInput>(&request.input),
        AgentRunSettingsGet => {
            input::<agent_run_settings::AgentRunSettingsGetInput>(&request.input)
        }
        SystemDescribe => input::<system::SystemDescribeInput>(&request.input),
        ServerStatus => input::<server::ServerStatusInput>(&request.input),
        BenchOpen => input::<bench::BenchOpenInput>(&request.input),
        RunCancel => input::<run::RunCancelInput>(&request.input),
        BenchList => input::<bench::BenchListInput>(&request.input),
        OrchestrationBootstrap => {
            input::<orchestration::OrchestrationBootstrapInput>(&request.input)
        }
        OrchestrationGet | OrchestrationCollectReports => {
            input::<orchestration::OrchestrationBenchInput>(&request.input)
        }
        OrchestrationListTasks => {
            input::<orchestration::OrchestrationListTasksInput>(&request.input)
        }
        _ => Err(ClientError::PrerequisiteUnavailable),
    }?;
    Ok(operation)
}
/// Owns the submitted generation even when the enclosing future is cancelled.
/// It does not infer server cancellation or NotApplied from local drop.
struct Submission<'a> {
    attempt: &'a mut Attempt,
    generation: crate::domain::attempt::Generation,
    resolved: bool,
}
impl Drop for Submission<'_> {
    fn drop(&mut self) {
        if !self.resolved {
            let _ = self.attempt.mark_unknown(self.generation);
        }
    }
}
pub async fn execute(
    transport: &mut dyn CallTransport,
    attempt: &mut Attempt,
) -> Result<(), ClientError> {
    validate_request(attempt.request())?;
    let generation = attempt
        .begin(transport.identity())
        .map_err(|_| ClientError::StaleGeneration)?;
    let mut submitted = Submission {
        attempt,
        generation,
        resolved: false,
    };
    match transport.call(submitted.attempt.request()).await {
        Ok(reply) => {
            submitted
                .attempt
                .complete(generation, reply)
                .map_err(|_| ClientError::StaleGeneration)?;
            submitted.resolved = true;
            Ok(())
        }
        Err(ClientError::Fault(fault)) => {
            submitted
                .attempt
                .fail(generation, fault.clone())
                .map_err(|_| ClientError::Protocol)?;
            submitted.resolved = true;
            Err(ClientError::Fault(fault))
        }
        Err(error) => Err(error),
    }
}
