use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

const DOCUMENT_EVENT_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDocumentChangedEvent {
    pub path: String,
}

pub struct DocumentWatchHandle {
    _watcher: RecommendedWatcher,
}

pub fn watch_document(
    path: String,
    notify: impl Fn(MarkdownDocumentChangedEvent) + Send + Sync + 'static,
) -> Result<DocumentWatchHandle, String> {
    let document_path = PathBuf::from(&path);
    let parent = document_path
        .parent()
        .ok_or_else(|| format!("Cannot watch document without parent directory: {path}"))?;

    if !parent.exists() {
        return Err(format!(
            "Cannot watch missing document directory: {}",
            parent.display()
        ));
    }

    let notify = Arc::new(notify);
    let last_event = Arc::new(Mutex::new(Instant::now() - DOCUMENT_EVENT_DEBOUNCE));
    let path_for_event = path.clone();
    let document_path_for_watcher = document_path.clone();
    let notify_for_watcher = Arc::clone(&notify);
    let last_event_for_watcher = Arc::clone(&last_event);

    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| match result {
            Ok(event) => {
                if !event_targets_document(&event.paths, &document_path_for_watcher) {
                    return;
                }
                if !should_emit_event(&last_event_for_watcher, DOCUMENT_EVENT_DEBOUNCE) {
                    return;
                }

                notify_for_watcher(MarkdownDocumentChangedEvent {
                    path: path_for_event.clone(),
                });
            }
            Err(error) => {
                eprintln!("Markdown document watcher event failed: {error}");
            }
        },
        Config::default(),
    )
    .map_err(|error| format!("Failed to start markdown document watcher: {error}"))?;

    watcher
        .watch(parent, RecursiveMode::NonRecursive)
        .map_err(|error| {
            format!(
                "Failed to watch document directory {}: {error}",
                parent.display()
            )
        })?;

    Ok(DocumentWatchHandle { _watcher: watcher })
}

fn event_targets_document(event_paths: &[PathBuf], document_path: &Path) -> bool {
    let document_file_name = document_path.file_name();

    event_paths.iter().any(|event_path| {
        event_path == document_path
            || (document_file_name.is_some() && event_path.file_name() == document_file_name)
    })
}

fn should_emit_event(last_event: &Arc<Mutex<Instant>>, debounce: Duration) -> bool {
    let now = Instant::now();
    let mut last_event = match last_event.lock() {
        Ok(last_event) => last_event,
        Err(error) => {
            eprintln!("Markdown document watcher debounce state failed: {error}");
            return true;
        }
    };

    if now.duration_since(*last_event) < debounce {
        return false;
    }

    *last_event = now;
    true
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::event_targets_document;

    #[test]
    fn detects_exact_document_path() {
        assert!(event_targets_document(
            &[PathBuf::from("/notes/doc.md")],
            &PathBuf::from("/notes/doc.md"),
        ));
    }

    #[test]
    fn detects_atomic_save_target_by_file_name() {
        assert!(event_targets_document(
            &[
                PathBuf::from("/notes/.doc.md.swp"),
                PathBuf::from("/notes/doc.md")
            ],
            &PathBuf::from("/notes/doc.md"),
        ));
    }

    #[test]
    fn ignores_unrelated_file_names() {
        assert!(!event_targets_document(
            &[PathBuf::from("/notes/other.md")],
            &PathBuf::from("/notes/doc.md"),
        ));
    }
}
