use crate::domain::{git_branch::GitBranch, git_branch_provider::GitBranchProvider};

pub fn list_git_branches(
    provider: &impl GitBranchProvider,
    working_directory: String,
) -> Result<Vec<GitBranch>, String> {
    let working_directory = working_directory.trim();

    if working_directory.is_empty() {
        return Err("Working directory is required.".to_owned());
    }

    provider.list_branches(working_directory)
}
