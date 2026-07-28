use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::domain::{
    session_window_state::SessionWindowState,
    session_window_state_repository::SessionWindowStateRepository,
};
use crate::infrastructure::json_store::{load_json_vec, save_json_vec};

const STORE_LABEL: &str = "session window states";

pub struct JsonSessionWindowStateRepository {
    store_path: PathBuf,
}

impl JsonSessionWindowStateRepository {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
        fs::create_dir_all(&dir)
            .map_err(|error| format!("Failed to create app data directory: {error}"))?;
        Ok(Self {
            store_path: dir.join("session-window-states.json"),
        })
    }
}

impl SessionWindowStateRepository for JsonSessionWindowStateRepository {
    fn load_states(&self) -> Result<Vec<SessionWindowState>, String> {
        load_json_vec(&self.store_path, STORE_LABEL)
    }

    fn save_states(&self, states: &[SessionWindowState]) -> Result<(), String> {
        save_json_vec(&self.store_path, STORE_LABEL, states)
    }
}
