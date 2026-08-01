use sha2::{Digest, Sha256};

use crate::{
    domain::{
        document_identity::DocumentIdentity,
        review_session::{ReviewDecision, ReviewSession},
    },
    ports::{
        clock::Clock,
        review_session_repository::{ReviewRepositoryError, ReviewSessionRepository},
    },
};

pub struct ReviewSessionService<R, C> {
    repository: R,
    clock: C,
}

impl<R: ReviewSessionRepository, C: Clock> ReviewSessionService<R, C> {
    pub fn new(repository: R, clock: C) -> Self {
        Self { repository, clock }
    }

    pub fn load_or_create(
        &self,
        document: DocumentIdentity,
    ) -> Result<ReviewSession, ReviewRepositoryError> {
        let session_id = session_id(&document);
        match self.repository.load(&session_id) {
            Ok(session) => Ok(session),
            Err(ReviewRepositoryError::NotFound) => {
                let now = self.clock.now_iso8601();
                Ok(ReviewSession {
                    session_id,
                    schema_version: 1,
                    revision: 0,
                    document,
                    decision: ReviewDecision::Draft,
                    annotations: Vec::new(),
                    created_at: now.clone(),
                    updated_at: now,
                })
            }
            Err(error) => Err(error),
        }
    }

    pub fn save(
        &self,
        mut session: ReviewSession,
        expected_revision: u64,
    ) -> Result<ReviewSession, ReviewRepositoryError> {
        session.validate().map_err(ReviewRepositoryError::Corrupt)?;
        session.updated_at = self.clock.now_iso8601();
        self.repository.save(&session, expected_revision)
    }

    pub fn reconcile(&self, session: &mut ReviewSession, markdown: &str) {
        for annotation in &mut session.annotations {
            crate::application::review_reconciliation::reconcile_annotation(annotation, markdown);
        }
    }

    pub fn confirm_relink(
        &self,
        session: &mut ReviewSession,
        relative_path: String,
        fingerprint: String,
    ) {
        session.document.relative_path = relative_path;
        session.document.fingerprint = fingerprint;
    }
}

pub fn session_id(document: &DocumentIdentity) -> String {
    let mut hasher = Sha256::new();
    hasher.update(document.root_id.as_bytes());
    hasher.update([0]);
    hasher.update(document.relative_path.as_bytes());
    format!("{:x}", hasher.finalize())
}
