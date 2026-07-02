use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::domain::{
    git_worktree::{GitWorktree, GitWorktreeCreateDraft},
    git_worktree_provider::GitWorktreeProvider,
};

pub fn list_git_worktrees(
    provider: &impl GitWorktreeProvider,
    working_directory: String,
    include_status: bool,
) -> Result<Vec<GitWorktree>, String> {
    let working_directory = normalize_required_path(working_directory, "Working directory")?;
    provider.list_worktrees(&working_directory, include_status)
}

pub fn create_git_worktree(
    provider: &impl GitWorktreeProvider,
    working_directory: String,
    draft: GitWorktreeCreateDraft,
) -> Result<(), String> {
    let working_directory = normalize_required_path(working_directory, "Working directory")?;
    let branch = normalize_optional(draft.branch)
        .map(Ok)
        .unwrap_or_else(new_worktree_name)?;
    let path = normalize_optional(Some(draft.path))
        .map(Ok)
        .unwrap_or_else(|| default_worktree_path(&working_directory, Some(&branch)))?;
    let reference = normalize_optional(draft.reference);

    provider.create_worktree(
        &working_directory,
        GitWorktreeCreateDraft {
            path,
            branch: Some(branch),
            reference,
        },
    )
}

pub fn delete_git_worktree(
    provider: &impl GitWorktreeProvider,
    working_directory: String,
    path: String,
) -> Result<(), String> {
    let working_directory = normalize_required_path(working_directory, "Working directory")?;
    let path = normalize_required_path(path, "Worktree path")?;
    provider.delete_worktree(&working_directory, &path)
}

fn normalize_required_path(value: String, field_name: &str) -> Result<String, String> {
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Err(format!("{field_name} is required."));
    }

    Ok(value)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_owned();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn default_worktree_path(
    working_directory: &str,
    branch_name: Option<&str>,
) -> Result<String, String> {
    let project_dir = Path::new(working_directory);
    let project_name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Failed to resolve project directory name.".to_owned())?;
    let parent = project_dir
        .parent()
        .ok_or_else(|| "Failed to resolve project parent directory.".to_owned())?;
    let worktree_name = branch_name
        .map(sanitize_path_segment)
        .filter(|name| !name.is_empty())
        .map(Ok)
        .unwrap_or_else(new_worktree_name)?;

    Ok(parent
        .join("worktrees")
        .join(project_name)
        .join(worktree_name)
        .to_string_lossy()
        .into_owned())
}

fn new_worktree_name() -> Result<String, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Failed to generate worktree name: {error}"))?
        .as_nanos();

    Ok(format!("worktree-{nanos:x}"))
}

fn sanitize_path_segment(value: &str) -> String {
    let mut sanitized = String::new();
    let mut last_was_separator = false;

    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            sanitized.push(character);
            last_was_separator = false;
        } else if !last_was_separator {
            sanitized.push('-');
            last_was_separator = true;
        }
    }

    sanitized
        .trim_matches(|character| character == '-' || character == '.')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::git_worktree::GitWorktreeStatus;

    #[derive(Default)]
    struct FakeGitWorktreeProvider {
        worktrees: Vec<GitWorktree>,
        created: RefCell<Vec<(String, GitWorktreeCreateDraft)>>,
        deleted: RefCell<Vec<(String, String)>>,
    }

    impl GitWorktreeProvider for FakeGitWorktreeProvider {
        fn list_worktrees(
            &self,
            working_directory: &str,
            _include_status: bool,
        ) -> Result<Vec<GitWorktree>, String> {
            assert_eq!(working_directory, "/repo");
            Ok(self.worktrees.clone())
        }

        fn create_worktree(
            &self,
            working_directory: &str,
            draft: GitWorktreeCreateDraft,
        ) -> Result<(), String> {
            self.created
                .borrow_mut()
                .push((working_directory.to_string(), draft));
            Ok(())
        }

        fn delete_worktree(&self, working_directory: &str, path: &str) -> Result<(), String> {
            self.deleted
                .borrow_mut()
                .push((working_directory.to_string(), path.to_string()));
            Ok(())
        }
    }

    #[test]
    fn list_git_worktrees_trims_working_directory() {
        let provider = FakeGitWorktreeProvider {
            worktrees: vec![GitWorktree {
                path: "/repo".into(),
                head: Some("abc123".into()),
                branch: Some("main".into()),
                status: GitWorktreeStatus::Clean,
                prune_reason: None,
                can_delete: false,
            }],
            ..Default::default()
        };

        let worktrees =
            list_git_worktrees(&provider, " /repo ".into(), true).expect("list succeeds");

        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].path, "/repo");
    }

    #[test]
    fn create_git_worktree_sanitizes_branch_for_default_path() {
        let provider = FakeGitWorktreeProvider::default();

        create_git_worktree(
            &provider,
            "/Users/me/project/agentic-workbench".into(),
            GitWorktreeCreateDraft {
                path: " ".into(),
                branch: Some(" feature/user login! ".into()),
                reference: Some(" main ".into()),
            },
        )
        .expect("create succeeds");

        let created = provider.created.borrow();
        let (working_directory, draft) = created.first().expect("create call should be captured");
        assert_eq!(working_directory, "/Users/me/project/agentic-workbench");
        assert_eq!(draft.branch.as_deref(), Some("feature/user login!"));
        assert_eq!(draft.reference.as_deref(), Some("main"));
        assert!(
            draft
                .path
                .ends_with("/worktrees/agentic-workbench/feature-user-login")
        );
    }

    #[test]
    fn delete_git_worktree_rejects_blank_paths_before_provider_call() {
        let provider = FakeGitWorktreeProvider::default();

        let error = delete_git_worktree(&provider, "/repo".into(), " ".into())
            .expect_err("blank path should be rejected");

        assert_eq!(error, "Worktree path is required.");
        assert!(provider.deleted.borrow().is_empty());
    }
}
