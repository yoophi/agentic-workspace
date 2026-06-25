use crate::domain::git_remote::GitRemote;

pub trait GitRemoteProvider {
    fn list_remotes(&self, working_directory: &str) -> Result<Vec<GitRemote>, String>;
}
