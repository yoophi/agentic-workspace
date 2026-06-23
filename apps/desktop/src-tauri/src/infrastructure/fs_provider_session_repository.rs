use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde_json::Value;
use walkdir::WalkDir;

use crate::domain::provider_session::{
    ProviderKind, ProviderSession, SessionScope, provider_kind_for,
};
use crate::ports::provider_session_repository::ProviderSessionRepository;

/// 각 provider가 로컬에 남긴 JSONL 세션 파일을 읽어 목록화하는 어댑터.
/// 루트 경로는 환경변수(`CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`PI_CODING_AGENT_SESSION_DIR`)를
/// 우선 따르고, 없으면 `$HOME` 아래 기본 위치를 사용한다.
#[derive(Clone, Debug, Default)]
pub struct FsProviderSessionRepository {
    roots: SessionRoots,
}

#[derive(Clone, Debug, Default)]
struct SessionRoots {
    claude: Option<PathBuf>,
    codex: Option<PathBuf>,
    pi: Option<PathBuf>,
}

impl FsProviderSessionRepository {
    pub fn new() -> Self {
        Self::default()
    }

    fn claude_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.roots.claude {
            return Ok(root.clone());
        }
        let base = std::env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or(home_dir()?.join(".claude"));
        Ok(base.join("projects"))
    }

    fn codex_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.roots.codex {
            return Ok(root.clone());
        }
        let base = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or(home_dir()?.join(".codex"));
        Ok(base.join("sessions"))
    }

    fn pi_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.roots.pi {
            return Ok(root.clone());
        }
        if let Some(value) = std::env::var_os("PI_CODING_AGENT_SESSION_DIR") {
            return Ok(PathBuf::from(value));
        }
        Ok(home_dir()?.join(".pi").join("agent").join("sessions"))
    }
}

impl ProviderSessionRepository for FsProviderSessionRepository {
    fn list(&self, agent_id: &str, scope: &SessionScope) -> Result<Vec<ProviderSession>> {
        let Some(kind) = provider_kind_for(agent_id) else {
            return Ok(Vec::new());
        };
        match kind {
            ProviderKind::Claude => scan_agent(agent_id, self.claude_root()?, scope, parse_claude),
            ProviderKind::Codex => scan_agent(agent_id, self.codex_root()?, scope, parse_codex),
            ProviderKind::Pi => scan_agent(agent_id, self.pi_root()?, scope, parse_pi),
        }
    }
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("HOME is not set"))
}

fn scan_agent(
    agent_id: &str,
    root: PathBuf,
    scope: &SessionScope,
    parser: fn(&str, &Path) -> Result<Option<ProviderSession>>,
) -> Result<Vec<ProviderSession>> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
    {
        if let Some(session) = parser(agent_id, entry.path())?
            && matches_scope(&session, scope)
        {
            sessions.push(session);
        }
    }

    Ok(sessions)
}

fn matches_scope(session: &ProviderSession, scope: &SessionScope) -> bool {
    match scope {
        SessionScope::All => true,
        SessionScope::Path(path) => session
            .cwd
            .as_ref()
            .is_some_and(|cwd| Path::new(cwd) == path),
    }
}

fn parse_claude(agent_id: &str, path: &Path) -> Result<Option<ProviderSession>> {
    // 부모 세션에 딸린 subagent transcript는 별도 세션으로 취급하지 않는다.
    if path
        .components()
        .any(|component| component.as_os_str() == OsStr::new("subagents"))
    {
        return Ok(None);
    }

    let metadata = fs::metadata(path)?;
    let mut id = file_stem_id(path);
    let mut cwd = None;
    let mut title = None;
    let mut message_count = 0;
    let mut created_at = None;
    let mut updated_at = None;
    let mut model = None;
    let mut branch = None;
    let mut source = None;

    for value in read_json_lines(path, 200)? {
        if let Some(session_id) = value.get("sessionId").and_then(Value::as_str) {
            id = session_id.to_string();
        }
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            apply_timestamp(&mut created_at, &mut updated_at, timestamp);
        }
        if cwd.is_none() {
            cwd = value
                .get("cwd")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        if model.is_none() {
            model = value
                .get("message")
                .and_then(|message| message.get("model"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        if branch.is_none() {
            branch = value
                .get("gitBranch")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        if source.is_none() {
            source = value
                .get("entrypoint")
                .or_else(|| value.get("promptSource"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
        }
        match value.get("type").and_then(Value::as_str) {
            Some("user") => {
                message_count += 1;
                if title.is_none() {
                    title = extract_claude_user_text(&value);
                }
            }
            Some("assistant") => message_count += 1,
            _ => {}
        }
    }

    Ok(Some(ProviderSession {
        agent_id: agent_id.to_string(),
        id,
        cwd,
        title,
        file: path.to_string_lossy().into_owned(),
        message_count,
        created_at: resolve_created(created_at, &metadata),
        updated_at: resolve_updated(updated_at, &metadata),
        model,
        branch,
        source,
    }))
}

fn parse_codex(agent_id: &str, path: &Path) -> Result<Option<ProviderSession>> {
    let metadata = fs::metadata(path)?;
    let mut id = file_stem_id(path);
    let mut cwd = None;
    let mut title = None;
    let mut message_count = 0;
    let mut created_at = None;
    let mut updated_at = None;
    let mut model = None;
    let mut branch = None;
    let mut source = None;

    for value in read_json_lines(path, 200)? {
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            apply_timestamp(&mut created_at, &mut updated_at, timestamp);
        }

        if value.get("type").and_then(Value::as_str) == Some("session_meta")
            && let Some(payload) = value.get("payload")
        {
            if let Some(meta_id) = payload.get("id").and_then(Value::as_str) {
                id = meta_id.to_string();
            }
            cwd = payload
                .get("cwd")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            source = payload
                .get("source")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            branch = payload
                .get("git")
                .and_then(|git| git.get("branch"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            if let Some(timestamp) = payload.get("timestamp").and_then(Value::as_str) {
                apply_timestamp(&mut created_at, &mut updated_at, timestamp);
            }
        }

        if value.get("type").and_then(Value::as_str) == Some("response_item") {
            let Some(payload) = value.get("payload") else {
                continue;
            };
            if payload.get("type").and_then(Value::as_str) == Some("message") {
                message_count += 1;
                if let Some(payload_model) = payload.get("model").and_then(Value::as_str) {
                    model = Some(payload_model.to_string());
                }
                if title.is_none() && payload.get("role").and_then(Value::as_str) == Some("user") {
                    title = extract_codex_user_text(payload);
                }
            }
        }
    }

    Ok(Some(ProviderSession {
        agent_id: agent_id.to_string(),
        id,
        cwd,
        title,
        file: path.to_string_lossy().into_owned(),
        message_count,
        created_at: resolve_created(created_at, &metadata),
        updated_at: resolve_updated(updated_at, &metadata),
        model,
        branch,
        source,
    }))
}

fn parse_pi(agent_id: &str, path: &Path) -> Result<Option<ProviderSession>> {
    let metadata = fs::metadata(path)?;
    let mut id = file_stem_id(path);
    let mut cwd = None;
    let mut title = None;
    let mut message_count = 0;
    let mut created_at = None;
    let mut updated_at = None;
    let mut model = None;

    for value in read_json_lines(path, 200)? {
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            apply_timestamp(&mut created_at, &mut updated_at, timestamp);
        }

        match value.get("type").and_then(Value::as_str) {
            Some("session") => {
                if let Some(header_id) = value.get("id").and_then(Value::as_str) {
                    id = header_id.to_string();
                }
                cwd = value
                    .get("cwd")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
            }
            Some("model_change") => {
                model = value
                    .get("modelId")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
            }
            Some("message") => {
                message_count += 1;
                if let Some(message_model) = value
                    .get("message")
                    .and_then(|message| message.get("model"))
                    .and_then(Value::as_str)
                {
                    model = Some(message_model.to_string());
                }
                if title.is_none() {
                    title = extract_pi_user_text(&value);
                }
            }
            Some("session_info") => {
                if let Some(name) = value.get("name").and_then(Value::as_str) {
                    title = Some(name.to_string());
                }
            }
            _ => {}
        }
    }

    Ok(Some(ProviderSession {
        agent_id: agent_id.to_string(),
        id,
        cwd,
        title,
        file: path.to_string_lossy().into_owned(),
        message_count,
        created_at: resolve_created(created_at, &metadata),
        updated_at: resolve_updated(updated_at, &metadata),
        model,
        branch: None,
        source: None,
    }))
}

fn file_stem_id(path: &Path) -> String {
    path.file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn resolve_created(parsed: Option<DateTime<Utc>>, metadata: &fs::Metadata) -> Option<String> {
    parsed
        .or_else(|| metadata.created().ok().map(system_time_to_utc))
        .map(|value| value.to_rfc3339())
}

fn resolve_updated(parsed: Option<DateTime<Utc>>, metadata: &fs::Metadata) -> Option<String> {
    parsed
        .or_else(|| metadata.modified().ok().map(system_time_to_utc))
        .map(|value| value.to_rfc3339())
}

fn system_time_to_utc(time: SystemTime) -> DateTime<Utc> {
    DateTime::<Utc>::from(time)
}

fn apply_timestamp(
    created_at: &mut Option<DateTime<Utc>>,
    updated_at: &mut Option<DateTime<Utc>>,
    value: &str,
) {
    let Some(parsed) = parse_timestamp(value) else {
        return;
    };

    if created_at.is_none_or(|current| parsed < current) {
        *created_at = Some(parsed);
    }
    if updated_at.is_none_or(|current| parsed > current) {
        *updated_at = Some(parsed);
    }
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .ok()
}

fn read_json_lines(path: &Path, max_lines: usize) -> Result<Vec<Value>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut values = Vec::new();

    for line in reader.lines().take(max_lines) {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            values.push(value);
        }
    }

    Ok(values)
}

fn extract_claude_user_text(value: &Value) -> Option<String> {
    let content = value.get("message")?.get("content")?;
    match content {
        Value::String(text) => Some(snippet(text)),
        Value::Array(items) => items
            .iter()
            .find_map(|item| item.get("text").and_then(Value::as_str).map(snippet)),
        _ => None,
    }
}

fn extract_codex_user_text(payload: &Value) -> Option<String> {
    payload
        .get("content")?
        .as_array()?
        .iter()
        .find_map(|item| item.get("text").and_then(Value::as_str).map(snippet))
}

fn extract_pi_user_text(value: &Value) -> Option<String> {
    let message = value.get("message")?;
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }

    let content = message.get("content")?;
    match content {
        Value::String(text) => Some(snippet(text)),
        Value::Array(items) => items
            .iter()
            .find_map(|item| item.get("text").and_then(Value::as_str).map(snippet)),
        _ => None,
    }
}

fn snippet(value: &str) -> String {
    let value = value.trim().replace(['\n', '\t'], " ");
    const MAX: usize = 80;
    if value.chars().count() <= MAX {
        return value;
    }
    value.chars().take(MAX).collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("acp-sess-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn write(&self, relative: &str, contents: &str) -> PathBuf {
            let target = self.path.join(relative);
            fs::create_dir_all(target.parent().expect("parent")).expect("create parent");
            let mut file = File::create(&target).expect("create file");
            file.write_all(contents.as_bytes()).expect("write");
            target
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn repo_with_codex(root: PathBuf) -> FsProviderSessionRepository {
        FsProviderSessionRepository {
            roots: SessionRoots {
                codex: Some(root),
                ..Default::default()
            },
        }
    }

    #[test]
    fn unsupported_agent_returns_empty() {
        let repo = FsProviderSessionRepository::new();
        let sessions = repo.list("opencode", &SessionScope::All).expect("list");
        assert!(sessions.is_empty());
    }

    #[test]
    fn provider_kind_mapping() {
        assert_eq!(provider_kind_for("opencode"), None);
        assert!(matches!(
            provider_kind_for("claude-code"),
            Some(ProviderKind::Claude)
        ));
        assert!(matches!(provider_kind_for("codex"), Some(ProviderKind::Codex)));
        assert!(matches!(
            provider_kind_for("pi-coding-agent"),
            Some(ProviderKind::Pi)
        ));
    }

    #[test]
    fn parses_codex_session_meta_and_filters_scope() {
        let dir = TempDir::new();
        dir.write(
            "rollout-a.jsonl",
            concat!(
                r#"{"type":"session_meta","payload":{"id":"sess-123","cwd":"/work/project","timestamp":"2026-06-01T10:00:00Z","source":"cli"}}"#,
                "\n",
                r#"{"type":"response_item","payload":{"type":"message","role":"user","model":"gpt-5","content":[{"text":"Fix the bug"}]}}"#,
                "\n",
                r#"{"type":"response_item","payload":{"type":"message","role":"assistant"}}"#,
                "\n"
            ),
        );
        let repo = repo_with_codex(dir.path.clone());

        let all = repo.list("codex", &SessionScope::All).expect("list all");
        assert_eq!(all.len(), 1);
        let session = &all[0];
        assert_eq!(session.id, "sess-123");
        assert_eq!(session.cwd.as_deref(), Some("/work/project"));
        assert_eq!(session.title.as_deref(), Some("Fix the bug"));
        assert_eq!(session.message_count, 2);
        assert_eq!(session.agent_id, "codex");

        // 일치하는 cwd만 통과한다.
        let matched = repo
            .list("codex", &SessionScope::Path(PathBuf::from("/work/project")))
            .expect("list matched");
        assert_eq!(matched.len(), 1);
        let missed = repo
            .list("codex", &SessionScope::Path(PathBuf::from("/other")))
            .expect("list missed");
        assert!(missed.is_empty());
    }
}
