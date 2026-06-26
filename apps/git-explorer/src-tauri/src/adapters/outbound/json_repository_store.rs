use std::{fs, path::PathBuf};

use crate::{application::ports::RepositoryStore, domain::repository::Repository};

pub struct JsonRepositoryStore {
    path: PathBuf,
}

impl JsonRepositoryStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl RepositoryStore for JsonRepositoryStore {
    fn list(&self) -> Result<Vec<Repository>, String> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.path)
            .map_err(|error| format!("Failed to read repository store: {error}"))?;

        serde_json::from_str(&content)
            .map_err(|error| format!("Failed to parse repository store: {error}"))
    }

    fn save_all(&self, repositories: &[Repository]) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create app data directory: {error}"))?;
        }

        let content = serde_json::to_string_pretty(repositories)
            .map_err(|error| format!("Failed to serialize repository store: {error}"))?;

        fs::write(&self.path, content)
            .map_err(|error| format!("Failed to write repository store: {error}"))
    }
}
