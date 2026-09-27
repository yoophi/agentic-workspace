use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    process::Command,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager, State};
use workbench_core::{
    application::{agent_run_settings_service, workbench_runtime::WorkbenchRuntime},
    infrastructure::event_hub::RunReplay,
};
use workbench_protocol::OperationId;

use crate::inbound::workbench_compat;
use crate::{
    application::{
        agent_exchange_service::AgentExchangeService,
        agent_tool_candidate_service::AgentToolCandidateService,
        appearance_preferences_service::AppearancePreferencesService,
        cancel_agent_run::CancelAgentRunUseCase,
        cancel_prompt_and_send::CancelPromptAndSendUseCase,
        coordinator_notification_dispatcher::CoordinatorNotificationDispatcher,
        orchestration_command_service::{DeliverTaskCommandRequest, OrchestrationCommandService},
        orchestration_service::{
            BindMainRunRequest, CoordinatorHandoffRequest, DelegateGoalOutcome,
            DelegateGoalRequest, DispatchPromptRequest, OrchestrationService,
            SetPresentationRequest, TaskActionRequest,
        },
        send_prompt::SendPromptUseCase,
        set_permission_mode::SetPermissionModeUseCase,
        start_agent_run::StartAgentRunUseCase,
        steer_prompt::SteerPromptUseCase,
        worktree_workspace_layout_service,
    },
    domain::{
        agent::AgentDescriptor,
        agent_exchange::{
            AgentExchange, AgentExchangeAckRequest, AgentWorkspaceSyncRequest,
            AgentWorkspaceSyncResponse, SendAgentExchangeRequest,
        },
        agent_orchestration::{
            AccessPolicy, MAIN_AGENT_NODE_ID, PromptDelivery, PromptDispatchTargetStatus,
            TaskCommand, TaskCommandKind, TaskCommandSource, TaskReportType, WorkerRuntimeProfile,
        },
        agent_run_settings::{
            APP_COMMAND_OVERRIDE_SETTINGS_KEY, AgentCommandSource, AgentRunSettings,
        },
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
        acp::runner::AcpAgentRunner,
        acp_agent_launch_factory::{inject_mcp_launch_env, normalize_run_request},
        acp_agent_worker_adapter::{AcpAgentWorkerAdapter, TauriAcpWorkerRuntime},
        agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState,
        fs_worktree_watcher::{WorktreeWatchHandle, watch_worktree},
        in_memory_agent_workspace_registry::{
            InMemoryAgentWorkspaceRegistry, TauriAgentExchangeEventSink,
        },
        json_acp_session_store::JsonAcpSessionStore,
        json_appearance_preferences_repository::JsonAppearancePreferencesRepository,
        json_orchestration_repository::JsonOrchestrationRepository,
        json_worktree_workspace_layout_repository::JsonWorkspaceLayoutRepository,
        mcp::{McpServerState, capability_registry::CapabilityPrincipal, title_tool},
        perf_log::{log_async_command, log_async_command_error, run_blocking_command},
        tauri_orchestration_event_sink::TauriOrchestrationEventSink,
        tauri_run_event_sink::TauriRunEventSink,
        window_manager,
    },
    ports::{
        agent_catalog::AgentCatalog,
        agent_worker::{AgentWorkerPort, StartWorkerOutcome, WorkerAssignment, WorkerBinding},
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
        permission::PermissionDecision,
    },
};

#[cfg(test)]
use crate::{
    domain::run::RalphLoopRequest,
    infrastructure::mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpLaunchEnv},
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

fn orchestration_error(error: crate::domain::agent_orchestration::OrchestrationError) -> String {
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .get_for_window(window.label())
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    let service =
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()));
    let stale_window_labels: Vec<_> = service
        .list_for_worktree(&worktree_path)
        .map_err(orchestration_error)?
        .into_iter()
        .filter_map(|session| session.bound_window_label)
        .filter(|label| app.get_webview_window(label).is_none())
        .collect();
    for label in stale_window_labels {
        service
            .release_window(&label)
            .map_err(orchestration_error)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    let service =
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()));
    let outcome = service
        .delegate_goal(window.label(), input)
        .map_err(orchestration_error)?;
    let snapshot = service
        .get_for_window(window.label())
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is unavailable.".to_string())?;
    let run_id = snapshot
        .nodes
        .iter()
        .find(|node| node.id == crate::domain::agent_orchestration::MAIN_AGENT_NODE_ID)
        .and_then(|node| node.current_run_id.clone())
        .ok_or_else(|| "Main Coordinator run is unavailable.".to_string())?;
    SendPromptUseCase::new(state.inner().clone())
        .execute(
            TauriRunEventSink::with_target(app, state.inner().clone(), window.label().to_string()),
            run_id,
            goal,
        )
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
        .list_child_tasks(window.label(), &input.generation_id)
        .map_err(orchestration_error)
}

#[tauri::command]
pub fn collect_orchestration_reports(
    app: AppHandle,
    window: tauri::Window,
) -> Result<Vec<crate::domain::agent_orchestration::TaskReport>, String> {
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    Ok(
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app))
            .get_for_window(window.label())
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    let snapshot = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    )
    .get_for_window(window.label())
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    let snapshot = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    )
    .get_for_window(window.label())
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
        .get_for_window(window.label())
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
        .get_for_window(window_label)
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
        window_label: window_label.into(),
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
            .get_for_window(window_label)
            .map_err(orchestration_error)?
            .ok_or_else(|| "Orchestration workspace is unavailable.".to_string());
    }
    let snapshot = service
        .get_for_window(window_label)
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
            window_label: window_label.into(),
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
            .get_for_window(window_label)
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
        let repository = JsonOrchestrationRepository::from_app(&app)?;
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()))
            .get_for_window(window.label())
            .map_err(orchestration_error)?
            .and_then(|session| session.active_coordinator_generation_id)
    };
    let successor_run_id = input.successor_run_id.clone();
    let repository = JsonOrchestrationRepository::from_app(&app)?;
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    let service = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    );
    let mut dispatch = service
        .record_prompt_dispatch(window.label(), input)
        .map_err(orchestration_error)?;
    let snapshot = service
        .get_for_window(window.label())
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
    let repository = JsonOrchestrationRepository::from_app(&app)?;
    let service = OrchestrationService::new(
        repository.clone(),
        TauriOrchestrationEventSink::new(app.clone()),
    );
    let snapshot = service
        .get_for_window(window.label())
        .map_err(orchestration_error)?
        .ok_or_else(|| "Orchestration workspace is not bootstrapped.".to_string())?;
    let mut live_run_ids = Vec::new();
    for run_id in snapshot
        .nodes
        .iter()
        .filter_map(|node| node.current_run_id.as_ref())
    {
        if state.active_owner_of(run_id).await.as_deref() == Some(window.label()) {
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
        .get_for_window(window.label())
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
    if let Ok(Some(session)) = service.get_for_window(window_label) {
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

pub struct WorktreeWatcherState {
    handles: Mutex<HashMap<String, WorktreeWatchHandle>>,
}

fn exchange_error(error: crate::domain::agent_exchange::AgentExchangeError) -> String {
    serde_json::to_string(&error).unwrap_or_else(|_| error.to_string())
}

#[tauri::command]
pub async fn sync_agent_workspace(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    workspace_registry: State<'_, InMemoryAgentWorkspaceRegistry>,
    mut request: AgentWorkspaceSyncRequest,
) -> Result<AgentWorkspaceSyncResponse, String> {
    let canonical = std::fs::canonicalize(&request.worktree_path)
        .map_err(|error| format!("Failed to resolve workspace path: {error}"))?;
    if !canonical.is_dir() {
        return Err("Workspace path must be a directory.".into());
    }
    request.worktree_path = canonical.to_string_lossy().into_owned();
    AgentExchangeService::new(
        workspace_registry.inner().clone(),
        state.inner().clone(),
        TauriAgentExchangeEventSink::new(app),
    )
    .sync_workspace(window.label().to_string(), request)
    .await
    .map_err(exchange_error)
}

#[tauri::command]
pub async fn send_agent_exchange(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    workspace_registry: State<'_, InMemoryAgentWorkspaceRegistry>,
    request: SendAgentExchangeRequest,
) -> Result<AgentExchange, String> {
    AgentExchangeService::new(
        workspace_registry.inner().clone(),
        state.inner().clone(),
        TauriAgentExchangeEventSink::new(app),
    )
    .send_user_exchange(window.label(), request)
    .await
    .map_err(exchange_error)
}

#[tauri::command]
pub async fn acknowledge_agent_exchange(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    workspace_registry: State<'_, InMemoryAgentWorkspaceRegistry>,
    request: AgentExchangeAckRequest,
) -> Result<AgentExchange, String> {
    AgentExchangeService::new(
        workspace_registry.inner().clone(),
        state.inner().clone(),
        TauriAgentExchangeEventSink::new(app),
    )
    .acknowledge(window.label(), request)
    .await
    .map_err(exchange_error)
}

#[tauri::command]
pub async fn list_agent_exchanges(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    workspace_registry: State<'_, InMemoryAgentWorkspaceRegistry>,
) -> Result<Vec<AgentExchange>, String> {
    Ok(AgentExchangeService::new(
        workspace_registry.inner().clone(),
        state.inner().clone(),
        TauriAgentExchangeEventSink::new(app),
    )
    .list_exchanges(window.label())
    .await)
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
        handles.remove(window_label);
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
    let event_app = app.clone();
    // watcher 시작은 내부에서 `git rev-parse`를 실행하므로 blocking pool에서 수행한다.
    let handle = run_blocking_command("start_worktree_watcher", move || {
        watch_worktree(working_directory, move |event| {
            if let Err(error) =
                event_app.emit_to(target_label.as_str(), WORKTREE_CHANGED_EVENT, event)
            {
                eprintln!("Failed to emit worktree change event: {error}");
            }
        })
    })
    .await?;
    let mut handles = state
        .handles
        .lock()
        .map_err(|error| format!("Failed to lock worktree watcher state: {error}"))?;

    handles.insert(window_label, handle);
    Ok(())
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

fn resolve_agent_run_launch_principal(
    app: &AppHandle,
    window_label: &str,
    panel_id: Option<&str>,
    run_id: &str,
) -> Result<Option<CapabilityPrincipal>, String> {
    if panel_id != Some(MAIN_AGENT_NODE_ID) {
        return Ok(None);
    }

    let repository = JsonOrchestrationRepository::from_app(app)?;
    let session =
        OrchestrationService::new(repository, TauriOrchestrationEventSink::new(app.clone()))
            .get_for_window(window_label)
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

#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    request: AgentRunRequest,
    panel_id: Option<String>,
) -> Result<AgentRun, String> {
    let mut request = normalize_run_request(request);
    let run_id = request
        .run_id
        .clone()
        .ok_or_else(|| "agent run id is unavailable after normalization".to_string())?;
    let launch_principal =
        resolve_agent_run_launch_principal(&app, window.label(), panel_id.as_deref(), &run_id)?;
    let launch_env = match launch_principal {
        Some(principal) => mcp_state
            .launch_env_for_principal(principal)
            .map_err(orchestration_error)?,
        None => mcp_state.launch_env(&run_id),
    };
    inject_mcp_launch_env(&mut request, launch_env);
    let catalog = ConfigurableAgentCatalog::from_env();
    if request
        .agent_command
        .as_deref()
        .is_none_or(|command| command.trim().is_empty())
    {
        // 038: 설정 저장소는 workbench-core가 소유한다. 같은 aggregate lock 안에서 읽는다(run 시작은 2단계 이관 대상).
        let runtime = workbench_runtime(&app);
        let settings = tokio::task::spawn_blocking(move || {
            runtime
                .coordinator()
                .with_agent_run_settings(|repository| {
                    agent_run_settings_service::get_settings(
                        repository,
                        APP_COMMAND_OVERRIDE_SETTINGS_KEY.into(),
                    )
                })
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
        if let Some(settings) = settings {
            let resolution = agent_run_settings_service::resolve_agent_command(
                &request.agent_id,
                &settings.command_overrides,
                catalog.command_for_agent(&request.agent_id),
            )
            .map_err(|error| error.to_string())?;
            if resolution.source != AgentCommandSource::DefaultCommand {
                request.agent_command = Some(resolution.command);
            }
        }
    }

    let owner_window_label = window.label().to_string();
    let session_store = JsonAcpSessionStore::from_app(&app)?;
    let sink =
        TauriRunEventSink::with_target(app, state.inner().clone(), owner_window_label.clone());
    let registry = state.inner().clone();
    let permissions = state.permissions();
    let runner = AcpAgentRunner::new(catalog, permissions, Arc::new(session_store));

    StartAgentRunUseCase::new(registry)
        .execute(runner, sink, request, Some(owner_window_label))
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn list_agent_tool_command_candidates(
    window: tauri::Window,
    state: State<'_, AppState>,
    input: AgentToolCandidateQuery,
) -> Result<AgentToolCandidateResponse, String> {
    let owner_window_label = window.label().to_string();
    let candidates = title_tool::tool_command_candidates(
        input.run_id.as_deref(),
        &input.agent_id,
        &input.working_directory,
    );
    AgentToolCandidateService::new(state.inner().clone())
        .list_candidates(&owner_window_label, input, candidates)
        .await
}

#[tauri::command]
pub async fn send_prompt_to_run(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let sink =
        TauriRunEventSink::with_target(app, state.inner().clone(), window.label().to_string());
    let registry = state.inner().clone();
    SendPromptUseCase::new(registry)
        .execute(sink, run_id, prompt)
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn steer_prompt_to_run(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let sink =
        TauriRunEventSink::with_target(app, state.inner().clone(), window.label().to_string());
    let registry = state.inner().clone();
    SteerPromptUseCase::new(registry)
        .execute(sink, run_id, prompt)
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn cancel_current_prompt_and_send_to_run(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let sink =
        TauriRunEventSink::with_target(app, state.inner().clone(), window.label().to_string());
    let registry = state.inner().clone();
    CancelPromptAndSendUseCase::new(registry)
        .execute(sink, run_id, prompt)
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn set_run_permission_mode(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    run_id: String,
    permission_mode: PermissionMode,
) -> Result<(), String> {
    let sink =
        TauriRunEventSink::with_target(app, state.inner().clone(), window.label().to_string());
    let registry = state.inner().clone();
    SetPermissionModeUseCase::new(registry)
        .execute(sink, run_id, permission_mode)
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let sink =
        TauriRunEventSink::with_target(app, state.inner().clone(), window.label().to_string());
    let registry = state.inner().clone();
    CancelAgentRunUseCase::new(registry)
        .execute(sink, run_id)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn respond_agent_permission(
    window: tauri::Window,
    state: State<'_, AppState>,
    run_id: String,
    permission_id: String,
    option_id: String,
) -> Result<(), String> {
    let owner = state
        .owner_of(&run_id)
        .await
        .ok_or_else(|| format!("unknown or finished run: {run_id}"))?;
    if owner != window.label() {
        return Err("permission response was sent from a non-owner window".to_string());
    }
    state
        .permissions()
        .respond_for_run(&run_id, &permission_id, PermissionDecision { option_id })
        .await
        .map_err(|err| err.to_string())
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

    // 회귀 방지: 과거 start_agent_run이 resume 필드를 None으로 덮어써 재사용이
    // 동작하지 않던 버그가 재발하지 않도록 보존을 검증한다.
    #[test]
    fn normalize_preserves_resume_fields() {
        let out = normalize_run_request(sample_request());
        assert_eq!(out.resume_session_id.as_deref(), Some("sess-1"));
        assert_eq!(out.resume_policy, Some(ResumePolicy::ResumeIfAvailable));
    }

    #[test]
    fn normalize_generates_run_id_and_clears_unsupported() {
        let out = normalize_run_request(sample_request());
        assert!(out.run_id.is_some_and(|id| !id.is_empty()));
        assert!(out.workspace_id.is_none());
        assert!(out.checkout_id.is_none());
    }

    #[test]
    fn normalize_sanitizes_ralph_loop_into_safe_range() {
        let mut request = sample_request();
        request.ralph_loop = Some(RalphLoopRequest {
            enabled: true,
            max_iterations: 10_000,
            prompt_template: "  continue  ".into(),
            stop_on_error: true,
            stop_on_permission: false,
            delay_ms: u64::MAX,
        });

        let loop_settings = normalize_run_request(request)
            .ralph_loop
            .expect("ralph loop should be preserved");
        assert_eq!(
            loop_settings.max_iterations,
            crate::domain::run::MAX_RALPH_ITERATIONS
        );
        assert_eq!(
            loop_settings.delay_ms,
            crate::domain::run::MAX_RALPH_DELAY_MS
        );
        assert_eq!(loop_settings.prompt_template, "continue");
    }

    #[test]
    fn normalize_keeps_existing_run_id() {
        let mut request = sample_request();
        request.run_id = Some("fixed-id".into());
        let out = normalize_run_request(request);
        assert_eq!(out.run_id.as_deref(), Some("fixed-id"));
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
