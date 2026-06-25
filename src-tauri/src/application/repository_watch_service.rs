use crate::{
    application::ports::{RepositoryStore, RepositoryWatcher},
    domain::repository_event::RepositoryChangeEvent,
};

pub struct RepositoryWatchService<S, W>
where
    S: RepositoryStore,
    W: RepositoryWatcher,
{
    store: S,
    watcher: W,
}

impl<S, W> RepositoryWatchService<S, W>
where
    S: RepositoryStore,
    W: RepositoryWatcher,
{
    pub fn new(store: S, watcher: W) -> Self {
        Self { store, watcher }
    }

    pub fn watch_registered_repositories(
        &self,
        notify: impl Fn(RepositoryChangeEvent) + Send + Sync + 'static,
    ) -> Result<W::WatchHandle, String> {
        let repositories = self.store.list()?;

        self.watcher
            .watch_repositories(&repositories, Box::new(notify))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    use crate::{
        application::ports::{RepositoryStore, RepositoryWatcher},
        domain::{repository::Repository, repository_event::RepositoryChangeEvent},
    };

    use super::RepositoryWatchService;

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

    #[derive(Clone, Default)]
    struct StaticWatcher {
        watched_ids: Arc<Mutex<Vec<String>>>,
    }

    impl RepositoryWatcher for StaticWatcher {
        type WatchHandle = Vec<String>;

        fn watch_repositories(
            &self,
            repositories: &[Repository],
            notify: Box<dyn Fn(RepositoryChangeEvent) + Send + Sync + 'static>,
        ) -> Result<Self::WatchHandle, String> {
            let ids = repositories
                .iter()
                .map(|repository| repository.id.clone())
                .collect::<Vec<_>>();

            if let Some(repository) = repositories.first() {
                notify(RepositoryChangeEvent::new(
                    repository.id.clone(),
                    repository.path.clone(),
                ));
            }

            *self.watched_ids.lock().expect("watcher state should lock") = ids.clone();
            Ok(ids)
        }
    }

    #[test]
    fn starts_watchers_for_registered_repositories() {
        let store = MemoryStore::default();
        store
            .save_all(&[Repository::new(
                "/tmp/repo".to_string(),
                "repo".to_string(),
                "/tmp/repo".to_string(),
            )])
            .expect("repository should be stored");
        let watcher = StaticWatcher::default();
        let received = Arc::new(Mutex::new(Vec::new()));
        let received_for_notify = Arc::clone(&received);
        let service = RepositoryWatchService::new(store, watcher.clone());

        let handle = service
            .watch_registered_repositories(move |event| {
                received_for_notify
                    .lock()
                    .expect("received events should lock")
                    .push(event.repository_id);
            })
            .expect("watcher should start");

        assert_eq!(handle, vec!["/tmp/repo"]);
        assert_eq!(
            watcher
                .watched_ids
                .lock()
                .expect("watcher state should lock")
                .as_slice(),
            ["/tmp/repo"]
        );
        assert_eq!(
            received
                .lock()
                .expect("received events should lock")
                .as_slice(),
            ["/tmp/repo"]
        );
    }
}
