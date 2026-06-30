use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

const WORKTREE_EVENT_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeChangedEvent {
    pub working_directory: String,
    pub changed_path: String,
    pub kind: WorktreeChangeKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeChangeKind {
    File,
    Git,
}

pub struct WorktreeWatchHandle {
    _watcher: RecommendedWatcher,
}

pub fn watch_worktree(
    working_directory: String,
    notify: impl Fn(WorktreeChangedEvent) + Send + Sync + 'static,
) -> Result<WorktreeWatchHandle, String> {
    let worktree_root = PathBuf::from(&working_directory);
    if !worktree_root.exists() {
        return Err(format!(
            "Cannot watch missing worktree path: {}",
            worktree_root.display()
        ));
    }

    let watched_paths = watched_paths_for_worktree(&worktree_root);
    if watched_paths.is_empty() {
        return Err(format!(
            "No existing watch paths for worktree: {}",
            worktree_root.display()
        ));
    }

    let git_paths = git_metadata_paths(&worktree_root);
    let notify = Arc::new(notify);
    let last_event = Arc::new(Mutex::new(Instant::now() - WORKTREE_EVENT_DEBOUNCE));
    let worktree_for_event = working_directory.clone();
    let notify_for_watcher = Arc::clone(&notify);
    let last_event_for_watcher = Arc::clone(&last_event);

    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| match result {
            Ok(event) => {
                if should_ignore_event(&event) {
                    return;
                }
                if !should_emit_event(&last_event_for_watcher, WORKTREE_EVENT_DEBOUNCE) {
                    return;
                }

                let changed_path = event
                    .paths
                    .first()
                    .and_then(|path| path.to_str())
                    .unwrap_or(&worktree_for_event)
                    .to_string();
                let kind = if event_touches_git_metadata(&event.paths, &git_paths) {
                    WorktreeChangeKind::Git
                } else {
                    WorktreeChangeKind::File
                };

                notify_for_watcher(WorktreeChangedEvent {
                    working_directory: worktree_for_event.clone(),
                    changed_path,
                    kind,
                });
            }
            Err(error) => {
                eprintln!("Worktree watcher event failed: {error}");
            }
        },
        Config::default(),
    )
    .map_err(|error| format!("Failed to start worktree watcher: {error}"))?;

    for path in watched_paths {
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|error| {
                format!("Failed to watch worktree path {}: {error}", path.display())
            })?;
    }

    Ok(WorktreeWatchHandle { _watcher: watcher })
}

fn should_emit_event(last_event: &Arc<Mutex<Instant>>, debounce: Duration) -> bool {
    let now = Instant::now();
    let mut last_event = match last_event.lock() {
        Ok(last_event) => last_event,
        Err(error) => {
            eprintln!("Worktree watcher debounce state failed: {error}");
            return true;
        }
    };

    if now.duration_since(*last_event) < debounce {
        return false;
    }

    *last_event = now;
    true
}

fn should_ignore_event(event: &notify::Event) -> bool {
    event.paths.iter().all(|path| {
        path.components().any(|component| {
            let component = component.as_os_str();
            component == "node_modules" || component == "target"
        })
    })
}

fn watched_paths_for_worktree(worktree_root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![worktree_root.to_path_buf()];

    for git_path in git_metadata_paths(worktree_root) {
        if git_path.exists() && !paths.contains(&git_path) {
            paths.push(git_path);
        }
    }

    paths
}

fn event_touches_git_metadata(paths: &[PathBuf], git_paths: &[PathBuf]) -> bool {
    paths.iter().any(|path| {
        git_paths
            .iter()
            .any(|git_path| path == git_path || path.starts_with(git_path))
    })
}

fn git_metadata_paths(worktree_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let dot_git = worktree_root.join(".git");
    if dot_git.exists() {
        paths.push(dot_git);
    }

    for argument in ["--git-dir", "--git-common-dir"] {
        if let Some(path) = git_rev_parse_path(worktree_root, argument)
            && !paths.contains(&path)
        {
            paths.push(path);
        }
    }

    paths
}

fn git_rev_parse_path(worktree_root: &Path, argument: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree_root)
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
        worktree_root.join(path)
    })
}

#[cfg(test)]
mod tests {
    use notify::Event;

    use super::{event_touches_git_metadata, should_ignore_event};

    #[test]
    fn detects_git_metadata_paths() {
        assert!(event_touches_git_metadata(
            &["/repo/.git/HEAD".into()],
            &["/repo/.git".into()]
        ));
        assert!(!event_touches_git_metadata(
            &["/repo/src/main.ts".into()],
            &["/repo/.git".into()]
        ));
    }

    #[test]
    fn ignores_build_dependency_directories_only_when_all_paths_match() {
        let ignored = Event::new(notify::EventKind::Any).add_path("/repo/node_modules/pkg".into());
        let mixed = Event::new(notify::EventKind::Any)
            .add_path("/repo/node_modules/pkg".into())
            .add_path("/repo/src/main.ts".into());

        assert!(should_ignore_event(&ignored));
        assert!(!should_ignore_event(&mixed));
    }
}
