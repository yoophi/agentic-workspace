use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    domain::{goal::ThreadGoal, goal_repository::GoalRepository},
    infrastructure::json_store::{load_json_vec, save_json_vec},
};

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
        load_json_vec(&self.store_path, "goals")
    }

    fn save_goals(&self, goals: &[ThreadGoal]) -> Result<(), String> {
        save_json_vec(&self.store_path, "goals", goals)
    }
}
