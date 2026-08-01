use crate::{
    application::{document_service::DocumentService, launch_target_service::LaunchTargetService},
    domain::document::MarkdownDocument,
    infrastructure::{
        fs_document_reader::FsDocumentReader,
        fs_document_watcher::{DocumentWatchHandle, watch_document},
    },
};
use crate::{
    infrastructure::{cli_installer::CliInstaller, macos_native_shell::MacOsNativeShell},
    ports::native_shell::NativeShell,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const WINDOW_HIGHLIGHT_EVENT: &str = "markdown-annotator://window-highlight";
const MARKDOWN_DOCUMENT_CHANGED_EVENT: &str = "workspace://markdown-document-changed";

pub struct DocumentWatcherState {
    handles: Mutex<HashMap<String, DocumentWatchHandle>>,
}

impl DocumentWatcherState {
    pub fn new() -> Self {
        Self {
            handles: Mutex::new(HashMap::new()),
        }
    }

    pub fn stop_for_window(&self, window_label: &str) -> Result<(), String> {
        let mut handles = self
            .handles
            .lock()
            .map_err(|error| format!("failed to lock document watcher state: {error}"))?;
        handles.remove(window_label);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallStatus {
    installed: bool,
    path: String,
    target: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTarget {
    path: String,
    kind: String,
}
#[tauri::command]
pub fn load_recent_targets(app: tauri::AppHandle) -> Result<Vec<RecentTarget>, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("recent.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn remember_recent(app: &tauri::AppHandle, path: &Path) -> Result<(), String> {
    let file = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("recent.json");
    let mut values = if file.exists() {
        serde_json::from_slice::<Vec<RecentTarget>>(&fs::read(&file).map_err(|e| e.to_string())?)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let canonical = path
        .canonicalize()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    values.retain(|item| item.path != canonical);
    values.insert(
        0,
        RecentTarget {
            path: canonical,
            kind: if path.is_dir() {
                "folder".into()
            } else {
                "document".into()
            },
        },
    );
    values.truncate(12);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?
    }
    fs::write(
        file,
        serde_json::to_vec_pretty(&values).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn read_markdown_file(path: String) -> Result<MarkdownDocument, String> {
    let service = DocumentService::new(FsDocumentReader);
    service.read_markdown_file(&path)
}
#[tauri::command]
pub fn get_build_info() -> crate::domain::build_info::BuildInfo {
    crate::domain::build_info::build_info()
}
fn open_singleton_page(app: &tauri::AppHandle, label: &str, title: &str) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App(format!("index.html?page={label}").into()),
    )
    .title(title)
    .inner_size(720.0, 640.0)
    .build()
    .map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    open_singleton_page(&app, "settings", "Markdown Annotator 설정")
}
#[tauri::command]
pub fn open_about_window(app: tauri::AppHandle) -> Result<(), String> {
    open_singleton_page(&app, "about", "Markdown Annotator 정보")
}

#[tauri::command]
pub fn start_markdown_document_watcher(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, DocumentWatcherState>,
    path: String,
) -> Result<(), String> {
    let window_label = window.label().to_string();
    let target_label = window_label.clone();
    let event_app = app.clone();
    let handle = watch_document(path, move |event| {
        if let Err(error) = event_app.emit_to(
            target_label.as_str(),
            MARKDOWN_DOCUMENT_CHANGED_EVENT,
            event,
        ) {
            eprintln!("failed to emit markdown document change event: {error}");
        }
    })?;
    let mut handles = state
        .handles
        .lock()
        .map_err(|error| format!("failed to lock document watcher state: {error}"))?;
    handles.insert(window_label, handle);
    Ok(())
}

#[tauri::command]
pub fn stop_markdown_document_watcher(
    window: WebviewWindow,
    state: tauri::State<'_, DocumentWatcherState>,
) -> Result<(), String> {
    state.stop_for_window(window.label())
}

#[tauri::command]
pub fn install_cli() -> Result<CliInstallStatus, String> {
    let current_exe =
        env::current_exe().map_err(|error| format!("failed to locate app executable: {error}"))?;
    let cli_path = user_bin_dir()?.join("ma");
    CliInstaller::new(&cli_path).install(&current_exe)?;

    Ok(cli_install_status(true, &cli_path, &current_exe))
}

#[tauri::command]
pub fn check_cli_installed() -> Result<CliInstallStatus, String> {
    let current_exe =
        env::current_exe().map_err(|error| format!("failed to locate app executable: {error}"))?;
    let cli_path = user_bin_dir()?.join("ma");
    let installed = CliInstaller::new(&cli_path).status(&current_exe);

    Ok(cli_install_status(installed, &cli_path, &current_exe))
}
#[tauri::command]
pub fn remove_cli() -> Result<(), String> {
    CliInstaller::new(user_bin_dir()?.join("ma")).remove()
}

#[tauri::command]
pub fn reveal_document_in_finder(root_path: String, relative_path: String) -> Result<(), String> {
    let shell = MacOsNativeShell;
    let path = shell.validated_display_path(Path::new(&root_path), &relative_path)?;
    shell.reveal(Path::new(&path))
}
#[tauri::command]
pub fn open_document_in_default_app(
    root_path: String,
    relative_path: String,
) -> Result<(), String> {
    let shell = MacOsNativeShell;
    let path = shell.validated_display_path(Path::new(&root_path), &relative_path)?;
    shell.open_default(Path::new(&path))
}
#[tauri::command]
pub fn validated_document_path(root_path: String, relative_path: String) -> Result<String, String> {
    MacOsNativeShell.validated_display_path(Path::new(&root_path), &relative_path)
}
#[tauri::command]
pub fn open_external_https(url: String) -> Result<(), String> {
    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("only HTTP/HTTPS links are allowed".into());
    }
    let status = std::process::Command::new("/usr/bin/open")
        .arg(parsed.as_str())
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("open failed: {status}"))
    }
}

#[tauri::command]
pub fn request_open_document_window(app: tauri::AppHandle, path: String) -> Result<(), String> {
    open_document_window_path(&app, &path)
}

#[tauri::command]
pub fn request_open_document_tab(
    app: tauri::AppHandle,
    _window: WebviewWindow,
    path: String,
) -> Result<(), String> {
    open_document_window_path(&app, &path)
}

pub fn open_welcome_window(app: &tauri::AppHandle) {
    if app.get_webview_window("main").is_some() {
        return;
    }

    let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Markdown Annotator")
        .inner_size(1280.0, 860.0)
        .min_inner_size(980.0, 680.0);

    match builder.build() {
        Ok(_) => {}
        Err(error) => eprintln!("failed to create main window: {error}"),
    }
}

pub fn open_document_window_path(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    remember_recent(app, Path::new(path))?;
    let cwd = env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    let target = LaunchTargetService::resolve(Some(Path::new(path)), &cwd)
        .map_err(|error| error.to_string())?;
    let label = target.root.root_id.as_str();
    let selected_path = target
        .selected_document
        .as_deref()
        .map(|relative| target.root.canonical_path.join(relative));

    if focus_if_open(app, label, selected_path.as_deref()) {
        return Ok(());
    }

    create_root_window(
        app,
        label,
        &target.root.canonical_path,
        selected_path.as_deref(),
    )
    .map(|_| ())
}

pub fn open_document_from_cli_args(
    app: &tauri::AppHandle,
    argv: &[String],
    cwd: &Path,
) -> Result<bool, String> {
    let Some(path) = cli_path_arg(argv) else {
        return Ok(false);
    };

    let absolute_path = resolve_cli_path(path, cwd)?;
    open_document_window_path(app, &absolute_path.to_string_lossy())?;
    Ok(true)
}

pub fn focus_any_window(app: &tauri::AppHandle) {
    if let Some(window) = app.webview_windows().into_values().next() {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn initial_cli_args() -> Result<Option<(Vec<String>, PathBuf)>, String> {
    let argv = env::args().collect::<Vec<_>>();
    if cli_path_arg(&argv).is_none() {
        return Ok(None);
    }

    let cwd = env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    Ok(Some((argv, cwd)))
}

fn create_root_window(
    app: &tauri::AppHandle,
    label: &str,
    root_path: &Path,
    selected_path: Option<&Path>,
) -> Result<WebviewWindow, String> {
    let encoded_root =
        utf8_percent_encode(&root_path.to_string_lossy(), NON_ALPHANUMERIC).to_string();
    let encoded_path = selected_path
        .map(|path| utf8_percent_encode(&path.to_string_lossy(), NON_ALPHANUMERIC).to_string());
    let url = encoded_path.map_or_else(
        || format!("index.html?root={encoded_root}"),
        |path| format!("index.html?root={encoded_root}&path={path}"),
    );
    let title = root_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Markdown folder");

    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(1280.0, 860.0)
        .min_inner_size(980.0, 680.0);

    let window = builder
        .build()
        .map_err(|error| format!("failed to create root window: {error}"))?;
    window
        .show()
        .map_err(|error| format!("failed to show root window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("failed to focus root window: {error}"))?;
    Ok(window)
}

fn focus_if_open(app: &tauri::AppHandle, label: &str, selected_path: Option<&Path>) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    if let Some(path) = selected_path {
        let _ = window.emit(
            "markdown-annotator://root-document-selected",
            path.to_string_lossy().to_string(),
        );
    }
    let _ = window.emit(WINDOW_HIGHLIGHT_EVENT, ());
    true
}

fn cli_path_arg(argv: &[String]) -> Option<&str> {
    argv.get(1)
        .map(String::as_str)
        .filter(|path| !path.is_empty())
}

fn resolve_cli_path(raw_path: &str, cwd: &Path) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path);
    let candidate = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };

    Ok(candidate)
}

fn cli_install_status(installed: bool, cli_path: &Path, app_exe: &Path) -> CliInstallStatus {
    CliInstallStatus {
        installed,
        path: cli_path.to_string_lossy().to_string(),
        target: app_exe.to_string_lossy().to_string(),
    }
}

fn user_bin_dir() -> Result<PathBuf, String> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "failed to locate HOME directory".to_string())?;
    Ok(home.join(".local").join("bin"))
}
