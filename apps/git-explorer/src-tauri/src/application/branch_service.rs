use crate::{
    application::ports::{GitBranchReader, RepositoryStore},
    domain::branch::GitBranch,
};

pub struct BranchService<S, R>
where
    S: RepositoryStore,
    R: GitBranchReader,
{
    store: S,
    reader: R,
}

impl<S, R> BranchService<S, R>
where
    S: RepositoryStore,
    R: GitBranchReader,
{
    pub fn new(store: S, reader: R) -> Self {
        Self { store, reader }
    }

    pub fn list_branches(&self, repository_id: String) -> Result<Vec<GitBranch>, String> {
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

        self.reader.list_branches(&repository.path)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        application::ports::{GitBranchReader, RepositoryStore},
        domain::{branch::GitBranch, repository::Repository},
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

    struct StaticBranchReader {
        branches: Vec<GitBranch>,
    }

    impl GitBranchReader for StaticBranchReader {
        fn list_branches(&self, _repository_path: &str) -> Result<Vec<GitBranch>, String> {
            Ok(self.branches.clone())
        }
    }

    #[test]
    fn rejects_unregistered_repository() {
        let service = BranchService::new(
            MemoryStore::default(),
            StaticBranchReader {
                branches: Vec::new(),
            },
        );

        let result = service.list_branches("/tmp/repo".to_string());

        assert_eq!(result.unwrap_err(), "Repository is not registered.");
    }

    #[test]
    fn returns_branches_for_registered_repository() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let branches = vec![GitBranch::new(
            "main".to_string(),
            "refs/heads/main".to_string(),
            false,
            true,
            Some("/tmp/repo".to_string()),
        )];
        let service = BranchService::new(
            store,
            StaticBranchReader {
                branches: branches.clone(),
            },
        );

        assert_eq!(
            service
                .list_branches("/tmp/repo".to_string())
                .expect("branches should be returned"),
            branches
        );
    }
}
