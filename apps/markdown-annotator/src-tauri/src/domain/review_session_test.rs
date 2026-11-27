use super::*;

fn session() -> ReviewSession {
    ReviewSession {
        session_id: "s".into(),
        schema_version: 1,
        revision: 0,
        document: DocumentIdentity {
            root_id: "r".into(),
            relative_path: "a.md".into(),
            fingerprint: "f".repeat(64),
            byte_length: 1,
            modified_at_ms: None,
        },
        decision: ReviewDecision::Draft,
        annotations: vec![],
        created_at: "now".into(),
        updated_at: "now".into(),
    }
}

#[test]
fn rejects_duplicate_annotation_ids() {
    let mut review = session();
    let annotation = ReviewAnnotation {
        annotation_id: "a".into(),
        group_id: None,
        annotation_type: AnnotationType::Note,
        status: AnnotationStatus::Open,
        comment: "note".into(),
        anchor: Anchor {
            block_id: None,
            selected_text: "x".into(),
            prefix: "".into(),
            suffix: "".into(),
            heading_path: vec![],
            start_offset: None,
            end_offset: None,
        },
        attachment_state: AttachmentState::Attached,
        created_at: "now".into(),
        updated_at: "now".into(),
    };
    review.annotations = vec![annotation.clone(), annotation];
    assert!(review.validate().is_err());
}

#[test]
fn approval_with_open_change_request_requires_confirmation() {
    let mut review = session();
    review.annotations.push(ReviewAnnotation {
        annotation_id: "a".into(),
        group_id: None,
        annotation_type: AnnotationType::ChangeRequest,
        status: AnnotationStatus::Open,
        comment: "fix".into(),
        anchor: Anchor {
            block_id: None,
            selected_text: "x".into(),
            prefix: "".into(),
            suffix: "".into(),
            heading_path: vec![],
            start_offset: None,
            end_offset: None,
        },
        attachment_state: AttachmentState::Attached,
        created_at: "now".into(),
        updated_at: "now".into(),
    });
    assert!(
        review
            .set_decision(ReviewDecision::Approved, false)
            .is_err()
    );
    review.set_decision(ReviewDecision::Approved, true).unwrap();
    assert_eq!(review.decision, ReviewDecision::Approved);
}
