use crate::domain::review_session::{AttachmentState, ReviewAnnotation};

pub fn reconcile_annotation(annotation: &mut ReviewAnnotation, markdown: &str) {
    let selected = annotation.anchor.selected_text.as_str();
    let matches = markdown.match_indices(selected).collect::<Vec<_>>();
    annotation.attachment_state = if selected.is_empty() {
        AttachmentState::Orphan
    } else if matches.len() == 1 {
        AttachmentState::Attached
    } else if matches.is_empty() {
        AttachmentState::Missing
    } else {
        AttachmentState::Conflict
    };
}

pub fn fingerprint_relink_candidate(
    old_fingerprint: &str,
    candidates: &[(String, String)],
) -> Option<String> {
    let matching = candidates
        .iter()
        .filter(|(_, fingerprint)| fingerprint == old_fingerprint)
        .collect::<Vec<_>>();
    (matching.len() == 1).then(|| matching[0].0.clone())
}

#[cfg(test)]
#[path = "review_reconciliation_test.rs"]
mod tests;
