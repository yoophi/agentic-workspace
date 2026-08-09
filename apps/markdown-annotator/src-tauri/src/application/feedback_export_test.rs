use super::*;
use crate::domain::{document_identity::DocumentIdentity, review_session::*};
fn session() -> ReviewSession {
    ReviewSession {
        session_id: "s".into(),
        schema_version: 1,
        revision: 1,
        document: DocumentIdentity {
            root_id: "r".into(),
            relative_path: "a.md".into(),
            fingerprint: "a".repeat(64),
            byte_length: 1,
            modified_at_ms: None,
        },
        decision: ReviewDecision::Approved,
        annotations: vec![],
        created_at: "".into(),
        updated_at: "".into(),
    }
}
#[test]
fn decision_only_export_is_deterministic() {
    let a = export_json(&session(), "2026-08-02T00:00:00Z", false, &[]).unwrap();
    let b = export_json(&session(), "2026-08-02T00:00:00Z", false, &[]).unwrap();
    assert_eq!(a, b);
    assert!(a.contains("\"schemaVersion\": 1"));
    assert!(export_markdown(&session(), false, &[]).contains("Approved"));
}
