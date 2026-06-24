use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::domain::{goal::ThreadGoal, goal_repository::GoalRepository};

pub struct JsonGoalRepository {
    store_path: PathBuf,
}

impl JsonGoalRepository {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

        fs::create_dir_all(&dir)
            .map_err(|error| format!("Failed to create app data directory: {error}"))?;

        Ok(Self {
            store_path: dir.join("goals.json"),
        })
    }
}

impl GoalRepository for JsonGoalRepository {
    fn load_goals(&self) -> Result<Vec<ThreadGoal>, String> {
        if !self.store_path.exists() {
            return Ok(Vec::new());
        }

        let contents = fs::read_to_string(&self.store_path)
            .map_err(|error| format!("Failed to read goals store: {error}"))?;

        serde_json::from_str(&contents)
            .map_err(|error| format!("Failed to parse goals store: {error}"))
    }

    fn save_goals(&self, goals: &[ThreadGoal]) -> Result<(), String> {
        let contents = serde_json::to_string_pretty(goals)
            .map_err(|error| format!("Failed to serialize goals: {error}"))?;

        fs::write(&self.store_path, contents)
            .map_err(|error| format!("Failed to write goals store: {error}"))
    }
}
