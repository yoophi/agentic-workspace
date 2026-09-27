//! `/v1/calls` 호출 도우미(044 T028). 데스크톱(외부 서버 모드)과 소유자 클라이언트가 소유자 자격 증명 또는 창 토큰으로
//! Workbench operation을 부른다. 루프백 전용(`client::request_with_origin`). 성공은 출력, 문제 응답은 fault(코드·메시지),
//! 닿지 못함은 전송 오류다. command에는 새 멱등성 키를 붙인다(호출자가 재시도하지 않는 한 번의 요청).

use serde_json::{Value, json};

use super::client::request_with_origin;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError {
    /// 끝점에 닿지 못했거나 응답이 깨졌다.
    Transport(String),
    /// 서버가 답한 거절.
    Fault {
        status: u16,
        code: String,
        message: String,
    },
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(reason) => write!(f, "Workbench server unreachable: {reason}"),
            Self::Fault { message, .. } => f.write_str(message),
        }
    }
}

impl std::error::Error for CallError {}

/// operation 하나를 부른다. `command`면 멱등성 키를 붙인다. `origin`은 창 토큰의 WebView 출처.
pub fn call(
    base_url: &str,
    bearer: &str,
    origin: Option<&str>,
    operation: &str,
    input: Value,
    command: bool,
) -> Result<Value, CallError> {
    let mut envelope = json!({
        "protocolVersion": workbench_protocol::PROTOCOL_VERSION,
        "operation": operation,
        "requestId": format!("req_{}", uuid::Uuid::new_v4().simple()),
        "input": input,
    });
    if command {
        envelope["idempotencyKey"] = json!(format!("idem_{}", uuid::Uuid::new_v4().simple()));
    }
    let (status, body) = request_with_origin(
        base_url,
        "POST",
        workbench_protocol::openapi::CALLS_PATH,
        Some(&envelope),
        Some(bearer),
        origin,
    )
    .map_err(CallError::Transport)?;
    if status == 200 && body["kind"] == "complete" {
        return Ok(body.get("output").cloned().unwrap_or(Value::Null));
    }
    Err(CallError::Fault {
        status,
        code: body["code"].as_str().unwrap_or("unknown").to_owned(),
        message: body["message"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Workbench call {operation} failed with HTTP {status}")),
    })
}
