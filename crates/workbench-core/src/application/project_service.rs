//! 프로젝트 use case. AW `application/project_service.rs`에서 이동했다(037).
//! 동작은 그대로이고, 오류가 `String`에서 `ProjectError`로 바뀌었으며 id 생성을 분리했다.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    domain::{
        project::{Project, ProjectDraft},
        project_error::ProjectError,
    },
    ports::project_repository::ProjectRepository,
};

pub fn list_projects(repository: &dyn ProjectRepository) -> Result<Vec<Project>, ProjectError> {
    repository.load_projects()
}

/// 기존 동작: id를 새로 만들어 저장한다.
pub fn create_project(
    repository: &dyn ProjectRepository,
    draft: ProjectDraft,
) -> Result<Project, ProjectError> {
    create_project_with_id(repository, new_project_id()?, draft)
}

/// intent-first 경로용: 호출자가 ledger에 예약해 둔 id로 저장한다. draft는 여기서 다시 정규화한다.
pub fn create_project_with_id(
    repository: &dyn ProjectRepository,
    id: String,
    draft: ProjectDraft,
) -> Result<Project, ProjectError> {
    let draft = normalize_draft(draft)?;
    let mut projects = repository.load_projects()?;
    let project = Project {
        id,
        name: draft.name,
        working_directory: draft.working_directory,
        description: draft.description,
    };

    projects.push(project.clone());
    repository.save_projects(&projects)?;

    Ok(project)
}

pub fn update_project(
    repository: &dyn ProjectRepository,
    id: String,
    draft: ProjectDraft,
) -> Result<Project, ProjectError> {
    let draft = normalize_draft(draft)?;
    let mut projects = repository.load_projects()?;
    let project = projects
        .iter_mut()
        .find(|project| project.id == id)
        .ok_or(ProjectError::NotFound)?;

    project.name = draft.name;
    project.working_directory = draft.working_directory;
    project.description = draft.description;

    let updated_project = project.clone();
    repository.save_projects(&projects)?;

    Ok(updated_project)
}

pub fn delete_project(repository: &dyn ProjectRepository, id: String) -> Result<(), ProjectError> {
    let mut projects = repository.load_projects()?;
    let original_len = projects.len();

    projects.retain(|project| project.id != id);

    if projects.len() == original_len {
        return Err(ProjectError::NotFound);
    }

    repository.save_projects(&projects)
}

/// trim 후 빈 값 검증. 지문 계산과 저장이 같은 정규화 결과를 쓴다.
pub fn normalize_draft(draft: ProjectDraft) -> Result<ProjectDraft, ProjectError> {
    let name = draft.name.trim().to_owned();
    let working_directory = draft.working_directory.trim().to_owned();
    let description = draft.description.and_then(|value| {
        let trimmed = value.trim().to_owned();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    if name.is_empty() {
        return Err(ProjectError::NameRequired);
    }

    if working_directory.is_empty() {
        return Err(ProjectError::WorkingDirectoryRequired);
    }

    Ok(ProjectDraft {
        name,
        working_directory,
        description,
    })
}

/// 기존 형식 `project-{unix_nanos}`를 유지한다. 저장 파일 호환을 위해 바꾸지 않는다.
pub fn new_project_id() -> Result<String, ProjectError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ProjectError::Clock(format!("Failed to generate project id: {error}")))?
        .as_nanos();

    Ok(format!("project-{nanos}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct MemoryRepository {
        projects: Mutex<Vec<Project>>,
    }

    impl ProjectRepository for MemoryRepository {
        fn load_projects(&self) -> Result<Vec<Project>, ProjectError> {
            Ok(self.projects.lock().unwrap().clone())
        }

        fn save_projects(&self, projects: &[Project]) -> Result<(), ProjectError> {
            *self.projects.lock().unwrap() = projects.to_vec();
            Ok(())
        }

        fn recover_from_backup(&self) -> Result<(), ProjectError> {
            Ok(())
        }
    }

    fn draft(name: &str, dir: &str, description: Option<&str>) -> ProjectDraft {
        ProjectDraft {
            name: name.into(),
            working_directory: dir.into(),
            description: description.map(str::to_owned),
        }
    }

    #[test]
    fn normalize_trims_and_drops_empty_description() {
        let normalized = normalize_draft(draft("  AW ", " /tmp/aw ", Some("   "))).unwrap();
        assert_eq!(normalized.name, "AW");
        assert_eq!(normalized.working_directory, "/tmp/aw");
        assert_eq!(normalized.description, None);
    }

    #[test]
    fn normalize_reports_legacy_error_strings() {
        assert_eq!(
            normalize_draft(draft("  ", "/tmp", None))
                .unwrap_err()
                .to_string(),
            "Project name is required."
        );
        assert_eq!(
            normalize_draft(draft("AW", "", None))
                .unwrap_err()
                .to_string(),
            "Working directory is required."
        );
    }

    #[test]
    fn create_with_id_persists_and_returns_project() {
        let repository = MemoryRepository::default();
        let project = create_project_with_id(
            &repository,
            "project-1".into(),
            draft("AW", "/tmp/aw", None),
        )
        .unwrap();
        assert_eq!(project.id, "project-1");
        assert_eq!(list_projects(&repository).unwrap(), vec![project]);
    }

    #[test]
    fn update_and_delete_report_not_found() {
        let repository = MemoryRepository::default();
        assert_eq!(
            update_project(&repository, "missing".into(), draft("x", "/y", None))
                .unwrap_err()
                .to_string(),
            "Project not found."
        );
        assert_eq!(
            delete_project(&repository, "missing".into()).unwrap_err(),
            ProjectError::NotFound
        );
    }

    #[test]
    fn new_project_id_keeps_legacy_prefix() {
        let id = new_project_id().unwrap();
        assert!(id.starts_with("project-"));
        assert!(id["project-".len()..].chars().all(|c| c.is_ascii_digit()));
    }
}
