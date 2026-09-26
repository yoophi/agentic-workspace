//! aggregate 단위 저장 조정. 읽기·쓰기·백업 복구가 **모두** 같은 lock을 거친다(research R14).
//! revision은 aggregate마다 `aggregate_revision` 테이블 값을 캐시한 것이고, `applied` commit 뒤 맞춘다(R5).
//!
//! 038(research R4): aggregate 이름으로 lock·revision·복구 재시도를 일반화했다. 저장 파일 4개는 고정 이름,
//! Git worktree 변경은 저장소마다 동적 이름(`git-worktrees:<canonical root>`)을 쓴다.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    domain::project_error::ProjectError,
    ports::{aggregate_lock::AggregateLock, project_repository::ProjectRepository},
};

pub const PROJECTS_AGGREGATE: &str = "projects";
pub const SAVED_PROMPTS_AGGREGATE: &str = "saved-prompts";
pub const GOALS_AGGREGATE: &str = "goals";
pub const AGENT_RUN_SETTINGS_AGGREGATE: &str = "agent-run-settings";

/// 저장 파일 4개의 aggregate. revision을 추적하고 `expectedRevision`을 지원한다.
pub const STORE_AGGREGATES: [&str; 4] = [
    PROJECTS_AGGREGATE,
    SAVED_PROMPTS_AGGREGATE,
    GOALS_AGGREGATE,
    AGENT_RUN_SETTINGS_AGGREGATE,
];

/// Git worktree 생성·삭제가 직렬화되는 단위. 같은 저장소 안에서만 서로 기다린다.
/// canonicalize에 실패하면(디렉터리 없음 등) 입력 그대로 쓴다 — 어차피 뒤의 Git 명령이 실패한다.
pub fn git_worktrees_aggregate(repo_root: &Path) -> String {
    let canonical = std::fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf());
    format!("git-worktrees:{}", canonical.display())
}

pub struct StorageCoordinator {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    revisions: Mutex<HashMap<String, u64>>,
    projects: Arc<dyn ProjectRepository>,
    #[cfg(feature = "test-hooks")]
    recovery_delay: Mutex<Option<std::time::Duration>>,
}

impl StorageCoordinator {
    pub fn new(projects: Arc<dyn ProjectRepository>, initial_projects_revision: u64) -> Self {
        let coordinator = Self {
            locks: Mutex::new(HashMap::new()),
            revisions: Mutex::new(HashMap::new()),
            projects,
            #[cfg(feature = "test-hooks")]
            recovery_delay: Mutex::new(None),
        };
        coordinator.set_revision_of(PROJECTS_AGGREGATE, initial_projects_revision);
        coordinator
    }

    /// `projects` aggregate의 revision. 037 호출자 호환.
    pub fn revision(&self) -> u64 {
        self.revision_of(PROJECTS_AGGREGATE)
    }

    pub fn revision_of(&self, aggregate: &str) -> u64 {
        self.revisions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(aggregate)
            .copied()
            .unwrap_or(0)
    }

    pub fn set_revision(&self, revision: u64) {
        self.set_revision_of(PROJECTS_AGGREGATE, revision);
    }

    pub fn set_revision_of(&self, aggregate: &str, revision: u64) {
        self.revisions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(aggregate.to_owned(), revision);
    }

    pub fn bump(&self) -> u64 {
        self.bump_of(PROJECTS_AGGREGATE)
    }

    pub fn bump_of(&self, aggregate: &str) -> u64 {
        let mut revisions = self
            .revisions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = revisions.entry(aggregate.to_owned()).or_insert(0);
        *entry += 1;
        *entry
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

    /// `aggregate`의 lock 안에서 `f`를 실행한다. 읽기든 쓰기든 같은 lock이다.
    /// `f`의 오류가 `is_corrupt`면 같은 lock 아래에서 `recover`를 부른 뒤 **한 번만** 다시 실행한다.
    pub fn with_aggregate<R, E>(
        &self,
        aggregate: &str,
        mut f: impl FnMut() -> Result<R, E>,
        is_corrupt: impl Fn(&E) -> bool,
        recover: impl Fn() -> Result<(), E>,
    ) -> Result<R, E> {
        self.run_locked(aggregate, || match f() {
            Err(error) if is_corrupt(&error) => {
                #[cfg(feature = "test-hooks")]
                if let Some(delay) = *self.recovery_delay.lock().unwrap() {
                    std::thread::sleep(delay);
                }
                recover()?;
                f()
            }
            other => other,
        })
    }

    /// 프로젝트 저장소를 lock 안에서 사용한다. `StoreCorrupt`면 백업 복구 후 한 번 다시 실행한다.
    pub fn with_projects<R>(
        &self,
        mut f: impl FnMut(&dyn ProjectRepository) -> Result<R, ProjectError>,
    ) -> Result<R, ProjectError> {
        self.with_aggregate(
            PROJECTS_AGGREGATE,
            || f(self.projects.as_ref()),
            |error| matches!(error, ProjectError::StoreCorrupt(_)),
            || self.projects.recover_from_backup(),
        )
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
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
        thread,
        time::Duration,
    };

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

    /// research R4: aggregate마다 lock과 revision이 독립이다.
    #[test]
    fn aggregates_have_independent_locks_and_revisions() {
        let dir = tempfile::tempdir().unwrap();
        let (_paths, coordinator) = coordinator(&dir);
        assert_eq!(coordinator.bump_of(GOALS_AGGREGATE), 1);
        assert_eq!(coordinator.bump_of(GOALS_AGGREGATE), 2);
        assert_eq!(coordinator.revision_of(SAVED_PROMPTS_AGGREGATE), 0);
        assert_eq!(coordinator.revision(), 0, "projects는 그대로");

        // goals lock을 잡은 채로 saved-prompts lock은 막히지 않는다.
        let entered = Arc::new(AtomicUsize::new(0));
        coordinator
            .with_aggregate::<(), ()>(
                GOALS_AGGREGATE,
                || {
                    let inner = Arc::clone(&coordinator);
                    let inner_entered = Arc::clone(&entered);
                    let handle = thread::spawn(move || {
                        inner
                            .with_aggregate::<(), ()>(
                                SAVED_PROMPTS_AGGREGATE,
                                || {
                                    inner_entered.fetch_add(1, Ordering::SeqCst);
                                    Ok(())
                                },
                                |_| false,
                                || Ok(()),
                            )
                            .unwrap();
                    });
                    handle.join().unwrap();
                    Ok(())
                },
                |_| false,
                || Ok(()),
            )
            .unwrap();
        assert_eq!(entered.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn with_aggregate_recovers_once_on_corrupt() {
        let dir = tempfile::tempdir().unwrap();
        let (_paths, coordinator) = coordinator(&dir);
        let attempts = AtomicUsize::new(0);
        let recovered = AtomicUsize::new(0);
        let result = coordinator.with_aggregate(
            GOALS_AGGREGATE,
            || {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    Err("corrupt")
                } else {
                    Ok(attempt)
                }
            },
            |error| *error == "corrupt",
            || {
                recovered.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        assert_eq!(result, Ok(1));
        assert_eq!(recovered.load(Ordering::SeqCst), 1);

        // 복구가 안 되는 오류는 그대로 전파된다.
        let result = coordinator.with_aggregate::<(), &str>(
            GOALS_AGGREGATE,
            || Err("io"),
            |error| *error == "corrupt",
            || Ok(()),
        );
        assert_eq!(result, Err("io"));
    }

    #[test]
    fn git_worktrees_aggregate_is_canonical() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("..").join("b");
        fs::create_dir_all(dir.path().join("a")).unwrap();
        fs::create_dir_all(dir.path().join("b")).unwrap();
        let direct = git_worktrees_aggregate(&dir.path().join("b"));
        assert_eq!(git_worktrees_aggregate(&nested), direct);
        assert!(direct.starts_with("git-worktrees:"));
        // 없는 경로는 입력 그대로.
        let missing = Path::new("/definitely/missing/repo");
        assert_eq!(
            git_worktrees_aggregate(missing),
            "git-worktrees:/definitely/missing/repo"
        );
    }
}
