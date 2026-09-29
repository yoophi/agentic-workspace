//! Safe machine projection: arbitrary peer messages/details never enter diagnostics.
use serde_json::{json, Value};
use workbench_client::ports::ClientError;
use workbench_protocol::{CallReply, FaultCode, Outcome, RequestId, WorkbenchFault};
pub struct CliError {
    pub code: &'static str,
    pub exit: u8,
    pub outcome: Outcome,
    pub retryable: bool,
    pub request_id: Option<RequestId>,
    pub state: Option<String>,
}
impl CliError {
    pub fn usage() -> Self {
        Self::new("invalidArgument", 2, Outcome::NotApplied, false)
    }
    pub fn new(code: &'static str, exit: u8, outcome: Outcome, retryable: bool) -> Self {
        Self {
            code,
            exit,
            outcome,
            retryable,
            request_id: None,
            state: None,
        }
    }
    pub fn from_client(error: ClientError) -> Self {
        match error {
            ClientError::Fault(fault) => Self::fault(fault),
            ClientError::InvalidInput => Self::usage(),
            ClientError::Unavailable => Self::new("unavailable", 8, Outcome::NotApplied, true),
            ClientError::Incompatible => {
                Self::new("unsupportedProtocol", 9, Outcome::NotApplied, false)
            }
            ClientError::Identity => Self::new("unauthenticated", 3, Outcome::NotApplied, false),
            ClientError::Deadline => Self::new("deadlineExceeded", 7, Outcome::Unknown, true),
            ClientError::Cancelled => Self::new("cancelled", 130, Outcome::Unknown, false),
            ClientError::PrerequisiteUnavailable => {
                Self::new("prerequisiteUnavailable", 8, Outcome::NotApplied, false)
            }
            ClientError::TransportUnknown => {
                Self::new("transportUnknown", 8, Outcome::Unknown, true)
            }
            ClientError::PrivateState | ClientError::StaleGeneration => {
                Self::new("privateState", 5, Outcome::Unknown, false)
            }
            ClientError::Protocol | ClientError::Limit(_) => {
                Self::new("protocolViolation", 1, Outcome::Unknown, false)
            }
        }
    }
    pub fn fault(fault: WorkbenchFault) -> Self {
        let mut error = Self::new(
            fault.code.as_str(),
            exit_for(fault.code),
            fault.outcome,
            fault.retryable,
        );
        error.request_id = Some(fault.request_id);
        error
    }
    pub fn value(&self) -> Value {
        let mut value = json!({"ok":false,"error":{"code":self.code,"outcome":self.outcome,"retryable":self.retryable}});
        if let Some(id) = &self.request_id {
            value["requestId"] = json!(id);
        }
        if let Some(state) = &self.state {
            value["attempt"] = json!({"retryState":state});
        }
        value
    }
}
pub fn exit_for(code: FaultCode) -> u8 {
    match code {
        FaultCode::InvalidArgument => 2,
        FaultCode::Unauthenticated | FaultCode::Forbidden => 3,
        FaultCode::NotFound => 4,
        FaultCode::Conflict | FaultCode::PreconditionFailed => 5,
        FaultCode::DeadlineExceeded => 7,
        FaultCode::RateLimited | FaultCode::Draining | FaultCode::Unavailable => 8,
        FaultCode::UnsupportedProtocol | FaultCode::UnsupportedSchema => 9,
        FaultCode::InteractionRequired => 10,
        FaultCode::Internal => 1,
    }
}
pub fn success(reply: CallReply, id: RequestId) -> Value {
    match reply {
        CallReply::Complete {
            output,
            revision,
            replayed,
        } => {
            let mut value = json!({"ok":true,"data":output,"requestId":id,"replayed":replayed});
            if let Some(revision) = revision {
                value["revision"] = json!(revision);
            }
            value
        }
        CallReply::Accepted {
            execution_id,
            revision,
        } => {
            let mut value = json!({"ok":true,"data":{"kind":"accepted","executionId":execution_id},"requestId":id});
            if let Some(revision) = revision {
                value["revision"] = json!(revision);
            }
            value
        }
    }
}
pub async fn finite(value: Value, stderr: bool) -> Result<(), CliError> {
    use tokio::io::AsyncWriteExt;
    let mut bytes = serde_json::to_vec(&value)
        .map_err(|_| CliError::new("internal", 1, Outcome::Unknown, false))?;
    bytes.push(b'\n');
    if stderr {
        let mut writer = tokio::io::stderr();
        match writer.write_all(&bytes).await {
            Ok(()) => writer.flush().await,
            Err(error) => Err(error),
        }
    } else {
        let mut writer = tokio::io::stdout();
        match writer.write_all(&bytes).await {
            Ok(()) => writer.flush().await,
            Err(error) => Err(error),
        }
    }
    .map_err(|_| CliError::new("outputUnavailable", 8, Outcome::Unknown, false))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_fault_exit_families_preserve_outcome_and_redact_private_details() {
        let expected = [2, 3, 3, 4, 5, 10, 5, 9, 9, 8, 8, 8, 7, 1];
        for (code, exit) in FaultCode::ALL.into_iter().zip(expected) {
            let fault = WorkbenchFault::new(code, RequestId::random(), "private-sentinel")
                .with_outcome(Outcome::Unknown)
                .with_details(json!({"token":"private-sentinel"}));
            let error = CliError::fault(fault);
            assert_eq!(error.exit, exit);
            assert_eq!(error.outcome, Outcome::Unknown);
            assert!(!error.value().to_string().contains("private-sentinel"));
        }
    }
}
