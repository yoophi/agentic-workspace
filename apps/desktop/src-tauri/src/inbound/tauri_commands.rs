use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::{
    application::{
        cancel_agent_run::CancelAgentRunUseCase, git_branch_service, git_remote_service,
        git_worktree_service, project_service, start_agent_run::StartAgentRunUseCase,
    },
    domain::{
        agent::AgentDescriptor,
        git_branch::GitBranch,
        git_remote::GitRemote,
        git_worktree::{GitWorktree, GitWorktreeCreateDraft},
        project::{Project, ProjectDraft},
        run::{AgentRun, AgentRunRequest},
    },
    infrastructure::{
        acp::runner::AcpAgentRunner, agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState, git_cli_branch_provider::GitCliBranchProvider,
        git_cli_remote_provider::GitCliRemoteProvider,
        git_cli_worktree_provider::GitCliWorktreeProvider,
        json_project_repository::JsonProjectRepository,
        noop_acp_session_store::NoopAcpSessionStore, tauri_run_event_sink::TauriRunEventSink,
    },
    ports::agent_catalog::AgentCatalog,
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

#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    state: State<'_, AppState>,
    mut request: AgentRunRequest,
) -> Result<AgentRun, String> {
    if request.run_id.as_deref().is_none_or(str::is_empty) {
        request.run_id = Some(uuid::Uuid::new_v4().to_string());
    }
    request.workspace_id = None;
    request.checkout_id = None;
    request.resume_session_id = None;
    request.resume_policy = None;
    request.ralph_loop = None;

    let sink = TauriRunEventSink::new(app, state.inner().clone());
    let registry = state.inner().clone();
    let permissions = state.permissions();
    let runner = AcpAgentRunner::new(
        ConfigurableAgentCatalog::from_env(),
        permissions,
        Arc::new(NoopAcpSessionStore),
    );

    StartAgentRunUseCase::new(registry)
        .execute(runner, sink, request, None)
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let sink = TauriRunEventSink::new(app, state.inner().clone());
    let registry = state.inner().clone();
    CancelAgentRunUseCase::new(registry)
        .execute(sink, run_id)
        .await;
    Ok(())
}
