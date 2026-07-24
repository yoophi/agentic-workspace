use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use crate::domain::{worktree_workspace_layout::WorkspaceLayoutSettings, worktree_workspace_layout_repository::WorkspaceLayoutRepository};
use crate::infrastructure::json_store::{load_json_vec, save_json_vec};
pub struct JsonWorkspaceLayoutRepository { store_path: PathBuf }
impl JsonWorkspaceLayoutRepository {
 pub fn from_app(app: &AppHandle) -> Result<Self, String> { let dir = app.path().app_data_dir().map_err(|e| format!("Failed to resolve app data directory: {e}"))?; fs::create_dir_all(&dir).map_err(|e| format!("Failed to create app data directory: {e}"))?; Ok(Self { store_path: dir.join("worktree-workspace-layouts.json") }) }
}
impl WorkspaceLayoutRepository for JsonWorkspaceLayoutRepository {
 fn load_layouts(&self) -> Result<Vec<WorkspaceLayoutSettings>, String> { load_json_vec(&self.store_path, "worktree workspace layouts") }
 fn save_layouts(&self, layouts: &[WorkspaceLayoutSettings]) -> Result<(), String> { save_json_vec(&self.store_path, "worktree workspace layouts", layouts) }
}
