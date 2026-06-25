use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{
    adapters::outbound::{
        git_cli::GitCliRepositoryValidator, json_repository_store::JsonRepositoryStore,
    },
    application::{
        branch_service::BranchService, history_service::HistoryService,
        repository_service::RepositoryService, worktree_service::WorktreeService,
    },
    domain::{
        branch::GitBranch,
        commit::{GitCommitDetail, GitCommitSummary, GitFileDiff},
        repository::Repository,
        worktree::GitWorktree,
    },
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRepositoryRequest {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryRequest {
    repository_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCommitDetailRequest {
    repository_id: String,
    commit_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFileDiffRequest {
    repository_id: String,
    commit_hash: String,
    file_path: String,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
pub fn list_repositories(app: AppHandle) -> Result<Vec<Repository>, String> {
    repository_service(app)?.list_repositories()
}

#[tauri::command]
pub fn create_repository(
    app: AppHandle,
    request: CreateRepositoryRequest,
) -> Result<Repository, String> {
    repository_service(app)?.create_repository(request.path)
}

#[tauri::command]
pub fn list_worktrees(
    app: AppHandle,
    request: RepositoryRequest,
) -> Result<Vec<GitWorktree>, String> {
    worktree_service(app)?.list_worktrees(request.repository_id)
}

#[tauri::command]
pub fn list_branches(app: AppHandle, request: RepositoryRequest) -> Result<Vec<GitBranch>, String> {
    branch_service(app)?.list_branches(request.repository_id)
}

#[tauri::command]
pub fn list_history(
    app: AppHandle,
    request: RepositoryRequest,
) -> Result<Vec<GitCommitSummary>, String> {
    history_service(app)?.list_history(request.repository_id)
}

#[tauri::command]
pub fn get_commit_detail(
    app: AppHandle,
    request: GetCommitDetailRequest,
) -> Result<GitCommitDetail, String> {
    history_service(app)?.get_commit_detail(request.repository_id, request.commit_hash)
}

#[tauri::command]
pub fn get_file_diff(app: AppHandle, request: GetFileDiffRequest) -> Result<GitFileDiff, String> {
    history_service(app)?.get_file_diff(
        request.repository_id,
        request.commit_hash,
        request.file_path,
    )
}

fn repository_service(
    app: AppHandle,
) -> Result<RepositoryService<JsonRepositoryStore, GitCliRepositoryValidator>, String> {
    let store = repository_store(app)?;
    let validator = GitCliRepositoryValidator;

    Ok(RepositoryService::new(store, validator))
}

fn worktree_service(
    app: AppHandle,
) -> Result<WorktreeService<JsonRepositoryStore, GitCliRepositoryValidator>, String> {
    let store = repository_store(app)?;
    let reader = GitCliRepositoryValidator;

    Ok(WorktreeService::new(store, reader))
}

fn branch_service(
    app: AppHandle,
) -> Result<BranchService<JsonRepositoryStore, GitCliRepositoryValidator>, String> {
    let store = repository_store(app)?;
    let reader = GitCliRepositoryValidator;

    Ok(BranchService::new(store, reader))
}

fn history_service(
    app: AppHandle,
) -> Result<HistoryService<JsonRepositoryStore, GitCliRepositoryValidator>, String> {
    let store = repository_store(app)?;
    let reader = GitCliRepositoryValidator;

    Ok(HistoryService::new(store, reader))
}

fn repository_store(app: AppHandle) -> Result<JsonRepositoryStore, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

    Ok(JsonRepositoryStore::new(
        app_data_dir.join("repositories.json"),
    ))
}
