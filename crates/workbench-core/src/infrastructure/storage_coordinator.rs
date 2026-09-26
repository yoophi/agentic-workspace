//! aggregate 단위 저장 조정. 읽기·쓰기·백업 복구가 **모두** 같은 lock을 거친다(research R14).
//! revision은 `aggregate_revision` 테이블의 값을 캐시한 것이고, `applied` commit 뒤 `bump()`로 맞춘다(R5).

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use crate::{
    domain::project_error::ProjectError,
    ports::{aggregate_lock::AggregateLock, project_repository::ProjectRepository},
};

pub const PROJECTS_AGGREGATE: &str = "projects";

pub struct StorageCoordinator {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    projects: Arc<dyn ProjectRepository>,
    revision: AtomicU64,
    #[cfg(feature = "test-hooks")]
    recovery_delay: Mutex<Option<std::time::Duration>>,
}

impl StorageCoordinator {
    pub fn new(projects: Arc<dyn ProjectRepository>, initial_revision: u64) -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
            projects,
            revision: AtomicU64::new(initial_revision),
            #[cfg(feature = "test-hooks")]
            recovery_delay: Mutex::new(None),
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::SeqCst)
    }

    pub fn set_revision(&self, revision: u64) {
        self.revision.store(revision, Ordering::SeqCst);
    }

    pub fn bump(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn projects_repository(&self) -> &Arc<dyn ProjectRepository> {
        &self.projects
    }

    /// 복구 직전에 잠깐 멈춰 인터리빙을 강제한다. 테스트 전용.
    #[cfg(feature = "test-hooks")]
    pub fn set_recovery_delay(&self, delay: Option<std::time::Duration>) {
        *self.recovery_delay.lock().unwrap() = delay;
    }

    fn lock_for(&self, aggregate: &str) -> Arc<Mutex<()>> {
        let mut locks = self
            .locks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        locks
            .entry(aggregate.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// 프로젝트 저장소를 lock 안에서 사용한다. 읽기든 쓰기든 같은 lock이다.
    /// `f`가 `StoreCorrupt`를 돌려주면 같은 lock 아래에서 백업 복구 후 한 번 다시 실행한다.
    pub fn with_projects<R>(
        &self,
        mut f: impl FnMut(&dyn ProjectRepository) -> Result<R, ProjectError>,
    ) -> Result<R, ProjectError> {
        self.run_locked(PROJECTS_AGGREGATE, || match f(self.projects.as_ref()) {
            Err(ProjectError::StoreCorrupt(_)) => {
                #[cfg(feature = "test-hooks")]
                if let Some(delay) = *self.recovery_delay.lock().unwrap() {
                    std::thread::sleep(delay);
                }
                self.projects.recover_from_backup()?;
                f(self.projects.as_ref())
            }
            other => other,
        })
    }
}

impl AggregateLock for StorageCoordinator {
    fn run_locked<R>(&self, aggregate: &str, f: impl FnOnce() -> R) -> R {
        let lock = self.lock_for(aggregate);
        let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        f()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, thread, time::Duration};

    use super::*;
    use crate::{
        application::project_service,
        domain::project::{Project, ProjectDraft},
        infrastructure::{data_paths::DataPaths, json_project_repository::JsonProjectRepository},
    };

    fn coordinator(dir: &tempfile::TempDir) -> (DataPaths, Arc<StorageCoordinator>) {
        let paths = DataPaths::new(dir.path());
        let repository = Arc::new(JsonProjectRepository::new(&paths));
        (paths, Arc::new(StorageCoordinator::new(repository, 0)))
    }

    fn project(id: &str) -> Project {
        Project {
            id: id.into(),
            name: id.into(),
            working_directory: "/tmp".into(),
            description: None,
        }
    }

    #[test]
    fn read_recovers_corrupt_primary_under_lock() {
        let dir = tempfile::tempdir().unwrap();
        let (paths, coordinator) = coordinator(&dir);
        coordinator
            .with_projects(|repo| repo.save_projects(&[project("a")]))
            .unwrap();
        coordinator
            .with_projects(|repo| repo.save_projects(&[project("a"), project("b")]))
            .unwrap();
        fs::write(paths.projects_file(), "corrupt").unwrap();

        let loaded = coordinator
            .with_projects(|repo| repo.load_projects())
            .unwrap();
        assert_eq!(loaded, vec![project("a")]);
    }

    #[test]
    fn concurrent_creates_are_serialized_by_the_aggregate_lock() {
        let dir = tempfile::tempdir().unwrap();
        let (_paths, coordinator) = coordinator(&dir);
        let handles: Vec<_> = (0..8)
            .map(|index| {
                let coordinator = Arc::clone(&coordinator);
                thread::spawn(move || {
                    coordinator
                        .with_projects(|repo| {
                            project_service::create_project_with_id(
                                repo,
                                format!("project-{index}"),
                                ProjectDraft {
                                    name: format!("p{index}"),
                                    working_directory: "/tmp".into(),
                                    description: None,
                                },
                            )
                        })
                        .unwrap();
                    coordinator.bump();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        let loaded = coordinator
            .with_projects(|repo| repo.load_projects())
            .unwrap();
        assert_eq!(loaded.len(), 8);
        assert_eq!(coordinator.revision(), 8);
    }

    #[test]
    fn revision_cache_bumps_monotonically() {
        let dir = tempfile::tempdir().unwrap();
        let (_paths, coordinator) = coordinator(&dir);
        coordinator.set_revision(5);
        assert_eq!(coordinator.bump(), 6);
        assert_eq!(coordinator.revision(), 6);
        thread::sleep(Duration::from_millis(1));
    }
}
