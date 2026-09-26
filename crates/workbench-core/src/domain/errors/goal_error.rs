#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GoalError {
    /// `"Working directory is required."` / `"Goal objective is required."`
    #[error("{0} is required.")]
    Required(&'static str),
    #[error("Goal not found.")]
    NotFound,
    #[error("{0}")]
    Storage(String),
    #[error("{0}")]
    StoreCorrupt(String),
}

impl GoalError {
    pub fn field_path(&self) -> Option<&'static str> {
        match self {
            GoalError::Required("Working directory") => Some("/workingDirectory"),
            GoalError::Required("Goal objective") => Some("/objective"),
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
            GoalError::Required("Working directory").to_string(),
            "Working directory is required."
        );
        assert_eq!(
            GoalError::Required("Goal objective").to_string(),
            "Goal objective is required."
        );
        assert_eq!(GoalError::NotFound.to_string(), "Goal not found.");
        assert_eq!(
            GoalError::Required("Goal objective").field_path(),
            Some("/objective")
        );
    }
}
