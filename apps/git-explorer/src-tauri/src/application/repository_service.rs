use crate::{
    application::ports::{GitRepositoryValidator, RepositoryStore},
    domain::repository::Repository,
};

pub struct RepositoryService<S, V>
where
    S: RepositoryStore,
    V: GitRepositoryValidator,
{
    store: S,
    validator: V,
}

impl<S, V> RepositoryService<S, V>
where
    S: RepositoryStore,
    V: GitRepositoryValidator,
{
    pub fn new(store: S, validator: V) -> Self {
        Self { store, validator }
    }

    pub fn list_repositories(&self) -> Result<Vec<Repository>, String> {
        self.store.list()
    }

    pub fn create_repository(&self, path: String) -> Result<Repository, String> {
        let requested_path = path.trim();

        if requested_path.is_empty() {
            return Err("Repository path is required.".to_string());
        }

        let git_root = self.validator.validate_repository(requested_path)?;
        let repository = Repository::new(
            git_root.clone(),
            repository_name_from_root(&git_root),
            requested_path.to_string(),
        );
        let mut repositories = self.store.list()?;

        if repositories.iter().any(|item| item.id == repository.id) {
            return Err("Repository is already registered.".to_string());
        }

        repositories.push(repository.clone());
        self.store.save_all(&repositories)?;

        Ok(repository)
    }

    pub fn rename_repository(
        &self,
        repository_id: String,
        name: String,
    ) -> Result<Repository, String> {
        let requested_id = repository_id.trim();
        let requested_name = name.trim();

        if requested_id.is_empty() {
            return Err("Repository id is required.".to_string());
        }

        if requested_name.is_empty() {
            return Err("Repository name is required.".to_string());
        }

        let mut repositories = self.store.list()?;
        let repository = repositories
            .iter_mut()
            .find(|repository| repository.id == requested_id)
            .ok_or_else(|| "Repository is not registered.".to_string())?;

        repository.name = requested_name.to_string();
        let renamed_repository = repository.clone();
        self.store.save_all(&repositories)?;

        Ok(renamed_repository)
    }

    pub fn delete_repository(&self, repository_id: String) -> Result<(), String> {
        let requested_id = repository_id.trim();

        if requested_id.is_empty() {
            return Err("Repository id is required.".to_string());
        }

        let mut repositories = self.store.list()?;
        let original_len = repositories.len();

        repositories.retain(|repository| repository.id != requested_id);

        if repositories.len() == original_len {
            return Err("Repository is not registered.".to_string());
        }

        self.store.save_all(&repositories)
    }
}

fn repository_name_from_root(git_root: &str) -> String {
    git_root
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(git_root)
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::application::ports::{GitRepositoryValidator, RepositoryStore};

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

    struct StaticValidator {
        root: String,
    }

    impl GitRepositoryValidator for StaticValidator {
        fn validate_repository(&self, _path: &str) -> Result<String, String> {
            Ok(self.root.clone())
        }
    }

    #[test]
    fn rejects_empty_repository_path() {
        let service = RepositoryService::new(
            MemoryStore::default(),
            StaticValidator {
                root: "/tmp/repo".to_string(),
            },
        );

        let result = service.create_repository("  ".to_string());

        assert_eq!(result.unwrap_err(), "Repository path is required.");
    }

    #[test]
    fn saves_valid_repository() {
        let store = MemoryStore::default();
        let service = RepositoryService::new(
            store.clone(),
            StaticValidator {
                root: "/tmp/repo".to_string(),
            },
        );

        let repository = service
            .create_repository("/tmp/repo".to_string())
            .expect("repository should be saved");

        assert_eq!(repository.id, "/tmp/repo");
        assert_eq!(repository.name, "repo");
        assert_eq!(store.list().expect("stored repositories").len(), 1);
    }

    #[test]
    fn rejects_duplicate_repository() {
        let store = MemoryStore::default();
        let service = RepositoryService::new(
            store,
            StaticValidator {
                root: "/tmp/repo".to_string(),
            },
        );

        service
            .create_repository("/tmp/repo".to_string())
            .expect("first registration should succeed");
        let result = service.create_repository("/tmp/repo".to_string());

        assert_eq!(result.unwrap_err(), "Repository is already registered.");
    }

    #[test]
    fn renames_registered_repository() {
        let store = MemoryStore::default();
        let service = RepositoryService::new(
            store,
            StaticValidator {
                root: "/tmp/repo".to_string(),
            },
        );
        service
            .create_repository("/tmp/repo".to_string())
            .expect("repository should be saved");

        let repository = service
            .rename_repository("/tmp/repo".to_string(), "New name".to_string())
            .expect("repository should be renamed");

        assert_eq!(repository.name, "New name");
        assert_eq!(
            service
                .list_repositories()
                .expect("repositories should load")[0]
                .name,
            "New name"
        );
    }

    #[test]
    fn rejects_empty_repository_name() {
        let service = RepositoryService::new(
            MemoryStore::default(),
            StaticValidator {
                root: "/tmp/repo".to_string(),
            },
        );

        let result = service.rename_repository("/tmp/repo".to_string(), "  ".to_string());

        assert_eq!(result.unwrap_err(), "Repository name is required.");
    }

    #[test]
    fn deletes_registered_repository() {
        let store = MemoryStore::default();
        let service = RepositoryService::new(
            store,
            StaticValidator {
                root: "/tmp/repo".to_string(),
            },
        );
        service
            .create_repository("/tmp/repo".to_string())
            .expect("repository should be saved");

        service
            .delete_repository("/tmp/repo".to_string())
            .expect("repository should be deleted");

        assert!(service
            .list_repositories()
            .expect("repositories should load")
            .is_empty());
    }

    #[test]
    fn derives_repository_name_from_windows_style_root() {
        assert_eq!(
            repository_name_from_root(r"C:\Users\yoophi\project\repo"),
            "repo"
        );
    }
}
