//! `git.listRemotes`·`git.listBranches`(AW `git_remote_service`·`git_branch_service`에서 이동).

use crate::{
    domain::{errors::GitError, git_branch::GitBranch, git_remote::GitRemote},
    ports::git_providers::{GitBranchProvider, GitRemoteProvider},
};

pub fn list_git_remotes(
    provider: &impl GitRemoteProvider,
    working_directory: String,
) -> Result<Vec<GitRemote>, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.list_remotes(&working_directory)
}

pub fn list_git_branches(
    provider: &impl GitBranchProvider,
    working_directory: String,
) -> Result<Vec<GitBranch>, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.list_branches(&working_directory)
}

pub(crate) fn normalize_required(value: String, label: &'static str) -> Result<String, GitError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(GitError::Required(label));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub;

    impl GitRemoteProvider for Stub {
        fn list_remotes(&self, working_directory: &str) -> Result<Vec<GitRemote>, GitError> {
            assert_eq!(working_directory, "/repo");
            Ok(Vec::new())
        }
    }

    #[test]
    fn trims_and_requires_working_directory() {
        assert!(list_git_remotes(&Stub, " /repo ".into())
            .unwrap()
            .is_empty());
        assert_eq!(
            list_git_remotes(&Stub, "  ".into()).unwrap_err(),
            GitError::Required("Working directory")
        );
    }
}
