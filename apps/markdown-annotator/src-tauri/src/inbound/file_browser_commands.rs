use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::Serialize;
use tauri::{Emitter, Manager};

use crate::{
    application::{
        file_browser_service::FileBrowserService, launch_target_service::LaunchTargetService,
    },
    domain::file_browser::{FileBrowserError, LaunchTarget},
    infrastructure::fs_file_browser::FsFileBrowser,
    infrastructure::fs_root_watcher::{RootWatchHandle, watch_root},
};

pub struct RootWatcherState {
    handles: Mutex<HashMap<String, RootWatchHandle>>,
}
impl RootWatcherState {
    pub fn new() -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub fn start_root_watcher(
    app: tauri::AppHandle,
    state: tauri::State<'_, RootWatcherState>,
    root_id: String,
    root_path: String,
) -> Result<(), String> {
    let event_app = app.clone();
    let handle = watch_root(root_id.clone(), Path::new(&root_path), move |event| {
        let _ = event_app.emit("markdown-annotator://root-changed", event);
    })?;
    state
        .handles
        .lock()
        .map_err(|error| error.to_string())?
        .insert(root_id, handle);
    Ok(())
}

#[tauri::command]
pub fn stop_root_watcher(
    state: tauri::State<'_, RootWatcherState>,
    root_id: String,
) -> Result<(), String> {
    state
        .handles
        .lock()
        .map_err(|error| error.to_string())?
        .remove(&root_id);
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadRootDocumentResponse {
    pub identity: crate::domain::document_identity::DocumentIdentity,
    pub markdown_text: String,
}

pub struct RootScanState {
    cancelled: Mutex<HashSet<String>>,
}

impl RootScanState {
    pub fn new() -> Self {
        Self {
            cancelled: Mutex::new(HashSet::new()),
        }
    }
    fn is_cancelled(&self, scan_id: &str) -> bool {
        self.cancelled
            .lock()
            .map(|ids| ids.contains(scan_id))
            .unwrap_or(true)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanEntryDto {
    path: String,
    kind: crate::domain::file_browser::ScannedEntryKind,
    size: u64,
    modified_at: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RootScanBatchDto {
    scan_id: String,
    sequence: u64,
    entries: Vec<ScanEntryDto>,
    visited_entries: u64,
    matched_documents: usize,
    warnings: Vec<crate::domain::file_browser::FileBrowserWarning>,
    completed: bool,
}

#[tauri::command]
pub fn resolve_launch_target(
    path: Option<String>,
    cwd: String,
) -> Result<LaunchTarget, FileBrowserError> {
    LaunchTargetService::resolve(path.as_deref().map(Path::new), Path::new(&cwd))
}

#[tauri::command]
pub fn read_root_markdown_document(
    root_path: String,
    relative_path: String,
) -> Result<ReadRootDocumentResponse, FileBrowserError> {
    let service = FileBrowserService::new(FsFileBrowser);
    let result = service.read_document(&PathBuf::from(root_path), &relative_path)?;
    Ok(ReadRootDocumentResponse {
        identity: result.identity,
        markdown_text: result.markdown_text,
    })
}

#[tauri::command]
pub fn start_root_scan(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, RootScanState>,
    root_path: String,
    excluded_directory_names: Vec<String>,
    scan_id: String,
) -> Result<(), String> {
    state
        .cancelled
        .lock()
        .map_err(|error| error.to_string())?
        .remove(&scan_id);
    let app = window.app_handle().clone();
    let label = window.label().to_string();
    tauri::async_runtime::spawn(async move {
        let service = FileBrowserService::new(FsFileBrowser);
        let result = tauri::async_runtime::spawn_blocking(move || {
            service.scan_root(Path::new(&root_path), &excluded_directory_names)
        })
        .await;
        let Ok(Ok(result)) = result else {
            return;
        };
        let matched_documents = result
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.kind,
                    crate::domain::file_browser::ScannedEntryKind::File
                )
            })
            .count();
        let chunks = result.entries.chunks(100).collect::<Vec<_>>();
        let chunk_count = chunks.len();
        for (index, chunk) in chunks.into_iter().enumerate() {
            if app.state::<RootScanState>().is_cancelled(&scan_id) {
                return;
            }
            let entries = chunk
                .iter()
                .map(|entry| ScanEntryDto {
                    path: entry.relative_path.clone(),
                    kind: entry.kind,
                    size: entry.size,
                    modified_at: entry.modified_at_ms.map(|value| value.to_string()),
                })
                .collect();
            let completed = index + 1 == chunk_count;
            let batch = RootScanBatchDto {
                scan_id: scan_id.clone(),
                sequence: index as u64,
                entries,
                visited_entries: result.visited_entries,
                matched_documents,
                warnings: if completed {
                    result.warnings.clone()
                } else {
                    Vec::new()
                },
                completed,
            };
            let _ = app.emit_to(&label, "markdown-annotator://root-scan-batch", batch);
        }
        if chunk_count == 0 && !app.state::<RootScanState>().is_cancelled(&scan_id) {
            let _ = app.emit_to(
                &label,
                "markdown-annotator://root-scan-batch",
                RootScanBatchDto {
                    scan_id,
                    sequence: 0,
                    entries: Vec::new(),
                    visited_entries: result.visited_entries,
                    matched_documents: 0,
                    warnings: result.warnings,
                    completed: true,
                },
            );
        }
    });
    Ok(())
}

#[tauri::command]
pub fn cancel_root_scan(
    state: tauri::State<'_, RootScanState>,
    scan_id: String,
) -> Result<(), String> {
    state
        .cancelled
        .lock()
        .map_err(|error| error.to_string())?
        .insert(scan_id);
    Ok(())
}
