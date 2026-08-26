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

/// Kiro 대화 로그에서 읽을 최대 줄 수. 실측 최대 세션이 1255줄(5.3MB)이라
/// 전부 읽으면 목록 조회가 느려진다. 다른 provider와 같은 상한을 쓰며, 그
/// 결과 대형 세션의 `message_count`는 실제보다 작게 나온다 — 목록의 보조
/// 표시값이므로 정확도보다 응답 속도를 택했다(specs/036 data-model §4.1).
const KIRO_MAX_LOG_LINES: usize = 200;

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
    kiro: Option<PathBuf>,
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

    fn kiro_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.roots.kiro {
            return Ok(root.clone());
        }
        if let Some(value) = std::env::var_os("KIRO_SESSION_DIR") {
            return Ok(PathBuf::from(value));
        }
        Ok(home_dir()?.join(".kiro").join("sessions").join("cli"))
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
            ProviderKind::Kiro => scan_agent(agent_id, self.kiro_root()?, scope, parse_kiro),
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

/// Kiro CLI 세션 하나를 요약한다.
///
/// Kiro는 세션을 `<uuid>.json`(메타)과 `<uuid>.jsonl`(대화 로그) 쌍으로 남긴다.
/// `scan_agent`가 `.jsonl`을 넘기므로 메타는 같은 stem의 `.json`에서 읽는다.
/// 목록 표시에 필요한 값은 전부 메타에 있어, 로그는 메시지 수를 세는 데만 쓴다.
///
/// 메타를 읽거나 해석할 수 없으면 이 세션만 건너뛴다(`Ok(None)`). 목록 전체를
/// 실패시키지 않기 위해서다.
fn parse_kiro(agent_id: &str, path: &Path) -> Result<Option<ProviderSession>> {
    let metadata = fs::metadata(path)?;
    let Ok(meta_contents) = fs::read_to_string(path.with_extension("json")) else {
        return Ok(None);
    };
    let Ok(meta) = serde_json::from_str::<Value>(&meta_contents) else {
        return Ok(None);
    };

    let meta_str = |key: &str| meta.get(key).and_then(Value::as_str).map(ToOwned::to_owned);

    let events = read_json_lines(path, KIRO_MAX_LOG_LINES)?;
    // 연결만 맺고 대화가 오가지 않은 세션은 이어갈 내용이 없으므로 목록에서 뺀다.
    // 제목 유무로 판단하지 않는 이유는, 대화 중이라 제목이 아직 없는 세션을
    // 잘못 제외하지 않기 위해서다(specs/036 research R4).
    if events.is_empty() {
        return Ok(None);
    }
    let message_count = events.iter().filter(|event| is_kiro_message(event)).count();

    let id = meta_str("session_id").unwrap_or_else(|| file_stem_id(path));
    let cwd = meta_str("cwd");
    // 제목은 대화가 어느 정도 진행된 뒤에 붙는다. 아직 없으면 첫 프롬프트로 대신한다.
    let title = meta_str("title").or_else(|| extract_kiro_prompt_text(&events));
    let created_at = meta
        .get("created_at")
        .and_then(Value::as_str)
        .and_then(parse_timestamp);
    let updated_at = meta
        .get("updated_at")
        .and_then(Value::as_str)
        .and_then(parse_timestamp);
    // 모델은 세션 상태 안에 중첩되어 있다.
    let model = meta
        .pointer("/session_state/rts_model_state/model_info/model_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

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
        // Kiro 메타에는 git 정보가 없고, 생성 경위 값은 모든 세션이 동일해
        // 의미가 없다(specs/036 research R2·R3).
        branch: None,
        source: None,
    }))
}

/// 메타에 제목이 없을 때 첫 사용자 프롬프트에서 제목을 만든다.
///
/// Kiro의 프롬프트 본문은 `data.content` 배열이고 각 항목이
/// `{"kind": "text", "data": "..."}` 형태다. 텍스트 항목만 골라 발췌한다.
fn extract_kiro_prompt_text(events: &[Value]) -> Option<String> {
    let prompt = events
        .iter()
        .find(|event| event.get("kind").and_then(Value::as_str) == Some("Prompt"))?;

    prompt
        .pointer("/data/content")?
        .as_array()?
        .iter()
        .find_map(|item| {
            if item.get("kind").and_then(Value::as_str) != Some("text") {
                return None;
            }
            item.get("data").and_then(Value::as_str).map(snippet)
        })
}

/// 대화 메시지로 셀 이벤트인지 판별한다. 도구 결과와 컨텍스트 압축은 메시지가
/// 아니므로 제외한다 — Codex 파서가 도구 호출을 빼고 메시지만 세는 것과 같다.
fn is_kiro_message(event: &Value) -> bool {
    matches!(
        event.get("kind").and_then(Value::as_str),
        Some("Prompt" | "AssistantMessage")
    )
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

    fn repo_with_kiro(root: PathBuf) -> FsProviderSessionRepository {
        FsProviderSessionRepository {
            roots: SessionRoots {
                kiro: Some(root),
                ..Default::default()
            },
        }
    }

    /// Kiro 세션 한 건을 fixture로 만든다. Kiro는 메타(`.json`)와
    /// 대화 로그(`.jsonl`)를 쌍으로 남긴다.
    fn write_kiro_session(dir: &TempDir, id: &str, meta: &str, log: &str) {
        dir.write(&format!("{id}.json"), meta);
        dir.write(&format!("{id}.jsonl"), log);
    }

    /// cwd와 제목을 바꿔 끼울 수 있는 기본 메타 fixture.
    fn kiro_meta(id: &str, cwd: &str, title: Option<&str>) -> String {
        let title_field = match title {
            Some(value) => format!("\"title\": \"{value}\","),
            None => String::new(),
        };
        format!(
            r#"{{
                "session_id": "{id}",
                "cwd": "{cwd}",
                {title_field}
                "created_at": "2026-08-21T08:15:59.452811Z",
                "updated_at": "2026-08-21T08:20:52.971217Z",
                "session_created_reason": "subagent",
                "session_state": {{
                    "agent_name": "kiro_default",
                    "rts_model_state": {{
                        "model_info": {{ "model_id": "claude-sonnet-5" }}
                    }}
                }}
            }}"#
        )
    }

    /// Prompt 1건 + AssistantMessage 2건 + ToolResults 1건.
    /// 메시지로 세어야 하는 것은 3건이다.
    ///
    /// 프롬프트 본문은 실제 Kiro 로그와 같은 `[{"kind":"text","data":"..."}]`
    /// 배열 형태로 둔다.
    const KIRO_LOG: &str = concat!(
        r#"{"kind":"Prompt","version":"v1","data":{"content":[{"kind":"text","data":"이 코드를 리팩터링해줘"}]}}"#,
        "\n",
        r#"{"kind":"AssistantMessage","version":"v1","data":{}}"#,
        "\n",
        r#"{"kind":"ToolResults","version":"v1","data":{}}"#,
        "\n",
        r#"{"kind":"AssistantMessage","version":"v1","data":{}}"#,
        "\n",
    );

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
        assert!(matches!(
            provider_kind_for("codex"),
            Some(ProviderKind::Codex)
        ));
        assert!(matches!(
            provider_kind_for("pi-coding-agent"),
            Some(ProviderKind::Pi)
        ));
        assert!(matches!(
            provider_kind_for("kiro-cli"),
            Some(ProviderKind::Kiro)
        ));
    }

    /// T-1: 정상 세션 1건의 모든 필드가 매핑대로 채워진다.
    #[test]
    fn parses_kiro_session_meta_into_provider_session() {
        let dir = TempDir::new();
        write_kiro_session(
            &dir,
            "kiro-1",
            &kiro_meta("kiro-1", "/work/project", Some("리팩터링 논의")),
            KIRO_LOG,
        );
        let repo = repo_with_kiro(dir.path.clone());

        let sessions = repo.list("kiro-cli", &SessionScope::All).expect("list");

        assert_eq!(sessions.len(), 1);
        let session = &sessions[0];
        assert_eq!(session.agent_id, "kiro-cli");
        assert_eq!(session.id, "kiro-1");
        assert_eq!(session.cwd.as_deref(), Some("/work/project"));
        assert_eq!(session.title.as_deref(), Some("리팩터링 논의"));
        assert_eq!(session.model.as_deref(), Some("claude-sonnet-5"));
        assert!(session.file.ends_with("kiro-1.jsonl"));
        // 메타의 시각이 RFC3339로 정규화되어 실린다.
        assert!(
            session
                .created_at
                .as_deref()
                .is_some_and(|value| value.starts_with("2026-08-21T08:15:59")),
            "created_at was {:?}",
            session.created_at
        );
        assert!(
            session
                .updated_at
                .as_deref()
                .is_some_and(|value| value.starts_with("2026-08-21T08:20:52")),
            "updated_at was {:?}",
            session.updated_at
        );
        // Kiro 메타에는 대응 정보가 없어 비워둔다.
        assert_eq!(session.branch, None);
        assert_eq!(session.source, None);
    }

    /// T-9: 메시지만 세고 도구 결과는 제외한다.
    #[test]
    fn kiro_message_count_excludes_tool_results() {
        let dir = TempDir::new();
        write_kiro_session(
            &dir,
            "kiro-1",
            &kiro_meta("kiro-1", "/work/project", Some("제목")),
            KIRO_LOG,
        );
        let repo = repo_with_kiro(dir.path.clone());

        let sessions = repo.list("kiro-cli", &SessionScope::All).expect("list");

        // Prompt 1 + AssistantMessage 2 = 3. ToolResults는 세지 않는다.
        assert_eq!(sessions[0].message_count, 3);
    }

    /// T-2, T-3: 작업 디렉터리 범위로 걸러진다.
    #[test]
    fn kiro_sessions_are_filtered_by_scope() {
        let dir = TempDir::new();
        write_kiro_session(
            &dir,
            "kiro-1",
            &kiro_meta("kiro-1", "/work/project", Some("제목")),
            KIRO_LOG,
        );
        let repo = repo_with_kiro(dir.path.clone());

        let matching = repo
            .list(
                "kiro-cli",
                &SessionScope::Path(PathBuf::from("/work/project")),
            )
            .expect("list matching");
        assert_eq!(matching.len(), 1);

        let other = repo
            .list("kiro-cli", &SessionScope::Path(PathBuf::from("/other")))
            .expect("list other");
        assert!(other.is_empty());
    }

    /// T-5: 제목이 아직 없으면 첫 프롬프트에서 만들어 쓴다.
    #[test]
    fn kiro_title_falls_back_to_first_prompt() {
        let dir = TempDir::new();
        write_kiro_session(
            &dir,
            "kiro-1",
            &kiro_meta("kiro-1", "/work/project", None),
            KIRO_LOG,
        );
        let repo = repo_with_kiro(dir.path.clone());

        let sessions = repo.list("kiro-cli", &SessionScope::All).expect("list");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title.as_deref(), Some("이 코드를 리팩터링해줘"));
    }

    /// T-4: 대화가 없는 세션은 빼되, 제목만 없는 세션은 남긴다.
    #[test]
    fn empty_kiro_sessions_are_excluded_but_untitled_ones_are_kept() {
        let dir = TempDir::new();
        // 연결만 맺고 끝난 세션 — 로그가 비어 있다.
        write_kiro_session(
            &dir,
            "kiro-empty",
            &kiro_meta("kiro-empty", "/work/project", None),
            "",
        );
        // 대화는 오갔지만 아직 제목이 붙지 않은 세션.
        write_kiro_session(
            &dir,
            "kiro-untitled",
            &kiro_meta("kiro-untitled", "/work/project", None),
            KIRO_LOG,
        );
        let repo = repo_with_kiro(dir.path.clone());

        let sessions = repo.list("kiro-cli", &SessionScope::All).expect("list");

        assert_eq!(sessions.len(), 1, "빈 세션만 제외되어야 한다");
        assert_eq!(sessions[0].id, "kiro-untitled");
    }

    /// T-6: 세션 하나가 깨져도 나머지 목록은 살아남는다.
    #[test]
    fn broken_kiro_metadata_does_not_fail_the_whole_list() {
        let dir = TempDir::new();
        write_kiro_session(&dir, "kiro-broken", "{ not valid json", KIRO_LOG);
        // 메타 파일 자체가 없는 경우도 함께 확인한다.
        dir.write("kiro-orphan.jsonl", KIRO_LOG);
        write_kiro_session(
            &dir,
            "kiro-ok",
            &kiro_meta("kiro-ok", "/work/project", Some("정상 세션")),
            KIRO_LOG,
        );
        let repo = repo_with_kiro(dir.path.clone());

        let sessions = repo.list("kiro-cli", &SessionScope::All).expect("list");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "kiro-ok");
    }

    /// T-7: 로그에 깨진 줄이 섞여도 유효 이벤트만 세고 세션은 유지한다.
    #[test]
    fn broken_kiro_log_lines_are_skipped() {
        let dir = TempDir::new();
        let log = concat!(
            r#"{"kind":"Prompt","version":"v1","data":{"content":[{"kind":"text","data":"질문"}]}}"#,
            "\n",
            "이 줄은 JSON이 아니다\n",
            r#"{"kind":"AssistantMessage","version":"v1","data":{}}"#,
            "\n",
        );
        write_kiro_session(
            &dir,
            "kiro-1",
            &kiro_meta("kiro-1", "/work/project", Some("제목")),
            log,
        );
        let repo = repo_with_kiro(dir.path.clone());

        let sessions = repo.list("kiro-cli", &SessionScope::All).expect("list");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].message_count, 2);
    }

    /// T-8: 세션 루트가 없으면 오류가 아니라 빈 목록이다.
    #[test]
    fn missing_kiro_root_returns_empty_list() {
        let dir = TempDir::new();
        let repo = repo_with_kiro(dir.path.join("does-not-exist"));

        let sessions = repo.list("kiro-cli", &SessionScope::All).expect("list");

        assert!(sessions.is_empty());
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
