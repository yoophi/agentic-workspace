use crate::domain::git_branch::GitBranch;

pub trait GitBranchProvider {
    fn list_branches(&self, working_directory: &str) -> Result<Vec<GitBranch>, String>;
}
