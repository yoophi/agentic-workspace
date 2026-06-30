use git_core::GitWorktreeStatusReader;

use crate::{
    application::ports::RepositoryStore,
    domain::commit::{GitWorktreeChanges, GitWorktreeFileDiff},
};

/// 등록된 repository의 미커밋(working-tree) status/diff를 조회하는 facade.
/// repositoryId를 path로 변환한 뒤 git-core `GitWorktreeStatusReader`에 위임한다.
pub struct WorktreeStatusService<S, R>
where
    S: RepositoryStore,
    R: GitWorktreeStatusReader,
{
    store: S,
    reader: R,
}

impl<S, R> WorktreeStatusService<S, R>
where
    S: RepositoryStore,
    R: GitWorktreeStatusReader,
{
    pub fn new(store: S, reader: R) -> Self {
        Self { store, reader }
    }

    pub fn get_worktree_status(&self, repository_id: String) -> Result<GitWorktreeChanges, String> {
        let repository_path = self.registered_repository_path(repository_id)?;
        self.reader.status(&repository_path)
    }

    pub fn get_worktree_file_diff(
        &self,
        repository_id: String,
        file_path: String,
    ) -> Result<GitWorktreeFileDiff, String> {
        let requested_path = file_path.trim();

        if requested_path.is_empty() {
            return Err("File path is required.".to_string());
        }

        let repository_path = self.registered_repository_path(repository_id)?;

        self.reader.diff(&repository_path, requested_path)
    }

    fn registered_repository_path(&self, repository_id: String) -> Result<String, String> {
        let requested_id = repository_id.trim();

        if requested_id.is_empty() {
            return Err("Repository id is required.".to_string());
        }

        self.store
            .list()?
            .into_iter()
            .find(|repository| repository.id == requested_id)
            .map(|repository| repository.path)
            .ok_or_else(|| "Repository is not registered.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use git_core::GitWorktreeStatusReader;

    use crate::{
        application::ports::RepositoryStore,
        domain::{
            commit::{GitWorktreeChanges, GitWorktreeFileDiff},
            repository::Repository,
        },
    };

    use super::*;

    #[derive(Clone, Default)]
    struct MemoryStore {
        repositories: Rc<RefCell<Vec<Repository>>>,
    }

    impl RepositoryStore for MemoryStore {
        fn list(&self) -> Result<Vec<Repository>, String> {
            Ok(self.repositories.borrow().clone())
        }

        fn save_all(&self, repositories: &[Repository]) -> Result<(), String> {
            *self.repositories.borrow_mut() = repositories.to_vec();
            Ok(())
        }
    }

    struct StaticWorktreeReader;

    impl GitWorktreeStatusReader for StaticWorktreeReader {
        fn status(&self, _repository_path: &str) -> Result<GitWorktreeChanges, String> {
            Ok(GitWorktreeChanges {
                working_directory: "/tmp/repo".to_string(),
                files: Vec::new(),
                staged_count: 0,
                unstaged_count: 0,
                untracked_count: 0,
                conflicted_count: 0,
            })
        }

        fn diff(
            &self,
            _repository_path: &str,
            file_path: &str,
        ) -> Result<GitWorktreeFileDiff, String> {
            Ok(GitWorktreeFileDiff {
                path: file_path.to_string(),
                content: "@@ -1 +1 @@\n-old\n+new\n".to_string(),
                is_binary: false,
                is_truncated: false,
            })
        }
    }

    fn store_with_repo() -> MemoryStore {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        store
    }

    #[test]
    fn rejects_unregistered_repository_for_status() {
        let service = WorktreeStatusService::new(MemoryStore::default(), StaticWorktreeReader);

        assert_eq!(
            service
                .get_worktree_status("/tmp/repo".to_string())
                .unwrap_err(),
            "Repository is not registered."
        );
    }

    #[test]
    fn returns_status_for_registered_repository() {
        let service = WorktreeStatusService::new(store_with_repo(), StaticWorktreeReader);

        let changes = service
            .get_worktree_status("/tmp/repo".to_string())
            .expect("status should be returned");

        assert_eq!(changes.working_directory, "/tmp/repo");
    }

    #[test]
    fn rejects_empty_file_path_for_diff() {
        let service = WorktreeStatusService::new(store_with_repo(), StaticWorktreeReader);

        assert_eq!(
            service
                .get_worktree_file_diff("/tmp/repo".to_string(), " ".to_string())
                .unwrap_err(),
            "File path is required."
        );
    }

    #[test]
    fn returns_diff_for_registered_repository() {
        let service = WorktreeStatusService::new(store_with_repo(), StaticWorktreeReader);

        let diff = service
            .get_worktree_file_diff("/tmp/repo".to_string(), "README.md".to_string())
            .expect("diff should be returned");

        assert_eq!(diff.path, "README.md");
        assert!(!diff.is_binary);
    }
}
