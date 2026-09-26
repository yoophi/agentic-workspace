//! Tauri command ↔ `Workbench.call` 호환 어댑터(037).
//!
//! 프론트엔드는 바뀌지 않는다: command 시그니처·직렬화·오류 문자열이 이전과 같아야 한다
//! (`specs/037-workbench-seam/contracts/tauri-compat-commands.md`). 이 모듈은 변환만 하고
//! 저장·업무 로직을 갖지 않는다. 오류는 `WorkbenchFault.message`만 돌려준다.

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;
use workbench_core::{
    application::workbench_runtime::WorkbenchRuntime,
    domain::{
        agent_run_settings::AgentRunSettings,
        git_worktree::GitWorktreeCreateDraft,
        project::Project,
        worktree_file::{WorktreeFileListKind, WorktreeFileListScope},
    },
};
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, CallRequest, IdempotencyKey, OperationId, PROTOCOL_VERSION,
    RequestId, Workbench, WorkbenchFault,
};

use super::tauri_commands::{
    GoalInput, GoalProgressInput, GoalUpdateInput, ProjectInput, SavedPromptInput,
};

/// 데스크톱 앱이 쓰는 고정 호출자. 3단계에서 토큰 기반으로 바뀐다.
pub fn desktop_principal() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop()
}

/// 조회 요청 봉투. 멱등성 키 없음.
pub fn query_request(operation: OperationId, input: Value) -> CallRequest {
    CallRequest {
        protocol_version: PROTOCOL_VERSION,
        operation: operation.as_str().to_owned(),
        request_id: RequestId::random(),
        input,
        idempotency_key: None,
        expected_revision: None,
        timeout_ms: None,
    }
}

/// 변경 요청 봉투. Tauri 경로에는 재시도 개념이 없어 멱등성 키를 호출마다 새로 만든다.
pub fn command_request(operation: OperationId, input: Value) -> CallRequest {
    CallRequest {
        protocol_version: PROTOCOL_VERSION,
        operation: operation.as_str().to_owned(),
        request_id: RequestId::random(),
        input,
        idempotency_key: Some(IdempotencyKey::random()),
        expected_revision: None,
        timeout_ms: None,
    }
}

/// `CallReply.output`을 command 반환 타입으로 푼다. `()`는 `null`을 받는다.
pub fn decode_output<Out: DeserializeOwned>(reply: CallReply) -> Result<Out, String> {
    let output = reply
        .output()
        .cloned()
        .ok_or_else(|| "Unexpected asynchronous reply.".to_owned())?;
    serde_json::from_value(output).map_err(|error| format!("Failed to decode reply: {error}"))
}

async fn dispatch<Out: DeserializeOwned>(
    runtime: &Arc<WorkbenchRuntime>,
    request: CallRequest,
) -> Result<Out, String> {
    match runtime.call(desktop_principal(), request).await {
        Ok(reply) => decode_output(reply),
        Err(fault) => Err(fault_to_string(&fault)),
    }
}

/// 조회 command 공통 경로: `*Input → Value` 변환은 호출자가 한다.
pub async fn call_query<Out: DeserializeOwned>(
    runtime: &Arc<WorkbenchRuntime>,
    operation: OperationId,
    input: Value,
) -> Result<Out, String> {
    dispatch(runtime, query_request(operation, input)).await
}

/// 변경 command 공통 경로.
pub async fn call_command<Out: DeserializeOwned>(
    runtime: &Arc<WorkbenchRuntime>,
    operation: OperationId,
    input: Value,
) -> Result<Out, String> {
    dispatch(runtime, command_request(operation, input)).await
}

// ---- 038 US1: 저장 단위 4 도메인의 `*Input → CallRequest.input` 변환 (contracts/tauri-compat-commands.md 변환 규칙) ----

pub fn project_update_input(id: String, input: ProjectInput) -> Value {
    serde_json::json!({
        "id": id,
        "name": input.name,
        "workingDirectory": input.working_directory,
        "description": input.description,
    })
}

pub fn project_delete_input(id: String) -> Value {
    serde_json::json!({ "id": id })
}

pub fn saved_prompt_create_input(input: SavedPromptInput) -> Value {
    serde_json::json!({ "label": input.label, "prompt": input.prompt })
}

pub fn saved_prompt_update_input(id: String, input: SavedPromptInput) -> Value {
    serde_json::json!({ "id": id, "label": input.label, "prompt": input.prompt })
}

pub fn saved_prompt_delete_input(id: String) -> Value {
    serde_json::json!({ "id": id })
}

pub fn goal_get_input(working_directory: String) -> Value {
    serde_json::json!({ "workingDirectory": working_directory })
}

pub fn goal_create_input(input: GoalInput) -> Value {
    let mut value = serde_json::json!({
        "workingDirectory": input.working_directory,
        "objective": input.objective,
    });
    if let Some(budget) = input.token_budget {
        value["tokenBudget"] = serde_json::json!(budget);
    }
    value
}

/// `tokenBudget`는 AW 시절과 같은 규칙: 프론트가 보낸 값(값·`null`)을 그대로 전달하고, 없으면 생략한다.
pub fn goal_update_input(working_directory: String, input: GoalUpdateInput) -> Value {
    let mut value = serde_json::json!({ "workingDirectory": working_directory });
    if let Some(objective) = input.objective {
        value["objective"] = serde_json::json!(objective);
    }
    if let Some(status) = input.status {
        value["status"] = serde_json::to_value(status).expect("status serializes");
    }
    if let Some(budget) = input.token_budget {
        value["tokenBudget"] = serde_json::json!(budget);
    }
    value
}

pub fn goal_clear_input(working_directory: String) -> Value {
    serde_json::json!({ "workingDirectory": working_directory })
}

pub fn goal_record_progress_input(working_directory: String, input: GoalProgressInput) -> Value {
    serde_json::json!({
        "workingDirectory": working_directory,
        "tokensUsed": input.tokens_used,
        "timeUsedSeconds": input.time_used_seconds,
    })
}

pub fn agent_run_settings_get_input(working_directory: String) -> Value {
    serde_json::json!({ "workingDirectory": working_directory })
}

pub fn agent_run_settings_save_input(settings: AgentRunSettings) -> Value {
    serde_json::json!({ "settings": settings })
}

// ---- 038 US2: Git·worktree 14개 command의 인자 → `CallRequest.input` (contracts/tauri-compat-commands.md 변환 규칙) ----
// `Option::None`은 필드를 생략한다. 서버 기본값은 오늘의 command 기본값과 같다(`includeStatus` → true, 범위 → 전체).

/// `workingDirectory` 하나만 받는 조회(`git.listRemotes`·`git.listBranches`·`worktree.listChanges`·`worktree.getChanges`).
pub fn working_directory_input(working_directory: String) -> Value {
    serde_json::json!({ "workingDirectory": working_directory })
}

pub fn git_list_worktrees_input(working_directory: String, include_status: Option<bool>) -> Value {
    let mut value = working_directory_input(working_directory);
    if let Some(include_status) = include_status {
        value["includeStatus"] = serde_json::json!(include_status);
    }
    value
}

pub fn git_create_worktree_input(
    working_directory: String,
    draft: GitWorktreeCreateDraft,
) -> Value {
    let mut value =
        serde_json::json!({ "workingDirectory": working_directory, "path": draft.path });
    if let Some(branch) = draft.branch {
        value["branch"] = serde_json::json!(branch);
    }
    if let Some(reference) = draft.reference {
        value["reference"] = serde_json::json!(reference);
    }
    value
}

pub fn git_delete_worktree_input(working_directory: String, path: String) -> Value {
    serde_json::json!({ "workingDirectory": working_directory, "path": path })
}

/// `workingDirectory` + 상대 `path`(`worktree.getFileDiff`·`worktree.readTextFile`).
pub fn worktree_path_input(working_directory: String, path: String) -> Value {
    serde_json::json!({ "workingDirectory": working_directory, "path": path })
}

pub fn worktree_list_files_input(
    working_directory: String,
    scope: Option<WorktreeFileListScope>,
) -> Value {
    let mut value = working_directory_input(working_directory);
    if let Some(scope) = scope {
        let mut scope_value = serde_json::json!({
            "kind": match scope.kind {
                WorktreeFileListKind::All => "all",
                WorktreeFileListKind::Markdown => "markdown",
            }
        });
        if let Some(dir) = scope.dir {
            scope_value["dir"] = serde_json::json!(dir);
        }
        if let Some(depth) = scope.depth {
            scope_value["depth"] = serde_json::json!(depth);
        }
        value["scope"] = scope_value;
    }
    value
}

/// 이력·그래프 페이지 요청(`worktree.listHistory`·`worktree.getGraph`).
pub fn worktree_page_input(
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Value {
    let mut value = working_directory_input(working_directory);
    if let Some(max_count) = max_count {
        value["maxCount"] = serde_json::json!(max_count);
    }
    if let Some(offset) = offset {
        value["offset"] = serde_json::json!(offset);
    }
    if let Some(cursor) = cursor {
        value["cursor"] = serde_json::json!(cursor);
    }
    value
}

pub fn worktree_commit_input(working_directory: String, commit_hash: String) -> Value {
    serde_json::json!({ "workingDirectory": working_directory, "commitHash": commit_hash })
}

pub fn worktree_commit_file_input(
    working_directory: String,
    commit_hash: String,
    path: String,
) -> Value {
    serde_json::json!({ "workingDirectory": working_directory, "commitHash": commit_hash, "path": path })
}

// ---- 038 US3: agent catalog·provider 세션 ----

pub fn agent_list_input() -> Value {
    serde_json::json!({})
}

/// `cwd: None`은 필드를 생략한다. 공백 `cwd`는 그대로 보낸다 — 서버가 오늘처럼 전체 범위로 해석한다.
pub fn agent_list_provider_sessions_input(agent_id: String, cwd: Option<String>) -> Value {
    let mut value = serde_json::json!({ "agentId": agent_id });
    if let Some(cwd) = cwd {
        value["cwd"] = serde_json::json!(cwd);
    }
    value
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

    fn strip_nulls(value: &mut Value) {
        match value {
            Value::Object(map) => {
                map.retain(|_, child| !child.is_null());
                map.values_mut().for_each(strip_nulls);
            }
            Value::Array(items) => items.iter_mut().for_each(strip_nulls),
            _ => {}
        }
    }

    fn assert_subset(expected: &Value, actual: &Value, label: &str) {
        match (expected, actual) {
            (Value::Object(exp), Value::Object(act)) => {
                for (key, exp_value) in exp {
                    let act_value = act
                        .get(key)
                        .unwrap_or_else(|| panic!("{label}: missing {key} in {actual}"));
                    assert_subset(exp_value, act_value, label);
                }
            }
            (Value::Array(exp), Value::Array(act)) => {
                assert_eq!(exp.len(), act.len(), "{label}: array length");
                for (exp_item, act_item) in exp.iter().zip(act) {
                    assert_subset(exp_item, act_item, label);
                }
            }
            (exp, act) => assert_eq!(exp, act, "{label}"),
        }
    }

    fn wd(input: &Value) -> String {
        input["workingDirectory"].as_str().unwrap().to_owned()
    }

    fn text(input: &Value, key: &str) -> String {
        input[key].as_str().unwrap().to_owned()
    }

    fn opt_usize(input: &Value, key: &str) -> Option<usize> {
        input
            .get(key)
            .and_then(Value::as_u64)
            .map(|value| value as usize)
    }

    /// 038 US1·US2: fixture의 `input`을 AW `*Input`으로 읽어 변환하면 fixture와 같은 `input`이 나와야 한다.
    /// (`operation` 접두어별로 변환 함수를 고른다. 키 누락·readonly fixture도 입력 형태는 같다.)
    #[test]
    fn inputs_match_fixture_shapes() {
        let all = fs::read_dir(fixtures_dir())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .map(|path| serde_json::from_str::<Value>(&fs::read_to_string(path).unwrap()).unwrap());
        let mut checked = 0;
        for fixture in all {
            // 서버의 unknown-field 규칙을 보는 fixture는 AW 타입이 그 필드를 갖지 않으므로 형태 비교 대상이 아니다.
            if fixture["name"].as_str().unwrap().contains("unknown-field") {
                continue;
            }
            for request in fixture_requests(&fixture) {
                let operation = request["operation"].as_str().unwrap();
                let input = request["input"].clone();
                let built = match operation {
                    "project.update" => project_update_input(
                        input["id"].as_str().unwrap().to_owned(),
                        serde_json::from_value(input.clone()).unwrap(),
                    ),
                    "project.delete" => project_delete_input(input["id"].as_str().unwrap().into()),
                    "savedPrompt.create" => {
                        saved_prompt_create_input(serde_json::from_value(input.clone()).unwrap())
                    }
                    "savedPrompt.update" => saved_prompt_update_input(
                        input["id"].as_str().unwrap().to_owned(),
                        serde_json::from_value(input.clone()).unwrap(),
                    ),
                    "savedPrompt.delete" => {
                        saved_prompt_delete_input(input["id"].as_str().unwrap().into())
                    }
                    "goal.get" => {
                        goal_get_input(input["workingDirectory"].as_str().unwrap().into())
                    }
                    "goal.create" => {
                        goal_create_input(serde_json::from_value(input.clone()).unwrap())
                    }
                    "goal.update" => goal_update_input(
                        input["workingDirectory"].as_str().unwrap().to_owned(),
                        serde_json::from_value(input.clone()).unwrap(),
                    ),
                    "goal.clear" => {
                        goal_clear_input(input["workingDirectory"].as_str().unwrap().into())
                    }
                    "goal.recordProgress" => goal_record_progress_input(
                        input["workingDirectory"].as_str().unwrap().to_owned(),
                        serde_json::from_value(input.clone()).unwrap(),
                    ),
                    "agentRunSettings.get" => agent_run_settings_get_input(
                        input["workingDirectory"].as_str().unwrap().into(),
                    ),
                    "agentRunSettings.save"
                        if input.get("settings").is_some()
                            && input.as_object().unwrap().len() == 1 =>
                    {
                        match serde_json::from_value::<AgentRunSettings>(input["settings"].clone())
                        {
                            Ok(settings) => agent_run_settings_save_input(settings),
                            Err(_) => continue, // 검증 실패 fixture(빈 workingDirectory 등)는 AW 타입도 거절하지 않지만 형태 비교 대상 아님
                        }
                    }
                    "git.listRemotes"
                    | "git.listBranches"
                    | "worktree.listChanges"
                    | "worktree.getChanges" => working_directory_input(wd(&input)),
                    "git.listWorktrees" => git_list_worktrees_input(
                        wd(&input),
                        input.get("includeStatus").and_then(Value::as_bool),
                    ),
                    "git.createWorktree" => git_create_worktree_input(
                        wd(&input),
                        serde_json::from_value(input.clone()).unwrap(),
                    ),
                    "git.deleteWorktree" => {
                        git_delete_worktree_input(wd(&input), text(&input, "path"))
                    }
                    "worktree.getFileDiff" | "worktree.readTextFile" => {
                        worktree_path_input(wd(&input), text(&input, "path"))
                    }
                    "worktree.listFiles" => worktree_list_files_input(
                        wd(&input),
                        input
                            .get("scope")
                            .map(|scope| serde_json::from_value(scope.clone()).unwrap()),
                    ),
                    "worktree.listHistory" | "worktree.getGraph" => worktree_page_input(
                        wd(&input),
                        opt_usize(&input, "maxCount"),
                        opt_usize(&input, "offset"),
                        input
                            .get("cursor")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    ),
                    "worktree.getCommitDetail" => {
                        worktree_commit_input(wd(&input), text(&input, "commitHash"))
                    }
                    "worktree.getCommitFileDiff" => worktree_commit_file_input(
                        wd(&input),
                        text(&input, "commitHash"),
                        text(&input, "path"),
                    ),
                    "agent.list" => agent_list_input(),
                    "agent.listProviderSessions" => match input.get("agentId") {
                        Some(agent_id) => agent_list_provider_sessions_input(
                            agent_id.as_str().unwrap().to_owned(),
                            input.get("cwd").and_then(Value::as_str).map(str::to_owned),
                        ),
                        None => continue, // agentId 누락 fixture: Tauri 인자는 필수라 이 형태가 생기지 않는다
                    },
                    _ => continue,
                };
                // `null`은 AW 타입이 None으로 읽어 생략하고, 도메인 타입의 serde default가 채운 필드는 fixture에 없을 수
                // 있다. 따라서 "fixture input ⊆ 변환 결과"를 재귀로 확인한다(서버는 두 형태를 같게 본다).
                let mut expected = input.clone();
                strip_nulls(&mut expected);
                assert_subset(
                    &expected,
                    &built,
                    &format!("{} / {operation}", fixture["name"]),
                );
                checked += 1;
            }
        }
        assert!(
            checked >= 13 + 52,
            "expected US1+US2 fixtures, checked {checked}"
        );
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
