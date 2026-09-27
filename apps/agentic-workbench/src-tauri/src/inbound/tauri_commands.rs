use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    process::Command,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager, State};
use workbench_core::{
    application::workbench_runtime::WorkbenchRuntime, infrastructure::event_hub::RunReplay,
};
use workbench_protocol::{
    EventItem, OperationId, StreamCursor, Subscription, Workbench, events::StreamKind,
};

use crate::inbound::workbench_compat;
use crate::{
    application::{
        appearance_preferences_service::AppearancePreferencesService,
        worktree_workspace_layout_service,
    },
    domain::{
        agent::AgentDescriptor,
        agent_run_settings::AgentRunSettings,
        agent_tool_candidate::{AgentToolCandidateQuery, AgentToolCandidateResponse},
        appearance_preferences::AppearancePreferences,
        git_branch::GitBranch,
        git_remote::GitRemote,
        git_worktree::{GitWorktree, GitWorktreeCreateDraft},
        git_worktree_changes::{GitWorktreeChanges, GitWorktreeFileDiff},
        goal::{GoalStatus, ThreadGoal},
        project::Project,
        provider_session::ProviderSession,
        run::{AgentRun, AgentRunRequest, PermissionMode},
        saved_prompt::SavedPrompt,
        worktree_change::WorktreeChange,
        worktree_file::{WorktreeFileEntry, WorktreeFileListScope, WorktreeTextFile},
        worktree_git::{
            GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff as WorktreeGitFileDiff,
        },
        worktree_workspace_layout::WorkspaceLayoutSettings,
    },
    infrastructure::{
        desktop_benches,
        json_appearance_preferences_repository::JsonAppearancePreferencesRepository,
        json_worktree_workspace_layout_repository::JsonWorkspaceLayoutRepository,
        perf_log::{log_async_command, log_async_command_error, run_blocking_command},
        window_manager,
    },
};

#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use workbench_host::{
    launch::inject_mcp_launch_env,
    mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpLaunchEnv},
};

const WORKTREE_CHANGED_EVENT: &str = "workspace://worktree-changed";
pub const APPEARANCE_PREFERENCES_CHANGED_EVENT: &str = "app://appearance-preferences-changed";
pub type AppearancePreferencesState =
    AppearancePreferencesService<JsonAppearancePreferencesRepository>;

#[tauri::command]
pub fn get_appearance_preferences(
    state: State<'_, AppearancePreferencesState>,
) -> Result<AppearancePreferences, String> {
    state.get()
}

#[tauri::command]
pub fn set_font_size_step(
    app: AppHandle,
    state: State<'_, AppearancePreferencesState>,
    font_size_step: i8,
) -> Result<AppearancePreferences, String> {
    broadcast_appearance_preferences(&app, state.set_font_size_step(font_size_step)?)
}

#[tauri::command]
pub fn adjust_font_size_step(
    app: AppHandle,
    state: State<'_, AppearancePreferencesState>,
    delta: i8,
) -> Result<AppearancePreferences, String> {
    broadcast_appearance_preferences(&app, state.adjust_font_size_step(delta)?)
}

fn broadcast_appearance_preferences(
    app: &AppHandle,
    preferences: AppearancePreferences,
) -> Result<AppearancePreferences, String> {
    app.emit(APPEARANCE_PREFERENCES_CHANGED_EVENT, preferences)
        .map_err(|error| format!("Failed to notify windows of appearance preferences: {error}"))?;
    Ok(preferences)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapOrchestrationInput {
    worktree_path: String,
    resume_workspace_id: Option<String>,
}

// ---- 041: orchestration command 18개는 `orchestration.*`·`run.replay` 호환 어댑터다 ----
// 창은 작업대(Bench)로 바뀌고 흐름은 core `OrchestrationRuntime`에 있다(specs/041-workbench-orchestration/contracts/
// tauri-compat.md). 인자·반환·오류 문자열은 오늘과 같다: 오류는 fault `details.orchestrationError` 원본의 JSON
// 문자열(없으면 문구 그대로), 결과의 `boundWindowLabel`은 이 창 label로 다시 채우고 `eventStreamId`는 뺀다.

/// fault → 오늘 orchestration command 오류 문자열.
fn orchestration_fault_string(fault: &workbench_protocol::WorkbenchFault) -> String {
    fault
        .details
        .as_ref()
        .and_then(|details| details.get("orchestrationError"))
        .map(|error| error.to_string())
        .unwrap_or_else(|| fault.message.clone())
}

async fn orchestration_call(
    app: &AppHandle,
    window: &tauri::Window,
    operation: OperationId,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let caller = caller(app, window)?;
    let request = if matches!(
        workbench_protocol::operations::spec_for(operation).kind,
        workbench_protocol::OperationKind::Query
    ) {
        workbench_compat::query_request(operation, input)
    } else {
        workbench_compat::command_request(operation, input)
    };
    match caller.runtime.call(caller.principal.clone(), request).await {
        Ok(reply) => workbench_compat::decode_output(reply),
        Err(fault) => Err(orchestration_fault_string(&fault)),
    }
}

/// 창의 작업대. 없으면 연다(창의 Worktree 경로, 없으면 `hint`).
async fn orchestration_bench(
    app: &AppHandle,
    window: &tauri::Window,
    hint: Option<&str>,
) -> Result<String, String> {
    desktop_benches::ensure(&caller(app, window)?, window.label(), hint).await
}

/// core 작업 영역 DTO → 오늘 결과 형태. `bound`면 이 창 label을 채운다.
fn session_for_window(mut session: serde_json::Value, label: Option<&str>) -> serde_json::Value {
    if let Some(object) = session.as_object_mut() {
        object.remove("eventStreamId");
        object.insert("boundWindowLabel".into(), json!(label));
    }
    session
}

/// 작업대에 묶인 작업 영역을 돌려주는 operation의 결과(이 창 label로 묶임 표시).
async fn bound_session(
    app: &AppHandle,
    window: &tauri::Window,
    operation: OperationId,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let session = orchestration_call(app, window, operation, input).await?;
    Ok(session_for_window(session, Some(window.label())))
}

#[tauri::command]
pub async fn bootstrap_orchestration_workspace(
    app: AppHandle,
    window: tauri::Window,
    input: BootstrapOrchestrationInput,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, Some(&input.worktree_path)).await?;
    bound_session(
        &app,
        &window,
        OperationId::OrchestrationBootstrap,
        json!({ "benchId": bench, "worktreePath": input.worktree_path,
                "resumeWorkspaceId": input.resume_workspace_id }),
    )
    .await
}

#[tauri::command]
pub async fn get_orchestration_workspace(
    app: AppHandle,
    window: tauri::Window,
) -> Result<serde_json::Value, String> {
    // 작업대가 없는 창에는 묶인 작업 영역도 없다(오늘 `None`).
    let Some(bench) = desktop_benches::lookup(window.label()) else {
        return Ok(serde_json::Value::Null);
    };
    let session = orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationGet,
        json!({ "benchId": bench }),
    )
    .await?;
    Ok(if session.is_null() {
        session
    } else {
        session_for_window(session, Some(window.label()))
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRecoverableOrchestrationInput {
    worktree_path: String,
}

#[tauri::command]
pub async fn list_recoverable_orchestration_workspaces(
    app: AppHandle,
    window: tauri::Window,
    input: ListRecoverableOrchestrationInput,
) -> Result<serde_json::Value, String> {
    // 사라진 창의 작업 영역은 창 `Destroyed` → 작업대 닫기가 이미 풀었다(오늘의 정리 단계가 필요 없다).
    let bench = orchestration_bench(&app, &window, Some(&input.worktree_path)).await?;
    let sessions = orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationListRecoverable,
        json!({ "benchId": bench, "worktreePath": input.worktree_path }),
    )
    .await?;
    Ok(match sessions {
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(|session| session_for_window(session, None))
                .collect(),
        ),
        other => other,
    })
}

/// `{benchId, request}` 모양 command 하나(작업 영역 결과).
macro_rules! session_request_command {
    ($name:ident, $operation:expr) => {
        #[tauri::command]
        pub async fn $name(
            app: AppHandle,
            window: tauri::Window,
            input: serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            let bench = orchestration_bench(&app, &window, None).await?;
            bound_session(
                &app,
                &window,
                $operation,
                json!({ "benchId": bench, "request": input }),
            )
            .await
        }
    };
}

session_request_command!(
    bind_main_coordinator_run,
    OperationId::OrchestrationBindCoordinator
);
session_request_command!(
    set_orchestration_presentation,
    OperationId::OrchestrationSetPresentation
);
session_request_command!(
    cancel_orchestration_task,
    OperationId::OrchestrationCancelTask
);
session_request_command!(
    retry_orchestration_task,
    OperationId::OrchestrationRetryTask
);
session_request_command!(
    reassign_orchestration_task,
    OperationId::OrchestrationReassignTask
);
session_request_command!(
    handoff_orchestration_coordinator,
    OperationId::OrchestrationHandoffCoordinator
);

#[tauri::command]
pub async fn delegate_orchestration_goal(
    app: AppHandle,
    window: tauri::Window,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, None).await?;
    orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationDelegateGoal,
        json!({ "benchId": bench, "request": input }),
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptManualChildInput {
    panel_id: String,
    title: String,
}

#[tauri::command]
pub async fn adopt_manual_orchestration_child(
    app: AppHandle,
    window: tauri::Window,
    input: AdoptManualChildInput,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, None).await?;
    bound_session(
        &app,
        &window,
        OperationId::OrchestrationAdoptManualChild,
        json!({ "benchId": bench, "panelId": input.panel_id, "title": input.title }),
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrchestrationTasksInput {
    generation_id: String,
}

#[tauri::command]
pub async fn list_orchestration_tasks(
    app: AppHandle,
    window: tauri::Window,
    input: ListOrchestrationTasksInput,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, None).await?;
    orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationListTasks,
        json!({ "benchId": bench, "generationId": input.generation_id }),
    )
    .await
}

#[tauri::command]
pub async fn collect_orchestration_reports(
    app: AppHandle,
    window: tauri::Window,
) -> Result<serde_json::Value, String> {
    // 작업 영역이 없으면 빈 목록(오늘과 같다).
    let Some(bench) = desktop_benches::lookup(window.label()) else {
        return Ok(json!([]));
    };
    orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationCollectReports,
        json!({ "benchId": bench }),
    )
    .await
}

#[tauri::command]
pub async fn send_orchestration_child_command(
    app: AppHandle,
    window: tauri::Window,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, None).await?;
    orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationSendChildCommand,
        json!({ "benchId": bench, "input": input }),
    )
    .await
}

#[tauri::command]
pub async fn respond_orchestration_input(
    app: AppHandle,
    window: tauri::Window,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, None).await?;
    orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationRespondInput,
        json!({ "benchId": bench, "request": input }),
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRuntimeEventsInput {
    run_id: String,
    after_sequence: u64,
}

/// 결과는 오늘처럼 항상 `RunReplay`다. 창에 작업대가 없거나 이 작업대가 재생할 수 없는 run이면 오늘의 Missing
/// 형태 — 다른 창의 run이 재생되던 누수를 막는다(contracts tauri-compat).
#[tauri::command]
pub async fn replay_orchestration_runtime_events(
    app: AppHandle,
    window: tauri::Window,
    input: ReplayRuntimeEventsInput,
) -> Result<RunReplay, String> {
    let missing = RunReplay {
        run_id: input.run_id.clone(),
        events: Vec::new(),
        last_sequence: 0,
        terminal: false,
        gap_detected: input.after_sequence > 0,
    };
    let Some(bench) = desktop_benches::lookup(window.label()) else {
        return Ok(missing);
    };
    let replay = orchestration_call(
        &app,
        &window,
        OperationId::RunReplay,
        json!({ "benchId": bench, "runId": input.run_id, "afterSequence": input.after_sequence }),
    )
    .await;
    Ok(replay
        .ok()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or(missing))
}

#[tauri::command]
pub async fn dispatch_orchestration_prompt(
    app: AppHandle,
    window: tauri::Window,
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, None).await?;
    orchestration_call(
        &app,
        &window,
        OperationId::OrchestrationDispatchPrompt,
        json!({ "benchId": bench, "request": input }),
    )
    .await
}

#[tauri::command]
pub async fn recover_orchestration_workspace(
    app: AppHandle,
    window: tauri::Window,
) -> Result<serde_json::Value, String> {
    let bench = orchestration_bench(&app, &window, None).await?;
    bound_session(
        &app,
        &window,
        OperationId::OrchestrationRecover,
        json!({ "benchId": bench }),
    )
    .await
}

/// 창별 worktree 구독 task(039 US3). task를 abort하면 구독 스트림이 drop되어 hub의 감시 참조 수가 내려간다.
pub struct WorktreeWatcherState {
    handles: Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
}

use workbench_core::domain::agent_exchange::{
    AgentExchangeAckRequest, AgentWorkspaceSyncRequest, SendAgentExchangeRequest,
};
use workbench_protocol::operations::exchange::{AgentExchangeDto, AgentWorkspaceSyncResponseDto};

// 040 US2: 교환 command 4개는 `exchange.*` 호환 어댑터다. 작업 영역은 창의 작업대에 묶이고, 오류는 오늘과 같은
// `{code, message}` JSON 문자열이다(도메인 코드는 fault `details.exchangeCode`).
fn exchange_command_error(fault: &workbench_protocol::WorkbenchFault) -> String {
    match fault
        .details
        .as_ref()
        .and_then(|details| details.get("exchangeCode"))
        .and_then(serde_json::Value::as_str)
    {
        Some(code) => workbench_compat::exchange_error_string(code, &fault.message),
        None => fault.message.clone(),
    }
}

async fn call_exchange<Out: serde::de::DeserializeOwned>(
    app: &AppHandle,
    window: &tauri::Window,
    request: workbench_protocol::CallRequest,
) -> Result<Out, String> {
    let caller = caller(app, window)?;
    match caller.runtime.call(caller.principal.clone(), request).await {
        Ok(reply) => workbench_compat::decode_output(reply),
        Err(fault) => Err(exchange_command_error(&fault)),
    }
}

fn unregistered_workspace() -> String {
    workbench_compat::exchange_error_string(
        "unknownWorkspace",
        "Agent workspace is not registered.",
    )
}

#[tauri::command]
pub async fn sync_agent_workspace(
    app: AppHandle,
    window: tauri::Window,
    request: AgentWorkspaceSyncRequest,
) -> Result<AgentWorkspaceSyncResponseDto, String> {
    let runtime = caller(&app, &window)?;
    let bench =
        desktop_benches::ensure(&runtime, window.label(), Some(&request.worktree_path)).await?;
    call_exchange(
        &app,
        &window,
        workbench_compat::command_request(
            OperationId::ExchangeSyncWorkspace,
            json!({ "benchId": bench, "request": request }),
        ),
    )
    .await
}

#[tauri::command]
pub async fn send_agent_exchange(
    app: AppHandle,
    window: tauri::Window,
    request: SendAgentExchangeRequest,
) -> Result<AgentExchangeDto, String> {
    let bench = desktop_benches::lookup(window.label()).ok_or_else(unregistered_workspace)?;
    call_exchange(
        &app,
        &window,
        workbench_compat::command_request(
            OperationId::ExchangeSend,
            json!({ "benchId": bench, "request": request }),
        ),
    )
    .await
}

#[tauri::command]
pub async fn acknowledge_agent_exchange(
    app: AppHandle,
    window: tauri::Window,
    request: AgentExchangeAckRequest,
) -> Result<AgentExchangeDto, String> {
    let bench = desktop_benches::lookup(window.label()).ok_or_else(|| {
        workbench_compat::exchange_error_string("unknownExchange", "Exchange was not found.")
    })?;
    call_exchange(
        &app,
        &window,
        workbench_compat::command_request(
            OperationId::ExchangeAcknowledge,
            json!({ "benchId": bench, "request": request }),
        ),
    )
    .await
}

#[tauri::command]
pub async fn list_agent_exchanges(
    app: AppHandle,
    window: tauri::Window,
) -> Result<Vec<AgentExchangeDto>, String> {
    let Some(bench) = desktop_benches::lookup(window.label()) else {
        return Ok(Vec::new());
    };
    call_exchange(
        &app,
        &window,
        workbench_compat::query_request(OperationId::ExchangeList, json!({ "benchId": bench })),
    )
    .await
}

impl WorktreeWatcherState {
    pub fn new() -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
        }
    }

    pub fn stop_for_window(&self, window_label: &str) -> Result<(), String> {
        let mut handles = self
            .handles
            .lock()
            .map_err(|error| format!("Failed to lock worktree watcher state: {error}"))?;
        if let Some(task) = handles.remove(window_label) {
            task.abort();
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub(crate) name: String,
    pub(crate) working_directory: String,
    pub(crate) description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPromptInput {
    pub(crate) label: String,
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalInput {
    pub(crate) working_directory: String,
    pub(crate) objective: String,
    pub(crate) token_budget: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalUpdateInput {
    pub(crate) objective: Option<String>,
    pub(crate) status: Option<GoalStatus>,
    pub(crate) token_budget: Option<Option<usize>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalProgressInput {
    pub(crate) tokens_used: usize,
    pub(crate) time_used_seconds: u64,
}

/// 앱 안의 런타임(embedded 모드). 외부 서버 모드에는 없다 — 호환 command는 정해진 오류를 돌려준다(044 T029).
fn workbench_runtime(app: &AppHandle) -> Result<Arc<WorkbenchRuntime>, String> {
    app.try_state::<Arc<WorkbenchRuntime>>()
        .map(|runtime| runtime.inner().clone())
        .ok_or_else(|| {
            crate::infrastructure::workbench_mode::MESSAGE_EXTERNAL_UNAVAILABLE.to_owned()
        })
}

fn workbench_mode(app: &AppHandle) -> crate::infrastructure::workbench_mode::WorkbenchMode {
    app.try_state::<crate::infrastructure::workbench_mode::WorkbenchMode>()
        .map(|mode| *mode)
        .unwrap_or(crate::infrastructure::workbench_mode::WorkbenchMode::Embedded)
}

fn external_server(
    app: &AppHandle,
) -> Option<Arc<crate::infrastructure::server_client::ExternalServer>> {
    app.try_state::<Arc<crate::infrastructure::server_client::ExternalServer>>()
        .map(|server| server.inner().clone())
}

/// 호환 경로 호출자: 호출한 창의 주체로 부른다(043 D2 — 네트워크 경로와 같은 창 주체, 작업대 소유가 창별로 갈린다).
/// 창 등록이 없으면(닫힌 뒤 늦게 도는 command) 거절한다 — 새 주체를 만들지 않는다(`window_principals`).
fn caller(app: &AppHandle, window: &tauri::Window) -> Result<workbench_compat::Caller, String> {
    let principal =
        crate::infrastructure::window_principals::current(window.label()).ok_or_else(|| {
            crate::infrastructure::window_principals::MESSAGE_WINDOW_NOT_REGISTERED.to_owned()
        })?;
    Ok(workbench_compat::Caller {
        runtime: workbench_runtime(app)?,
        principal,
    })
}

// 037·038: 아래 command들은 `Workbench.call`을 거치는 호환 어댑터다. 시그니처·직렬화·오류 문구는 이전과 같다
// (specs/038-workbench-domains/contracts/tauri-compat-commands.md). 저장소·업무 로직은 workbench-core에 있다.
#[tauri::command]
pub async fn list_projects(app: AppHandle, window: tauri::Window) -> Result<Vec<Project>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_projects",
        workbench_compat::call_list_projects(&runtime),
    )
    .await
}

#[tauri::command]
pub async fn create_project(
    app: AppHandle,
    window: tauri::Window,
    input: ProjectInput,
) -> Result<Project, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "create_project",
        workbench_compat::call_create_project(&runtime, input),
    )
    .await
}

#[tauri::command]
pub async fn update_project(
    app: AppHandle,
    window: tauri::Window,
    id: String,
    input: ProjectInput,
) -> Result<Project, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "update_project",
        workbench_compat::call_command(
            &runtime,
            OperationId::ProjectUpdate,
            workbench_compat::project_update_input(id, input),
        ),
    )
    .await
}

#[tauri::command]
pub async fn delete_project(
    app: AppHandle,
    window: tauri::Window,
    id: String,
) -> Result<(), String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "delete_project",
        workbench_compat::call_command(
            &runtime,
            OperationId::ProjectDelete,
            workbench_compat::project_delete_input(id),
        ),
    )
    .await
}

#[tauri::command]
pub async fn list_saved_prompts(
    app: AppHandle,
    window: tauri::Window,
) -> Result<Vec<SavedPrompt>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_saved_prompts",
        workbench_compat::call_query(&runtime, OperationId::SavedPromptList, json!({})),
    )
    .await
}

#[tauri::command]
pub async fn create_saved_prompt(
    app: AppHandle,
    window: tauri::Window,
    input: SavedPromptInput,
) -> Result<SavedPrompt, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "create_saved_prompt",
        workbench_compat::call_command(
            &runtime,
            OperationId::SavedPromptCreate,
            workbench_compat::saved_prompt_create_input(input),
        ),
    )
    .await
}

#[tauri::command]
pub async fn update_saved_prompt(
    app: AppHandle,
    window: tauri::Window,
    id: String,
    input: SavedPromptInput,
) -> Result<SavedPrompt, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "update_saved_prompt",
        workbench_compat::call_command(
            &runtime,
            OperationId::SavedPromptUpdate,
            workbench_compat::saved_prompt_update_input(id, input),
        ),
    )
    .await
}

#[tauri::command]
pub async fn delete_saved_prompt(
    app: AppHandle,
    window: tauri::Window,
    id: String,
) -> Result<(), String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "delete_saved_prompt",
        workbench_compat::call_command(
            &runtime,
            OperationId::SavedPromptDelete,
            workbench_compat::saved_prompt_delete_input(id),
        ),
    )
    .await
}

#[tauri::command]
pub async fn get_goal(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
) -> Result<Option<ThreadGoal>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "get_goal",
        workbench_compat::call_query(
            &runtime,
            OperationId::GoalGet,
            workbench_compat::goal_get_input(working_directory),
        ),
    )
    .await
}

#[tauri::command]
pub async fn create_goal(
    app: AppHandle,
    window: tauri::Window,
    input: GoalInput,
) -> Result<ThreadGoal, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "create_goal",
        workbench_compat::call_command(
            &runtime,
            OperationId::GoalCreate,
            workbench_compat::goal_create_input(input),
        ),
    )
    .await
}

#[tauri::command]
pub async fn update_goal(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    input: GoalUpdateInput,
) -> Result<ThreadGoal, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "update_goal",
        workbench_compat::call_command(
            &runtime,
            OperationId::GoalUpdate,
            workbench_compat::goal_update_input(working_directory, input),
        ),
    )
    .await
}

#[tauri::command]
pub async fn clear_goal(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
) -> Result<(), String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "clear_goal",
        workbench_compat::call_command(
            &runtime,
            OperationId::GoalClear,
            workbench_compat::goal_clear_input(working_directory),
        ),
    )
    .await
}

#[tauri::command]
pub async fn record_goal_progress(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    input: GoalProgressInput,
) -> Result<ThreadGoal, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "record_goal_progress",
        workbench_compat::call_command(
            &runtime,
            OperationId::GoalRecordProgress,
            workbench_compat::goal_record_progress_input(working_directory, input),
        ),
    )
    .await
}

#[tauri::command]
pub async fn get_agent_run_settings(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
) -> Result<Option<AgentRunSettings>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "get_agent_run_settings",
        workbench_compat::call_query(
            &runtime,
            OperationId::AgentRunSettingsGet,
            workbench_compat::agent_run_settings_get_input(working_directory),
        ),
    )
    .await
}

#[tauri::command]
pub async fn save_agent_run_settings(
    app: AppHandle,
    window: tauri::Window,
    settings: AgentRunSettings,
) -> Result<AgentRunSettings, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "save_agent_run_settings",
        workbench_compat::call_command(
            &runtime,
            OperationId::AgentRunSettingsSave,
            workbench_compat::agent_run_settings_save_input(settings),
        ),
    )
    .await
}

#[tauri::command]
pub fn get_worktree_workspace_layout(
    app: AppHandle,
    working_directory: String,
) -> Result<Option<WorkspaceLayoutSettings>, String> {
    worktree_workspace_layout_service::get_layout(
        &JsonWorkspaceLayoutRepository::from_app(&app)?,
        working_directory,
    )
}

#[tauri::command]
pub fn save_worktree_workspace_layout(
    app: AppHandle,
    layout: WorkspaceLayoutSettings,
) -> Result<WorkspaceLayoutSettings, String> {
    worktree_workspace_layout_service::save_layout(
        &JsonWorkspaceLayoutRepository::from_app(&app)?,
        layout,
    )
}

#[tauri::command]
pub async fn list_git_remotes(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
) -> Result<Vec<GitRemote>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_git_remotes",
        workbench_compat::call_query(
            &runtime,
            OperationId::GitListRemotes,
            workbench_compat::working_directory_input(working_directory),
        ),
    )
    .await
}

#[tauri::command]
pub async fn list_git_branches(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
) -> Result<Vec<GitBranch>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_git_branches",
        workbench_compat::call_query(
            &runtime,
            OperationId::GitListBranches,
            workbench_compat::working_directory_input(working_directory),
        ),
    )
    .await
}

#[tauri::command]
pub async fn list_git_worktrees(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    include_status: Option<bool>,
) -> Result<Vec<GitWorktree>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_git_worktrees",
        workbench_compat::call_query(
            &runtime,
            OperationId::GitListWorktrees,
            workbench_compat::git_list_worktrees_input(working_directory, include_status),
        ),
    )
    .await
}

#[tauri::command]
pub async fn list_worktree_changes(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
) -> Result<Vec<WorktreeChange>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_worktree_changes",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeListChanges,
            workbench_compat::working_directory_input(working_directory),
        ),
    )
    .await
}

#[tauri::command]
pub async fn create_git_worktree(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    input: GitWorktreeCreateDraft,
) -> Result<(), String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "create_git_worktree",
        workbench_compat::call_command(
            &runtime,
            OperationId::GitCreateWorktree,
            workbench_compat::git_create_worktree_input(working_directory, input),
        ),
    )
    .await
}

#[tauri::command]
pub async fn delete_git_worktree(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    path: String,
) -> Result<(), String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "delete_git_worktree",
        workbench_compat::call_command(
            &runtime,
            OperationId::GitDeleteWorktree,
            workbench_compat::git_delete_worktree_input(working_directory, path),
        ),
    )
    .await
}

#[tauri::command]
pub async fn get_worktree_changes(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
) -> Result<GitWorktreeChanges, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "get_worktree_changes",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeGetChanges,
            workbench_compat::working_directory_input(working_directory),
        ),
    )
    .await
}

#[tauri::command]
pub async fn get_worktree_file_diff(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    path: String,
) -> Result<GitWorktreeFileDiff, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "get_worktree_file_diff",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeGetFileDiff,
            workbench_compat::worktree_path_input(working_directory, path),
        ),
    )
    .await
}

#[tauri::command]
pub async fn list_worktree_files(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    scope: Option<WorktreeFileListScope>,
) -> Result<Vec<WorktreeFileEntry>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_worktree_files",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeListFiles,
            workbench_compat::worktree_list_files_input(working_directory, scope),
        ),
    )
    .await
}

#[tauri::command]
pub async fn read_worktree_text_file(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    path: String,
) -> Result<WorktreeTextFile, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "read_worktree_text_file",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeReadTextFile,
            workbench_compat::worktree_path_input(working_directory, path),
        ),
    )
    .await
}

#[tauri::command]
pub async fn start_worktree_watcher(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, WorktreeWatcherState>,
    working_directory: String,
) -> Result<(), String> {
    let window_label = window.label().to_string();
    let target_label = window_label.clone();
    let workbench_compat::Caller { runtime, principal } = caller(&app, &window)?;
    let requested = working_directory.clone();
    // 첫 구독자면 감시를 시작하며 내부에서 `git rev-parse`를 실행하므로 blocking pool에서 구독한다.
    let mut stream = run_blocking_command("start_worktree_watcher", move || {
        let subscription = Subscription {
            cursors: vec![StreamCursor {
                stream_id: StreamKind::Worktree.stream_id(&working_directory),
                epoch: runtime.epoch().to_owned(),
                after_sequence: 0,
            }],
        };
        runtime
            .events(principal, subscription)
            .map_err(|fault| fault.message)
    })
    .await?;
    let task = tauri::async_runtime::spawn(async move {
        while let Some(item) = stream.next_item().await {
            let EventItem::Event { event } = item else {
                continue;
            };
            if let Err(error) = app.emit_to(
                target_label.as_str(),
                WORKTREE_CHANGED_EVENT,
                worktree_changed_payload(event.body, &requested),
            ) {
                eprintln!("Failed to emit worktree change event: {error}");
            }
        }
    });
    let mut handles = state
        .handles
        .lock()
        .map_err(|error| format!("Failed to lock worktree watcher state: {error}"))?;
    if let Some(previous) = handles.insert(window_label, task) {
        previous.abort();
    }
    Ok(())
}

/// 스트림은 실제 경로로 공유되지만 화면은 자신이 넘긴 경로 문자열로 이벤트를 거른다: `workingDirectory`를 되돌린다.
fn worktree_changed_payload(mut body: serde_json::Value, requested: &str) -> serde_json::Value {
    if let Some(object) = body.as_object_mut() {
        object.insert("workingDirectory".into(), requested.into());
    }
    body
}

#[tauri::command]
pub fn stop_worktree_watcher(
    window: tauri::Window,
    state: State<'_, WorktreeWatcherState>,
) -> Result<(), String> {
    state.stop_for_window(window.label())
}

#[tauri::command]
pub async fn list_worktree_git_history(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitHistory, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_worktree_git_history",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeListHistory,
            workbench_compat::worktree_page_input(working_directory, max_count, offset, cursor),
        ),
    )
    .await
}

#[tauri::command]
pub async fn get_worktree_git_graph(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitGraph, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "get_worktree_git_graph",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeGetGraph,
            workbench_compat::worktree_page_input(working_directory, max_count, offset, cursor),
        ),
    )
    .await
}

#[tauri::command]
pub async fn get_worktree_commit_detail(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    commit_hash: String,
) -> Result<GitCommitDetail, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "get_worktree_commit_detail",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeGetCommitDetail,
            workbench_compat::worktree_commit_input(working_directory, commit_hash),
        ),
    )
    .await
}

#[tauri::command]
pub async fn get_worktree_commit_file_diff(
    app: AppHandle,
    window: tauri::Window,
    working_directory: String,
    commit_hash: String,
    path: String,
) -> Result<WorktreeGitFileDiff, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "get_worktree_commit_file_diff",
        workbench_compat::call_query(
            &runtime,
            OperationId::WorktreeGetCommitFileDiff,
            workbench_compat::worktree_commit_file_input(working_directory, commit_hash, path),
        ),
    )
    .await
}

#[tauri::command]
pub async fn list_agents(app: AppHandle, window: tauri::Window) -> Vec<AgentDescriptor> {
    // 오늘 command는 실패하지 않는 `Vec`를 돌려줬다(catalog 오류는 기본값). 시그니처를 유지하기 위해
    // 호출이 실패하면 빈 목록을 돌려주고 오류는 perf 로그에 남긴다(닫힌 창의 늦은 호출 포함).
    let result: Result<Vec<AgentDescriptor>, String> = match caller(&app, &window) {
        Ok(runtime) => {
            log_async_command(
                "list_agents",
                workbench_compat::call_query(
                    &runtime,
                    OperationId::AgentList,
                    workbench_compat::agent_list_input(),
                ),
            )
            .await
        }
        Err(error) => Err(error),
    };
    result.unwrap_or_else(|error| {
        log_async_command_error("list_agents", &error);
        Vec::new()
    })
}

/// 선택한 provider(`agent_id`)가 로컬에 남긴 네이티브 세션을 조회한다.
/// `cwd`가 주어지면 해당 작업 디렉터리의 세션만, 없으면 전체를 돌려준다.
#[tauri::command]
pub async fn list_provider_sessions(
    app: AppHandle,
    window: tauri::Window,
    agent_id: String,
    cwd: Option<String>,
) -> Result<Vec<ProviderSession>, String> {
    let runtime = caller(&app, &window)?;
    log_async_command(
        "list_provider_sessions",
        workbench_compat::call_query(
            &runtime,
            OperationId::AgentListProviderSessions,
            workbench_compat::agent_list_provider_sessions_input(agent_id, cwd),
        ),
    )
    .await
}

#[tauri::command]
pub fn open_worktree_window(
    app: AppHandle,
    project_id: String,
    project_name: String,
    worktree_path: String,
    mode: String,
) -> Result<(), String> {
    window_manager::open_session_window(&app, &project_id, &project_name, &worktree_path, &mode)
}

fn window_origin(window: &tauri::WebviewWindow) -> Result<String, String> {
    let url = window.url().map_err(|error| error.to_string())?;
    crate::infrastructure::workbench_http::origin_of(&url)
        .ok_or_else(|| crate::infrastructure::workbench_http::MESSAGE_ORIGIN_NOT_ALLOWED.to_owned())
}

/// 042·043·044: 이 창의 WebView 출처와 창 주체(incarnation)에 묶인 Workbench HTTP 연결 정보(짧은 토큰). 외부 서버 모드는
/// 서버를 찾거나 띄운 뒤(임대 포함) 소유자 자격 증명으로 창 토큰을 받는다. 출력 모양은 043과 같다.
#[tauri::command]
pub async fn get_workbench_connection(
    app: AppHandle,
    window: tauri::WebviewWindow,
) -> Result<crate::infrastructure::workbench_http::WorkbenchConnection, String> {
    let origin = window_origin(&window)?;
    let label = window.label().to_owned();
    if let Some(server) = external_server(&app) {
        let incarnation = crate::infrastructure::window_principals::incarnation(&label)
            .ok_or_else(|| {
                crate::infrastructure::window_principals::MESSAGE_WINDOW_NOT_REGISTERED.to_owned()
            })?;
        let (base_url, token, expires_at) = server
            .issue_window_token(&label, &incarnation, &origin)
            .await?;
        return Ok(crate::infrastructure::workbench_http::WorkbenchConnection {
            base_url,
            token,
            expires_at,
            incarnation: Some(incarnation),
        });
    }
    app.state::<crate::infrastructure::workbench_http::WorkbenchHttp>()
        .connection_for(&origin, &label)
}

/// 044: 이 앱의 Workbench 모드(`external`·`embedded`). 화면이 부팅 실패 때 호환 경로로 갈지(embedded만) 정한다.
#[tauri::command]
pub fn get_workbench_mode(app: AppHandle) -> String {
    workbench_mode(&app).as_str().to_owned()
}

/// 044 T029: 창 제목 적용(데스크톱 표현): 창 제목과 네이티브 Window 메뉴 동기화. 외부 서버 모드에서는 서버가 창에 넣지
/// 않으므로 화면이 제목 이벤트를 받아 이 command를 부른다(R3).
#[tauri::command]
pub fn apply_window_title(
    app: AppHandle,
    window: tauri::WebviewWindow,
    title: String,
) -> Result<(), String> {
    window
        .set_title(&title)
        .map_err(|error| error.to_string())?;
    crate::infrastructure::native_window_menu::sync_window_menu(&app)
        .map_err(|error| error.to_string())
}

/// 043: 네트워크 경로 창이 자기 작업대 id를 얻는다(호환 경로가 창 label로 넣던 값). `open`이면 없을 때 연다(경로는 창의
/// Worktree, 없으면 `hint`) — 호환 command의 `ensure`와 같다. `open`이 아니면 있을 때만 돌려준다(`lookup`, 없으면 `null`).
/// 작업대는 이 창의 주체로 열린다(작업대 소유 = 창).
#[tauri::command]
pub async fn ensure_window_bench(
    app: AppHandle,
    window: tauri::WebviewWindow,
    open: bool,
    hint: Option<String>,
) -> Result<Option<String>, String> {
    if let Some(server) = external_server(&app) {
        if !open {
            return Ok(desktop_benches::lookup(window.label()));
        }
        let incarnation = crate::infrastructure::window_principals::incarnation(window.label())
            .ok_or_else(|| {
                crate::infrastructure::window_principals::MESSAGE_WINDOW_NOT_REGISTERED.to_owned()
            })?;
        let origin = window_origin(&window)?;
        return desktop_benches::ensure_external(
            &server,
            window.label(),
            &incarnation,
            &origin,
            hint.as_deref(),
        )
        .await
        .map(Some);
    }
    let window = window.as_ref().window();
    let caller = caller(&app, &window)?;
    if !open {
        return Ok(desktop_benches::lookup(window.label()));
    }
    desktop_benches::ensure(&caller, window.label(), hint.as_deref())
        .await
        .map(Some)
}

/// 043: 이 창(현재 incarnation)은 이벤트를 네트워크 구독으로 받는다 — 앱 내부 삽입 전달을 끈다(중복 금지, FR-007).
#[tauri::command]
pub fn declare_network_delivery(
    app: AppHandle,
    window: tauri::Window,
    incarnation: String,
) -> Result<(), String> {
    if workbench_mode(&app) == crate::infrastructure::workbench_mode::WorkbenchMode::External {
        return Ok(()); // 044: 외부 서버 모드에는 삽입 전달이 없다
    }
    crate::infrastructure::tauri_desktop_bridge::declare_network_delivery(
        window.label(),
        &incarnation,
    )
}

/// 창의 페이지가 호환 경로로 부팅했을 때(043): 이전 페이지가 남긴 네트워크 전달 선언을 거둔다.
#[tauri::command]
pub fn withdraw_network_delivery(app: AppHandle, window: tauri::Window) {
    if workbench_mode(&app) == crate::infrastructure::workbench_mode::WorkbenchMode::External {
        return;
    }
    crate::infrastructure::tauri_desktop_bridge::withdraw_network_delivery(window.label());
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    window_manager::open_settings_window(&app)
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    let url = url.trim();
    validate_external_browser_url(url)?;
    open_url_with_system_browser(url)
}

fn validate_external_browser_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    let (scheme, rest) = trimmed
        .split_once(':')
        .ok_or_else(|| "external URL must include a scheme".to_string())?;

    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return Err("only http and https links can be opened externally".to_string());
    }

    if !rest.starts_with("//") {
        return Err("external URL must include a host".to_string());
    }

    let host = rest[2..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim();
    if host.is_empty() {
        return Err("external URL must include a host".to_string());
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn open_url_with_system_browser(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open external URL: {error}"))
}

#[cfg(target_os = "windows")]
fn open_url_with_system_browser(url: &str) -> Result<(), String> {
    Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open external URL: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_url_with_system_browser(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open external URL: {error}"))
}

// 040 US1: run command 8개는 `Workbench.call`의 `run.*` 호환 어댑터다. 창은 작업대(Bench)로 바뀌고 소유 검사는
// 서버가 한다(specs/040-workbench-owners/contracts/tauri-compat.md). 인자·반환·오류 문구는 이전과 같다.
#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    window: tauri::Window,
    request: AgentRunRequest,
    panel_id: Option<String>,
) -> Result<AgentRun, String> {
    let runtime = caller(&app, &window)?;
    let bench = desktop_benches::ensure(&runtime, window.label(), request.cwd.as_deref()).await?;
    workbench_compat::call_command(
        &runtime,
        OperationId::RunStart,
        workbench_compat::run_start_input(&bench, &request, panel_id.as_deref()),
    )
    .await
}

#[tauri::command]
pub async fn list_agent_tool_command_candidates(
    app: AppHandle,
    window: tauri::Window,
    input: AgentToolCandidateQuery,
) -> Result<AgentToolCandidateResponse, String> {
    let runtime = caller(&app, &window)?;
    let bench = desktop_benches::ensure(
        &runtime,
        window.label(),
        Some(input.working_directory.as_str()),
    )
    .await?;
    workbench_compat::call_query(
        &runtime,
        OperationId::RunListToolCandidates,
        json!({ "benchId": bench, "query": input }),
    )
    .await
}

/// 창에 작업대가 없으면 그 창이 소유한 run도 없다(작업대는 처음 run을 시작할 때 열린다).
fn bench_or_inactive(window: &tauri::Window) -> Result<String, String> {
    desktop_benches::lookup(window.label()).ok_or_else(|| "agent run is not active".to_owned())
}

async fn run_prompt_command(
    app: &AppHandle,
    window: &tauri::Window,
    operation: OperationId,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let bench = bench_or_inactive(window)?;
    workbench_compat::call_command(
        &caller(app, window)?,
        operation,
        json!({ "benchId": bench, "runId": run_id, "prompt": prompt }),
    )
    .await
}

#[tauri::command]
pub async fn send_prompt_to_run(
    app: AppHandle,
    window: tauri::Window,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    run_prompt_command(&app, &window, OperationId::RunSendPrompt, run_id, prompt).await
}

#[tauri::command]
pub async fn steer_prompt_to_run(
    app: AppHandle,
    window: tauri::Window,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    run_prompt_command(&app, &window, OperationId::RunSteer, run_id, prompt).await
}

#[tauri::command]
pub async fn cancel_current_prompt_and_send_to_run(
    app: AppHandle,
    window: tauri::Window,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    run_prompt_command(&app, &window, OperationId::RunCancelAndSend, run_id, prompt).await
}

#[tauri::command]
pub async fn set_run_permission_mode(
    app: AppHandle,
    window: tauri::Window,
    run_id: String,
    permission_mode: PermissionMode,
) -> Result<(), String> {
    let bench = bench_or_inactive(&window)?;
    workbench_compat::call_command(
        &caller(&app, &window)?,
        OperationId::RunSetPermissionMode,
        json!({ "benchId": bench, "runId": run_id, "mode": permission_mode }),
    )
    .await
}

#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    window: tauri::Window,
    run_id: String,
) -> Result<(), String> {
    // 오늘처럼 항상 성공: 작업대가 없으면 취소할 run도 없다.
    let Some(bench) = desktop_benches::lookup(window.label()) else {
        return Ok(());
    };
    workbench_compat::call_command(
        &caller(&app, &window)?,
        OperationId::RunCancel,
        json!({ "benchId": bench, "runId": run_id }),
    )
    .await
}

#[tauri::command]
pub async fn respond_agent_permission(
    app: AppHandle,
    window: tauri::Window,
    run_id: String,
    permission_id: String,
    option_id: String,
) -> Result<(), String> {
    let bench = desktop_benches::lookup(window.label())
        .ok_or_else(|| format!("unknown or finished run: {run_id}"))?;
    workbench_compat::call_command(
        &caller(&app, &window)?,
        OperationId::RunRespondPermission,
        json!({
            "benchId": bench,
            "runId": run_id,
            "permissionId": permission_id,
            "optionId": option_id,
        }),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::run::ResumePolicy;

    fn sample_request() -> AgentRunRequest {
        AgentRunRequest {
            goal: "do it".into(),
            agent_id: "codex".into(),
            workspace_id: Some("ws".into()),
            checkout_id: Some("co".into()),
            cwd: Some("/tmp".into()),
            agent_command: None,
            agent_env: None,
            mcp_servers: Vec::new(),
            stdio_buffer_limit_mb: None,
            auto_allow: None,
            run_id: None,
            resume_session_id: Some("sess-1".into()),
            resume_policy: Some(ResumePolicy::ResumeIfAvailable),
            permission_mode: None,
            model_id: None,
            effort_id: None,
            context_size: None,
            ralph_loop: None,
        }
    }

    /// 041: orchestration command 오류는 fault `details.orchestrationError`에서 다시 만든다 — 오늘
    /// `serde_json::to_string(&OrchestrationError)`와 바이트가 같아야 한다(화면이 JSON으로 파싱한다).
    #[test]
    fn orchestration_fault_string_is_byte_identical_to_the_domain_error_json() {
        use workbench_core::domain::agent_orchestration::{
            OrchestrationError, OrchestrationErrorCode,
        };
        let error = OrchestrationError::new(
            OrchestrationErrorCode::NotFound,
            "Orchestration workspace is not bootstrapped.",
        );
        let fault = workbench_protocol::WorkbenchFault::new(
            workbench_protocol::FaultCode::NotFound,
            workbench_protocol::RequestId::random(),
            error.message.clone(),
        )
        .with_details(serde_json::json!({ "orchestrationError": &error }));
        assert_eq!(
            orchestration_fault_string(&fault),
            serde_json::to_string(&error).unwrap()
        );
        let plain = workbench_protocol::WorkbenchFault::new(
            workbench_protocol::FaultCode::PreconditionFailed,
            workbench_protocol::RequestId::random(),
            "Orchestration workspace is not bootstrapped.",
        );
        assert_eq!(
            orchestration_fault_string(&plain),
            "Orchestration workspace is not bootstrapped."
        );
    }

    #[test]
    fn inject_mcp_launch_env_preserves_existing_user_env() {
        let mut request = sample_request();
        request.agent_env = Some(BTreeMap::from([
            ("USER_VALUE".to_string(), "keep".to_string()),
            ("PATH".to_string(), "/custom/bin".to_string()),
        ]));

        inject_mcp_launch_env(
            &mut request,
            McpLaunchEnv {
                url: "http://127.0.0.1:1000/mcp".into(),
                token: "secret".into(),
                run_id: "run-1".into(),
            },
        );

        let env = request.agent_env.unwrap();
        assert_eq!(env.get("USER_VALUE").map(String::as_str), Some("keep"));
        assert_eq!(env.get("PATH").map(String::as_str), Some("/custom/bin"));
        assert_eq!(
            env.get(AW_MCP_URL_ENV).map(String::as_str),
            Some("http://127.0.0.1:1000/mcp")
        );
        assert_eq!(
            env.get(AW_MCP_TOKEN_ENV).map(String::as_str),
            Some("secret")
        );
        assert_eq!(
            env.get(AW_MCP_RUN_ID_ENV).map(String::as_str),
            Some("run-1")
        );
        assert_eq!(request.mcp_servers.len(), 1);
        assert_eq!(
            serde_json::to_value(&request.mcp_servers).unwrap(),
            serde_json::json!([
                {
                    "type": "http",
                    "name": "agentic_workbench",
                    "url": "http://127.0.0.1:1000/mcp",
                    "headers": [
                        {
                            "name": "Authorization",
                            "value": "Bearer secret"
                        }
                    ]
                }
            ])
        );
        assert!(request.goal.contains("Agentic Workbench MCP tools"));
        assert!(request.goal.contains("set_window_title"));
        assert!(request.goal.contains("runId`: `run-1`"));
        assert!(request.goal.contains("User request:\ndo it"));
    }

    #[test]
    fn external_url_validation_allows_http_and_https() {
        assert!(validate_external_browser_url("https://example.com/docs").is_ok());
        assert!(validate_external_browser_url("http://localhost:1420").is_ok());
    }

    #[test]
    fn external_url_validation_rejects_non_browser_schemes() {
        assert!(validate_external_browser_url("javascript:alert(1)").is_err());
        assert!(validate_external_browser_url("file:///tmp/readme.md").is_err());
        assert!(validate_external_browser_url("/relative/path").is_err());
        assert!(validate_external_browser_url("https://").is_err());
        assert!(validate_external_browser_url("https:///docs").is_err());
    }
}

#[cfg(test)]
#[path = "compat_parity_tests.rs"]
mod compat_parity_tests;
