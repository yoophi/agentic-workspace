use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RootChangedEvent {
    pub root_id: String,
    pub revision: u64,
    pub paths: Vec<String>,
    pub rescan_hint: bool,
}
pub struct RootWatchHandle {
    _watcher: RecommendedWatcher,
}

pub fn watch_root(
    root_id: String,
    path: &Path,
    notify: impl Fn(RootChangedEvent) + Send + Sync + 'static,
) -> Result<RootWatchHandle, String> {
    let root = path.canonicalize().map_err(|error| error.to_string())?;
    let revision = Arc::new(AtomicU64::new(0));
    let callback = Arc::new(notify);
    let (sender, receiver) = std::sync::mpsc::channel::<Vec<PathBuf>>();
    let root_for_worker = root.clone();
    let callback_for_worker = callback.clone();
    let root_id_for_worker = root_id.clone();
    std::thread::spawn(move || {
        while let Ok(first) = receiver.recv() {
            let mut pending = first;
            while let Ok(next) = receiver.recv_timeout(Duration::from_millis(180)) {
                pending.extend(next);
            }
            let paths = pending
                .into_iter()
                .filter_map(|path| {
                    path.strip_prefix(&root_for_worker)
                        .ok()
                        .map(|path| path.to_string_lossy().replace('\\', "/"))
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            callback_for_worker(RootChangedEvent {
                root_id: root_id_for_worker.clone(),
                revision: revision.fetch_add(1, Ordering::SeqCst) + 1,
                rescan_hint: paths.len() != 1,
                paths,
            });
        }
    });
    let mut watcher = RecommendedWatcher::new(
        {
            let sender = sender.clone();
            move |result: notify::Result<notify::Event>| {
                if let Ok(event) = result {
                    let _ = sender.send(event.paths);
                }
            }
        },
        Config::default(),
    )
    .map_err(|error| error.to_string())?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| error.to_string())?;
    Ok(RootWatchHandle { _watcher: watcher })
}

#[cfg(test)]
#[path = "fs_root_watcher_test.rs"]
mod tests;
