use crate::{
    application::document_service::DocumentService, domain::document::MarkdownDocument,
    infrastructure::fs_document_reader::FsDocumentReader,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const WINDOW_HIGHLIGHT_EVENT: &str = "markdown-annotator://window-highlight";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallStatus {
    installed: bool,
    path: String,
    target: String,
}

#[tauri::command]
pub fn read_markdown_file(path: String) -> Result<MarkdownDocument, String> {
    let service = DocumentService::new(FsDocumentReader);
    service.read_markdown_file(&path)
}

#[tauri::command]
pub fn install_cli() -> Result<CliInstallStatus, String> {
    let current_exe =
        env::current_exe().map_err(|error| format!("failed to locate app executable: {error}"))?;
    let bin_dir = user_bin_dir()?;
    fs::create_dir_all(&bin_dir).map_err(|error| {
        format!(
            "failed to create CLI install directory {}: {error}",
            bin_dir.display()
        )
    })?;

    let cli_path = bin_dir.join("ma");
    if cli_path.is_dir() {
        return Err(format!(
            "cannot install ma because path is a directory: {}",
            cli_path.display()
        ));
    }

    let script = cli_launcher_script(&current_exe);
    fs::write(&cli_path, script)
        .map_err(|error| format!("failed to write {}: {error}", cli_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&cli_path, fs::Permissions::from_mode(0o755)).map_err(|error| {
            format!("failed to mark {} executable: {error}", cli_path.display())
        })?;
    }

    Ok(cli_install_status(true, &cli_path, &current_exe))
}

#[tauri::command]
pub fn check_cli_installed() -> Result<CliInstallStatus, String> {
    let current_exe =
        env::current_exe().map_err(|error| format!("failed to locate app executable: {error}"))?;
    let cli_path = user_bin_dir()?.join("ma");
    let expected = cli_launcher_script(&current_exe);
    let installed = fs::read_to_string(&cli_path)
        .map(|content| content == expected)
        .unwrap_or(false);

    Ok(cli_install_status(installed, &cli_path, &current_exe))
}

#[tauri::command]
pub fn request_open_document_window(app: tauri::AppHandle, path: String) -> Result<(), String> {
    open_document_window_path(&app, &path)
}

#[tauri::command]
pub fn request_open_document_tab(
    app: tauri::AppHandle,
    window: WebviewWindow,
    path: String,
) -> Result<(), String> {
    let document_path = resolve_markdown_file(&path)?;
    let label = label_for_document(&document_path);

    if focus_if_open(&app, &label) {
        return Ok(());
    }

    let new_window = create_document_window(&app, &label, &document_path)?;
    attach_window_as_tab(&window, &new_window);
    Ok(())
}

pub fn open_welcome_window(app: &tauri::AppHandle) {
    if app.get_webview_window("main").is_some() {
        return;
    }

    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Markdown Annotator")
        .inner_size(1280.0, 860.0)
        .min_inner_size(980.0, 680.0);

    #[cfg(target_os = "macos")]
    {
        builder = builder.tabbing_identifier("markdown-annotator");
    }

    match builder.build() {
        Ok(window) => show_native_tab_bar(&window),
        Err(error) => eprintln!("failed to create main window: {error}"),
    }
}

pub fn open_document_window_path(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    let document_path = resolve_markdown_file(path)?;
    let label = label_for_document(&document_path);

    if focus_if_open(app, &label) {
        return Ok(());
    }

    create_document_window(app, &label, &document_path).map(|_| ())
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
        show_native_tab_bar(&window);
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

fn create_document_window(
    app: &tauri::AppHandle,
    label: &str,
    path: &Path,
) -> Result<WebviewWindow, String> {
    let encoded_path = utf8_percent_encode(&path.to_string_lossy(), NON_ALPHANUMERIC).to_string();
    let url = format!("index.html?path={encoded_path}");
    let title = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Markdown document");

    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title(title)
        .inner_size(1280.0, 860.0)
        .min_inner_size(980.0, 680.0);

    #[cfg(target_os = "macos")]
    {
        builder = builder.tabbing_identifier("markdown-annotator");
    }

    let window = builder
        .build()
        .map_err(|error| format!("failed to create document window: {error}"))?;
    show_native_tab_bar(&window);
    Ok(window)
}

#[cfg(target_os = "macos")]
fn attach_window_as_tab(base_window: &WebviewWindow, new_window: &WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowOrderingMode};

    let Ok(base_ptr) = base_window.ns_window() else {
        return;
    };
    let Ok(new_ptr) = new_window.ns_window() else {
        return;
    };

    let base_ns_window: &NSWindow = unsafe { &*base_ptr.cast::<NSWindow>() };
    let new_ns_window: &NSWindow = unsafe { &*new_ptr.cast::<NSWindow>() };

    if base_ns_window.tabbingIdentifier().to_string() == "markdown-annotator" {
        base_ns_window.addTabbedWindow_ordered(new_ns_window, NSWindowOrderingMode::Above);
    }
}

#[cfg(not(target_os = "macos"))]
fn attach_window_as_tab(_base_window: &WebviewWindow, _new_window: &WebviewWindow) {}

fn focus_if_open(app: &tauri::AppHandle, label: &str) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };

    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    show_native_tab_bar(&window);
    let _ = window.emit(WINDOW_HIGHLIGHT_EVENT, ());
    true
}

#[cfg(target_os = "macos")]
fn show_native_tab_bar(window: &WebviewWindow) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSWindow;

    let window = window.clone();
    let app = window.app_handle().clone();
    let _ = app.run_on_main_thread(move || {
        let Some(_mtm) = MainThreadMarker::new() else {
            return;
        };
        let Ok(ptr) = window.ns_window() else {
            return;
        };
        let ns_window: &NSWindow = unsafe { &*ptr.cast::<NSWindow>() };

        if ns_window
            .tabGroup()
            .map(|tab_group| tab_group.isTabBarVisible())
            .unwrap_or(false)
        {
            return;
        }

        ns_window.toggleTabBar(None);
    });
}

#[cfg(not(target_os = "macos"))]
fn show_native_tab_bar(_window: &WebviewWindow) {}

fn label_for_document(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("document-{}", hasher.finish())
}

fn resolve_markdown_file(raw_path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_path);
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve {raw_path}: {error}"))?;

    if !canonical.is_file() {
        return Err(format!(
            "target must be a markdown file: {}",
            canonical.display()
        ));
    }

    if !is_markdown_file(&canonical) {
        return Err(format!(
            "target must be a markdown file: {}",
            canonical.display()
        ));
    }

    Ok(canonical)
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

fn cli_launcher_script(app_exe: &Path) -> String {
    format!(
        r#"#!/bin/sh
APP_EXE={}
if [ ! -x "$APP_EXE" ]; then
  echo "ma: Markdown Annotator executable is not available: $APP_EXE" >&2
  exit 1
fi
nohup "$APP_EXE" "$@" >/dev/null 2>&1 &
"#,
        shell_quote(&app_exe.to_string_lossy())
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdx"
            )
        })
        .unwrap_or(false)
}
