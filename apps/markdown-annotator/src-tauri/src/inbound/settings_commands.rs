use crate::{
    application::preferences_service::PreferencesService,
    domain::global_preferences::GlobalPreferences,
    infrastructure::json_preferences_repository::JsonPreferencesRepository,
};
use tauri::{Emitter, Manager};
fn service(
    app: &tauri::AppHandle,
) -> Result<PreferencesService<JsonPreferencesRepository>, String> {
    Ok(PreferencesService::new(JsonPreferencesRepository::new(
        app.path().app_data_dir().map_err(|e| e.to_string())?,
    )))
}
#[tauri::command]
pub fn load_preferences(app: tauri::AppHandle) -> Result<GlobalPreferences, String> {
    service(&app)?.load()
}
#[tauri::command]
pub fn save_preferences(
    app: tauri::AppHandle,
    preferences: GlobalPreferences,
    expected_revision: u64,
) -> Result<GlobalPreferences, String> {
    let saved = service(&app)?.save(&preferences, expected_revision)?;
    app.emit("markdown-annotator://preferences-changed", &saved)
        .map_err(|e| e.to_string())?;
    Ok(saved)
}
#[tauri::command]
pub fn reset_preferences(app: tauri::AppHandle) -> Result<GlobalPreferences, String> {
    let saved = service(&app)?.reset()?;
    app.emit("markdown-annotator://preferences-changed", &saved)
        .map_err(|e| e.to_string())?;
    Ok(saved)
}
#[tauri::command]
pub fn trash_review_data(app: tauri::AppHandle, scope: String) -> Result<(), String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let source = match scope.as_str() {
        "recent" => base.join("recent.json"),
        "all" => base.join("reviews"),
        value if value.starts_with("session:") => base
            .join("reviews/sessions")
            .join(format!("{}.json", &value[8..])),
        _ => return Err("unsupported delete scope".into()),
    };
    if !source.exists() {
        return Ok(());
    }
    crate::application::data_management_service::DataManagementService::new(base)
        .trash_path(&source)
        .map(|_| ())
}
