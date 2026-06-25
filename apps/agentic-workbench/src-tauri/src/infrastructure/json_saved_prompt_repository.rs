use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    domain::{saved_prompt::SavedPrompt, saved_prompt_repository::SavedPromptRepository},
    infrastructure::json_store::{load_json_vec, save_json_vec},
};

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
        load_json_vec(&self.store_path, "saved prompts")
    }

    fn save_saved_prompts(&self, prompts: &[SavedPrompt]) -> Result<(), String> {
        save_json_vec(&self.store_path, "saved prompts", prompts)
    }
}
