use serde::{Deserialize, Serialize};

use super::document_identity::DocumentIdentity;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnnotationType {
    ChangeRequest,
    Question,
    Note,
    Delete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStatus {
    Open,
    Resolved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttachmentState {
    Attached,
    Conflict,
    Orphan,
    Missing,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    pub block_id: Option<String>,
    pub selected_text: String,
    pub prefix: String,
    pub suffix: String,
    pub heading_path: Vec<String>,
    pub start_offset: Option<u64>,
    pub end_offset: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAnnotation {
    pub annotation_id: String,
    pub group_id: Option<String>,
    pub annotation_type: AnnotationType,
    pub status: AnnotationStatus,
    pub comment: String,
    pub anchor: Anchor,
    pub attachment_state: AttachmentState,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewDecision {
    Draft,
    ChangesRequested,
    Approved,
    Stopped,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSession {
    pub session_id: String,
    pub schema_version: u32,
    pub revision: u64,
    pub document: DocumentIdentity,
    pub decision: ReviewDecision,
    pub annotations: Vec<ReviewAnnotation>,
    pub created_at: String,
    pub updated_at: String,
}

impl ReviewSession {
    pub fn validate(&self) -> Result<(), String> {
        let mut annotation_ids = std::collections::HashSet::new();
        if self
            .annotations
            .iter()
            .any(|annotation| !annotation_ids.insert(annotation.annotation_id.as_str()))
        {
            return Err("annotation ID must be unique".into());
        }
        Ok(())
    }
    pub fn set_decision(
        &mut self,
        decision: ReviewDecision,
        confirmed: bool,
    ) -> Result<(), String> {
        let blocking = self.annotations.iter().any(|annotation| {
            annotation.status == AnnotationStatus::Open
                && matches!(
                    annotation.annotation_type,
                    AnnotationType::ChangeRequest | AnnotationType::Delete
                )
        });
        if decision == ReviewDecision::Approved && blocking && !confirmed {
            return Err("approval requires confirmation".into());
        }
        self.decision = decision;
        Ok(())
    }
}

#[cfg(test)]
#[path = "review_session_test.rs"]
mod tests;
