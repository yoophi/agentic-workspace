use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::domain::{saved_prompt::SavedPrompt, saved_prompt_repository::SavedPromptRepository};

pub struct JsonSavedPromptRepository {
    store_path: PathBuf,
}

impl JsonSavedPromptRepository {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

        fs::create_dir_all(&dir)
            .map_err(|error| format!("Failed to create app data directory: {error}"))?;

        Ok(Self {
            store_path: dir.join("saved-prompts.json"),
        })
    }
}

impl SavedPromptRepository for JsonSavedPromptRepository {
    fn load_saved_prompts(&self) -> Result<Vec<SavedPrompt>, String> {
        if !self.store_path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&self.store_path)
            .map_err(|error| format!("Failed to read saved prompts store: {error}"))?;

        serde_json::from_str(&contents)
            .map_err(|error| format!("Failed to parse saved prompts store: {error}"))
    }

    fn save_saved_prompts(&self, prompts: &[SavedPrompt]) -> Result<(), String> {
        let contents = serde_json::to_string_pretty(prompts)
            .map_err(|error| format!("Failed to serialize saved prompts: {error}"))?;

        fs::write(&self.store_path, contents)
            .map_err(|error| format!("Failed to write saved prompts store: {error}"))
    }
}
