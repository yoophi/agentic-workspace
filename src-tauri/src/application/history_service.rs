use crate::{
    application::ports::{GitHistoryReader, RepositoryStore},
    domain::commit::{GitCommitDetail, GitCommitSummary, GitFileDiff},
};

pub struct HistoryService<S, R>
where
    S: RepositoryStore,
    R: GitHistoryReader,
{
    store: S,
    reader: R,
}

impl<S, R> HistoryService<S, R>
where
    S: RepositoryStore,
    R: GitHistoryReader,
{
    pub fn new(store: S, reader: R) -> Self {
        Self { store, reader }
    }

    pub fn list_history(&self, repository_id: String) -> Result<Vec<GitCommitSummary>, String> {
        let repository_path = self.registered_repository_path(repository_id)?;

        self.reader.list_history(&repository_path)
    }

    pub fn get_commit_detail(
        &self,
        repository_id: String,
        commit_hash: String,
    ) -> Result<GitCommitDetail, String> {
        let requested_hash = commit_hash.trim();

        if requested_hash.is_empty() {
            return Err("Commit hash is required.".to_string());
        }

        let repository_path = self.registered_repository_path(repository_id)?;

        self.reader
            .get_commit_detail(&repository_path, requested_hash)
    }

    pub fn get_file_diff(
        &self,
        repository_id: String,
        commit_hash: String,
        file_path: String,
    ) -> Result<GitFileDiff, String> {
        let requested_hash = commit_hash.trim();
        let requested_path = file_path.trim();

        if requested_hash.is_empty() {
            return Err("Commit hash is required.".to_string());
        }

        if requested_path.is_empty() {
            return Err("File path is required.".to_string());
        }

        let repository_path = self.registered_repository_path(repository_id)?;

        self.reader
            .get_file_diff(&repository_path, requested_hash, requested_path)
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

    use crate::{
        application::ports::{GitHistoryReader, RepositoryStore},
        domain::{
            commit::{GitCommitDetail, GitCommitFileChange, GitCommitSummary, GitFileDiff},
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

    struct StaticHistoryReader {
        commits: Vec<GitCommitSummary>,
    }

    impl GitHistoryReader for StaticHistoryReader {
        fn list_history(&self, _repository_path: &str) -> Result<Vec<GitCommitSummary>, String> {
            Ok(self.commits.clone())
        }

        fn get_commit_detail(
            &self,
            _repository_path: &str,
            commit_hash: &str,
        ) -> Result<GitCommitDetail, String> {
            Ok(GitCommitDetail::new(
                commit_hash.to_string(),
                "Initial commit".to_string(),
                "A Developer".to_string(),
                "2026-06-25T00:00:00+09:00".to_string(),
                vec![GitCommitFileChange::new(
                    "README.md".to_string(),
                    "A".to_string(),
                )],
            ))
        }

        fn get_file_diff(
            &self,
            _repository_path: &str,
            commit_hash: &str,
            file_path: &str,
        ) -> Result<GitFileDiff, String> {
            Ok(GitFileDiff::new(
                commit_hash.to_string(),
                file_path.to_string(),
                "@@ -1 +1 @@\n-old\n+new\n".to_string(),
                false,
                false,
            ))
        }
    }

    #[test]
    fn rejects_unregistered_repository() {
        let service = HistoryService::new(
            MemoryStore::default(),
            StaticHistoryReader {
                commits: Vec::new(),
            },
        );

        let result = service.list_history("/tmp/repo".to_string());

        assert_eq!(result.unwrap_err(), "Repository is not registered.");
    }

    #[test]
    fn returns_history_for_registered_repository() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let commits = vec![GitCommitSummary::new(
            "abc123".to_string(),
            "Initial commit".to_string(),
            "A Developer".to_string(),
            "2026-06-25T00:00:00+09:00".to_string(),
        )];
        let service = HistoryService::new(
            store,
            StaticHistoryReader {
                commits: commits.clone(),
            },
        );

        assert_eq!(
            service
                .list_history("/tmp/repo".to_string())
                .expect("history should be returned"),
            commits
        );
    }

    #[test]
    fn rejects_empty_commit_hash() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let service = HistoryService::new(
            store,
            StaticHistoryReader {
                commits: Vec::new(),
            },
        );

        let result = service.get_commit_detail("/tmp/repo".to_string(), "  ".to_string());

        assert_eq!(result.unwrap_err(), "Commit hash is required.");
    }

    #[test]
    fn returns_commit_detail_for_registered_repository() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let service = HistoryService::new(
            store,
            StaticHistoryReader {
                commits: Vec::new(),
            },
        );

        let detail = service
            .get_commit_detail("/tmp/repo".to_string(), "abc123".to_string())
            .expect("commit detail should be returned");

        assert_eq!(detail.hash, "abc123");
        assert_eq!(detail.files.len(), 1);
    }

    #[test]
    fn rejects_empty_file_path() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let service = HistoryService::new(
            store,
            StaticHistoryReader {
                commits: Vec::new(),
            },
        );

        let result = service.get_file_diff(
            "/tmp/repo".to_string(),
            "abc123".to_string(),
            " ".to_string(),
        );

        assert_eq!(result.unwrap_err(), "File path is required.");
    }

    #[test]
    fn returns_file_diff_for_registered_repository() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let service = HistoryService::new(
            store,
            StaticHistoryReader {
                commits: Vec::new(),
            },
        );

        let diff = service
            .get_file_diff(
                "/tmp/repo".to_string(),
                "abc123".to_string(),
                "README.md".to_string(),
            )
            .expect("file diff should be returned");

        assert_eq!(diff.commit_hash, "abc123");
        assert_eq!(diff.path, "README.md");
        assert!(!diff.is_binary);
    }
}
