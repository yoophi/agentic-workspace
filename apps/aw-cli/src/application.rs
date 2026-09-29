//! CLI use cases. All mutations publish private identity before the first call.
use crate::{
    inbound::{read_input, request, Command, Input, Options, WatchInput},
    infrastructure::output::{success, CliError},
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use workbench_client::{
    application::{
        admission::{admit, CallerProfile},
        call::{execute, publish_attempt},
    },
    domain::attempt::{Attempt, AttemptState},
    infrastructure::{
        http::HttpConnection, locator::read_descriptor, retry_store::PrivateRetryStore,
    },
    ports::{CallTransport, ClientError, RetryRecord, RetryStore},
};
use workbench_protocol::{OperationId, Outcome, RequestId};
pub struct Receipt {
    pub state: String,
    pub request_id: RequestId,
    pub outcome: Outcome,
}
pub type ReceiptSlot = Arc<Mutex<Option<Receipt>>>;
fn catalog(operation: Option<OperationId>) -> Value {
    let values:Vec<_>=workbench_protocol::operations::OPERATIONS.iter().filter(|s|operation.is_none_or(|id|id==s.id)).map(|spec|{
  let (input_schema,output_schema)=workbench_protocol::operations::schema_for(spec.id);
  json!({"id":spec.id.as_str(),"kind":spec.kind,"effect":spec.effect,"idempotent":spec.idempotent,"idempotencyScope":spec.idempotency_scope,"requiredScopes":spec.required_scopes,"inputSchema":input_schema,"outputSchema":output_schema})
 }).collect();
    json!(values)
}
pub async fn run(options: Options, receipt: ReceiptSlot) -> Result<Value, CliError> {
    if let Command::Operations(operation) = options.command {
        return Ok(json!({"ok":true,"data":catalog(operation),"requestId":RequestId::random()}));
    }
    let mut store = None;
    let mut state_path = None;
    let mut generation = 1;
    let mut deferred_cancel = None;
    let mut prepared_attempt = None;
    let mut call = match &options.command {
        Command::Call {
            operation,
            input,
            key,
            revision,
        } => {
            if let Some(path) = &options.retry_state {
                let opened = PrivateRetryStore::open(path, options.limits.clone())
                    .map_err(CliError::from_client)?;
                let restored = opened.load().map_err(CliError::from_client)?;
                *receipt
                    .lock()
                    .map_err(|_| CliError::new("internal", 1, Outcome::Unknown, false))? =
                    Some(Receipt {
                        state: path.to_str().ok_or_else(CliError::usage)?.to_owned(),
                        request_id: restored.request.request_id.clone(),
                        outcome: restored.outcome,
                    });
                if operation.is_some_and(|id| id.as_str() != restored.request.operation) {
                    return Err(CliError::usage());
                }
                if let Some(result) = &restored.result {
                    let retryable = matches!(result,Err(fault) if workbench_client::domain::attempt::fault_allows_explicit_retry(fault));
                    if !retryable {
                        return result
                            .clone()
                            .map(|reply| success(reply, restored.request.request_id))
                            .map_err(CliError::fault);
                    }
                }
                state_path = Some(path.clone());
                generation = restored.generation;
                store = Some(opened);
                restored.request
            } else {
                let operation = operation.ok_or_else(CliError::usage)?;
                admit(operation.as_str(), CallerProfile::Owner)
                    .map_err(|_| CliError::from_client(ClientError::PrerequisiteUnavailable))?;
                match input {
                    Some(Input::Cancel(run_id)) => {
                        deferred_cancel = Some((run_id.clone(), key.clone(), *revision));
                        workbench_protocol::CallRequest::command(operation, json!({}))
                    }
                    source => {
                        let input = match source {
                            Some(Input::Source(source)) => {
                                read_input(source, &options.limits).await?
                            }
                            None => json!({}),
                            _ => unreachable!(),
                        };
                        request(operation, input, key.clone(), *revision)?
                    }
                }
            }
        }
        Command::Watch { input, run, after } => {
            if run.is_none() {
                let source = input.as_deref().ok_or_else(CliError::usage)?;
                let value = read_input(source, &options.limits).await?;
                let watch: WatchInput =
                    serde_json::from_value(value).map_err(|_| CliError::usage())?;
                if workbench_protocol::events::parse_stream_id(&watch.stream_id).is_none()
                    || watch.epoch.is_empty()
                {
                    return Err(CliError::usage());
                }
                let _cursor = workbench_protocol::workbench::StreamCursor {
                    stream_id: watch.stream_id,
                    epoch: watch.epoch,
                    after_sequence: watch.after_sequence,
                };
            } else {
                let _after_sequence = *after;
            }
            return Err(CliError::from_client(ClientError::PrerequisiteUnavailable));
        }
        Command::Operations(_) => unreachable!(),
    };
    let descriptor = options
        .descriptor
        .as_ref()
        .ok_or_else(|| CliError::from_client(ClientError::Unavailable))?;
    let endpoint =
        Arc::new(read_descriptor(descriptor, CallerProfile::Owner).map_err(CliError::from_client)?);
    // Retry identity is checked before credentials or a command can be transmitted.
    if let Some(opened) = &store {
        let restored = opened.load().map_err(CliError::from_client)?;
        if restored.endpoint != *endpoint.identity() {
            let mut error = CliError::from_client(ClientError::Incompatible);
            error.outcome = restored.outcome;
            return Err(error);
        }
    }
    let mut connection = HttpConnection::connect(endpoint.clone(), options.limits.clone())
        .await
        .map_err(|error| {
            let mut error = CliError::from_client(error);
            if store.is_none() {
                error.outcome = Outcome::NotApplied;
            }
            error
        })?;
    // This projection is reachable only after RunCancel admission succeeds. The
    // current gate rejects it before lookup; no readiness flag or fallback exists.
    if let Some((run_id, key, revision)) = deferred_cancel {
        let reply = connection
            .call(&workbench_protocol::CallRequest::query(
                OperationId::BenchList,
                json!({}),
            ))
            .await
            .map_err(CliError::from_client)?;
        let benches: Vec<workbench_protocol::operations::bench::BenchSummaryDto> =
            serde_json::from_value(
                reply
                    .output()
                    .cloned()
                    .ok_or_else(|| CliError::from_client(ClientError::Protocol))?,
            )
            .map_err(|_| CliError::from_client(ClientError::Protocol))?;
        let owners: Vec<_> = benches
            .iter()
            .filter(|bench| bench.runs.iter().any(|run| run.run_id == run_id))
            .collect();
        let bench = match owners.as_slice() {
            [bench] => *bench,
            [] => return Err(CliError::new("notFound", 4, Outcome::NotApplied, false)),
            _ => return Err(CliError::new("conflict", 5, Outcome::NotApplied, false)),
        };
        call = request(
            OperationId::RunCancel,
            crate::inbound::cancel_input(&run_id, &bench.bench_id),
            key,
            revision,
        )?;
    }
    if workbench_protocol::operations::spec_for(
        OperationId::parse(&call.operation).ok_or_else(CliError::usage)?,
    )
    .kind
        == workbench_protocol::OperationKind::Command
    {
        if let Some(opened) = &store {
            let advanced = opened
                .begin_retry(generation, connection.identity())
                .map_err(CliError::from_client)?;
            generation = advanced.generation;
            if let Some(receipt) = receipt
                .lock()
                .map_err(|_| CliError::new("internal", 1, Outcome::Unknown, false))?
                .as_mut()
            {
                receipt.outcome = Outcome::Unknown;
            }
        } else {
            let directory = options.state_dir.as_ref().ok_or_else(|| {
                let mut error = CliError::from_client(ClientError::PrivateState);
                error.outcome = Outcome::NotApplied;
                error
            })?;
            let path = directory.join(format!(
                "{}.json",
                key_namespace(
                    call.idempotency_key
                        .as_ref()
                        .ok_or_else(CliError::usage)?
                        .as_str()
                )
            ));
            let opened = PrivateRetryStore::open(&path, options.limits.clone()).map_err(|e| {
                let mut e = CliError::from_client(e);
                e.outcome = Outcome::NotApplied;
                e
            })?;
            prepared_attempt = Some(
                publish_attempt(
                    &opened,
                    &RetryRecord {
                        request: call.clone(),
                        endpoint: endpoint.identity().clone(),
                        generation: 1,
                        outcome: Outcome::Unknown,
                        result: None,
                    },
                )
                .map_err(|e| {
                    let mut e = CliError::from_client(e);
                    e.outcome = Outcome::NotApplied;
                    e
                })?,
            );
            store = Some(opened);
            state_path = Some(path);
        }
        let path = state_path.as_ref().ok_or_else(CliError::usage)?;
        let state = path.to_str().ok_or_else(CliError::usage)?.to_owned();
        *receipt
            .lock()
            .map_err(|_| CliError::new("internal", 1, Outcome::Unknown, false))? = Some(Receipt {
            state,
            request_id: call.request_id.clone(),
            outcome: Outcome::Unknown,
        });
    }
    let mut attempt = match prepared_attempt {
        Some(attempt) => attempt,
        None => Attempt::new(call.clone(), endpoint.identity().clone())
            .map_err(|_| CliError::usage())?,
    };
    let called = execute(&mut connection, &mut attempt).await;
    if let Some(receipt) = receipt
        .lock()
        .map_err(|_| CliError::new("internal", 1, Outcome::Unknown, false))?
        .as_mut()
    {
        receipt.outcome = attempt.outcome();
    }
    let mut error = called.err().map(CliError::from_client);
    if let Some(opened) = &store {
        let result = match attempt.state() {
            AttemptState::Complete(reply) => Some(Ok(reply.clone())),
            AttemptState::Fault(fault) => Some(Err(fault.clone())),
            _ => None,
        };
        if let Some(result) = result {
            if let Err(failed) = opened.complete(generation, result) {
                let mut failed = CliError::from_client(failed);
                failed.outcome = attempt.outcome();
                error = Some(failed);
            }
        }
    }
    let settled = connection.close().await;
    if error.is_none() {
        if let Err(failed) = settled {
            let mut failed = CliError::from_client(failed);
            failed.outcome = attempt.outcome();
            error = Some(failed);
        }
    }
    if let Some(mut error) = error {
        error.request_id = Some(call.request_id);
        if let Some(path) = state_path {
            error.state = path.to_str().map(str::to_owned);
        }
        return Err(error);
    }
    match attempt.state() {
        AttemptState::Complete(reply) => Ok(success(reply.clone(), call.request_id)),
        _ => Err(CliError::new("internal", 1, Outcome::Unknown, false)),
    }
}

fn key_namespace(key: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(key.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
