use crate::domain::worktree_workspace_layout::WorkspaceLayoutSettings;
pub trait WorkspaceLayoutRepository {
    fn load_layouts(&self) -> Result<Vec<WorkspaceLayoutSettings>, String>;
    fn save_layouts(&self, layouts: &[WorkspaceLayoutSettings]) -> Result<(), String>;
}
