use crate::domain::review_session::ReviewSession;

#[derive(Debug, PartialEq)]
pub enum ReviewRepositoryError {
    NotFound,
    RevisionConflict { expected: u64, actual: u64 },
    UnsupportedSchema(u32),
    Corrupt(String),
    Io(String),
}
impl std::fmt::Display for ReviewRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ReviewRepositoryError {}

pub trait ReviewSessionRepository {
    fn load(&self, session_id: &str) -> Result<ReviewSession, ReviewRepositoryError>;
    fn save(
        &self,
        session: &ReviewSession,
        expected_revision: u64,
    ) -> Result<ReviewSession, ReviewRepositoryError>;
    fn trash(&self, session_id: &str) -> Result<(), ReviewRepositoryError>;
}
