//! `git.listWorktrees`·`git.createWorktree`·`git.deleteWorktree`(AW `git_worktree_service`에서 이동).
//!
//! 생성은 두 단계로 나뉜다: [`normalize_create_request`]가 호출자 의도를 정규화하고(멱등성 지문의 입력),
//! [`resolve_create_draft`]가 기본 branch·경로를 채운다(예약과 실행이 같은 경로를 쓴다, FR-005).

use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

use crate::{
    application::git_service::normalize_required,
    domain::{
        errors::GitError,
        git_worktree::{GitWorktree, GitWorktreeCreateDraft},
    },
    ports::git_providers::GitWorktreeProvider,
};

pub fn list_git_worktrees(
    provider: &impl GitWorktreeProvider,
    working_directory: String,
    include_status: bool,
) -> Result<Vec<GitWorktree>, GitError> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.list_worktrees(&working_directory, include_status)
}

/// 정규화된 생성 요청(trim, 빈 문자열 → 없음). 기본값은 아직 채우지 않는다 — 같은 멱등성 키의 재요청이
/// 같은 지문을 갖게 하기 위해서다(기본 branch 이름은 시각에서 만든다).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorktreeRequest {
    pub working_directory: String,
    pub path: Option<String>,
    pub branch: Option<String>,
    pub reference: Option<String>,
}

pub fn normalize_create_request(
    working_directory: String,
    draft: GitWorktreeCreateDraft,
) -> Result<CreateWorktreeRequest, GitError> {
    Ok(CreateWorktreeRequest {
        working_directory: normalize_required(working_directory, "Working directory")?,
        path: normalize_optional(Some(draft.path)),
        branch: normalize_optional(draft.branch),
        reference: normalize_optional(draft.reference),
    })
}

/// 기본 branch(`worktree-{nanos:x}`)와 기본 경로(`<parent>/worktrees/<repoName>/<branch>`)를 채운다.
pub fn resolve_create_draft(
    request: &CreateWorktreeRequest,
) -> Result<GitWorktreeCreateDraft, GitError> {
    let branch = match &request.branch {
        Some(branch) => branch.clone(),
        None => new_worktree_name()?,
    };
    let path = match &request.path {
        Some(path) => path.clone(),
        None => default_worktree_path(&request.working_directory, Some(&branch))?,
    };
    Ok(GitWorktreeCreateDraft {
        path,
        branch: Some(branch),
        reference: request.reference.clone(),
    })
}

pub fn create_git_worktree(
    provider: &impl GitWorktreeProvider,
    working_directory: String,
    draft: GitWorktreeCreateDraft,
) -> Result<(), GitError> {
    let request = normalize_create_request(working_directory, draft)?;
    let draft = resolve_create_draft(&request)?;
    provider.create_worktree(&request.working_directory, draft)
}

/// 삭제 요청의 정규화(두 필드 필수, trim).
pub fn normalize_delete_request(
    working_directory: String,
    path: String,
) -> Result<(String, String), GitError> {
    Ok((
        normalize_required(working_directory, "Working directory")?,
        normalize_required(path, "Worktree path")?,
    ))
}

pub fn delete_git_worktree(
    provider: &impl GitWorktreeProvider,
    working_directory: String,
    path: String,
) -> Result<(), GitError> {
    let (working_directory, path) = normalize_delete_request(working_directory, path)?;
    provider.delete_worktree(&working_directory, &path)
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
) -> Result<String, GitError> {
    let project_dir = Path::new(working_directory);
    let project_name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(GitError::Unresolvable(
            "Failed to resolve project directory name.",
        ))?;
    let parent = project_dir.parent().ok_or(GitError::Unresolvable(
        "Failed to resolve project parent directory.",
    ))?;
    let worktree_name = match branch_name
        .map(sanitize_path_segment)
        .filter(|name| !name.is_empty())
    {
        Some(name) => name,
        None => new_worktree_name()?,
    };

    Ok(parent
        .join("worktrees")
        .join(project_name)
        .join(worktree_name)
        .to_string_lossy()
        .into_owned())
}

fn new_worktree_name() -> Result<String, GitError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| GitError::Clock(format!("Failed to generate worktree name: {error}")))?
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
        ) -> Result<Vec<GitWorktree>, GitError> {
            assert_eq!(working_directory, "/repo");
            Ok(self.worktrees.clone())
        }

        fn create_worktree(
            &self,
            working_directory: &str,
            draft: GitWorktreeCreateDraft,
        ) -> Result<(), GitError> {
            self.created
                .borrow_mut()
                .push((working_directory.to_string(), draft));
            Ok(())
        }

        fn delete_worktree(&self, working_directory: &str, path: &str) -> Result<(), GitError> {
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
        assert!(draft
            .path
            .ends_with("/worktrees/agentic-workbench/feature-user-login"));
    }

    #[test]
    fn delete_git_worktree_rejects_blank_paths_before_provider_call() {
        let provider = FakeGitWorktreeProvider::default();

        let error = delete_git_worktree(&provider, "/repo".into(), " ".into())
            .expect_err("blank path should be rejected");

        assert_eq!(error, GitError::Required("Worktree path"));
        assert_eq!(error.to_string(), "Worktree path is required.");
        assert!(provider.deleted.borrow().is_empty());
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    #[test]
    fn normalize_keeps_intent_and_resolve_fills_defaults() {
        let request = normalize_create_request(
            " /Users/me/project/aw ".into(),
            GitWorktreeCreateDraft {
                path: "  ".into(),
                branch: None,
                reference: Some(" ".into()),
            },
        )
        .unwrap();
        assert_eq!(
            request,
            CreateWorktreeRequest {
                working_directory: "/Users/me/project/aw".into(),
                path: None,
                branch: None,
                reference: None,
            }
        );
        let draft = resolve_create_draft(&request).unwrap();
        let branch = draft.branch.clone().unwrap();
        assert!(branch.starts_with("worktree-"), "{branch}");
        assert_eq!(
            draft.path,
            format!("/Users/me/project/worktrees/aw/{branch}")
        );
    }

    #[test]
    fn root_directory_cannot_derive_a_default_path() {
        let request = normalize_create_request(
            "/".into(),
            GitWorktreeCreateDraft {
                path: String::new(),
                branch: Some("b".into()),
                reference: None,
            },
        )
        .unwrap();
        let error = resolve_create_draft(&request).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Failed to resolve project directory name."
        );
    }
}
