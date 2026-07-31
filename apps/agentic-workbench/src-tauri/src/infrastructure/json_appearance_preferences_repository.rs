use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use tauri::AppHandle;

use crate::{
    domain::appearance_preferences::AppearancePreferences,
    infrastructure::json_store::{app_data_store_path, load_json, save_json},
    ports::appearance_preferences_repository::AppearancePreferencesRepository,
};

const STORE_LABEL: &str = "appearance preferences";
const STORE_FILE: &str = "appearance-preferences.json";

pub struct JsonAppearancePreferencesRepository {
    store_path: PathBuf,
}

impl JsonAppearancePreferencesRepository {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        Ok(Self::new(app_data_store_path(app, STORE_FILE)?))
    }

    pub fn new(store_path: PathBuf) -> Self {
        Self { store_path }
    }

    /// The store is canonical when its bytes already round-trip through the domain
    /// type unchanged. A missing file, an out-of-range step, or an unknown field all
    /// fail this check, so every window reads the same normalized document.
    fn is_canonical(&self, preferences: &AppearancePreferences) -> bool {
        fs::read_to_string(&self.store_path)
            .ok()
            .and_then(|contents| serde_json::from_str::<Value>(&contents).ok())
            .zip(serde_json::to_value(preferences).ok())
            .is_some_and(|(stored, canonical)| stored == canonical)
    }

    fn preserve_corrupt_current(&self) {
        if !self.store_path.exists() {
            return;
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        let corrupt_path = self
            .store_path
            .with_file_name(format!("appearance-preferences.corrupt-{timestamp}.json"));
        let _ = fs::copy(&self.store_path, corrupt_path);
    }
}

impl AppearancePreferencesRepository for JsonAppearancePreferencesRepository {
    fn load_preferences(&self) -> Result<AppearancePreferences, String> {
        match load_json(&self.store_path, STORE_LABEL) {
            Ok(preferences) => {
                if !self.is_canonical(&preferences) {
                    save_json(&self.store_path, STORE_LABEL, &preferences)?;
                }
                Ok(preferences)
            }
            Err(_) => {
                self.preserve_corrupt_current();
                let preferences = AppearancePreferences::default();
                save_json(&self.store_path, STORE_LABEL, &preferences)?;
                Ok(preferences)
            }
        }
    }

    fn save_preferences(&self, preferences: &AppearancePreferences) -> Result<(), String> {
        save_json(&self.store_path, STORE_LABEL, preferences)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_default_roundtrips_and_keeps_atomic_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STORE_FILE);
        let repository = JsonAppearancePreferencesRepository::new(path.clone());

        assert_eq!(
            repository.load_preferences().unwrap(),
            AppearancePreferences::default()
        );
        repository
            .save_preferences(&AppearancePreferences::with_font_size_step(1))
            .unwrap();
        repository
            .save_preferences(&AppearancePreferences::with_font_size_step(2))
            .unwrap();

        assert_eq!(
            repository
                .load_preferences()
                .unwrap()
                .font_size_step
                .value(),
            2
        );
        assert!(path.with_extension("json.bak").exists());
    }

    #[test]
    fn rewrites_out_of_range_values_to_canonical_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STORE_FILE);
        fs::write(&path, r#"{"fontSizeStep":42}"#).unwrap();
        let repository = JsonAppearancePreferencesRepository::new(path.clone());

        assert_eq!(
            repository.load_preferences().unwrap(),
            AppearancePreferences::default()
        );
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("\"fontSizeStep\": 0")
        );
    }

    #[test]
    fn recovers_from_valid_backup_before_falling_back() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STORE_FILE);
        let repository = JsonAppearancePreferencesRepository::new(path.clone());
        repository
            .save_preferences(&AppearancePreferences::with_font_size_step(1))
            .unwrap();
        repository
            .save_preferences(&AppearancePreferences::with_font_size_step(2))
            .unwrap();
        fs::write(&path, "{ broken").unwrap();

        assert_eq!(
            repository
                .load_preferences()
                .unwrap()
                .font_size_step
                .value(),
            1
        );
    }

    #[test]
    fn preserves_corrupt_current_and_writes_default_when_backup_also_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(STORE_FILE);
        fs::write(&path, "{ current broken").unwrap();
        fs::write(path.with_extension("json.bak"), "{ backup broken").unwrap();
        let repository = JsonAppearancePreferencesRepository::new(path.clone());

        assert_eq!(
            repository.load_preferences().unwrap(),
            AppearancePreferences::default()
        );
        let preserved = fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"));
        assert!(preserved);
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("\"fontSizeStep\": 0")
        );
    }
}
