use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::{
    project::{Project, ProjectDraft},
    project_repository::ProjectRepository,
};

pub fn list_projects(repository: &impl ProjectRepository) -> Result<Vec<Project>, String> {
    repository.load_projects()
}

pub fn create_project(
    repository: &impl ProjectRepository,
    draft: ProjectDraft,
) -> Result<Project, String> {
    let draft = normalize_draft(draft)?;
    let mut projects = repository.load_projects()?;
    let project = Project {
        id: new_project_id()?,
        name: draft.name,
        working_directory: draft.working_directory,
        description: draft.description,
    };

    projects.push(project.clone());
    repository.save_projects(&projects)?;

    Ok(project)
}

pub fn update_project(
    repository: &impl ProjectRepository,
    id: String,
    draft: ProjectDraft,
) -> Result<Project, String> {
    let draft = normalize_draft(draft)?;
    let mut projects = repository.load_projects()?;
    let project = projects
        .iter_mut()
        .find(|project| project.id == id)
        .ok_or_else(|| "Project not found.".to_owned())?;

    project.name = draft.name;
    project.working_directory = draft.working_directory;
    project.description = draft.description;

    let updated_project = project.clone();
    repository.save_projects(&projects)?;

    Ok(updated_project)
}

pub fn delete_project(repository: &impl ProjectRepository, id: String) -> Result<(), String> {
    let mut projects = repository.load_projects()?;
    let original_len = projects.len();

    projects.retain(|project| project.id != id);

    if projects.len() == original_len {
        return Err("Project not found.".to_owned());
    }

    repository.save_projects(&projects)
}

fn normalize_draft(draft: ProjectDraft) -> Result<ProjectDraft, String> {
    let name = draft.name.trim().to_owned();
    let working_directory = draft.working_directory.trim().to_owned();
    let description = draft.description.and_then(|value| {
        let trimmed = value.trim().to_owned();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    if name.is_empty() {
        return Err("Project name is required.".to_owned());
    }

    if working_directory.is_empty() {
        return Err("Working directory is required.".to_owned());
    }

    Ok(ProjectDraft {
        name,
        working_directory,
        description,
    })
}

fn new_project_id() -> Result<String, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Failed to generate project id: {error}"))?
        .as_nanos();

    Ok(format!("project-{nanos}"))
}
