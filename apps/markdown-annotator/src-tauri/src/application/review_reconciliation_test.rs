use super::*;
use crate::domain::review_session::*;
fn annotation(text: &str) -> ReviewAnnotation {
    ReviewAnnotation {
        annotation_id: "a".into(),
        group_id: None,
        annotation_type: AnnotationType::Note,
        status: AnnotationStatus::Open,
        comment: "".into(),
        anchor: Anchor {
            block_id: None,
            selected_text: text.into(),
            prefix: "".into(),
            suffix: "".into(),
            heading_path: vec![],
            start_offset: None,
            end_offset: None,
        },
        attachment_state: AttachmentState::Attached,
        created_at: "".into(),
        updated_at: "".into(),
    }
}
#[test]
fn uses_exact_unique_match_only() {
    let mut a = annotation("needle");
    reconcile_annotation(&mut a, "a needle b");
    assert_eq!(a.attachment_state, AttachmentState::Attached);
    reconcile_annotation(&mut a, "needle needle");
    assert_eq!(a.attachment_state, AttachmentState::Conflict);
    reconcile_annotation(&mut a, "none");
    assert_eq!(a.attachment_state, AttachmentState::Missing);
}
#[test]
fn proposes_only_one_fingerprint_rename() {
    assert_eq!(
        fingerprint_relink_candidate("f", &[("new.md".into(), "f".into())]),
        Some("new.md".into())
    );
    assert_eq!(
        fingerprint_relink_candidate(
            "f",
            &[("a.md".into(), "f".into()), ("b.md".into(), "f".into())]
        ),
        None
    );
}
