//! `worktree.listChanges`·`worktree.getChanges`·`worktree.getFileDiff`(AW `worktree_changes_service`·
//! `git_worktree_changes_service`에서 이동).

use git_core::domain::{GitWorktreeChanges, GitWorktreeFileDiff};

use crate::{
    application::git_service::normalize_required,
    domain::{errors::GitError, worktree_change::WorktreeChange},
    ports::git_providers::{WorktreeChangeProvider, WorktreeStatusProvider},
};

pub fn list_worktree_changes(
    provider: &impl WorktreeChangeProvider,
    working_directory: String,
) -> Result<Vec<WorktreeChange>, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.list_changes(&working_directory)
}

pub fn get_worktree_changes(
    provider: &impl WorktreeStatusProvider,
    working_directory: String,
) -> Result<GitWorktreeChanges, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.status(&working_directory)
}

pub fn get_worktree_file_diff(
    provider: &impl WorktreeStatusProvider,
    working_directory: String,
    path: String,
) -> Result<GitWorktreeFileDiff, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let path = normalize_required(path, "File path")?;
    provider.diff(&working_directory, &path)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use git_core::{GitChangedFile, GitChangedFileGroup};

    use super::*;
    use crate::domain::worktree_change::WorktreeChangeType;

    struct StubProvider {
        calls: RefCell<Vec<String>>,
        changes: Vec<WorktreeChange>,
    }

    impl WorktreeChangeProvider for StubProvider {
        fn list_changes(&self, working_directory: &str) -> Result<Vec<WorktreeChange>, GitError> {
            self.calls.borrow_mut().push(working_directory.to_owned());
            Ok(self.changes.clone())
        }
    }

    fn change(path: &str) -> WorktreeChange {
        WorktreeChange {
            path: path.to_owned(),
            old_path: None,
            change_type: WorktreeChangeType::Modified,
            binary: false,
            diff: Some("diff".to_owned()),
            content: None,
            truncated: false,
        }
    }

    #[test]
    fn rejects_blank_working_directory() {
        let provider = StubProvider {
            calls: RefCell::new(Vec::new()),
            changes: Vec::new(),
        };
        let result = list_worktree_changes(&provider, "   ".to_owned());
        assert_eq!(result.unwrap_err(), GitError::Required("Working directory"));
        assert!(provider.calls.borrow().is_empty());
    }

    #[test]
    fn trims_directory_and_delegates_to_provider() {
        let provider = StubProvider {
            calls: RefCell::new(Vec::new()),
            changes: vec![change("src/lib.rs")],
        };
        let result = list_worktree_changes(&provider, " /repo/worktree ".to_owned())
            .expect("changes should be listed");
        assert_eq!(result.len(), 1);
        assert_eq!(provider.calls.borrow().as_slice(), ["/repo/worktree"]);
    }

    struct FakeReader;

    impl WorktreeStatusProvider for FakeReader {
        fn status(&self, working_directory: &str) -> Result<GitWorktreeChanges, GitError> {
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

        fn diff(
            &self,
            _working_directory: &str,
            path: &str,
        ) -> Result<GitWorktreeFileDiff, GitError> {
            Ok(GitWorktreeFileDiff {
                path: path.to_string(),
                content: "diff --git".into(),
                is_binary: false,
                is_truncated: false,
            })
        }
    }

    #[test]
    fn trims_working_directory_for_status_lookup() {
        let changes =
            get_worktree_changes(&FakeReader, " /repo ".into()).expect("changes should load");
        assert_eq!(changes.working_directory, "/repo");
        assert_eq!(changes.staged_count, 1);
    }

    #[test]
    fn rejects_blank_file_diff_path_before_provider_call() {
        let error = get_worktree_file_diff(&FakeReader, "/repo".into(), " ".into())
            .expect_err("blank path should fail");
        assert_eq!(error.to_string(), "File path is required.");
    }
}
