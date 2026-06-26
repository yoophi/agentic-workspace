use crate::{
    application::ports::{GitWorktreeReader, RepositoryStore},
    domain::worktree::GitWorktree,
};

pub struct WorktreeService<S, R>
where
    S: RepositoryStore,
    R: GitWorktreeReader,
{
    store: S,
    reader: R,
}

impl<S, R> WorktreeService<S, R>
where
    S: RepositoryStore,
    R: GitWorktreeReader,
{
    pub fn new(store: S, reader: R) -> Self {
        Self { store, reader }
    }

    pub fn list_worktrees(&self, repository_id: String) -> Result<Vec<GitWorktree>, String> {
        let requested_id = repository_id.trim();

        if requested_id.is_empty() {
            return Err("Repository id is required.".to_string());
        }

        let repository = self
            .store
            .list()?
            .into_iter()
            .find(|repository| repository.id == requested_id)
            .ok_or_else(|| "Repository is not registered.".to_string())?;

        self.reader.list_worktrees(&repository.path)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        application::ports::{GitWorktreeReader, RepositoryStore},
        domain::{repository::Repository, worktree::GitWorktree},
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

    struct StaticWorktreeReader {
        worktrees: Vec<GitWorktree>,
    }

    impl GitWorktreeReader for StaticWorktreeReader {
        fn list_worktrees(&self, _repository_path: &str) -> Result<Vec<GitWorktree>, String> {
            Ok(self.worktrees.clone())
        }
    }

    #[test]
    fn rejects_empty_repository_id() {
        let service = WorktreeService::new(
            MemoryStore::default(),
            StaticWorktreeReader {
                worktrees: Vec::new(),
            },
        );

        let result = service.list_worktrees("  ".to_string());

        assert_eq!(result.unwrap_err(), "Repository id is required.");
    }

    #[test]
    fn rejects_unregistered_repository() {
        let service = WorktreeService::new(
            MemoryStore::default(),
            StaticWorktreeReader {
                worktrees: Vec::new(),
            },
        );

        let result = service.list_worktrees("/tmp/repo".to_string());

        assert_eq!(result.unwrap_err(), "Repository is not registered.");
    }

    #[test]
    fn returns_worktrees_for_registered_repository() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let worktrees = vec![GitWorktree::new(
            "/tmp/repo".to_string(),
            Some("main".to_string()),
            "abc123".to_string(),
            false,
            true,
        )];
        let service = WorktreeService::new(
            store,
            StaticWorktreeReader {
                worktrees: worktrees.clone(),
            },
        );

        assert_eq!(
            service
                .list_worktrees("/tmp/repo".to_string())
                .expect("worktrees should be returned"),
            worktrees
        );
    }
}
