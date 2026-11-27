use tauri::Manager;

use crate::{
    application::{
        feedback_export_service::{export_json, export_markdown},
        review_session_service::ReviewSessionService,
    },
    domain::{document_identity::DocumentIdentity, review_session::ReviewSession},
    infrastructure::json_review_session_repository::JsonReviewSessionRepository,
    ports::clock::SystemClock,
};
use serde::Serialize;

fn service(
    app: &tauri::AppHandle,
) -> Result<ReviewSessionService<JsonReviewSessionRepository, SystemClock>, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(ReviewSessionService::new(
        JsonReviewSessionRepository::new(base),
        SystemClock,
    ))
}

#[tauri::command]
pub fn load_review_session(
    app: tauri::AppHandle,
    root_id: String,
    relative_path: String,
    fingerprint: String,
    byte_length: u64,
    modified_at_ms: Option<u64>,
) -> Result<ReviewSession, String> {
    service(&app)?
        .load_or_create(DocumentIdentity {
            root_id,
            relative_path,
            fingerprint,
            byte_length,
            modified_at_ms,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_review_session(
    app: tauri::AppHandle,
    session: ReviewSession,
    expected_revision: u64,
) -> Result<ReviewSession, String> {
    service(&app)?
        .save(session, expected_revision)
        .map_err(|error| error.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackExport {
    json: String,
    markdown: String,
}
#[tauri::command]
pub fn export_review_feedback(
    session: ReviewSession,
    exported_at: String,
    include_resolved: bool,
    selected_ids: Vec<String>,
) -> Result<FeedbackExport, String> {
    Ok(FeedbackExport {
        json: export_json(&session, &exported_at, include_resolved, &selected_ids)?,
        markdown: export_markdown(&session, include_resolved, &selected_ids),
    })
}
#[tauri::command]
pub fn save_feedback_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content).map_err(|error| error.to_string())
}
