//! `stop` 부명령(044 T041, contracts/server-lifecycle.md §1·§6): 안내 파일의 서버를 확인하고 소유자 자격 증명으로
//! `server.stop{mode}`을 부른 뒤, 그 인스턴스의 안내 파일이 사라질 때까지(정지 완료) 기다린다.
//!
//! 종료 코드: 0 정지함(또는 서버 없음), 5 `default`가 활동 작업으로 거절됨(blocker JSON을 표준 출력에), 1 그 밖.

use std::{path::Path, time::Duration};

use serde_json::{Value, json};

use super::{
    client::{request, verify_instance},
    descriptor::read_descriptor,
    lock::server_dir,
};

pub const EXIT_ACTIVE_WORK: i32 = 5;
/// 안내 파일 삭제를 확인하는 주기.
const POLL: Duration = Duration::from_millis(100);
/// `default`·`force` 정지 완료를 기다리는 상한. `wait`는 활동 작업이 끝날 때까지 기다린다(사용자가 끊을 수 있다).
const STOP_DEADLINE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopMode {
    Default,
    Wait,
    Force,
}

impl StopMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Wait => "wait",
            Self::Force => "force",
        }
    }
}

/// 정지 요청 결과: 종료 코드와 표준 출력에 쓸 JSON.
pub struct StopResult {
    pub code: i32,
    pub stdout: Option<Value>,
    pub stderr: Option<String>,
}

fn failed(message: impl Into<String>) -> StopResult {
    StopResult {
        code: 1,
        stdout: None,
        stderr: Some(message.into()),
    }
}

pub fn stop(data_dir: &Path, mode: StopMode) -> StopResult {
    let dir = server_dir(data_dir);
    let descriptor = match read_descriptor(&dir) {
        Ok(Some(descriptor)) => descriptor,
        Ok(None) => {
            return StopResult {
                code: 0,
                stdout: Some(json!({ "stopped": false, "reason": "no server" })),
                stderr: None,
            };
        }
        Err(error) => return failed(error.to_string()),
    };
    if let Err(error) = verify_instance(&descriptor) {
        return failed(error.to_string());
    }
    let envelope = json!({
        "protocolVersion": workbench_protocol::PROTOCOL_VERSION,
        "operation": "server.stop",
        "requestId": format!("req_{}", uuid::Uuid::new_v4().simple()),
        "idempotencyKey": format!("idem_{}", uuid::Uuid::new_v4().simple()),
        "input": { "mode": mode.as_str() },
    });
    let (status, body) = match request(
        &descriptor.base_url,
        "POST",
        workbench_protocol::openapi::CALLS_PATH,
        Some(&envelope),
        Some(&descriptor.owner_token),
    ) {
        Ok(answer) => answer,
        Err(error) => return failed(format!("Workbench server unreachable: {error}")),
    };
    if status != 200 || body["kind"] != "complete" {
        if body["code"] == "conflict" {
            return StopResult {
                code: EXIT_ACTIVE_WORK,
                stdout: Some(json!({
                    "stopped": false,
                    "reason": "activeWork",
                    "activeWork": body["details"]["activeWork"],
                })),
                stderr: body["message"].as_str().map(str::to_owned),
            };
        }
        return failed(format!("server.stop answered {status}: {body}"));
    }
    let deadline = (mode != StopMode::Wait).then(|| std::time::Instant::now() + STOP_DEADLINE);
    loop {
        match read_descriptor(&dir) {
            Ok(Some(current)) if current.instance_id == descriptor.instance_id => {}
            Ok(_) => break,
            Err(error) => return failed(error.to_string()),
        }
        if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
            return failed("the server accepted the stop but did not finish in time");
        }
        std::thread::sleep(POLL);
    }
    StopResult {
        code: 0,
        stdout: Some(json!({
            "stopped": true,
            "instanceId": descriptor.instance_id,
            "state": body["output"]["state"],
        })),
        stderr: None,
    }
}
