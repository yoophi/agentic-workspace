#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SavedPromptError {
    /// `"Button label is required."` / `"Prompt is required."`
    #[error("{0} is required.")]
    Required(&'static str),
    #[error("Saved prompt not found.")]
    NotFound,
    #[error("{0}")]
    Storage(String),
    /// 저장 파일은 있으나 파싱할 수 없음. coordinator가 lock 안에서 백업 복구를 시도한다.
    #[error("{0}")]
    StoreCorrupt(String),
    #[error("{0}")]
    Clock(String),
}

impl SavedPromptError {
    pub fn field_path(&self) -> Option<&'static str> {
        match self {
            SavedPromptError::Required("Button label") => Some("/label"),
            SavedPromptError::Required("Prompt") => Some("/prompt"),
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
            SavedPromptError::Required("Button label").to_string(),
            "Button label is required."
        );
        assert_eq!(
            SavedPromptError::Required("Prompt").to_string(),
            "Prompt is required."
        );
        assert_eq!(
            SavedPromptError::NotFound.to_string(),
            "Saved prompt not found."
        );
        assert_eq!(
            SavedPromptError::Required("Button label").field_path(),
            Some("/label")
        );
        assert_eq!(SavedPromptError::NotFound.field_path(), None);
    }
}
