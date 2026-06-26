use crate::{
    application::ports::{GitHistoryReader, RepositoryStore},
    domain::commit::{GitCommitDetail, GitCommitGraph, GitCommitHistory, GitFileDiff},
};

const DEFAULT_HISTORY_LIMIT: usize = 100;
const MAX_HISTORY_LIMIT: usize = 500;
const DEFAULT_GRAPH_LIMIT: usize = 300;
const MAX_GRAPH_LIMIT: usize = 500;

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

    pub fn list_history(
        &self,
        repository_id: String,
        max_count: Option<usize>,
        offset: Option<usize>,
        included_refs: Vec<String>,
        excluded_refs: Vec<String>,
    ) -> Result<GitCommitHistory, String> {
        let repository_path = self.registered_repository_path(repository_id)?;
        let limit = max_count
            .unwrap_or(DEFAULT_HISTORY_LIMIT)
            .clamp(1, MAX_HISTORY_LIMIT);
        let offset = offset.unwrap_or(0);
        let included_refs = normalize_branch_refs(included_refs)?;
        let excluded_refs = normalize_branch_refs(excluded_refs)?;

        self.reader.list_history(
            &repository_path,
            limit,
            offset,
            &included_refs,
            &excluded_refs,
        )
    }

    pub fn get_commit_graph(
        &self,
        repository_id: String,
        max_count: Option<usize>,
        offset: Option<usize>,
        included_refs: Vec<String>,
        excluded_refs: Vec<String>,
    ) -> Result<GitCommitGraph, String> {
        let repository_path = self.registered_repository_path(repository_id)?;
        let limit = max_count
            .unwrap_or(DEFAULT_GRAPH_LIMIT)
            .clamp(1, MAX_GRAPH_LIMIT);
        let offset = offset.unwrap_or(0);
        let included_refs = normalize_branch_refs(included_refs)?;
        let excluded_refs = normalize_branch_refs(excluded_refs)?;

        self.reader.get_commit_graph(
            &repository_path,
            limit,
            offset,
            &included_refs,
            &excluded_refs,
        )
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

fn normalize_branch_refs(refs: Vec<String>) -> Result<Vec<String>, String> {
    let mut normalized_refs = Vec::new();

    for git_ref in refs {
        let trimmed_ref = git_ref.trim();

        if trimmed_ref.is_empty() {
            continue;
        }

        if !trimmed_ref.starts_with("refs/heads/") && !trimmed_ref.starts_with("refs/remotes/") {
            return Err(format!("Unsupported branch ref: {trimmed_ref}"));
        }

        if normalized_refs
            .iter()
            .any(|existing_ref| existing_ref == trimmed_ref)
        {
            continue;
        }

        normalized_refs.push(trimmed_ref.to_string());
    }

    Ok(normalized_refs)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        application::ports::{GitHistoryReader, RepositoryStore},
        domain::{
            commit::{
                GitCommitDetail, GitCommitFileChange, GitCommitHistory, GitCommitPage,
                GitCommitSummary, GitFileDiff,
            },
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
        received_included_refs: Rc<RefCell<Vec<String>>>,
        received_excluded_refs: Rc<RefCell<Vec<String>>>,
    }

    fn static_reader(commits: Vec<GitCommitSummary>) -> StaticHistoryReader {
        StaticHistoryReader {
            commits,
            received_included_refs: Rc::new(RefCell::new(Vec::new())),
            received_excluded_refs: Rc::new(RefCell::new(Vec::new())),
        }
    }

    impl GitHistoryReader for StaticHistoryReader {
        fn list_history(
            &self,
            _repository_path: &str,
            limit: usize,
            offset: usize,
            included_refs: &[String],
            excluded_refs: &[String],
        ) -> Result<GitCommitHistory, String> {
            *self.received_included_refs.borrow_mut() = included_refs.to_vec();
            *self.received_excluded_refs.borrow_mut() = excluded_refs.to_vec();
            let commits = self
                .commits
                .iter()
                .skip(offset)
                .take(limit)
                .cloned()
                .collect::<Vec<_>>();

            Ok(GitCommitHistory::new(
                commits.clone(),
                GitCommitPage::new(offset, limit, self.commits.len(), commits.len()),
            ))
        }

        fn get_commit_graph(
            &self,
            _repository_path: &str,
            limit: usize,
            offset: usize,
            included_refs: &[String],
            excluded_refs: &[String],
        ) -> Result<GitCommitGraph, String> {
            *self.received_included_refs.borrow_mut() = included_refs.to_vec();
            *self.received_excluded_refs.borrow_mut() = excluded_refs.to_vec();
            Ok(GitCommitGraph::new(
                Vec::new(),
                Vec::new(),
                crate::domain::commit::GitGraphPage::new(offset, limit, 0, 0),
                crate::domain::commit::GitGraphLayoutHints::default_row_layout(),
            ))
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
        let service = HistoryService::new(MemoryStore::default(), static_reader(Vec::new()));

        let result =
            service.list_history("/tmp/repo".to_string(), None, None, Vec::new(), Vec::new());

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
        let service = HistoryService::new(store, static_reader(commits.clone()));

        assert_eq!(
            service
                .list_history(
                    "/tmp/repo".to_string(),
                    Some(100),
                    Some(0),
                    Vec::new(),
                    Vec::new()
                )
                .expect("history should be returned"),
            GitCommitHistory::new(
                commits.clone(),
                GitCommitPage::new(0, 100, commits.len(), commits.len()),
            )
        );
    }

    #[test]
    fn returns_paged_history_for_registered_repository() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let commits = vec![
            GitCommitSummary::new(
                "abc123".to_string(),
                "Initial commit".to_string(),
                "A Developer".to_string(),
                "2026-06-25T00:00:00+09:00".to_string(),
            ),
            GitCommitSummary::new(
                "def456".to_string(),
                "Add feature".to_string(),
                "B Developer".to_string(),
                "2026-06-25T01:00:00+09:00".to_string(),
            ),
        ];
        let service = HistoryService::new(store, static_reader(commits.clone()));

        let history = service
            .list_history(
                "/tmp/repo".to_string(),
                Some(1),
                Some(1),
                Vec::new(),
                Vec::new(),
            )
            .expect("history should be returned");

        assert_eq!(history.commits, vec![commits[1].clone()]);
        assert_eq!(history.page.limit, 1);
        assert_eq!(history.page.offset, 1);
        assert!(!history.page.has_more);
    }

    #[test]
    fn passes_normalized_branch_refs_to_history_reader() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let included_refs = Rc::new(RefCell::new(Vec::new()));
        let excluded_refs = Rc::new(RefCell::new(Vec::new()));
        let service = HistoryService::new(
            store,
            StaticHistoryReader {
                commits: Vec::new(),
                received_included_refs: included_refs.clone(),
                received_excluded_refs: excluded_refs.clone(),
            },
        );

        service
            .list_history(
                "/tmp/repo".to_string(),
                None,
                None,
                vec![
                    " refs/heads/main ".to_string(),
                    "refs/heads/main".to_string(),
                ],
                vec!["refs/remotes/origin/main".to_string()],
            )
            .expect("history should be returned");

        assert_eq!(*included_refs.borrow(), vec!["refs/heads/main"]);
        assert_eq!(*excluded_refs.borrow(), vec!["refs/remotes/origin/main"]);
    }

    #[test]
    fn rejects_unsupported_branch_refs() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let service = HistoryService::new(store, static_reader(Vec::new()));

        let result = service.list_history(
            "/tmp/repo".to_string(),
            None,
            None,
            vec!["--all".to_string()],
            Vec::new(),
        );

        assert_eq!(result.unwrap_err(), "Unsupported branch ref: --all");
    }

    #[test]
    fn returns_commit_graph_for_registered_repository() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let service = HistoryService::new(store, static_reader(Vec::new()));

        let graph = service
            .get_commit_graph(
                "/tmp/repo".to_string(),
                Some(1000),
                Some(2),
                Vec::new(),
                Vec::new(),
            )
            .expect("commit graph should be returned");

        assert_eq!(graph.page.limit, 500);
        assert_eq!(graph.page.offset, 2);
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
        let service = HistoryService::new(store, static_reader(Vec::new()));

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
        let service = HistoryService::new(store, static_reader(Vec::new()));

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
        let service = HistoryService::new(store, static_reader(Vec::new()));

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
        let service = HistoryService::new(store, static_reader(Vec::new()));

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
