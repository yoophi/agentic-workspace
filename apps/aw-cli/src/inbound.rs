//! Strict argv projection. Private payloads are accepted only through bounded input.
use crate::infrastructure::output::CliError;
use serde::Deserialize;
use std::path::PathBuf;
use workbench_client::{
    application::call::validate_request,
    domain::limits::{Limits, Resource},
};
use workbench_protocol::{CallRequest, IdempotencyKey, OperationId};
#[derive(Clone)]
pub enum Input {
    Source(String),
    Cancel(String),
}
#[derive(Clone)]
pub enum Command {
    Operations(Option<OperationId>),
    Call {
        operation: Option<OperationId>,
        input: Option<Input>,
        key: Option<IdempotencyKey>,
        revision: Option<u64>,
    },
    Watch {
        input: Option<String>,
        run: Option<String>,
        after: u64,
    },
}
pub struct Options {
    pub command: Command,
    pub descriptor: Option<PathBuf>,
    pub state_dir: Option<PathBuf>,
    pub retry_state: Option<PathBuf>,
    pub limits: Limits,
}
fn usage() -> CliError {
    CliError::usage()
}
pub fn parse(args: Vec<String>) -> Result<Options, CliError> {
    let mut positional = Vec::new();
    let mut descriptor = None;
    let mut state_dir = None;
    let mut retry_state = None;
    let mut input = None;
    let mut key = None;
    let mut revision = None;
    let mut after = 0;
    let mut timeout = None;
    let mut index = 0;
    let mut seen = std::collections::HashSet::new();
    while index < args.len() {
        let arg = &args[index];
        if arg.starts_with("--") {
            if !seen.insert(arg.clone()) {
                return Err(usage());
            }
            if arg == "--json" {
                index += 1;
                continue;
            }
            let value = args.get(index + 1).ok_or_else(usage)?;
            if value.starts_with("--") {
                return Err(usage());
            }
            match arg.as_str() {
                "--descriptor" => descriptor = Some(PathBuf::from(value)),
                "--state-dir" => state_dir = Some(PathBuf::from(value)),
                "--retry-state" => retry_state = Some(PathBuf::from(value)),
                "--input" => input = Some(value.clone()),
                "--idempotency-key" => key = Some(IdempotencyKey::new(value).map_err(|_| usage())?),
                "--expected-revision" => revision = Some(value.parse().map_err(|_| usage())?),
                "--after" => after = value.parse().map_err(|_| usage())?,
                "--timeout-ms" => timeout = Some(value.parse::<u64>().map_err(|_| usage())?),
                _ => return Err(usage()),
            };
            index += 2;
        } else {
            positional.push(arg.as_str());
            index += 1;
        }
    }
    let command = match positional.as_slice() {
        ["operations"] => Command::Operations(None),
        ["operations", op] => Command::Operations(Some(OperationId::parse(op).ok_or_else(usage)?)),
        ["project", "list"] => Command::Call {
            operation: Some(OperationId::ProjectList),
            input: input.clone().map(Input::Source),
            key: key.clone(),
            revision,
        },
        ["server", "status"] => Command::Call {
            operation: Some(OperationId::ServerStatus),
            input: input.clone().map(Input::Source),
            key: key.clone(),
            revision,
        },
        ["run", "start"] => Command::Call {
            operation: Some(OperationId::RunStart),
            input: input.clone().map(Input::Source),
            key: key.clone(),
            revision,
        },
        ["run", "cancel", run] => {
            if run.len() > 128 || run.is_empty() {
                return Err(usage());
            }
            Command::Call {
                operation: Some(OperationId::RunCancel),
                input: Some(Input::Cancel((*run).to_owned())),
                key: key.clone(),
                revision,
            }
        }
        ["call", op] => Command::Call {
            operation: Some(OperationId::parse(op).ok_or_else(usage)?),
            input: input.clone().map(Input::Source),
            key: key.clone(),
            revision,
        },
        ["call"] if retry_state.is_some() => Command::Call {
            operation: None,
            input: input.clone().map(Input::Source),
            key: key.clone(),
            revision,
        },
        ["events", "watch"] => Command::Watch {
            input: input.clone(),
            run: None,
            after,
        },
        ["run", "watch", run] if !run.is_empty() && run.len() <= 128 => Command::Watch {
            input: input.clone(),
            run: Some((*run).into()),
            after,
        },
        _ => return Err(usage()),
    };
    if retry_state.is_some()
        && (input.is_some() || key.is_some() || revision.is_some() || state_dir.is_some())
    {
        return Err(usage());
    }
    if matches!(command, Command::Operations(_))
        && (input.is_some() || key.is_some() || revision.is_some() || retry_state.is_some())
    {
        return Err(usage());
    }
    let mut config = workbench_client::domain::limits::LimitConfig::default();
    if let Some(ms) = timeout {
        config.request_timeout = std::time::Duration::from_millis(ms);
    }
    let limits = Limits::new(config).map_err(|_| usage())?;
    Ok(Options {
        command,
        descriptor: descriptor
            .or_else(|| std::env::var_os("AW_SERVER_DESCRIPTOR").map(PathBuf::from)),
        state_dir: state_dir.or_else(|| std::env::var_os("AW_CLI_STATE_DIR").map(PathBuf::from)),
        retry_state,
        limits,
    })
}
pub async fn read_input(source: &str, limits: &Limits) -> Result<serde_json::Value, CliError> {
    use tokio::io::AsyncReadExt;
    let mut bytes = Vec::new();
    let max = limits.maximum(Resource::Input) as u64 + 1;
    if source == "-" {
        tokio::io::stdin()
            .take(max)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| usage())?;
    } else {
        return Err(usage());
    }
    limits
        .check_add(Resource::Input, 0, bytes.len())
        .map_err(|_| usage())?;
    serde_json::from_slice(&bytes).map_err(|_| usage())
}
pub fn request(
    operation: OperationId,
    input: serde_json::Value,
    key: Option<IdempotencyKey>,
    revision: Option<u64>,
) -> Result<CallRequest, CliError> {
    let mut request = if workbench_protocol::operations::spec_for(operation).kind
        == workbench_protocol::OperationKind::Command
    {
        CallRequest::command(operation, input)
    } else {
        CallRequest::query(operation, input)
    };
    if key.is_some() {
        request.idempotency_key = key;
    }
    request.expected_revision = revision;
    validate_request(&request).map_err(CliError::from_client)?;
    Ok(request)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchInput {
    pub stream_id: String,
    pub epoch: String,
    pub after_sequence: u64,
}

pub fn cancel_input(run_id: &str, bench_id: &str) -> serde_json::Value {
    serde_json::to_value(workbench_protocol::operations::run::RunCancelInput {
        run_id: run_id.to_owned(),
        bench_id: bench_id.to_owned(),
    })
    .expect("typed cancel serializes")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancel_projection_uses_protocol_dto_instead_of_private_stdin_marker() {
        let options = parse(vec!["run".into(), "cancel".into(), "r".into()])
            .unwrap_or_else(|_| panic!("parse rejected"));
        assert!(
            matches!(options.command,Command::Call {input:Some(Input::Cancel(ref run)),..} if run=="r")
        );
        let value = cancel_input("r", "b");
        let typed: workbench_protocol::operations::run::RunCancelInput =
            serde_json::from_value(value.clone()).unwrap();
        assert_eq!(typed.run_id, "r");
        assert_eq!(typed.bench_id, "b");
        assert_eq!(value, serde_json::json!({"benchId":"b","runId":"r"}));
    }
}
