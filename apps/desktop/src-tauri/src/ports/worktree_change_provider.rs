use crate::domain::worktree_change::WorktreeChange;

pub trait WorktreeChangeProvider {
    fn list_changes(&self, working_directory: String) -> Result<Vec<WorktreeChange>, String>;
}
