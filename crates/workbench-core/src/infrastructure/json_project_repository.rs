//! `projects.json` 어댑터. AW `infrastructure/json_project_repository.rs`에서 이동하며
//! `from_app(&AppHandle)`을 `new(&DataPaths)`로 바꿨다.

use std::path::PathBuf;

use crate::{
    domain::{project::Project, project_error::ProjectError},
    infrastructure::{
        data_paths::DataPaths,
        json_store::{self, StoreError},
    },
    ports::project_repository::ProjectRepository,
};

const LABEL: &str = "projects";

pub struct JsonProjectRepository {
    store_path: PathBuf,
}

impl JsonProjectRepository {
    pub fn new(paths: &DataPaths) -> Self {
        Self {
            store_path: paths.projects_file(),
        }
    }

    pub fn store_path(&self) -> &std::path::Path {
        &self.store_path
    }
}

fn map_store_error(error: StoreError) -> ProjectError {
    match error {
        StoreError::PrimaryCorrupt { .. } => ProjectError::StoreCorrupt(error.to_string()),
        other => ProjectError::Storage(other.to_string()),
    }
}

impl ProjectRepository for JsonProjectRepository {
    fn load_projects(&self) -> Result<Vec<Project>, ProjectError> {
        json_store::load_json_vec(&self.store_path, LABEL).map_err(map_store_error)
    }

    fn save_projects(&self, projects: &[Project]) -> Result<(), ProjectError> {
        json_store::save_json_vec(&self.store_path, LABEL, projects).map_err(map_store_error)
    }

    fn recover_from_backup(&self) -> Result<(), ProjectError> {
        json_store::recover_from_backup::<Vec<Project>>(&self.store_path, LABEL)
            .map(|_| ())
            .map_err(map_store_error)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn project(id: &str) -> Project {
        Project {
            id: id.into(),
            name: id.to_uppercase(),
            working_directory: format!("/tmp/{id}"),
            description: None,
        }
    }

    #[test]
    fn round_trips_projects_through_data_paths() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        let repository = JsonProjectRepository::new(&paths);
        assert!(repository.load_projects().unwrap().is_empty());
        repository.save_projects(&[project("a")]).unwrap();
        assert_eq!(repository.load_projects().unwrap(), vec![project("a")]);
        assert!(paths.projects_file().exists());
    }

    #[test]
    fn corrupt_file_reports_store_corrupt_and_recovers_from_backup() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        let repository = JsonProjectRepository::new(&paths);
        repository.save_projects(&[project("a")]).unwrap();
        repository
            .save_projects(&[project("a"), project("b")])
            .unwrap();
        fs::write(paths.projects_file(), "garbage").unwrap();

        let error = repository.load_projects().unwrap_err();
        assert!(matches!(error, ProjectError::StoreCorrupt(_)), "{error}");

        repository.recover_from_backup().unwrap();
        assert_eq!(repository.load_projects().unwrap(), vec![project("a")]);
    }
}
