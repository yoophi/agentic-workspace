use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::{
    application::{
        cancel_agent_run::CancelAgentRunUseCase, git_branch_service, git_remote_service,
        git_worktree_service, list_provider_sessions::ListProviderSessionsUseCase, project_service,
        send_prompt::SendPromptUseCase, start_agent_run::StartAgentRunUseCase,
    },
    domain::{
        agent::AgentDescriptor,
        git_branch::GitBranch,
        git_remote::GitRemote,
        git_worktree::{GitWorktree, GitWorktreeCreateDraft},
        project::{Project, ProjectDraft},
        provider_session::{ProviderSession, SessionScope},
        run::{AgentRun, AgentRunRequest},
    },
    infrastructure::{
        acp::runner::AcpAgentRunner, agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState,
        fs_provider_session_repository::FsProviderSessionRepository,
        git_cli_branch_provider::GitCliBranchProvider,
        git_cli_remote_provider::GitCliRemoteProvider,
        git_cli_worktree_provider::GitCliWorktreeProvider,
        json_project_repository::JsonProjectRepository,
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

impl From<ProjectInput> for ProjectDraft {
    fn from(input: ProjectInput) -> Self {
        Self {
            name: input.name,
            working_directory: input.working_directory,
            description: input.description,
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
    worktree_path: String,
    mode: String,
) -> Result<(), String> {
    window_manager::open_session_window(&app, &project_id, &worktree_path, &mode)
}

/// 클라이언트가 보낸 run 요청을 실행 직전 형태로 정규화한다. run_id를 보장하고
/// 아직 지원하지 않는 필드(workspace/checkout/ralph_loop)는 비운다.
/// 단, resume_session_id/resume_policy는 **보존**해야 기존 세션 재사용이 동작한다.
fn normalize_run_request(mut request: AgentRunRequest) -> AgentRunRequest {
    if request.run_id.as_deref().is_none_or(str::is_empty) {
        request.run_id = Some(uuid::Uuid::new_v4().to_string());
    }
    request.workspace_id = None;
    request.checkout_id = None;
    request.ralph_loop = None;
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
        assert!(out.ralph_loop.is_none());
    }

    #[test]
    fn normalize_keeps_existing_run_id() {
        let mut request = sample_request();
        request.run_id = Some("fixed-id".into());
        let out = normalize_run_request(request);
        assert_eq!(out.run_id.as_deref(), Some("fixed-id"));
    }
}
