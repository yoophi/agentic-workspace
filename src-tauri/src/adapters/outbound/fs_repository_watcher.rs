use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{
    application::ports::RepositoryWatcher,
    domain::{repository::Repository, repository_event::RepositoryChangeEvent},
};

const REPOSITORY_EVENT_DEBOUNCE: Duration = Duration::from_millis(500);

pub struct FsRepositoryWatcher;

pub struct RepositoryWatchHandle {
    _watchers: Vec<RecommendedWatcher>,
}

impl RepositoryWatcher for FsRepositoryWatcher {
    type WatchHandle = RepositoryWatchHandle;

    fn watch_repositories(
        &self,
        repositories: &[Repository],
        notify: Box<dyn Fn(RepositoryChangeEvent) + Send + Sync + 'static>,
    ) -> Result<Self::WatchHandle, String> {
        let notify = Arc::new(notify);
        let last_events = Arc::new(Mutex::new(HashMap::<String, Instant>::new()));
        let mut watchers = Vec::new();

        for repository in repositories {
            let watched_paths = watched_paths_for_repository(&repository.path);

            if watched_paths.is_empty() {
                eprintln!(
                    "No existing watch paths for repository: {}",
                    repository.path
                );
                continue;
            }

            let repository_id = repository.id.clone();
            let repository_path = repository.path.clone();
            let notify_for_watcher = Arc::clone(&notify);
            let last_events_for_watcher = Arc::clone(&last_events);
            let mut watcher = RecommendedWatcher::new(
                move |result: notify::Result<notify::Event>| match result {
                    Ok(event) => {
                        if should_ignore_event(&event) {
                            return;
                        }

                        let changed_path = event
                            .paths
                            .first()
                            .and_then(|path| path.to_str())
                            .unwrap_or(&repository_path)
                            .to_string();

                        if should_emit_repository_event(
                            &repository_id,
                            &last_events_for_watcher,
                            REPOSITORY_EVENT_DEBOUNCE,
                        ) {
                            notify_for_watcher(RepositoryChangeEvent::new(
                                repository_id.clone(),
                                changed_path,
                            ));
                        }
                    }
                    Err(error) => {
                        eprintln!("Repository watcher event failed: {error}");
                    }
                },
                Config::default(),
            )
            .map_err(|error| format!("Failed to start repository watcher: {error}"))?;

            for path in watched_paths {
                watcher
                    .watch(&path, RecursiveMode::Recursive)
                    .map_err(|error| {
                        format!(
                            "Failed to watch repository path {}: {error}",
                            path.display()
                        )
                    })?;
            }

            watchers.push(watcher);
        }

        Ok(RepositoryWatchHandle {
            _watchers: watchers,
        })
    }
}

fn should_emit_repository_event(
    repository_id: &str,
    last_events: &Arc<Mutex<HashMap<String, Instant>>>,
    debounce: Duration,
) -> bool {
    let now = Instant::now();
    let mut last_events = match last_events.lock() {
        Ok(last_events) => last_events,
        Err(error) => {
            eprintln!("Repository watcher debounce state failed: {error}");
            return true;
        }
    };

    if last_events
        .get(repository_id)
        .is_some_and(|last_event| now.duration_since(*last_event) < debounce)
    {
        return false;
    }

    last_events.insert(repository_id.to_string(), now);
    true
}

fn should_ignore_event(event: &notify::Event) -> bool {
    event.paths.iter().all(|path| {
        path.components()
            .any(|component| component.as_os_str() == "target")
    })
}

fn watched_paths_for_repository(repository_path: &str) -> Vec<PathBuf> {
    let mut paths = HashSet::new();
    let repository_root = PathBuf::from(repository_path);

    if repository_root.exists() {
        paths.insert(repository_root.clone());
    }

    for git_path in git_metadata_paths(&repository_root) {
        if git_path.exists() {
            paths.insert(git_path);
        }
    }

    paths.into_iter().collect()
}

fn git_metadata_paths(repository_root: &Path) -> Vec<PathBuf> {
    ["--git-dir", "--git-common-dir"]
        .iter()
        .filter_map(|argument| git_rev_parse_path(repository_root, argument))
        .collect()
}

fn git_rev_parse_path(repository_root: &Path, argument: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(["rev-parse", argument])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if value.is_empty() {
        return None;
    }

    let path = PathBuf::from(value);

    Some(if path.is_absolute() {
        path
    } else {
        repository_root.join(path)
    })
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use notify::Event;

    use super::{should_emit_repository_event, should_ignore_event};

    #[test]
    fn coalesces_events_within_debounce_window() {
        let last_events = Arc::new(Mutex::new(Default::default()));

        assert!(should_emit_repository_event(
            "repo",
            &last_events,
            Duration::from_secs(1)
        ));
        assert!(!should_emit_repository_event(
            "repo",
            &last_events,
            Duration::from_secs(1)
        ));
    }

    #[test]
    fn ignores_target_directory_events() {
        let event = Event::new(notify::EventKind::Any).add_path("/repo/target/debug/app".into());

        assert!(should_ignore_event(&event));
    }
}
