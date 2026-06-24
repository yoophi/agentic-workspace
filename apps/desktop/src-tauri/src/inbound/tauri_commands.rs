use serde::Deserialize;
use std::{process::Command, sync::Arc};
use tauri::{AppHandle, State};

use crate::{
    application::{
        agent_run_settings_service, cancel_agent_run::CancelAgentRunUseCase, git_branch_service,
        git_remote_service, git_worktree_changes_service, git_worktree_service, goal_service,
        list_provider_sessions::ListProviderSessionsUseCase, project_service, saved_prompt_service,
        send_prompt::SendPromptUseCase, set_permission_mode::SetPermissionModeUseCase,
        start_agent_run::StartAgentRunUseCase,
    },
    domain::{
        agent::AgentDescriptor,
        agent_run_settings::AgentRunSettings,
        git_branch::GitBranch,
        git_remote::GitRemote,
        git_worktree::{GitWorktree, GitWorktreeCreateDraft},
        git_worktree_changes::{GitFileDiff, GitWorktreeChanges},
        goal::{GoalDraft, GoalProgressUpdate, GoalStatus, GoalUpdate, ThreadGoal},
        project::{Project, ProjectDraft},
        provider_session::{ProviderSession, SessionScope},
        run::{AgentRun, AgentRunRequest, PermissionMode, RalphLoopRequest},
        saved_prompt::{SavedPrompt, SavedPromptDraft},
    },
    infrastructure::{
        acp::runner::AcpAgentRunner, agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState,
        fs_provider_session_repository::FsProviderSessionRepository,
        git_cli_branch_provider::GitCliBranchProvider,
        git_cli_remote_provider::GitCliRemoteProvider,
        git_cli_worktree_changes_provider::GitCliWorktreeChangesProvider,
        git_cli_worktree_provider::GitCliWorktreeProvider,
        json_agent_run_settings_repository::JsonAgentRunSettingsRepository,
        json_goal_repository::JsonGoalRepository, json_project_repository::JsonProjectRepository,
        json_saved_prompt_repository::JsonSavedPromptRepository,
        noop_acp_session_store::NoopAcpSessionStore, tauri_run_event_sink::TauriRunEventSink,
        window_manager,
    },
    ports::{agent_catalog::AgentCatalog, permission::PermissionDecision},
};

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
pub fn list_git_remotes(working_directory: String) -> Result<Vec<GitRemote>, String> {
    git_remote_service::list_git_remotes(&GitCliRemoteProvider, working_directory)
}

#[tauri::command]
pub fn list_git_branches(working_directory: String) -> Result<Vec<GitBranch>, String> {
    git_branch_service::list_git_branches(&GitCliBranchProvider, working_directory)
}

#[tauri::command]
pub fn list_git_worktrees(working_directory: String) -> Result<Vec<GitWorktree>, String> {
    git_worktree_service::list_git_worktrees(&GitCliWorktreeProvider, working_directory)
}

#[tauri::command]
pub fn create_git_worktree(
    working_directory: String,
    input: GitWorktreeCreateDraft,
) -> Result<(), String> {
    git_worktree_service::create_git_worktree(&GitCliWorktreeProvider, working_directory, input)
}

#[tauri::command]
pub fn delete_git_worktree(working_directory: String, path: String) -> Result<(), String> {
    git_worktree_service::delete_git_worktree(&GitCliWorktreeProvider, working_directory, path)
}

#[tauri::command]
pub fn get_worktree_changes(working_directory: String) -> Result<GitWorktreeChanges, String> {
    git_worktree_changes_service::get_worktree_changes(
        &GitCliWorktreeChangesProvider,
        working_directory,
    )
}

#[tauri::command]
pub fn get_worktree_file_diff(
    working_directory: String,
    path: String,
) -> Result<GitFileDiff, String> {
    git_worktree_changes_service::get_worktree_file_diff(
        &GitCliWorktreeChangesProvider,
        working_directory,
        path,
    )
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

#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    window: tauri::Window,
    state: State<'_, AppState>,
    request: AgentRunRequest,
) -> Result<AgentRun, String> {
    let request = normalize_run_request(request);

    let owner_window_label = window.label().to_string();
    let sink =
        TauriRunEventSink::with_target(app, state.inner().clone(), owner_window_label.clone());
    let registry = state.inner().clone();
    let permissions = state.permissions();
    let runner = AcpAgentRunner::new(
        ConfigurableAgentCatalog::from_env(),
        permissions,
        Arc::new(NoopAcpSessionStore),
    );

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
