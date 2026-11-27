use crate::domain::review_session::{ReviewAnnotation, ReviewSession};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportDocument<'a> {
    relative_path: &'a str,
    fingerprint: &'a str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportAnnotation<'a> {
    id: &'a str,
    group_id: &'a Option<String>,
    #[serde(rename = "type")]
    kind: crate::domain::review_session::AnnotationType,
    status: crate::domain::review_session::AnnotationStatus,
    comment: &'a str,
    anchor: ExportAnchor<'a>,
    attachment_state: crate::domain::review_session::AttachmentState,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportAnchor<'a> {
    block_id: &'a Option<String>,
    selected_text: &'a str,
    prefix: &'a str,
    suffix: &'a str,
    heading_path: &'a [String],
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Export<'a> {
    schema_version: u8,
    exported_at: &'a str,
    document: ExportDocument<'a>,
    decision: crate::domain::review_session::ReviewDecision,
    annotations: Vec<ExportAnnotation<'a>>,
}

fn export_annotation(annotation: &ReviewAnnotation) -> ExportAnnotation<'_> {
    ExportAnnotation {
        id: &annotation.annotation_id,
        group_id: &annotation.group_id,
        kind: annotation.annotation_type,
        status: annotation.status,
        comment: &annotation.comment,
        anchor: ExportAnchor {
            block_id: &annotation.anchor.block_id,
            selected_text: &annotation.anchor.selected_text,
            prefix: &annotation.anchor.prefix,
            suffix: &annotation.anchor.suffix,
            heading_path: &annotation.anchor.heading_path,
        },
        attachment_state: annotation.attachment_state,
    }
}
pub fn export_json(
    session: &ReviewSession,
    exported_at: &str,
    include_resolved: bool,
    selected_ids: &[String],
) -> Result<String, String> {
    let mut annotations = session
        .annotations
        .iter()
        .filter(|item| {
            (include_resolved
                || matches!(
                    item.status,
                    crate::domain::review_session::AnnotationStatus::Open
                ))
                && (selected_ids.is_empty() || selected_ids.contains(&item.annotation_id))
        })
        .collect::<Vec<_>>();
    annotations.sort_by(|a, b| a.annotation_id.cmp(&b.annotation_id));
    serde_json::to_string_pretty(&Export {
        schema_version: 1,
        exported_at,
        document: ExportDocument {
            relative_path: &session.document.relative_path,
            fingerprint: &session.document.fingerprint,
        },
        decision: session.decision,
        annotations: annotations.into_iter().map(export_annotation).collect(),
    })
    .map_err(|error| error.to_string())
}
pub fn export_markdown(
    session: &ReviewSession,
    include_resolved: bool,
    selected_ids: &[String],
) -> String {
    let mut output = format!(
        "# 검토 피드백: {}\n\n결정: {:?}\n",
        session.document.relative_path, session.decision
    );
    for item in session.annotations.iter().filter(|item| {
        (include_resolved
            || matches!(
                item.status,
                crate::domain::review_session::AnnotationStatus::Open
            ))
            && (selected_ids.is_empty() || selected_ids.contains(&item.annotation_id))
    }) {
        output.push_str(&format!(
            "\n## {:?}\n\n> {}\n\n{}\n",
            item.annotation_type, item.anchor.selected_text, item.comment
        ));
    }
    output
}

#[cfg(test)]
#[path = "feedback_export_test.rs"]
mod tests;
