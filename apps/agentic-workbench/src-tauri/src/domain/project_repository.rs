use crate::domain::project::Project;

pub trait ProjectRepository {
    fn load_projects(&self) -> Result<Vec<Project>, String>;
    fn save_projects(&self, projects: &[Project]) -> Result<(), String>;
}
