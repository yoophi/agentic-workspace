//! worktree 파일 목록·미리보기 오류(research R7). 안전 규칙의 문구와 값은 AW와 같다(FR-007).

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorktreeFileError {
    /// `"Working directory is required."` / `"File path is required."`
    #[error("{0} is required.")]
    Required(&'static str),
    #[error("Working directory must be a directory.")]
    NotADirectory,
    #[error("File path must stay inside the worktree.")]
    OutsideWorktree,
    #[error("Only regular files can be previewed.")]
    NotRegularFile,
    #[error("Only UTF-8 text files can be previewed.")]
    NotUtf8,
    /// 사전에 확인 가능한 경로 없음(`canonicalize`의 `ErrorKind::NotFound`). 문구는 오늘과 같다.
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Io(String),
}

impl WorktreeFileError {
    pub fn field_path(&self) -> Option<&'static str> {
        match self {
            WorktreeFileError::Required("Working directory") => Some("/workingDirectory"),
            WorktreeFileError::Required("File path") | WorktreeFileError::NotUtf8 => Some("/path"),
            _ => None,
        }
    }

    /// 경로 해석 실패를 없음/기타로 나눈다. 문구는 호출자가 만든 그대로.
    pub fn from_io(message: String, error: &std::io::Error) -> Self {
        if error.kind() == std::io::ErrorKind::NotFound {
            WorktreeFileError::NotFound(message)
        } else {
            WorktreeFileError::Io(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_legacy_strings() {
        assert_eq!(
            WorktreeFileError::OutsideWorktree.to_string(),
            "File path must stay inside the worktree."
        );
        assert_eq!(
            WorktreeFileError::NotADirectory.to_string(),
            "Working directory must be a directory."
        );
        assert_eq!(
            WorktreeFileError::NotRegularFile.to_string(),
            "Only regular files can be previewed."
        );
        assert_eq!(
            WorktreeFileError::NotUtf8.to_string(),
            "Only UTF-8 text files can be previewed."
        );
        assert_eq!(
            WorktreeFileError::Required("File path").to_string(),
            "File path is required."
        );
    }
}
