use anyhow::Result;
use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
    thread,
    time::{Duration, Instant},
};

pub const TOOL_FILE_CHANGE_TEXT_LIMIT: usize = 128 * 1024;

pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else if let Some(parent) = path.parent() {
        let parent = if parent.as_os_str().is_empty() {
            env::current_dir()?
        } else if parent.exists() {
            parent.canonicalize()?
        } else {
            normalize_path(parent)?
        };
        Ok(parent.join(path.file_name().unwrap_or_default()))
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

pub fn clean_tool_title(title: Option<&str>) -> String {
    let text = title.unwrap_or("").trim();
    if text.eq_ignore_ascii_case("none") {
        String::new()
    } else {
        text.to_string()
    }
}

pub fn extract_locations(update: &Value) -> Vec<String> {
    let paths = update
        .get("locations")
        .and_then(Value::as_array)
        .map(|locations| {
            locations
                .iter()
                .filter_map(|location| location.get("path").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.contains(&path) {
            deduped.push(path);
        }
    }
    deduped
}

pub fn string_param<'a>(params: &'a Value, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing string param: {key}"))
}

pub fn select_lines(content: &str, start: usize, limit: Option<usize>) -> String {
    let mut selected = String::new();
    let iter = content.split_inclusive('\n').skip(start);
    match limit {
        Some(limit) => {
            for line in iter.take(limit) {
                selected.push_str(line);
            }
        }
        None => {
            for line in iter {
                selected.push_str(line);
            }
        }
    }
    selected
}

pub fn truncate_for_tool_file_change(content: &str) -> (String, bool) {
    if content.len() <= TOOL_FILE_CHANGE_TEXT_LIMIT {
        return (content.to_string(), false);
    }

    let mut end = TOOL_FILE_CHANGE_TEXT_LIMIT;
    while !content.is_char_boundary(end) {
        end -= 1;
    }
    (content[..end].to_string(), true)
}

pub fn simple_unified_diff(path: &str, before: Option<&str>, after: Option<&str>) -> String {
    let mut lines = Vec::new();
    lines.push(format!("--- a/{path}"));
    lines.push(format!("+++ b/{path}"));
    lines.push("@@ -1 +1 @@".to_string());
    if let Some(before) = before {
        for line in before.lines() {
            lines.push(format!("-{line}"));
        }
    }
    if let Some(after) = after {
        for line in after.lines() {
            lines.push(format!("+{line}"));
        }
    }
    lines.join("\n")
}

/// macOS GUI 앱은 Finder/Launchpad에서 실행될 때 로그인 셸을 거치지 않아
/// PATH가 최소값(`/usr/bin:/bin:/usr/sbin:/sbin`)으로 제한된다. 그 결과
/// `npx`·`node` 등 버전 매니저(nvm·fnm·asdf·volta)나 Homebrew가 설치한
/// 비표준 경로의 실행 파일을 spawn하지 못한다.
///
/// 사용자의 로그인 셸에서 실제 PATH를 한 번 조회해 캐시한 뒤, 현재 PATH와
/// 병합해 돌려준다. 셸 설정(`.zshrc`/`.zprofile` 등)에 정의된 모든 경로가
/// 자동으로 포함된다.
pub fn enriched_path() -> &'static str {
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(build_enriched_path)
}

fn build_enriched_path() -> String {
    let mut dirs: Vec<String> = Vec::new();
    let push_unique = |dirs: &mut Vec<String>, value: &str| {
        for part in value.split(':') {
            if !part.is_empty() && !dirs.iter().any(|existing| existing == part) {
                dirs.push(part.to_string());
            }
        }
    };

    // 로그인 셸에서 가져온 PATH를 앞쪽에 둬 사용자가 의도한 우선순위를 따른다.
    if let Some(login_path) = login_shell_path() {
        push_unique(&mut dirs, &login_path);
    }
    if let Ok(current) = env::var("PATH") {
        push_unique(&mut dirs, &current);
    }
    // 셸 조회가 실패한 경우를 대비한 흔한 fallback 위치.
    for fallback in ["/opt/homebrew/bin", "/usr/local/bin"] {
        push_unique(&mut dirs, fallback);
    }

    dirs.join(":")
}

/// 로그인 + 인터랙티브 셸을 실행해 PATH를 출력시킨다. 인터랙티브(`-i`)로
/// 띄워야 nvm 등 `.zshrc`/`.bashrc`에 정의된 PATH 변경이 적용된다.
fn login_shell_path() -> Option<String> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut child = std::process::Command::new(&shell)
        .args(["-ilc", "printf '%s' \"$PATH\""])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                let output = child.wait_with_output().ok()?;
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return (!path.is_empty()).then_some(path);
            }
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => return None,
        }
    }
}

/// `program`을 enriched PATH 안에서 실제 실행 파일의 절대경로로 변환한다.
/// 이미 경로 구분자(`/`)를 포함하면 그대로 둔다. 후보를 찾지 못하면 원본을
/// 돌려줘 호출부의 에러 메시지가 일관되게 유지되도록 한다.
pub fn resolve_program(program: &str) -> String {
    if program.contains('/') {
        return program.to_string();
    }
    for dir in enriched_path().split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join(program);
        if is_executable(&candidate) {
            return candidate.to_string_lossy().into_owned();
        }
    }
    program.to_string()
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

pub fn display_command(command: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(command.to_string());
    parts.extend(args.iter().cloned());
    shell_words::join(parts)
}

pub fn rpc_to_anyhow(err: RpcError) -> anyhow::Error {
    match err.data {
        Some(data) => anyhow::anyhow!("JSON-RPC error {}: {} ({data})", err.code, err.message),
        None => anyhow::anyhow!("JSON-RPC error {}: {}", err.code, err.message),
    }
}

#[derive(Debug)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}
