//! Tauri command ↔ `Workbench.call` 호환 어댑터(037).
//!
//! 프론트엔드는 바뀌지 않는다: command 시그니처·직렬화·오류 문자열이 이전과 같아야 한다
//! (`specs/037-workbench-seam/contracts/tauri-compat-commands.md`). 이 모듈은 변환만 하고
//! 저장·업무 로직을 갖지 않는다. 오류는 `WorkbenchFault.message`만 돌려준다.

use std::sync::Arc;

use workbench_core::{application::workbench_runtime::WorkbenchRuntime, domain::project::Project};
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, CallRequest, IdempotencyKey, OperationId, PROTOCOL_VERSION,
    RequestId, Workbench, WorkbenchFault,
};

use super::tauri_commands::ProjectInput;

/// 데스크톱 앱이 쓰는 고정 호출자. 3단계에서 토큰 기반으로 바뀐다.
pub fn desktop_principal() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop()
}

pub fn list_projects_request(request_id: RequestId) -> CallRequest {
    CallRequest {
        protocol_version: PROTOCOL_VERSION,
        operation: OperationId::ProjectList.as_str().to_owned(),
        request_id,
        input: serde_json::json!({}),
        idempotency_key: None,
        expected_revision: None,
        timeout_ms: None,
    }
}

pub fn create_project_request(
    input: ProjectInput,
    idempotency_key: IdempotencyKey,
    request_id: RequestId,
) -> CallRequest {
    CallRequest {
        protocol_version: PROTOCOL_VERSION,
        operation: OperationId::ProjectCreate.as_str().to_owned(),
        request_id,
        input: serde_json::json!({
            "name": input.name,
            "workingDirectory": input.working_directory,
            "description": input.description,
        }),
        idempotency_key: Some(idempotency_key),
        expected_revision: None,
        timeout_ms: None,
    }
}

pub fn reply_to_projects(reply: CallReply) -> Result<Vec<Project>, String> {
    let output = reply
        .output()
        .cloned()
        .ok_or_else(|| "Unexpected asynchronous reply for project.list.".to_owned())?;
    serde_json::from_value(output).map_err(|error| format!("Failed to decode projects: {error}"))
}

pub fn reply_to_project(reply: CallReply) -> Result<Project, String> {
    let output = reply
        .output()
        .cloned()
        .ok_or_else(|| "Unexpected asynchronous reply for project.create.".to_owned())?;
    serde_json::from_value(output).map_err(|error| format!("Failed to decode project: {error}"))
}

/// 코드·outcome을 붙이지 않는다. 화면이 이 문자열을 그대로 보여 준다.
pub fn fault_to_string(fault: &WorkbenchFault) -> String {
    fault.message.clone()
}

pub async fn call_list_projects(runtime: &Arc<WorkbenchRuntime>) -> Result<Vec<Project>, String> {
    let request = list_projects_request(RequestId::random());
    match runtime.call(desktop_principal(), request).await {
        Ok(reply) => reply_to_projects(reply),
        Err(fault) => Err(fault_to_string(&fault)),
    }
}

/// Tauri 경로에는 재시도 개념이 없어 멱등성 키를 호출마다 새로 만든다.
pub async fn call_create_project(
    runtime: &Arc<WorkbenchRuntime>,
    input: ProjectInput,
) -> Result<Project, String> {
    let request = create_project_request(input, IdempotencyKey::random(), RequestId::random());
    match runtime.call(desktop_principal(), request).await {
        Ok(reply) => reply_to_project(reply),
        Err(fault) => Err(fault_to_string(&fault)),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::{Value, json};
    use workbench_protocol::{FaultCode, operations::project::ProjectCreateInput};

    use super::*;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../crates/workbench-protocol/fixtures")
    }

    fn create_fixtures() -> Vec<Value> {
        let mut paths: Vec<PathBuf> = fs::read_dir(fixtures_dir())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("project-create-") && name.ends_with(".json")
                    })
            })
            .collect();
        paths.sort();
        assert!(!paths.is_empty());
        paths
            .into_iter()
            .map(|path| serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap())
            .collect()
    }

    fn fixture_requests(fixture: &Value) -> Vec<Value> {
        if let Some(request) = fixture.get("request") {
            vec![request.clone()]
        } else {
            fixture["requests"].as_array().cloned().unwrap_or_default()
        }
    }

    #[test]
    fn create_request_matches_fixture_shape_for_every_create_fixture() {
        for fixture in create_fixtures() {
            for expected in fixture_requests(&fixture) {
                let Some(key) = expected.get("idempotencyKey").and_then(Value::as_str) else {
                    continue; // 키 누락 fixture는 Tauri 경로에서 발생하지 않는다(항상 키를 만든다)
                };
                let input: ProjectInput =
                    serde_json::from_value(expected["input"].clone()).unwrap();
                let built = create_project_request(
                    input,
                    IdempotencyKey::new(key).unwrap(),
                    RequestId::new(expected["requestId"].as_str().unwrap()).unwrap(),
                );
                let built_json = serde_json::to_value(&built).unwrap();
                assert_eq!(
                    built_json["protocolVersion"], expected["protocolVersion"],
                    "{}",
                    fixture["name"]
                );
                assert_eq!(
                    built_json["operation"], expected["operation"],
                    "{}",
                    fixture["name"]
                );
                assert_eq!(
                    built_json["requestId"], expected["requestId"],
                    "{}",
                    fixture["name"]
                );
                assert_eq!(
                    built_json["idempotencyKey"], expected["idempotencyKey"],
                    "{}",
                    fixture["name"]
                );
                let built_input: ProjectCreateInput =
                    serde_json::from_value(built_json["input"].clone()).unwrap();
                let expected_input: ProjectCreateInput =
                    serde_json::from_value(expected["input"].clone()).unwrap();
                assert_eq!(built_input, expected_input, "{}", fixture["name"]);
            }
        }
    }

    #[test]
    fn fault_string_is_exactly_the_message_for_every_fixture_fault() {
        for fixture in create_fixtures() {
            let Some(fault) = fixture.get("expect").and_then(|expect| expect.get("fault")) else {
                continue;
            };
            let code: FaultCode = serde_json::from_value(fault["code"].clone()).unwrap();
            let Some(message) = fault["message"].as_str() else {
                continue; // message를 고정하지 않은 fixture(예: forbidden)
            };
            let built = WorkbenchFault::new(code, RequestId::new("r").unwrap(), message);
            assert_eq!(fault_to_string(&built), message, "{}", fixture["name"]);
        }
    }

    #[test]
    fn list_request_is_a_plain_query() {
        let request = list_projects_request(RequestId::new("r1").unwrap());
        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            json!({"protocolVersion": 1, "operation": "project.list", "requestId": "r1", "input": {}})
        );
    }

    #[test]
    fn replies_decode_into_domain_projects() {
        let reply = CallReply::complete(
            json!([{"id": "project-1", "name": "AW", "workingDirectory": "/tmp/aw", "description": null}]),
            None,
        );
        let projects = reply_to_projects(reply).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, "project-1");

        let reply = CallReply::complete(
            json!({"id": "project-2", "name": "B", "workingDirectory": "/b", "description": "d"}),
            Some(1),
        );
        assert_eq!(
            reply_to_project(reply).unwrap().description,
            Some("d".into())
        );

        let accepted = CallReply::Accepted {
            execution_id: "x".into(),
            revision: None,
        };
        assert!(reply_to_project(accepted).is_err());
    }
}
