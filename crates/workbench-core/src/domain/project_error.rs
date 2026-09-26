//! 프로젝트 도메인 오류. `Display`는 AW가 `Result<_, String>`으로 돌려주던 문구와 바이트 단위로 같다
//! (`specs/037-workbench-seam/contracts/tauri-compat-commands.md` 골든).

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectError {
    #[error("Project name is required.")]
    NameRequired,
    #[error("Working directory is required.")]
    WorkingDirectoryRequired,
    #[error("Project not found.")]
    NotFound,
    /// 저장 파일 읽기·쓰기 실패. 메시지는 json_store가 만든 문구 그대로.
    #[error("{0}")]
    Storage(String),
    /// 저장 파일은 있으나 파싱할 수 없음. coordinator가 lock 안에서 백업 복구를 시도한다.
    #[error("{0}")]
    StoreCorrupt(String),
    #[error("{0}")]
    Clock(String),
}

impl ProjectError {
    /// 검증 오류의 JSON pointer 경로. Fault `details.path`에 실린다.
    pub fn field_path(&self) -> Option<&'static str> {
        match self {
            ProjectError::NameRequired => Some("/name"),
            ProjectError::WorkingDirectoryRequired => Some("/workingDirectory"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matches_legacy_strings() {
        assert_eq!(
            ProjectError::NameRequired.to_string(),
            "Project name is required."
        );
        assert_eq!(
            ProjectError::WorkingDirectoryRequired.to_string(),
            "Working directory is required."
        );
        assert_eq!(ProjectError::NotFound.to_string(), "Project not found.");
        assert_eq!(
            ProjectError::Storage("Failed to read projects store x: y".into()).to_string(),
            "Failed to read projects store x: y"
        );
        assert_eq!(ProjectError::StoreCorrupt("bad".into()).to_string(), "bad");
        assert_eq!(
            ProjectError::Clock("Failed to generate project id: e".into()).to_string(),
            "Failed to generate project id: e"
        );
    }

    #[test]
    fn validation_errors_expose_field_paths() {
        assert_eq!(ProjectError::NameRequired.field_path(), Some("/name"));
        assert_eq!(
            ProjectError::WorkingDirectoryRequired.field_path(),
            Some("/workingDirectory")
        );
        assert_eq!(ProjectError::NotFound.field_path(), None);
    }
}
