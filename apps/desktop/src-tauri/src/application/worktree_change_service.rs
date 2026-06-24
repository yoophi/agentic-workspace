use crate::{
    domain::worktree_change::WorktreeChange,
    ports::worktree_change_provider::WorktreeChangeProvider,
};

pub fn list_worktree_changes(
    provider: &impl WorktreeChangeProvider,
    working_directory: String,
) -> Result<Vec<WorktreeChange>, String> {
    let working_directory = working_directory.trim();
    if working_directory.is_empty() {
        return Err("Working directory is required.".to_string());
    }

    provider.list_changes(working_directory.to_string())
}
