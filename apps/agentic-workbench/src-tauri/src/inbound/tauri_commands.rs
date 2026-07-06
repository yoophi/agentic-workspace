use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    process::Command,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, State};

use crate::{
    application::{
        agent_run_settings_service, cancel_agent_run::CancelAgentRunUseCase, git_branch_service,
        git_remote_service, git_worktree_changes_service, git_worktree_service, goal_service,
        list_provider_sessions::ListProviderSessionsUseCase, project_service, saved_prompt_service,
        send_prompt::SendPromptUseCase, set_permission_mode::SetPermissionModeUseCase,
        start_agent_run::StartAgentRunUseCase, worktree_changes_service, worktree_file_service,
        worktree_git_service,
    },
    domain::{
        agent::AgentDescriptor,
        agent_run_settings::{
            APP_COMMAND_OVERRIDE_SETTINGS_KEY, AgentCommandSource, AgentRunSettings,
        },
        git_branch::GitBranch,
        git_remote::GitRemote,
        git_worktree::{GitWorktree, GitWorktreeCreateDraft},
        git_worktree_changes::{GitWorktreeChanges, GitWorktreeFileDiff},
        goal::{GoalDraft, GoalProgressUpdate, GoalStatus, GoalUpdate, ThreadGoal},
        project::{Project, ProjectDraft},
        provider_session::{ProviderSession, SessionScope},
        run::{AgentRun, AgentRunRequest, PermissionMode, RalphLoopRequest},
        saved_prompt::{SavedPrompt, SavedPromptDraft},
        worktree_change::WorktreeChange,
        worktree_file::{WorktreeFileEntry, WorktreeFileListScope, WorktreeTextFile},
        worktree_git::{
            GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff as WorktreeGitFileDiff,
        },
    },
    infrastructure::{
        acp::runner::AcpAgentRunner,
        agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState,
        fs_provider_session_repository::FsProviderSessionRepository,
        fs_worktree_file_provider::FsWorktreeFileProvider,
        fs_worktree_watcher::{WorktreeWatchHandle, watch_worktree},
        git_cli_branch_provider::GitCliBranchProvider,
        git_cli_remote_provider::GitCliRemoteProvider,
        git_cli_worktree_change_provider::GitCliWorktreeChangeProvider,
        git_cli_worktree_git_provider::GitCliWorktreeGitProvider,
        git_cli_worktree_provider::GitCliWorktreeProvider,
        json_acp_session_store::JsonAcpSessionStore,
        json_agent_run_settings_repository::JsonAgentRunSettingsRepository,
        json_goal_repository::JsonGoalRepository,
        json_project_repository::JsonProjectRepository,
        json_saved_prompt_repository::JsonSavedPromptRepository,
        mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV, McpLaunchEnv, McpServerState},
        perf_log::run_blocking_command,
        tauri_run_event_sink::TauriRunEventSink,
        window_manager,
    },
    ports::{agent_catalog::AgentCatalog, permission::PermissionDecision},
};

const WORKTREE_CHANGED_EVENT: &str = "workspace://worktree-changed";

pub struct WorktreeWatcherState {
    handles: Mutex<HashMap<String, WorktreeWatchHandle>>,
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
    name: String,
    working_directory: String,
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPromptInput {
    label: String,
    prompt: String,
}

impl From<SavedPromptInput> for SavedPromptDraft {
    fn from(input: SavedPromptInput) -> Self {
        Self {
            label: input.label,
            prompt: input.prompt,
        }
    }
}

impl From<ProjectInput> for ProjectDraft {
    fn from(input: ProjectInput) -> Self {
        Self {
            name: input.name,
            working_directory: input.working_directory,
            description: input.description,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalInput {
    working_directory: String,
    objective: String,
    token_budget: Option<usize>,
}

impl From<GoalInput> for GoalDraft {
    fn from(input: GoalInput) -> Self {
        Self {
            working_directory: input.working_directory,
            objective: input.objective,
            token_budget: input.token_budget,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalUpdateInput {
    objective: Option<String>,
    status: Option<GoalStatus>,
    token_budget: Option<Option<usize>>,
}

impl From<GoalUpdateInput> for GoalUpdate {
    fn from(input: GoalUpdateInput) -> Self {
        Self {
            objective: input.objective,
            status: input.status,
            token_budget: input.token_budget,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalProgressInput {
    tokens_used: usize,
    time_used_seconds: u64,
}

impl From<GoalProgressInput> for GoalProgressUpdate {
    fn from(input: GoalProgressInput) -> Self {
        Self {
            tokens_used: input.tokens_used,
            time_used_seconds: input.time_used_seconds,
        }
    }
}

#[tauri::command]
pub fn list_projects(app: AppHandle) -> Result<Vec<Project>, String> {
    let repository = JsonProjectRepository::from_app(&app)?;
    project_service::list_projects(&repository)
}

#[tauri::command]
pub fn create_project(app: AppHandle, input: ProjectInput) -> Result<Project, String> {
    let repository = JsonProjectRepository::from_app(&app)?;
    project_service::create_project(&repository, input.into())
}

#[tauri::command]
pub fn update_project(app: AppHandle, id: String, input: ProjectInput) -> Result<Project, String> {
    let repository = JsonProjectRepository::from_app(&app)?;
    project_service::update_project(&repository, id, input.into())
}

#[tauri::command]
pub fn delete_project(app: AppHandle, id: String) -> Result<(), String> {
    let repository = JsonProjectRepository::from_app(&app)?;
    project_service::delete_project(&repository, id)
}

#[tauri::command]
pub fn list_saved_prompts(app: AppHandle) -> Result<Vec<SavedPrompt>, String> {
    let repository = JsonSavedPromptRepository::from_app(&app)?;
    saved_prompt_service::list_saved_prompts(&repository)
}

#[tauri::command]
pub fn create_saved_prompt(app: AppHandle, input: SavedPromptInput) -> Result<SavedPrompt, String> {
    let repository = JsonSavedPromptRepository::from_app(&app)?;
    saved_prompt_service::create_saved_prompt(&repository, input.into())
}

#[tauri::command]
pub fn update_saved_prompt(
    app: AppHandle,
    id: String,
    input: SavedPromptInput,
) -> Result<SavedPrompt, String> {
    let repository = JsonSavedPromptRepository::from_app(&app)?;
    saved_prompt_service::update_saved_prompt(&repository, id, input.into())
}

#[tauri::command]
pub fn delete_saved_prompt(app: AppHandle, id: String) -> Result<(), String> {
    let repository = JsonSavedPromptRepository::from_app(&app)?;
    saved_prompt_service::delete_saved_prompt(&repository, id)
}

#[tauri::command]
pub fn get_goal(app: AppHandle, working_directory: String) -> Result<Option<ThreadGoal>, String> {
    let repository = JsonGoalRepository::from_app(&app)?;
    goal_service::get_goal(&repository, working_directory)
}

#[tauri::command]
pub fn create_goal(app: AppHandle, input: GoalInput) -> Result<ThreadGoal, String> {
    let repository = JsonGoalRepository::from_app(&app)?;
    goal_service::create_goal(&repository, input.into())
}

#[tauri::command]
pub fn update_goal(
    app: AppHandle,
    working_directory: String,
    input: GoalUpdateInput,
) -> Result<ThreadGoal, String> {
    let repository = JsonGoalRepository::from_app(&app)?;
    goal_service::update_goal(&repository, working_directory, input.into())
}

#[tauri::command]
pub fn clear_goal(app: AppHandle, working_directory: String) -> Result<(), String> {
    let repository = JsonGoalRepository::from_app(&app)?;
    goal_service::clear_goal(&repository, working_directory)
}

#[tauri::command]
pub fn record_goal_progress(
    app: AppHandle,
    working_directory: String,
    input: GoalProgressInput,
) -> Result<ThreadGoal, String> {
    let repository = JsonGoalRepository::from_app(&app)?;
    goal_service::record_goal_progress(&repository, working_directory, input.into())
}

#[tauri::command]
pub fn get_agent_run_settings(
    app: AppHandle,
    working_directory: String,
) -> Result<Option<AgentRunSettings>, String> {
    let repository = JsonAgentRunSettingsRepository::from_app(&app)?;
    agent_run_settings_service::get_settings(&repository, working_directory)
}

#[tauri::command]
pub fn save_agent_run_settings(
    app: AppHandle,
    settings: AgentRunSettings,
) -> Result<AgentRunSettings, String> {
    let repository = JsonAgentRunSettingsRepository::from_app(&app)?;
    agent_run_settings_service::save_settings(&repository, settings)
}

#[tauri::command]
pub async fn list_git_remotes(working_directory: String) -> Result<Vec<GitRemote>, String> {
    run_blocking_command("list_git_remotes", move || {
        git_remote_service::list_git_remotes(&GitCliRemoteProvider, working_directory)
    })
    .await
}

#[tauri::command]
pub async fn list_git_branches(working_directory: String) -> Result<Vec<GitBranch>, String> {
    run_blocking_command("list_git_branches", move || {
        git_branch_service::list_git_branches(&GitCliBranchProvider, working_directory)
    })
    .await
}

#[tauri::command]
pub async fn list_git_worktrees(
    working_directory: String,
    include_status: Option<bool>,
) -> Result<Vec<GitWorktree>, String> {
    run_blocking_command("list_git_worktrees", move || {
        git_worktree_service::list_git_worktrees(
            &GitCliWorktreeProvider,
            working_directory,
            include_status.unwrap_or(true),
        )
    })
    .await
}

#[tauri::command]
pub async fn list_worktree_changes(
    working_directory: String,
) -> Result<Vec<WorktreeChange>, String> {
    run_blocking_command("list_worktree_changes", move || {
        worktree_changes_service::list_worktree_changes(
            &GitCliWorktreeChangeProvider,
            working_directory,
        )
    })
    .await
}

#[tauri::command]
pub async fn create_git_worktree(
    working_directory: String,
    input: GitWorktreeCreateDraft,
) -> Result<(), String> {
    run_blocking_command("create_git_worktree", move || {
        git_worktree_service::create_git_worktree(&GitCliWorktreeProvider, working_directory, input)
    })
    .await
}

#[tauri::command]
pub async fn delete_git_worktree(working_directory: String, path: String) -> Result<(), String> {
    run_blocking_command("delete_git_worktree", move || {
        git_worktree_service::delete_git_worktree(&GitCliWorktreeProvider, working_directory, path)
    })
    .await
}

#[tauri::command]
pub async fn get_worktree_changes(working_directory: String) -> Result<GitWorktreeChanges, String> {
    run_blocking_command("get_worktree_changes", move || {
        git_worktree_changes_service::get_worktree_changes(
            &git_core::GitCliWorktreeStatusReader,
            working_directory,
        )
    })
    .await
}

#[tauri::command]
pub async fn get_worktree_file_diff(
    working_directory: String,
    path: String,
) -> Result<GitWorktreeFileDiff, String> {
    run_blocking_command("get_worktree_file_diff", move || {
        git_worktree_changes_service::get_worktree_file_diff(
            &git_core::GitCliWorktreeStatusReader,
            working_directory,
            path,
        )
    })
    .await
}

#[tauri::command]
pub async fn list_worktree_files(
    working_directory: String,
    scope: Option<WorktreeFileListScope>,
) -> Result<Vec<WorktreeFileEntry>, String> {
    run_blocking_command("list_worktree_files", move || {
        worktree_file_service::list_worktree_files(
            &FsWorktreeFileProvider,
            working_directory,
            scope,
        )
    })
    .await
}

#[tauri::command]
pub async fn read_worktree_text_file(
    working_directory: String,
    path: String,
) -> Result<WorktreeTextFile, String> {
    run_blocking_command("read_worktree_text_file", move || {
        worktree_file_service::read_worktree_text_file(
            &FsWorktreeFileProvider,
            working_directory,
            path,
        )
    })
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
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitHistory, String> {
    run_blocking_command("list_worktree_git_history", move || {
        worktree_git_service::list_worktree_git_history(
            &GitCliWorktreeGitProvider,
            working_directory,
            max_count,
            offset,
            cursor,
        )
    })
    .await
}

#[tauri::command]
pub async fn get_worktree_git_graph(
    working_directory: String,
    max_count: Option<usize>,
    offset: Option<usize>,
    cursor: Option<String>,
) -> Result<GitCommitGraph, String> {
    run_blocking_command("get_worktree_git_graph", move || {
        worktree_git_service::get_worktree_git_graph(
            &GitCliWorktreeGitProvider,
            working_directory,
            max_count,
            offset,
            cursor,
        )
    })
    .await
}

#[tauri::command]
pub async fn get_worktree_commit_detail(
    working_directory: String,
    commit_hash: String,
) -> Result<GitCommitDetail, String> {
    run_blocking_command("get_worktree_commit_detail", move || {
        worktree_git_service::get_worktree_commit_detail(
            &GitCliWorktreeGitProvider,
            working_directory,
            commit_hash,
        )
    })
    .await
}

#[tauri::command]
pub async fn get_worktree_commit_file_diff(
    working_directory: String,
    commit_hash: String,
    path: String,
) -> Result<WorktreeGitFileDiff, String> {
    run_blocking_command("get_worktree_commit_file_diff", move || {
        worktree_git_service::get_worktree_commit_file_diff(
            &GitCliWorktreeGitProvider,
            working_directory,
            commit_hash,
            path,
        )
    })
    .await
}

#[tauri::command]
pub fn list_agents() -> Vec<AgentDescriptor> {
    ConfigurableAgentCatalog::from_env().list_agents()
}

/// 선택한 provider(`agent_id`)가 로컬에 남긴 네이티브 세션을 조회한다.
/// `cwd`가 주어지면 해당 작업 디렉터리의 세션만, 없으면 전체를 돌려준다.
#[tauri::command]
pub fn list_provider_sessions(
    agent_id: String,
    cwd: Option<String>,
) -> Result<Vec<ProviderSession>, String> {
    let scope = match cwd {
        Some(path) if !path.trim().is_empty() => SessionScope::Path(path.into()),
        _ => SessionScope::All,
    };
    ListProviderSessionsUseCase::new(FsProviderSessionRepository::new())
        .execute(&agent_id, &scope, Some(50))
        .map_err(|error| error.to_string())
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

/// 클라이언트가 보낸 run 요청을 실행 직전 형태로 정규화한다. run_id를 보장하고
/// 아직 지원하지 않는 필드(workspace/checkout)는 비운다. ralph_loop은 안전 범위로
/// 정규화해 그대로 전달한다.
/// 단, resume_session_id/resume_policy는 **보존**해야 기존 세션 재사용이 동작한다.
fn normalize_run_request(mut request: AgentRunRequest) -> AgentRunRequest {
    if request.run_id.as_deref().is_none_or(str::is_empty) {
        request.run_id = Some(uuid::Uuid::new_v4().to_string());
    }
    request.workspace_id = None;
    request.checkout_id = None;
    request.ralph_loop = request.ralph_loop.map(RalphLoopRequest::sanitized);
    request
}

fn inject_mcp_launch_env(request: &mut AgentRunRequest, env: McpLaunchEnv) {
    let agent_env = request.agent_env.get_or_insert_with(BTreeMap::new);
    agent_env.insert(AW_MCP_URL_ENV.to_string(), env.url.clone());
    agent_env.insert(AW_MCP_TOKEN_ENV.to_string(), env.token.clone());
    agent_env.insert(AW_MCP_RUN_ID_ENV.to_string(), env.run_id.clone());
    request.mcp_servers.push(env.server_config());
    request.goal = with_mcp_agent_instructions(&request.goal, &env.agent_instructions());
}

fn with_mcp_agent_instructions(goal: &str, instructions: &str) -> String {
    format!(
        "{instructions}\n---\n\nUser request:\n{goal}",
        instructions = instructions.trim(),
        goal = goal.trim()
    )
}

#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    mcp_state: State<'_, McpServerState>,
    request: AgentRunRequest,
) -> Result<AgentRun, String> {
    let mut request = normalize_run_request(request);
    let run_id = request
        .run_id
        .clone()
        .ok_or_else(|| "agent run id is unavailable after normalization".to_string())?;
    inject_mcp_launch_env(&mut request, mcp_state.launch_env(&run_id));
    let catalog = ConfigurableAgentCatalog::from_env();
    if request
        .agent_command
        .as_deref()
        .is_none_or(|command| command.trim().is_empty())
    {
        let settings_repository = JsonAgentRunSettingsRepository::from_app(&app)?;
        if let Some(settings) = agent_run_settings_service::get_settings(
            &settings_repository,
            APP_COMMAND_OVERRIDE_SETTINGS_KEY.into(),
        )? {
            let resolution = agent_run_settings_service::resolve_agent_command(
                &request.agent_id,
                &settings.command_overrides,
                catalog.command_for_agent(&request.agent_id),
            )?;
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
            context_size: None,
            ralph_loop: None,
        }
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
