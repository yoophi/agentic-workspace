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
    AuthenticatedPrincipal, EventItem, OperationId, StreamCursor, Subscription, Workbench,
    events::StreamKind,
};

use crate::inbound::workbench_compat;
use crate::{
    application::{
        appearance_preferences_service::AppearancePreferencesService,
        coordinator_notification_dispatcher::CoordinatorNotificationDispatcher,
        orchestration_command_service::{DeliverTaskCommandRequest, OrchestrationCommandService},
        orchestration_service::{
            BindMainRunRequest, CoordinatorHandoffRequest, DelegateGoalOutcome,
            DelegateGoalRequest, DispatchPromptRequest, OrchestrationService,
            SetPresentationRequest, TaskActionRequest,
        },
        send_prompt::SendPromptUseCase,
        worktree_workspace_layout_service,
    },
    domain::{
        agent::AgentDescriptor,
        agent_orchestration::{
            AccessPolicy, MAIN_AGENT_NODE_ID, PromptDelivery, PromptDispatchTargetStatus,
            TaskCommand, TaskCommandKind, TaskCommandSource, TaskReportType, WorkerRuntimeProfile,
        },
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
        acp_agent_worker_adapter::{AcpAgentWorkerAdapter, TauriAcpWorkerRuntime},
        agent_session_registry::AppState,
        desktop_benches,
        json_appearance_preferences_repository::JsonAppearancePreferencesRepository,
        json_orchestration_repository::JsonOrchestrationRepository,
        json_worktree_workspace_layout_repository::JsonWorkspaceLayoutRepository,
        mcp::{McpServerState, capability_registry::CapabilityPrincipal},
        perf_log::{log_async_command, log_async_command_error, run_blocking_command},
        tauri_orchestration_event_sink::TauriOrchestrationEventSink,
        window_manager,
    },
    ports::{
        agent_worker::{AgentWorkerPort, StartWorkerOutcome, WorkerAssignment, WorkerBinding},
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
    },
};

#[cfg(test)]
use crate::infrastructure::{
    acp_agent_launch_factory::inject_mcp_launch_env,
    mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpLaunchEnv},
};
#[cfg(test)]
use std::collections::BTreeMap;

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

/// 041 과도기: orchestration 저장소는 core로 옮겼고 US1 compat 전환 전까지 command가 직접 연다.
pub(crate) fn orchestration_repository(
    app: &AppHandle,
) -> Result<JsonOrchestrationRepository, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Failed to create app data directory: {error}"))?;
    Ok(JsonOrchestrationRepository::from_path(
        directory.join("orchestration-sessions.json"),
    ))
}

pub(crate) fn orchestration_error(
    error: crate::domain::agent_orchestration::OrchestrationError,
) -> String {
    serde_json::to_string(&error).unwrap_or_else(|_| error.to_string())
}

#[tauri::command]
pub fn bootstrap_orchestration_workspace(
    app: AppHandle,
    window: tauri::Window,
    input: BootstrapOrchestrationInput,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let canonical = std::fs::canonicalize(&input.worktree_path)
        .map_err(|error| format!("Failed to resolve workspace path: {error}"))?;
    if !canonical.is_dir() {
        return Err("Workspace path must be a directory.".into());
    }
    let repository = orchestration_repository(&app)?;
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .bootstrap(
            canonical.to_string_lossy().as_ref(),
            window.label(),
            input.resume_workspace_id.as_deref(),
        )
        .map_err(orchestration_error)
}

#[tauri::command]
pub fn get_orchestration_workspace(
    app: AppHandle,
    window: tauri::Window,
) -> Result<Option<crate::domain::agent_orchestration::OrchestrationSession>, String> {
    let repository = orchestration_repository(&app)?;
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .get_for_bench(window.label())
        .map_err(orchestration_error)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRecoverableOrchestrationInput {
    worktree_path: String,
}

#[tauri::command]
pub fn list_recoverable_orchestration_workspaces(
    app: AppHandle,
    input: ListRecoverableOrchestrationInput,
) -> Result<Vec<crate::domain::agent_orchestration::OrchestrationSession>, String> {
    let canonical = std::fs::canonicalize(&input.worktree_path)
        .map_err(|error| format!("Failed to resolve workspace path: {error}"))?;
    if !canonical.is_dir() {
        return Err("Workspace path must be a directory.".into());
    }
    let worktree_path = canonical.to_string_lossy().to_string();
    let repository = orchestration_repository(&app)?;
    let service =
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()));
    let stale_window_labels: Vec<_> = service
        .list_for_worktree(&worktree_path)
        .map_err(orchestration_error)?
        .into_iter()
        .filter_map(|session| session.bound_bench_id)
        .filter(|label| app.get_webview_window(label).is_none())
        .collect();
    for label in stale_window_labels {
        service.release_bench(&label).map_err(orchestration_error)?;
    }
    service
        .list_recoverable(&worktree_path)
        .map_err(orchestration_error)
}

#[tauri::command]
pub fn bind_main_coordinator_run(
    app: AppHandle,
    window: tauri::Window,
    mcp_state: State<'_, McpServerState>,
    input: BindMainRunRequest,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let run_id = input.run_id.clone();
    let binding_state = input.state;
    let repository = orchestration_repository(&app)?;
    let session = OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .bind_main_run(window.label(), input)
        .map_err(orchestration_error)?;
    if binding_state == crate::application::orchestration_service::MainRunBindingState::Active
        && let Some(generation_id) = session.active_coordinator_generation_id.clone()
    {
        mcp_state
            .bind_run_principal(
                crate::infrastructure::mcp::capability_registry::CapabilityPrincipal::coordinator(
                    session.id.clone(),
                    window.label(),
                    run_id,
                    generation_id,
                ),
            )
            .map_err(orchestration_error)?;
    }
    Ok(session)
}

#[tauri::command]
pub async fn delegate_orchestration_goal(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    input: DelegateGoalRequest,
) -> Result<DelegateGoalOutcome, String> {
    let goal = input.goal.clone();
    let repository = orchestration_repository(&app)?;
    let service =
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()));
    let outcome = service
        .delegate_goal(window.label(), input)
        .map_err(orchestration_error)?;
    let snapshot = service
        .get_for_bench(window.label())
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is unavailable.".to_string())?;
    let run_id = snapshot
        .nodes
        .iter()
        .find(|node| node.id == crate::domain::agent_orchestration::MAIN_AGENT_NODE_ID)
        .and_then(|node| node.current_run_id.clone())
        .ok_or_else(|| "Main Coordinator run is unavailable.".to_string())?;
    // 040 과도기: Main run 이벤트는 그 창 작업대의 sink로(run 소유자 = 작업대).
    let bench = desktop_benches::lookup(window.label()).unwrap_or_default();
    SendPromptUseCase::new(state.inner().clone())
        .execute(workbench_runtime(&app).run_sink(&bench), run_id, goal)
        .await
        .map_err(String::from)?;
    Ok(outcome)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptManualChildInput {
    panel_id: String,
    title: String,
}

#[tauri::command]
pub fn adopt_manual_orchestration_child(
    app: AppHandle,
    window: tauri::Window,
    input: AdoptManualChildInput,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let repository = orchestration_repository(&app)?;
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .adopt_manual_child(window.label(), &input.panel_id, &input.title)
        .map_err(orchestration_error)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrchestrationTasksInput {
    generation_id: String,
}

#[tauri::command]
pub fn list_orchestration_tasks(
    app: AppHandle,
    window: tauri::Window,
    input: ListOrchestrationTasksInput,
) -> Result<Vec<crate::domain::agent_orchestration::OrchestrationTask>, String> {
    let repository = orchestration_repository(&app)?;
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .list_child_tasks(window.label(), &input.generation_id)
        .map_err(orchestration_error)
}

#[tauri::command]
pub fn collect_orchestration_reports(
    app: AppHandle,
    window: tauri::Window,
) -> Result<Vec<crate::domain::agent_orchestration::TaskReport>, String> {
    let repository = orchestration_repository(&app)?;
    Ok(
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
            .get_for_bench(window.label())
            .map_err(orchestration_error)?
            .map(|session| session.reports)
            .unwrap_or_default(),
    )
}

#[tauri::command]
pub fn set_orchestration_presentation(
    app: AppHandle,
    window: tauri::Window,
    input: SetPresentationRequest,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let repository = orchestration_repository(&app)?;
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .set_presentation(window.label(), input)
        .map_err(orchestration_error)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliverTaskCommandInput {
    request_id: String,
    task_id: String,
    kind: TaskCommandKind,
    message: Option<String>,
    input_report_id: Option<String>,
    delivery: PromptDelivery,
    expected_task_revision: Option<u64>,
}

#[tauri::command]
pub async fn send_orchestration_child_command(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    input: DeliverTaskCommandInput,
) -> Result<TaskCommand, String> {
    let repository = orchestration_repository(&app)?;
    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
        app.clone(),
        state.inner().clone(),
        mcp_state.inner().clone(),
    ));
    let command = OrchestrationCommandService::new(repository.clone(), adapter)
        .deliver(
            window.label(),
            DeliverTaskCommandRequest {
                request_id: input.request_id,
                task_id: input.task_id,
                kind: input.kind,
                message: input.message,
                input_report_id: input.input_report_id,
                delivery: input.delivery,
                source: TaskCommandSource::User,
                expected_task_revision: input.expected_task_revision,
            },
        )
        .await
        .map_err(orchestration_error)?;
    emit_orchestration_runtime_update(&app, &repository, window.label(), "taskCommandDelivery");
    Ok(command)
}

#[tauri::command]
pub async fn respond_orchestration_input(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    input: TaskActionRequest,
) -> Result<TaskCommand, String> {
    let repository = orchestration_repository(&app)?;
    let snapshot = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    )
    .get_for_bench(window.label())
    .map_err(orchestration_error)?
    .ok_or_else(|| "Orchestration workspace is unavailable.".to_string())?;
    let input_report_id = snapshot
        .reports
        .iter()
        .rev()
        .find(|report| {
            report.task_id == input.task_id && report.report_type == TaskReportType::InputRequest
        })
        .map(|report| report.id.clone());
    let task_revision = snapshot
        .tasks
        .iter()
        .find(|task| task.id == input.task_id)
        .map(|task| task.revision);
    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
        app,
        state.inner().clone(),
        mcp_state.inner().clone(),
    ));
    OrchestrationCommandService::new(repository, adapter)
        .deliver(
            window.label(),
            DeliverTaskCommandRequest {
                request_id: input.request_id,
                task_id: input.task_id,
                kind: TaskCommandKind::InputResponse,
                message: input.message,
                input_report_id,
                delivery: PromptDelivery::Queue,
                source: TaskCommandSource::User,
                expected_task_revision: task_revision,
            },
        )
        .await
        .map_err(orchestration_error)
}

#[tauri::command]
pub async fn cancel_orchestration_task(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    input: TaskActionRequest,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let repository = orchestration_repository(&app)?;
    let snapshot = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    )
    .get_for_bench(window.label())
    .map_err(orchestration_error)?
    .ok_or_else(|| "Orchestration workspace is unavailable.".to_string())?;
    let task = snapshot.tasks.iter().find(|task| task.id == input.task_id);
    let task_revision = task.map(|task| task.revision);
    let has_active_run = task
        .and_then(|task| task.assigned_node_id.as_ref())
        .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
        .and_then(|node| node.current_run_id.as_ref())
        .is_some();
    if !has_active_run {
        return OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
            .cancel_task(window.label(), input)
            .map_err(orchestration_error);
    }
    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
        app.clone(),
        state.inner().clone(),
        mcp_state.inner().clone(),
    ));
    OrchestrationCommandService::new(repository.clone(), adapter)
        .deliver(
            window.label(),
            DeliverTaskCommandRequest {
                request_id: input.request_id,
                task_id: input.task_id.clone(),
                kind: TaskCommandKind::Cancel,
                message: None,
                input_report_id: None,
                delivery: PromptDelivery::Queue,
                source: TaskCommandSource::User,
                expected_task_revision: task_revision,
            },
        )
        .await
        .map_err(orchestration_error)?;
    let _ = mcp_state.orchestration_scheduler().release(&input.task_id);
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .get_for_bench(window.label())
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is unavailable.".to_string())
}

#[tauri::command]
pub async fn retry_orchestration_task(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    input: TaskActionRequest,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let repository = orchestration_repository(&app)?;
    let service =
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()));
    stop_existing_task_worker(
        &app,
        window.label(),
        state.inner().clone(),
        mcp_state.inner().clone(),
        &service,
        &input.task_id,
    )
    .await?;
    service
        .retry_task(window.label(), input.clone())
        .map_err(orchestration_error)?;
    launch_orchestration_task_for_ui(
        &app,
        window.label(),
        state.inner().clone(),
        mcp_state.inner().clone(),
        &service,
        &input.task_id,
    )
    .await
}

#[tauri::command]
pub async fn reassign_orchestration_task(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    input: TaskActionRequest,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let repository = orchestration_repository(&app)?;
    let service = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    );
    stop_existing_task_worker(
        &app,
        window.label(),
        state.inner().clone(),
        mcp_state.inner().clone(),
        &service,
        &input.task_id,
    )
    .await?;
    service
        .reassign_task(window.label(), input.clone())
        .map_err(orchestration_error)?;
    launch_orchestration_task_for_ui(
        &app,
        window.label(),
        state.inner().clone(),
        mcp_state.inner().clone(),
        &service,
        &input.task_id,
    )
    .await
}

async fn stop_existing_task_worker(
    app: &AppHandle,
    window_label: &str,
    state: AppState,
    mcp_state: McpServerState,
    service: &OrchestrationService<JsonOrchestrationRepository, TauriOrchestrationEventSink>,
    task_id: &str,
) -> Result<(), String> {
    let Some(snapshot) = service
        .get_for_bench(window_label)
        .map_err(orchestration_error)?
    else {
        return Ok(());
    };
    let Some(task) = snapshot.tasks.iter().find(|task| task.id == task_id) else {
        return Ok(());
    };
    let Some(node) = task
        .assigned_node_id
        .as_ref()
        .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
    else {
        return Ok(());
    };
    let Some(run_id) = node.current_run_id.as_ref() else {
        return Ok(());
    };
    let binding = WorkerBinding {
        workspace_id: snapshot.id.clone(),
        bench_id: window_label.into(),
        node_id: node.id.clone(),
        task_id: task.id.clone(),
        run_id: run_id.clone(),
    };
    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
        app.clone(),
        state,
        mcp_state.clone(),
    ));
    if adapter.is_active(&binding).await {
        let _ = adapter.cancel_worker(&binding).await;
    }
    mcp_state
        .revoke_run_capability(run_id)
        .map_err(orchestration_error)
}

async fn launch_orchestration_task_for_ui(
    app: &AppHandle,
    window_label: &str,
    state: AppState,
    mcp_state: McpServerState,
    service: &OrchestrationService<JsonOrchestrationRepository, TauriOrchestrationEventSink>,
    task_id: &str,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    if let crate::application::orchestration_scheduler::LeaseOutcome::Queued { .. } = mcp_state
        .orchestration_scheduler()
        .acquire(task_id)
        .map_err(orchestration_error)?
    {
        return service
            .get_for_bench(window_label)
            .map_err(orchestration_error)?
            .ok_or_else(|| "Orchestration workspace is unavailable.".to_string());
    }
    let snapshot = service
        .get_for_bench(window_label)
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is unavailable.".to_string())?;
    let task = snapshot
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .ok_or_else(|| "Task is unavailable.".to_string())?;
    let node = task
        .assigned_node_id
        .as_ref()
        .and_then(|node_id| snapshot.nodes.iter().find(|node| node.id == *node_id))
        .ok_or_else(|| "Assigned Child is unavailable.".to_string())?;
    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
        app.clone(),
        state,
        mcp_state.clone(),
    ));
    let planned_run_id = uuid::Uuid::new_v4().to_string();
    let outcome = adapter
        .start_worker(WorkerAssignment {
            workspace_id: snapshot.id.clone(),
            bench_id: window_label.into(),
            worktree_path: snapshot.worktree_path.clone(),
            node_id: node.id.clone(),
            task_id: task.id.clone(),
            attempt: task.attempt,
            planned_run_id,
            role: node.role.clone(),
            objective: task.objective.clone(),
            constraints: task.constraints.clone(),
            expected_result: task.expected_result.clone(),
            runtime_profile: WorkerRuntimeProfile {
                agent_profile_id: std::env::var("AW_ORCHESTRATION_AGENT_PROFILE")
                    .unwrap_or_else(|_| "codex".into()),
                provider_id: "acp".into(),
                model_id: None,
                access_policy: AccessPolicy::ReadOnly,
                supports_read_only: true,
            },
            mcp_capability: String::new(),
        })
        .await
        .map_err(orchestration_error)?;
    match outcome {
        StartWorkerOutcome::Started { run_id } => service
            .bind_child_run(window_label, &task.id, &node.id, &run_id)
            .map_err(orchestration_error),
        StartWorkerOutcome::Queued { .. } => service
            .get_for_bench(window_label)
            .map_err(orchestration_error)?
            .ok_or_else(|| "Orchestration workspace is unavailable.".to_string()),
        StartWorkerOutcome::Failed {
            code: _,
            message,
            retryable: _,
        } => {
            let _ = mcp_state.orchestration_scheduler().release(task_id);
            Err(message)
        }
    }
}

#[tauri::command]
pub fn handoff_orchestration_coordinator(
    app: AppHandle,
    window: tauri::Window,
    mcp_state: State<'_, McpServerState>,
    input: CoordinatorHandoffRequest,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let previous_generation = {
        let repository = orchestration_repository(&app)?;
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()))
            .get_for_bench(window.label())
            .map_err(orchestration_error)?
            .and_then(|session| session.active_coordinator_generation_id)
    };
    let successor_run_id = input.successor_run_id.clone();
    let repository = orchestration_repository(&app)?;
    let session = OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .handoff_coordinator(window.label(), input)
        .map_err(orchestration_error)?;
    if let Some(generation_id) = previous_generation {
        mcp_state
            .revoke_generation_capabilities(&session.id, &generation_id)
            .map_err(orchestration_error)?;
    }
    if let Some(generation_id) = session.active_coordinator_generation_id.clone() {
        mcp_state
            .bind_run_principal(
                crate::infrastructure::mcp::capability_registry::CapabilityPrincipal::coordinator(
                    session.id.clone(),
                    window.label(),
                    successor_run_id,
                    generation_id,
                ),
            )
            .map_err(orchestration_error)?;
    }
    Ok(session)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRuntimeEventsInput {
    run_id: String,
    after_sequence: u64,
}

#[tauri::command]
pub fn replay_orchestration_runtime_events(
    app: AppHandle,
    input: ReplayRuntimeEventsInput,
) -> RunReplay {
    // 039: run journal은 core 이벤트 hub에 있다. 응답 형태는 오늘의 `RuntimeEventSnapshot`과 같다.
    workbench_runtime(&app)
        .events_hub()
        .replay_run(&input.run_id, input.after_sequence)
}

#[tauri::command]
pub async fn dispatch_orchestration_prompt(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    input: DispatchPromptRequest,
) -> Result<crate::domain::agent_orchestration::PromptDispatch, String> {
    let repository = orchestration_repository(&app)?;
    let service = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    );
    let mut dispatch = service
        .record_prompt_dispatch(window.label(), input)
        .map_err(orchestration_error)?;
    let snapshot = service
        .get_for_bench(window.label())
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is unavailable.".to_string())?;

    for target in dispatch.targets.clone() {
        let Some(node) = snapshot.nodes.iter().find(|node| {
            node.id == target.panel_id
                && node.kind == crate::domain::agent_orchestration::AgentNodeKind::Child
        }) else {
            continue;
        };
        let Some(task) = node
            .assigned_task_id
            .as_ref()
            .and_then(|task_id| snapshot.tasks.iter().find(|task| task.id == *task_id))
        else {
            dispatch = service
                .update_prompt_dispatch_target(
                    window.label(),
                    &dispatch.id,
                    &target.request_id,
                    PromptDispatchTargetStatus::Rejected,
                    Some(("unknownTask".into(), "Child has no assigned task.".into())),
                )
                .map_err(orchestration_error)?;
            continue;
        };
        let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
            app.clone(),
            state.inner().clone(),
            mcp_state.inner().clone(),
        ));
        let command = OrchestrationCommandService::new(repository.clone(), adapter)
            .deliver(
                window.label(),
                DeliverTaskCommandRequest {
                    request_id: target.request_id.clone(),
                    task_id: task.id.clone(),
                    kind: TaskCommandKind::Message,
                    message: Some(dispatch.message.clone()),
                    input_report_id: None,
                    delivery: dispatch.delivery,
                    source: TaskCommandSource::User,
                    expected_task_revision: Some(task.revision),
                },
            )
            .await;
        dispatch = match command {
            Ok(command)
                if command.status
                    == crate::domain::agent_orchestration::TaskCommandStatus::Accepted =>
            {
                service
                    .update_prompt_dispatch_target(
                        window.label(),
                        &dispatch.id,
                        &target.request_id,
                        PromptDispatchTargetStatus::Delivered,
                        None,
                    )
                    .map_err(orchestration_error)?
            }
            Ok(command) => service
                .update_prompt_dispatch_target(
                    window.label(),
                    &dispatch.id,
                    &target.request_id,
                    PromptDispatchTargetStatus::Failed,
                    command
                        .failure
                        .map(|failure| (format!("{:?}", failure.code), failure.message)),
                )
                .map_err(orchestration_error)?,
            Err(error) => service
                .update_prompt_dispatch_target(
                    window.label(),
                    &dispatch.id,
                    &target.request_id,
                    PromptDispatchTargetStatus::Failed,
                    Some((format!("{:?}", error.code), error.message)),
                )
                .map_err(orchestration_error)?,
        };
    }
    Ok(dispatch)
}

#[tauri::command]
pub async fn recover_orchestration_workspace(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
) -> Result<crate::domain::agent_orchestration::OrchestrationSession, String> {
    let repository = orchestration_repository(&app)?;
    let service = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    );
    let snapshot = service
        .get_for_bench(window.label())
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is not bootstrapped.".to_string())?;
    let mut live_run_ids = Vec::new();
    // 040 과도기: run 소유자는 창의 작업대다.
    let bench = desktop_benches::lookup(window.label());
    for run_id in snapshot
        .nodes
        .iter()
        .filter_map(|node| node.current_run_id.as_ref())
    {
        if bench.is_some() && state.active_owner_of(run_id).await == bench {
            live_run_ids.push(run_id.clone());
        }
    }
    let reconciled = service
        .reconcile_runtime(window.label(), &live_run_ids)
        .map_err(orchestration_error)?;
    let active_task_ids = reconciled
        .tasks
        .iter()
        .filter(|task| {
            task.status == crate::domain::agent_orchestration::TaskStatus::Running
                && task
                    .assigned_node_id
                    .as_ref()
                    .and_then(|node_id| reconciled.nodes.iter().find(|node| node.id == *node_id))
                    .and_then(|node| node.current_run_id.as_ref())
                    .is_some_and(|run_id| live_run_ids.contains(run_id))
        })
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let ready_task_ids = reconciled
        .tasks
        .iter()
        .filter(|task| task.status == crate::domain::agent_orchestration::TaskStatus::Ready)
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    mcp_state
        .orchestration_scheduler()
        .reconcile(&active_task_ids, &ready_task_ids)
        .map_err(orchestration_error)?;
    let _ = repository.pending_outbox().map_err(orchestration_error)?;
    let adapter = AcpAgentWorkerAdapter::new(TauriAcpWorkerRuntime::new(
        app.clone(),
        state.inner().clone(),
        mcp_state.inner().clone(),
    ));
    OrchestrationCommandService::new(repository.clone(), adapter.clone())
        .reconcile_pending(window.label())
        .map_err(orchestration_error)?;
    let dispatcher = CoordinatorNotificationDispatcher::new(repository.clone(), adapter);
    dispatcher
        .recover_interrupted(window.label())
        .map_err(orchestration_error)?;
    let dispatch_app = app.clone();
    let dispatch_repository = repository.clone();
    let dispatch_window_label = window.label().to_string();
    tokio::spawn(async move {
        let _ = dispatcher.dispatch_pending(&dispatch_window_label).await;
        emit_orchestration_runtime_update(
            &dispatch_app,
            &dispatch_repository,
            &dispatch_window_label,
            "notificationRecovery",
        );
    });
    service
        .get_for_bench(window.label())
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is not bootstrapped.".to_string())
}

fn emit_orchestration_runtime_update(
    app: &AppHandle,
    repository: &JsonOrchestrationRepository,
    window_label: &str,
    reason: &str,
) {
    let service = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    );
    if let Ok(Some(session)) = service.get_for_bench(window_label) {
        let _ = TauriOrchestrationEventSink::new(app.clone()).emit(
            window_label,
            OrchestrationEvent {
                workspace_id: session.id,
                revision: session.revision,
                reason: reason.into(),
                task_id: None,
                node_id: None,
            },
        );
    }
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
    request: workbench_protocol::CallRequest,
) -> Result<Out, String> {
    let runtime = workbench_runtime(app);
    match runtime
        .call(workbench_compat::desktop_principal(), request)
        .await
    {
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
    let runtime = workbench_runtime(&app);
    let bench =
        desktop_benches::ensure(&runtime, window.label(), Some(&request.worktree_path)).await?;
    call_exchange(
        &app,
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

fn workbench_runtime(app: &AppHandle) -> Arc<WorkbenchRuntime> {
    app.state::<Arc<WorkbenchRuntime>>().inner().clone()
}

// 037·038: 아래 command들은 `Workbench.call`을 거치는 호환 어댑터다. 시그니처·직렬화·오류 문구는 이전과 같다
// (specs/038-workbench-domains/contracts/tauri-compat-commands.md). 저장소·업무 로직은 workbench-core에 있다.
#[tauri::command]
pub async fn list_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    let runtime = workbench_runtime(&app);
    log_async_command(
        "list_projects",
        workbench_compat::call_list_projects(&runtime),
    )
    .await
}

#[tauri::command]
pub async fn create_project(app: AppHandle, input: ProjectInput) -> Result<Project, String> {
    let runtime = workbench_runtime(&app);
    log_async_command(
        "create_project",
        workbench_compat::call_create_project(&runtime, input),
    )
    .await
}

#[tauri::command]
pub async fn update_project(
    app: AppHandle,
    id: String,
    input: ProjectInput,
) -> Result<Project, String> {
    let runtime = workbench_runtime(&app);
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
pub async fn delete_project(app: AppHandle, id: String) -> Result<(), String> {
    let runtime = workbench_runtime(&app);
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
pub async fn list_saved_prompts(app: AppHandle) -> Result<Vec<SavedPrompt>, String> {
    let runtime = workbench_runtime(&app);
    log_async_command(
        "list_saved_prompts",
        workbench_compat::call_query(&runtime, OperationId::SavedPromptList, json!({})),
    )
    .await
}

#[tauri::command]
pub async fn create_saved_prompt(
    app: AppHandle,
    input: SavedPromptInput,
) -> Result<SavedPrompt, String> {
    let runtime = workbench_runtime(&app);
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
    id: String,
    input: SavedPromptInput,
) -> Result<SavedPrompt, String> {
    let runtime = workbench_runtime(&app);
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
pub async fn delete_saved_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
) -> Result<Option<ThreadGoal>, String> {
    let runtime = workbench_runtime(&app);
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
pub async fn create_goal(app: AppHandle, input: GoalInput) -> Result<ThreadGoal, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    input: GoalUpdateInput,
) -> Result<ThreadGoal, String> {
    let runtime = workbench_runtime(&app);
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
pub async fn clear_goal(app: AppHandle, working_directory: String) -> Result<(), String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    input: GoalProgressInput,
) -> Result<ThreadGoal, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
) -> Result<Option<AgentRunSettings>, String> {
    let runtime = workbench_runtime(&app);
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
    settings: AgentRunSettings,
) -> Result<AgentRunSettings, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
) -> Result<Vec<GitRemote>, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
) -> Result<Vec<GitBranch>, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    include_status: Option<bool>,
) -> Result<Vec<GitWorktree>, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
) -> Result<Vec<WorktreeChange>, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    input: GitWorktreeCreateDraft,
) -> Result<(), String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    path: String,
) -> Result<(), String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
) -> Result<GitWorktreeChanges, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    path: String,
) -> Result<GitWorktreeFileDiff, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    scope: Option<WorktreeFileListScope>,
) -> Result<Vec<WorktreeFileEntry>, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    path: String,
) -> Result<WorktreeTextFile, String> {
    let runtime = workbench_runtime(&app);
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
    let runtime = workbench_runtime(&app);
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
            .events(AuthenticatedPrincipal::desktop(), subscription)
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
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitHistory, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitGraph, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    commit_hash: String,
) -> Result<GitCommitDetail, String> {
    let runtime = workbench_runtime(&app);
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
    working_directory: String,
    commit_hash: String,
    path: String,
) -> Result<WorktreeGitFileDiff, String> {
    let runtime = workbench_runtime(&app);
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
pub async fn list_agents(app: AppHandle) -> Vec<AgentDescriptor> {
    let runtime = workbench_runtime(&app);
    // 오늘 command는 실패하지 않는 `Vec`를 돌려줬다(catalog 오류는 기본값). 시그니처를 유지하기 위해
    // 호출이 실패하면 빈 목록을 돌려주고 오류는 perf 로그에 남긴다.
    let result: Result<Vec<AgentDescriptor>, String> = log_async_command(
        "list_agents",
        workbench_compat::call_query(
            &runtime,
            OperationId::AgentList,
            workbench_compat::agent_list_input(),
        ),
    )
    .await;
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
    agent_id: String,
    cwd: Option<String>,
) -> Result<Vec<ProviderSession>, String> {
    let runtime = workbench_runtime(&app);
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

pub(crate) fn resolve_agent_run_launch_principal(
    app: &AppHandle,
    window_label: &str,
    panel_id: Option<&str>,
    run_id: &str,
) -> Result<Option<CapabilityPrincipal>, String> {
    if panel_id != Some(MAIN_AGENT_NODE_ID) {
        return Ok(None);
    }

    let repository = orchestration_repository(app)?;
    let session =
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()))
            .get_for_bench(window_label)
            .map_err(orchestration_error)?;
    coordinator_principal_for_bound_session(panel_id, run_id, window_label, session.as_ref())
}

fn coordinator_principal_for_bound_session(
    panel_id: Option<&str>,
    run_id: &str,
    window_label: &str,
    session: Option<&crate::domain::agent_orchestration::OrchestrationSession>,
) -> Result<Option<CapabilityPrincipal>, String> {
    if panel_id != Some(MAIN_AGENT_NODE_ID) {
        return Ok(None);
    }
    let session =
        session.ok_or_else(|| "Main Coordinator workspace is unavailable.".to_string())?;
    let generation_id = session
        .active_coordinator_generation_id
        .clone()
        .ok_or_else(|| "Main Coordinator generation must be bound before launch.".to_string())?;
    let generation = session
        .generations
        .iter()
        .find(|generation| generation.id == generation_id)
        .ok_or_else(|| "Active Main Coordinator generation is unavailable.".to_string())?;
    if generation.run_id != run_id {
        return Err(
            "Main Coordinator generation does not match the run being launched.".to_string(),
        );
    }

    Ok(Some(CapabilityPrincipal::coordinator(
        session.id.clone(),
        window_label,
        run_id,
        generation_id,
    )))
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
    let runtime = workbench_runtime(&app);
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
    let runtime = workbench_runtime(&app);
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
        &workbench_runtime(app),
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
        &workbench_runtime(&app),
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
        &workbench_runtime(&app),
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
        &workbench_runtime(&app),
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

    fn coordinator_session(
        run_id: &str,
    ) -> crate::domain::agent_orchestration::OrchestrationSession {
        use crate::domain::agent_orchestration::{
            CoordinatorGeneration, CoordinatorGenerationStatus,
        };

        let mut session = crate::domain::agent_orchestration::OrchestrationSession::new(
            "workspace-1",
            "/repo",
            "window-1",
            "2026-07-27T00:00:00Z",
        );
        session.active_coordinator_generation_id = Some("generation-1".into());
        session.generations.push(CoordinatorGeneration {
            id: "generation-1".into(),
            ordinal: 1,
            main_node_id: MAIN_AGENT_NODE_ID.into(),
            run_id: run_id.into(),
            previous_generation_id: None,
            status: CoordinatorGenerationStatus::Active,
            started_at: "2026-07-27T00:00:00Z".into(),
            ended_at: None,
            handoff_summary: None,
            successor_generation_id: None,
        });
        session
    }

    #[test]
    fn main_launch_uses_prebound_coordinator_principal() {
        let session = coordinator_session("run-main");

        let principal = coordinator_principal_for_bound_session(
            Some(MAIN_AGENT_NODE_ID),
            "run-main",
            "window-1",
            Some(&session),
        )
        .unwrap()
        .expect("Main should receive a Coordinator principal");

        assert_eq!(
            principal.actor_kind,
            crate::infrastructure::mcp::capability_registry::CapabilityActorKind::Coordinator
        );
        assert_eq!(principal.run_id, "run-main");
        assert_eq!(principal.generation_id.as_deref(), Some("generation-1"));
    }

    #[test]
    fn main_launch_rejects_an_unbound_successor_run() {
        let session = coordinator_session("run-current");

        let error = coordinator_principal_for_bound_session(
            Some(MAIN_AGENT_NODE_ID),
            "run-successor",
            "window-1",
            Some(&session),
        )
        .unwrap_err();

        assert!(error.contains("does not match"));
    }

    #[test]
    fn non_main_launch_keeps_legacy_principal_path() {
        let session = coordinator_session("run-main");
        let principal = coordinator_principal_for_bound_session(
            Some("extra-agent-run-1"),
            "run-extra",
            "window-1",
            Some(&session),
        )
        .unwrap();

        assert!(principal.is_none());
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
