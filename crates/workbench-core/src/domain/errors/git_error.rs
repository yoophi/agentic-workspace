//! Git CLI 어댑터·서비스의 오류(research R7). Git이 낸 stderr는 **해석하지 않고** 문장 그대로 싣는다(grill Q5).
//! `Display`는 AW가 `Result<_, String>`으로 돌려주던 문구와 같다.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GitError {
    /// `"Working directory is required."` / `"Worktree path is required."` / `"File path is required."` /
    /// `"Commit hash is required."`
    #[error("{0} is required.")]
    Required(&'static str),
    /// git 실행 파일을 찾을 수 없음(`Command::output()`의 `ErrorKind::NotFound`). 문구는 오늘과 같은
    /// `"Failed to run git...: {error}"`.
    #[error("{0}")]
    GitNotFound(String),
    /// git 명령 비정상 종료 또는 그 밖의 실행 실패. 문구는 오늘과 같다(대개 git stderr 포함).
    #[error("{0}")]
    CommandFailed(String),
    /// Git 명령 전 파일시스템 준비 실패(worktree 부모 디렉터리 생성 등).
    #[error("{0}")]
    Io(String),
    /// 삭제 대상 경로가 `git worktree list`에 없다.
    #[error("Git worktree not found.")]
    WorktreeNotFound,
    #[error("Worktree has changes and cannot be deleted.")]
    WorktreeHasChanges,
    #[error("Worktree status is not resolved yet and cannot be deleted.")]
    StatusUnresolved,
    /// 기본 worktree 경로를 만들 수 없음: `"Failed to resolve project directory name."` /
    /// `"Failed to resolve project parent directory."`
    #[error("{0}")]
    Unresolvable(&'static str),
    #[error("{0}")]
    Clock(String),
}

impl GitError {
    pub fn field_path(&self) -> Option<&'static str> {
        match self {
            GitError::Required("Working directory") => Some("/workingDirectory"),
            GitError::Required("Worktree path") | GitError::Required("File path") => Some("/path"),
            GitError::Required("Commit hash") => Some("/commitHash"),
            GitError::Unresolvable(_) => Some("/workingDirectory"),
            _ => None,
        }
    }

    /// `Command::output()` 실패를 분류한다. 실행 파일 없음만 `GitNotFound`(→ `unavailable`, 재시도 가능)이다.
    pub fn spawn(context: &str, error: &std::io::Error) -> Self {
        let message = format!("{context}: {error}");
        if error.kind() == std::io::ErrorKind::NotFound {
            GitError::GitNotFound(message)
        } else {
            GitError::CommandFailed(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_legacy_strings() {
        assert_eq!(
            GitError::Required("Working directory").to_string(),
            "Working directory is required."
        );
        assert_eq!(
            GitError::Required("Worktree path").to_string(),
            "Worktree path is required."
        );
        assert_eq!(
            GitError::WorktreeHasChanges.to_string(),
            "Worktree has changes and cannot be deleted."
        );
        assert_eq!(
            GitError::StatusUnresolved.to_string(),
            "Worktree status is not resolved yet and cannot be deleted."
        );
        assert_eq!(
            GitError::WorktreeNotFound.to_string(),
            "Git worktree not found."
        );
        assert_eq!(
            GitError::Unresolvable("Failed to resolve project directory name.").to_string(),
            "Failed to resolve project directory name."
        );
    }

    #[test]
    fn spawn_errors_keep_context_and_classify_missing_binary() {
        let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
        let error = GitError::spawn("Failed to run git", &missing);
        assert!(matches!(error, GitError::GitNotFound(_)));
        assert!(error.to_string().starts_with("Failed to run git: "));
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert!(matches!(
            GitError::spawn("Failed to run git", &denied),
            GitError::CommandFailed(_)
        ));
    }
}
