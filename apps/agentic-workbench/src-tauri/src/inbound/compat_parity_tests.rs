//! 043 T020: 호환 command 인자 → Workbench operation 입력의 golden(`src/shared/api/transport/compat-parity.golden.json`).
//! 이 시험은 compat 경로가 쓰는 **실제 인자 타입**(Tauri가 인자를 역직렬화하는 타입)과 입력 생성 함수로 `expected`를
//! 다시 계산해 파일과 비교한다 — 파일이 compat 코드와 어긋나면 실패한다(`UPDATE_GOLDEN=1`이면 다시 쓴다). TS 표
//! (`command-table.ts`)는 같은 파일의 `wire`(없으면 `expected`)를 그대로 만들어야 한다.
//!
//! DTO를 통째로 넘기는 command(`semantic`)는 compat이 Rust 타입으로 한 번 읽었다 다시 쓰므로 바이트가 원본과 다르다
//! (생략 필드가 null·기본값으로 채워짐). 화면은 원본을 보낸다(`wire`). 둘을 서버의 operation 입력 DTO로 읽었을 때 같아야
//! 한다 — 서버에게는 같은 입력이다.

use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use workbench_protocol::operations::{agent_run_settings, exchange, run};

use super::*;
use crate::inbound::workbench_compat as compat;

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../src/shared/api/transport/compat-parity.golden.json")
}

fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> T {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .unwrap_or_else(|error| panic!("argument {key}: {error}"))
}

fn s(args: &Value, key: &str) -> String {
    arg(args, key)
}

/// 서버가 읽는 모양으로 비교한다(역직렬화 뒤 다시 직렬화).
fn decoded<T: DeserializeOwned + serde::Serialize>(input: &Value) -> Value {
    let typed: T = serde_json::from_value(input.clone())
        .unwrap_or_else(|error| panic!("server would reject {input}: {error}"));
    serde_json::to_value(typed).unwrap()
}

/// (operation, compat 입력, 화면이 보낼 원본 — DTO 통째 전달일 때만)
fn compat_input(command: &str, args: &Value, bench: &str) -> (OperationId, Value, Option<Value>) {
    let wd = || s(args, "workingDirectory");
    let orchestration_request = |op| {
        (
            op,
            json!({ "benchId": bench, "request": args["input"] }),
            None,
        )
    };
    match command {
        "list_projects" => (
            OperationId::ProjectList,
            compat::list_projects_request(workbench_protocol::RequestId::random()).input,
            None,
        ),
        "create_project" => (
            OperationId::ProjectCreate,
            compat::create_project_request(
                arg(args, "input"),
                workbench_protocol::IdempotencyKey::random(),
                workbench_protocol::RequestId::random(),
            )
            .input,
            None,
        ),
        "update_project" => (
            OperationId::ProjectUpdate,
            compat::project_update_input(s(args, "id"), arg(args, "input")),
            None,
        ),
        "delete_project" => (
            OperationId::ProjectDelete,
            compat::project_delete_input(s(args, "id")),
            None,
        ),
        "list_saved_prompts" => (OperationId::SavedPromptList, json!({}), None),
        "create_saved_prompt" => (
            OperationId::SavedPromptCreate,
            compat::saved_prompt_create_input(arg(args, "input")),
            None,
        ),
        "update_saved_prompt" => (
            OperationId::SavedPromptUpdate,
            compat::saved_prompt_update_input(s(args, "id"), arg(args, "input")),
            None,
        ),
        "delete_saved_prompt" => (
            OperationId::SavedPromptDelete,
            compat::saved_prompt_delete_input(s(args, "id")),
            None,
        ),
        "get_goal" => (OperationId::GoalGet, compat::goal_get_input(wd()), None),
        "create_goal" => (
            OperationId::GoalCreate,
            compat::goal_create_input(arg(args, "input")),
            None,
        ),
        "update_goal" => (
            OperationId::GoalUpdate,
            compat::goal_update_input(wd(), arg(args, "input")),
            None,
        ),
        "clear_goal" => (OperationId::GoalClear, compat::goal_clear_input(wd()), None),
        "record_goal_progress" => (
            OperationId::GoalRecordProgress,
            compat::goal_record_progress_input(wd(), arg(args, "input")),
            None,
        ),
        "get_agent_run_settings" => (
            OperationId::AgentRunSettingsGet,
            compat::agent_run_settings_get_input(wd()),
            None,
        ),
        "save_agent_run_settings" => {
            let settings: AgentRunSettings = arg(args, "settings");
            let expected = compat::agent_run_settings_save_input(settings);
            let wire = json!({ "settings": args["settings"] });
            (OperationId::AgentRunSettingsSave, expected, Some(wire))
        }
        "list_git_remotes" => (
            OperationId::GitListRemotes,
            compat::working_directory_input(wd()),
            None,
        ),
        "list_git_branches" => (
            OperationId::GitListBranches,
            compat::working_directory_input(wd()),
            None,
        ),
        "list_git_worktrees" => (
            OperationId::GitListWorktrees,
            compat::git_list_worktrees_input(wd(), arg(args, "includeStatus")),
            None,
        ),
        "list_worktree_changes" => (
            OperationId::WorktreeListChanges,
            compat::working_directory_input(wd()),
            None,
        ),
        "create_git_worktree" => (
            OperationId::GitCreateWorktree,
            compat::git_create_worktree_input(wd(), arg::<GitWorktreeCreateDraft>(args, "input")),
            None,
        ),
        "delete_git_worktree" => (
            OperationId::GitDeleteWorktree,
            compat::git_delete_worktree_input(wd(), s(args, "path")),
            None,
        ),
        "get_worktree_changes" => (
            OperationId::WorktreeGetChanges,
            compat::working_directory_input(wd()),
            None,
        ),
        "get_worktree_file_diff" => (
            OperationId::WorktreeGetFileDiff,
            compat::worktree_path_input(wd(), s(args, "path")),
            None,
        ),
        "list_worktree_files" => (
            OperationId::WorktreeListFiles,
            compat::worktree_list_files_input(wd(), arg(args, "scope")),
            None,
        ),
        "read_worktree_text_file" => (
            OperationId::WorktreeReadTextFile,
            compat::worktree_path_input(wd(), s(args, "path")),
            None,
        ),
        "list_worktree_git_history" => (
            OperationId::WorktreeListHistory,
            compat::worktree_page_input(
                wd(),
                arg(args, "maxCount"),
                arg(args, "offset"),
                arg(args, "cursor"),
            ),
            None,
        ),
        "get_worktree_git_graph" => (
            OperationId::WorktreeGetGraph,
            compat::worktree_page_input(
                wd(),
                arg(args, "maxCount"),
                arg(args, "offset"),
                arg(args, "cursor"),
            ),
            None,
        ),
        "get_worktree_commit_detail" => (
            OperationId::WorktreeGetCommitDetail,
            compat::worktree_commit_input(wd(), s(args, "commitHash")),
            None,
        ),
        "get_worktree_commit_file_diff" => (
            OperationId::WorktreeGetCommitFileDiff,
            compat::worktree_commit_file_input(wd(), s(args, "commitHash"), s(args, "path")),
            None,
        ),
        "list_agents" => (OperationId::AgentList, compat::agent_list_input(), None),
        "list_provider_sessions" => (
            OperationId::AgentListProviderSessions,
            compat::agent_list_provider_sessions_input(s(args, "agentId"), arg(args, "cwd")),
            None,
        ),
        "start_agent_run" => {
            let request: AgentRunRequest = arg(args, "request");
            let panel: Option<String> = arg(args, "panelId");
            let expected = compat::run_start_input(bench, &request, panel.as_deref());
            let mut wire = json!({ "benchId": bench, "request": args["request"] });
            if let Some(panel) = panel {
                wire["panelId"] = json!(panel);
            }
            (OperationId::RunStart, expected, Some(wire))
        }
        "list_agent_tool_command_candidates" => {
            let query: AgentToolCandidateQuery = arg(args, "input");
            let expected = json!({ "benchId": bench, "query": query });
            let wire = json!({ "benchId": bench, "query": args["input"] });
            (OperationId::RunListToolCandidates, expected, Some(wire))
        }
        "send_prompt_to_run" | "steer_prompt_to_run" | "cancel_current_prompt_and_send_to_run" => (
            match command {
                "send_prompt_to_run" => OperationId::RunSendPrompt,
                "steer_prompt_to_run" => OperationId::RunSteer,
                _ => OperationId::RunCancelAndSend,
            },
            json!({ "benchId": bench, "runId": s(args, "runId"), "prompt": s(args, "prompt") }),
            None,
        ),
        "set_run_permission_mode" => {
            let mode: PermissionMode = arg(args, "permissionMode");
            (
                OperationId::RunSetPermissionMode,
                json!({ "benchId": bench, "runId": s(args, "runId"), "mode": mode }),
                None,
            )
        }
        "cancel_agent_run" => (
            OperationId::RunCancel,
            json!({ "benchId": bench, "runId": s(args, "runId") }),
            None,
        ),
        "respond_agent_permission" => (
            OperationId::RunRespondPermission,
            json!({ "benchId": bench, "runId": s(args, "runId"), "permissionId": s(args, "permissionId"), "optionId": s(args, "optionId") }),
            None,
        ),
        "sync_agent_workspace" => {
            let request: AgentWorkspaceSyncRequest = arg(args, "request");
            (
                OperationId::ExchangeSyncWorkspace,
                json!({ "benchId": bench, "request": request }),
                Some(json!({ "benchId": bench, "request": args["request"] })),
            )
        }
        "send_agent_exchange" => {
            let request: SendAgentExchangeRequest = arg(args, "request");
            (
                OperationId::ExchangeSend,
                json!({ "benchId": bench, "request": request }),
                Some(json!({ "benchId": bench, "request": args["request"] })),
            )
        }
        "acknowledge_agent_exchange" => {
            let request: AgentExchangeAckRequest = arg(args, "request");
            (
                OperationId::ExchangeAcknowledge,
                json!({ "benchId": bench, "request": request }),
                Some(json!({ "benchId": bench, "request": args["request"] })),
            )
        }
        "discard_agent_exchange_delivery" => (
            OperationId::ExchangeDiscardDelivery,
            json!({ "benchId": bench, "requestId": s(args, "requestId") }),
            None,
        ),
        "list_agent_exchanges" => (OperationId::ExchangeList, json!({ "benchId": bench }), None),
        "bootstrap_orchestration_workspace" => {
            let input: BootstrapOrchestrationInput = arg(args, "input");
            (
                OperationId::OrchestrationBootstrap,
                json!({ "benchId": bench, "worktreePath": input.worktree_path, "resumeWorkspaceId": input.resume_workspace_id }),
                None,
            )
        }
        "get_orchestration_workspace" => (
            OperationId::OrchestrationGet,
            json!({ "benchId": bench }),
            None,
        ),
        "list_recoverable_orchestration_workspaces" => {
            let input: ListRecoverableOrchestrationInput = arg(args, "input");
            (
                OperationId::OrchestrationListRecoverable,
                json!({ "benchId": bench, "worktreePath": input.worktree_path }),
                None,
            )
        }
        "delegate_orchestration_goal" => {
            orchestration_request(OperationId::OrchestrationDelegateGoal)
        }
        "adopt_manual_orchestration_child" => {
            let input: AdoptManualChildInput = arg(args, "input");
            (
                OperationId::OrchestrationAdoptManualChild,
                json!({ "benchId": bench, "panelId": input.panel_id, "title": input.title }),
                None,
            )
        }
        "list_orchestration_tasks" => {
            let input: ListOrchestrationTasksInput = arg(args, "input");
            (
                OperationId::OrchestrationListTasks,
                json!({ "benchId": bench, "generationId": input.generation_id }),
                None,
            )
        }
        "collect_orchestration_reports" => (
            OperationId::OrchestrationCollectReports,
            json!({ "benchId": bench }),
            None,
        ),
        "send_orchestration_child_command" => (
            OperationId::OrchestrationSendChildCommand,
            json!({ "benchId": bench, "input": args["input"] }),
            None,
        ),
        "respond_orchestration_input" => {
            orchestration_request(OperationId::OrchestrationRespondInput)
        }
        "replay_orchestration_runtime_events" => {
            let input: ReplayRuntimeEventsInput = arg(args, "input");
            (
                OperationId::RunReplay,
                json!({ "benchId": bench, "runId": input.run_id, "afterSequence": input.after_sequence }),
                None,
            )
        }
        "dispatch_orchestration_prompt" => {
            orchestration_request(OperationId::OrchestrationDispatchPrompt)
        }
        "recover_orchestration_workspace" => (
            OperationId::OrchestrationRecover,
            json!({ "benchId": bench }),
            None,
        ),
        "bind_main_coordinator_run" => {
            orchestration_request(OperationId::OrchestrationBindCoordinator)
        }
        "set_orchestration_presentation" => {
            orchestration_request(OperationId::OrchestrationSetPresentation)
        }
        "cancel_orchestration_task" => orchestration_request(OperationId::OrchestrationCancelTask),
        "retry_orchestration_task" => orchestration_request(OperationId::OrchestrationRetryTask),
        "reassign_orchestration_task" => {
            orchestration_request(OperationId::OrchestrationReassignTask)
        }
        "handoff_orchestration_coordinator" => {
            orchestration_request(OperationId::OrchestrationHandoffCoordinator)
        }
        other => panic!("golden case for an unknown command: {other}"),
    }
}

/// DTO 통째 전달: 화면 원본과 compat 입력이 서버에게 같은 입력인지.
fn assert_semantically_equal(operation: OperationId, wire: &Value, expected: &Value) {
    let (left, right) = match operation {
        OperationId::AgentRunSettingsSave => (
            decoded::<agent_run_settings::AgentRunSettingsSaveInput>(wire),
            decoded::<agent_run_settings::AgentRunSettingsSaveInput>(expected),
        ),
        OperationId::RunStart => (
            decoded::<run::RunStartInput>(wire),
            decoded::<run::RunStartInput>(expected),
        ),
        OperationId::RunListToolCandidates => (
            decoded::<run::RunListToolCandidatesInput>(wire),
            decoded::<run::RunListToolCandidatesInput>(expected),
        ),
        OperationId::ExchangeSyncWorkspace => (
            decoded::<exchange::ExchangeSyncWorkspaceInput>(wire),
            decoded::<exchange::ExchangeSyncWorkspaceInput>(expected),
        ),
        OperationId::ExchangeSend => (
            decoded::<exchange::ExchangeSendInput>(wire),
            decoded::<exchange::ExchangeSendInput>(expected),
        ),
        OperationId::ExchangeAcknowledge => (
            decoded::<exchange::ExchangeAcknowledgeInput>(wire),
            decoded::<exchange::ExchangeAcknowledgeInput>(expected),
        ),
        other => panic!("no semantic decoder for {}", other.as_str()),
    };
    assert_eq!(
        left,
        right,
        "{} wire and compat input differ for the server",
        operation.as_str()
    );
}

#[test]
fn compat_parity_golden_matches_the_compat_input_builders() {
    let path = golden_path();
    let mut golden: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut changed = false;
    let mut commands = std::collections::BTreeSet::new();
    for case in golden["cases"].as_array_mut().unwrap() {
        let command = case["command"].as_str().unwrap().to_owned();
        commands.insert(command.clone());
        let bench = case["benchId"].as_str().unwrap_or("").to_owned();
        let (operation, expected, wire) = compat_input(&command, &case["args"], &bench);
        if let Some(wire) = &wire {
            assert_semantically_equal(operation, wire, &expected);
        }
        let mut computed = json!({ "operation": operation.as_str(), "input": expected });
        if let Some(wire) = wire {
            computed["wire"] = wire;
        }
        if case["expected"] != computed {
            changed = true;
            case["expected"] = computed;
        }
    }
    if changed {
        if std::env::var("UPDATE_GOLDEN").as_deref() == Ok("1") {
            std::fs::write(&path, serde_json::to_string_pretty(&golden).unwrap() + "\n").unwrap();
        } else {
            panic!("compat parity golden is stale: rerun with UPDATE_GOLDEN=1 and review the diff");
        }
    }
    // 서버 소유 command 전부를 덮는다(인벤토리 `reviews/command-inventory.md`의 S 표, 044 Codex r7에서 교환 전달 포기 추가).
    assert_eq!(commands.len(), 62, "commands covered: {commands:?}");
}
