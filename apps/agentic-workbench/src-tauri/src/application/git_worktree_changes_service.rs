use crate::domain::{
    git_worktree_changes::{GitFileDiff, GitWorktreeChanges},
    git_worktree_changes_provider::GitWorktreeChangesProvider,
};

pub fn get_worktree_changes(
    provider: &impl GitWorktreeChangesProvider,
    working_directory: String,
) -> Result<GitWorktreeChanges, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.status(&working_directory)
}

pub fn get_worktree_file_diff(
    provider: &impl GitWorktreeChangesProvider,
    working_directory: String,
    path: String,
) -> Result<GitFileDiff, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let path = normalize_required(path, "File path")?;
    provider.diff(&working_directory, &path)
}

fn normalize_required(value: String, field_name: &str) -> Result<String, String> {
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Err(format!("{field_name} is required."));
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::git_worktree_changes::{
        GitChangedFile, GitChangedFileGroup, GitFileDiff, GitWorktreeChanges,
    };

    struct FakeChangesProvider;

    impl GitWorktreeChangesProvider for FakeChangesProvider {
        fn status(&self, working_directory: &str) -> Result<GitWorktreeChanges, String> {
            Ok(GitWorktreeChanges {
                working_directory: working_directory.to_string(),
                files: vec![GitChangedFile {
                    path: "src/main.ts".into(),
                    old_path: None,
                    staged_status: Some("M".into()),
                    unstaged_status: None,
                    group: GitChangedFileGroup::Staged,
                }],
                staged_count: 1,
                unstaged_count: 0,
                untracked_count: 0,
                conflicted_count: 0,
            })
        }

        fn diff(&self, _working_directory: &str, path: &str) -> Result<GitFileDiff, String> {
            Ok(GitFileDiff {
                path: path.to_string(),
                diff: "diff --git".into(),
                truncated: false,
                binary: false,
            })
        }
    }

    #[test]
    fn trims_working_directory_for_status_lookup() {
        let changes = get_worktree_changes(&FakeChangesProvider, " /repo ".into())
            .expect("changes should load");

        assert_eq!(changes.working_directory, "/repo");
        assert_eq!(changes.staged_count, 1);
    }

    #[test]
    fn rejects_blank_file_diff_path_before_provider_call() {
        let error = get_worktree_file_diff(&FakeChangesProvider, "/repo".into(), " ".into())
            .expect_err("blank path should fail");

        assert_eq!(error, "File path is required.");
    }
}
