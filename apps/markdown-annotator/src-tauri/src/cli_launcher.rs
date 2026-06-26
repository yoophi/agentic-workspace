use std::{
    env, fs,
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use url::Url;

pub enum CliMode {
    Dev,
    Release,
}

struct DevPaths {
    app_dir: PathBuf,
    src_tauri_dir: PathBuf,
}

struct DevServerEndpoint {
    display_url: String,
    hosts: Vec<String>,
    port: u16,
}

pub fn run(command_name: &str, mode: CliMode) -> Result<(), String> {
    let Some(raw_target) = parse_target_arg(command_name)? else {
        println!("usage: {command_name} <markdown-file>");
        return Ok(());
    };

    let cwd = env::current_dir().map_err(|error| format!("failed to read cwd: {error}"))?;
    let target = resolve_markdown_file(&raw_target, &cwd)?;
    let app = match mode {
        CliMode::Dev => find_dev_app()?,
        CliMode::Release => find_release_app()?,
    };

    debug_log(command_name, format!("launch target: {}", target.display()));
    debug_log(command_name, format!("app executable: {}", app.display()));

    if matches!(mode, CliMode::Dev) {
        ensure_dev_server(command_name)?;
    }

    launch_app(command_name, &app, &target)
}

fn parse_target_arg(command_name: &str) -> Result<Option<String>, String> {
    let mut args = env::args().skip(1);
    let Some(target) = args.next() else {
        return Err(format!("usage: {command_name} <markdown-file>"));
    };

    if matches!(target.as_str(), "-h" | "--help") {
        return Ok(None);
    }

    if args.next().is_some() {
        return Err(format!("usage: {command_name} <markdown-file>"));
    }

    Ok(Some(target))
}

fn resolve_markdown_file(raw_target: &str, cwd: &Path) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw_target);
    let candidate = if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    };

    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("failed to resolve {}: {error}", candidate.display()))?;

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

fn find_release_app() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("MARKDOWN_ANNOTATOR_APP_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }

        return Err(format!(
            "MARKDOWN_ANNOTATOR_APP_PATH does not point to an executable file: {}",
            path.display()
        ));
    }

    let current_exe = env::current_exe()
        .map_err(|error| format!("failed to locate markdown-annotator-cli executable: {error}"))?;
    let exe_dir = current_exe.parent().ok_or_else(|| {
        "failed to locate markdown-annotator-cli executable directory".to_string()
    })?;

    for candidate in release_sibling_app_candidates(exe_dir) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    for candidate in installed_app_candidates() {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err("failed to locate Markdown Annotator app. Set MARKDOWN_ANNOTATOR_APP_PATH to the installed app executable".to_string())
}

fn find_dev_app() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("MARKDOWN_ANNOTATOR_APP_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }

        return Err(format!(
            "MARKDOWN_ANNOTATOR_APP_PATH does not point to an executable file: {}",
            path.display()
        ));
    }

    let current_exe = env::current_exe()
        .map_err(|error| format!("failed to locate ma-dev executable: {error}"))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "failed to locate ma-dev executable directory".to_string())?;
    let candidate = exe_dir.join(app_executable_name());

    ensure_dev_app_built(&current_exe)?;

    if !candidate.is_file() {
        return Err(format!(
            "failed to locate development app executable: {}",
            candidate.display()
        ));
    }

    Ok(candidate)
}

#[cfg(unix)]
fn launch_app(command_name: &str, app: &Path, target: &Path) -> Result<(), String> {
    let app_dir = app.parent().ok_or_else(|| {
        format!(
            "failed to locate app executable directory: {}",
            app.display()
        )
    })?;
    let app_name = app
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("failed to read app executable name: {}", app.display()))?;

    Command::new("nohup")
        .arg(format!("./{app_name}"))
        .arg(target)
        .current_dir(app_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to launch {}: {error}", app.display()))?;

    debug_log(command_name, "spawned app through nohup");
    Ok(())
}

#[cfg(not(unix))]
fn launch_app(_command_name: &str, app: &Path, target: &Path) -> Result<(), String> {
    Command::new(app)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("failed to launch {}: {error}", app.display()))?;

    Ok(())
}

fn ensure_dev_server(command_name: &str) -> Result<(), String> {
    let paths = infer_dev_paths_from_exe()?;
    let endpoint = read_dev_server_endpoint(&paths.src_tauri_dir)?;

    if dev_server_is_running(&endpoint) {
        debug_log(
            command_name,
            format!(
                "Vite dev server is already running at {}",
                endpoint.display_url
            ),
        );
        return Ok(());
    }

    debug_log(
        command_name,
        format!(
            "starting Vite dev server for {} in {}",
            endpoint.display_url,
            paths.app_dir.display()
        ),
    );
    Command::new("pnpm")
        .arg("run")
        .arg("dev")
        .env("VITE_DEV_SERVER_PORT", endpoint.port.to_string())
        .current_dir(&paths.app_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            format!(
                "failed to start Vite dev server in {}: {error}",
                paths.app_dir.display()
            )
        })?;

    wait_for_dev_server(&endpoint)
}

fn infer_dev_paths_from_exe() -> Result<DevPaths, String> {
    let current_exe = env::current_exe()
        .map_err(|error| format!("failed to locate ma-dev executable: {error}"))?;

    let Some(src_tauri_dir) = current_exe
        .ancestors()
        .find(|candidate| candidate.join("tauri.conf.json").is_file())
    else {
        return Err("failed to infer src-tauri directory from ma-dev executable".to_string());
    };

    let Some(app_dir) = src_tauri_dir.parent().map(Path::to_path_buf) else {
        return Err(format!(
            "failed to infer app package directory from {}",
            src_tauri_dir.display()
        ));
    };

    Ok(DevPaths {
        app_dir,
        src_tauri_dir: src_tauri_dir.to_path_buf(),
    })
}

fn ensure_dev_app_built(ma_dev_exe: &Path) -> Result<(), String> {
    let Some(src_tauri_dir) = ma_dev_exe
        .ancestors()
        .find(|candidate| candidate.join("tauri.conf.json").is_file())
    else {
        return Err("failed to infer src-tauri directory from ma-dev executable".to_string());
    };

    let status = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("markdown-annotator")
        .current_dir(src_tauri_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| {
            format!(
                "failed to build development app in {}: {error}",
                src_tauri_dir.display()
            )
        })?;

    if !status.success() {
        return Err(format!(
            "failed to build development app in {}: {status}",
            src_tauri_dir.display()
        ));
    }

    Ok(())
}

fn wait_for_dev_server(endpoint: &DevServerEndpoint) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(15);

    while Instant::now() < deadline {
        if dev_server_is_running(endpoint) {
            return Ok(());
        }

        thread::sleep(Duration::from_millis(150));
    }

    Err(format!(
        "Vite dev server did not start at {} within 15 seconds",
        endpoint.display_url
    ))
}

fn dev_server_is_running(endpoint: &DevServerEndpoint) -> bool {
    endpoint
        .hosts
        .iter()
        .any(|host| TcpStream::connect((host.as_str(), endpoint.port)).is_ok())
}

fn read_dev_server_endpoint(src_tauri_dir: &Path) -> Result<DevServerEndpoint, String> {
    let config_path = src_tauri_dir.join("tauri.conf.json");
    let config_text = fs::read_to_string(&config_path)
        .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
    let config: serde_json::Value = serde_json::from_str(&config_text)
        .map_err(|error| format!("failed to parse {}: {error}", config_path.display()))?;
    let dev_url = config
        .pointer("/build/devUrl")
        .and_then(|value| value.as_str())
        .ok_or_else(|| format!("missing build.devUrl in {}", config_path.display()))?;
    let url = Url::parse(dev_url)
        .map_err(|error| format!("failed to parse build.devUrl {dev_url}: {error}"))?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!(
            "unsupported build.devUrl scheme for ma-dev: {}",
            url.scheme()
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| format!("missing host in build.devUrl {dev_url}"))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| format!("missing port in build.devUrl {dev_url}"))?;

    Ok(DevServerEndpoint {
        display_url: dev_url.to_string(),
        hosts: loopback_hosts_for(host),
        port,
    })
}

fn loopback_hosts_for(host: &str) -> Vec<String> {
    match host {
        "localhost" => vec![
            "127.0.0.1".to_string(),
            "::1".to_string(),
            "localhost".to_string(),
        ],
        "0.0.0.0" => vec!["127.0.0.1".to_string(), "localhost".to_string()],
        "::" => vec!["::1".to_string(), "localhost".to_string()],
        _ => vec![host.to_string()],
    }
}

fn debug_log(command_name: &str, message: impl AsRef<str>) {
    if env::var_os("MA_VERBOSE").is_some() {
        eprintln!("{command_name}: {}", message.as_ref());
    }
}

fn release_sibling_app_candidates(exe_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    {
        candidates.push(exe_dir.join("../MacOS/markdown-annotator"));
    }

    #[cfg(not(target_os = "macos"))]
    {
        candidates.push(exe_dir.join(app_executable_name()));
    }

    candidates
}

fn installed_app_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from(
            "/Applications/Markdown Annotator.app/Contents/MacOS/markdown-annotator",
        ));
        candidates.push(PathBuf::from(
            "/Applications/markdown-annotator.app/Contents/MacOS/markdown-annotator",
        ));
    }

    candidates
}

fn app_executable_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "markdown-annotator.exe"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "markdown-annotator"
    }
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
