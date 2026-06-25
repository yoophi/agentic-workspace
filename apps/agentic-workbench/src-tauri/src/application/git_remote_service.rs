use crate::domain::{git_remote::GitRemote, git_remote_provider::GitRemoteProvider};

pub fn list_git_remotes(
    provider: &impl GitRemoteProvider,
    working_directory: String,
) -> Result<Vec<GitRemote>, String> {
    let working_directory = working_directory.trim();

    if working_directory.is_empty() {
        return Err("Working directory is required.".to_owned());
    }

    provider.list_remotes(working_directory)
}
