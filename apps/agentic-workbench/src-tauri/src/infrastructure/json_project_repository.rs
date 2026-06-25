use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    domain::{project::Project, project_repository::ProjectRepository},
    infrastructure::json_store::{load_json_vec, save_json_vec},
};

pub struct JsonProjectRepository {
    store_path: PathBuf,
}

impl JsonProjectRepository {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;

        fs::create_dir_all(&dir)
            .map_err(|error| format!("Failed to create app data directory: {error}"))?;

        Ok(Self {
            store_path: dir.join("projects.json"),
        })
    }
}

impl ProjectRepository for JsonProjectRepository {
    fn load_projects(&self) -> Result<Vec<Project>, String> {
        load_json_vec(&self.store_path, "projects")
    }

    fn save_projects(&self, projects: &[Project]) -> Result<(), String> {
        save_json_vec(&self.store_path, "projects", projects)
    }
}
