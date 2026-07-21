use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;

use crate::{
    domain::events::{
        LifecycleStatus, RunEvent, ToolFileChange, ToolFileChangeKind, ToolFileChangeStatus,
    },
    infrastructure::acp::{
        permission_flow,
        session_update_mapper::{MappedSessionUpdate, map_session_update},
        terminal::TerminalHandler,
        transport::RpcPeer,
        util::{
            clean_tool_title, expand_tilde, extract_locations, normalize_path, select_lines,
            simple_unified_diff, string_param, truncate_for_tool_file_change,
        },
    },
    ports::{event_sink::RunEventSink, permission::PermissionDecisionPort},
};

pub struct AcpClient<S, P>
where
    S: RunEventSink,
    P: PermissionDecisionPort,
{
    run_id: String,
    workspace: PathBuf,
    auto_allow: bool,
    permission_decisions: P,
    terminals: TerminalHandler,
    last_tool_signature: Mutex<
        Option<(
            Option<String>,
            String,
            String,
            Vec<String>,
            Vec<ToolFileChange>,
        )>,
    >,
    pending_tool_locations: Mutex<HashMap<String, Vec<String>>>,
    raw_event_log_path: PathBuf,
    sink: S,
}

impl<S, P> AcpClient<S, P>
where
    S: RunEventSink,
    P: PermissionDecisionPort,
{
    pub fn new(
        run_id: String,
        workspace: PathBuf,
        auto_allow: bool,
        permission_decisions: P,
        sink: S,
    ) -> Self {
        let raw_event_log_path = workspace
            .join(".acp-raw-events")
            .join(format!("{run_id}.jsonl"));
        Self {
            run_id,
            workspace,
            auto_allow,
            permission_decisions,
            terminals: TerminalHandler::new(),
            last_tool_signature: Mutex::new(None),
            pending_tool_locations: Mutex::new(HashMap::new()),
            raw_event_log_path,
            sink,
        }
    }

    fn emit(&self, event: RunEvent) {
        self.sink.emit(&self.run_id, event);
    }

    /// Emit a RunEvent on behalf of the transport layer (used from
    /// `read_loop` to report JSON-RPC parse failures).
    pub fn emit_raw(&self, event: RunEvent) {
        self.emit(event);
    }

    pub fn record_raw_rpc_message(&self, message: &Value) {
        if let Err(err) = self.write_raw_rpc_message(message) {
            eprintln!("failed to write ACP raw event log: {err}");
        }
    }

    pub async fn handle_request(
        self: Arc<Self>,
        peer: RpcPeer,
        id: Value,
        method: String,
        params: Value,
    ) {
        let result = match method.as_str() {
            "session/request_permission" => self.request_permission(params).await,
            "fs/read_text_file" => self.read_text_file(params).await,
            "fs/write_text_file" => self.write_text_file(params).await,
            "terminal/create" => self.create_terminal(params).await,
            "terminal/output" => self.terminal_output(params).await,
            "terminal/wait_for_exit" => self.wait_for_terminal_exit(params).await,
            "terminal/kill" => self.kill_terminal(params).await,
            "terminal/release" => self.release_terminal(params).await,
            method if method.starts_with("ext/") => {
                self.emit(RunEvent::Raw {
                    method: method.to_string(),
                    payload: params,
                });
                Ok(json!({}))
            }
            _ => Err(anyhow!("unsupported client method: {method}")),
        };

        match result {
            Ok(result) => {
                let _ = peer.respond_ok(id, result).await;
            }
            Err(err) => {
                let _ = peer
                    .respond_error(
                        id,
                        -32603,
                        "Internal error",
                        Some(json!({"details": err.to_string()})),
                    )
                    .await;
            }
        }
    }

    pub async fn handle_notification(&self, method: &str, params: Value) {
        if method == "session/update" {
            self.session_update(params).await;
        } else {
            self.emit(RunEvent::Raw {
                method: method.to_string(),
                payload: params,
            });
        }
    }

    async fn request_permission(&self, params: Value) -> Result<Value> {
        permission_flow::request_permission(
            params,
            &self.run_id,
            self.auto_allow,
            &self.permission_decisions,
            |event| self.emit(event),
        )
        .await
    }

    async fn session_update(&self, params: Value) {
        match map_session_update(&params) {
            MappedSessionUpdate::Event(event) => self.emit(event),
            MappedSessionUpdate::Tool(update) => self.tool_update(&update).await,
            MappedSessionUpdate::Ignored => {}
        }
    }

    async fn tool_update(&self, update: &Value) {
        let status = update
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let title = clean_tool_title(update.get("title").and_then(Value::as_str));
        let tool_call_id = extract_tool_call_id(update);
        let mut locations = extract_locations(update);
        let file_changes = extract_tool_file_changes(update, status.as_str());

        if let Some(tool_call_id) = &tool_call_id {
            if !locations.is_empty() {
                self.pending_tool_locations
                    .lock()
                    .await
                    .insert(tool_call_id.clone(), locations.clone());
            }
        }

        if status == "pending" && title.is_empty() {
            if let Some(tool_call_id) = &tool_call_id {
                self.pending_tool_locations
                    .lock()
                    .await
                    .insert(tool_call_id.clone(), locations);
                return;
            }
        }

        if locations.is_empty() {
            if let Some(tool_call_id) = &tool_call_id {
                if let Some(cached) = self
                    .pending_tool_locations
                    .lock()
                    .await
                    .get(tool_call_id)
                    .cloned()
                {
                    locations = cached;
                }
            }
        }

        if matches!(status.as_str(), "completed" | "failed") {
            if let Some(tool_call_id) = &tool_call_id {
                self.pending_tool_locations
                    .lock()
                    .await
                    .remove(tool_call_id);
            }
        }

        let label = if !title.is_empty() {
            title
        } else if let Some(tool_call_id) = &tool_call_id {
            format!("id={tool_call_id}")
        } else {
            String::new()
        };
        let signature = (
            tool_call_id.clone(),
            status.clone(),
            label.clone(),
            locations.clone(),
            file_changes.clone(),
        );
        {
            let mut last = self.last_tool_signature.lock().await;
            if last.as_ref() == Some(&signature) {
                return;
            }
            *last = Some(signature);
        }

        self.emit(RunEvent::Tool {
            tool_call_id,
            status,
            title: label,
            locations,
            file_changes,
        });
    }

    async fn read_text_file(&self, params: Value) -> Result<Value> {
        let path = string_param(&params, "path")?;
        let target = self.resolve_inside_workspace(path)?;
        let content =
            fs::read_to_string(&target).with_context(|| format!("reading {}", target.display()))?;
        let start = params
            .get("line")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .saturating_sub(1) as usize;
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v as usize);
        let selected = select_lines(&content, start, limit);
        Ok(json!({"content": selected}))
    }

    async fn write_text_file(&self, params: Value) -> Result<Value> {
        let path = string_param(&params, "path")?;
        let content = string_param(&params, "content")?;
        let target = self.resolve_inside_workspace(path)?;
        let file_change = build_write_file_change(&self.workspace, &target, content);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let write_result = fs::write(&target, content);
        let status = if write_result.is_ok() {
            "completed"
        } else {
            "failed"
        };
        let mut file_change = file_change;
        if let Err(err) = &write_result {
            file_change.status = ToolFileChangeStatus::Failed;
            file_change.message = Some(err.to_string());
        }
        self.emit(RunEvent::Tool {
            tool_call_id: None,
            status: status.to_string(),
            title: "fs.write_text_file".into(),
            locations: vec![target.display().to_string()],
            file_changes: vec![file_change],
        });
        write_result?;
        self.emit(RunEvent::FileSystem {
            operation: "write".into(),
            path: target.display().to_string(),
        });
        Ok(json!({}))
    }

    async fn create_terminal(&self, params: Value) -> Result<Value> {
        self.terminals
            .create(
                params,
                self.workspace.clone(),
                |path| self.resolve_inside_workspace(path),
                |event| self.emit(event),
            )
            .await
    }

    async fn terminal_output(&self, params: Value) -> Result<Value> {
        self.terminals.output(params).await
    }

    async fn wait_for_terminal_exit(&self, params: Value) -> Result<Value> {
        self.terminals
            .wait_for_exit(params, |event| self.emit(event))
            .await
    }

    async fn kill_terminal(&self, params: Value) -> Result<Value> {
        self.terminals.kill(params, |event| self.emit(event)).await
    }

    async fn release_terminal(&self, params: Value) -> Result<Value> {
        self.terminals.release(params).await
    }

    fn resolve_inside_workspace(&self, path: &str) -> Result<PathBuf> {
        let mut target = expand_tilde(path);
        if !target.is_absolute() {
            target = self.workspace.join(target);
        }
        let resolved = normalize_path(&target)?;
        if resolved != self.workspace && !resolved.starts_with(&self.workspace) {
            bail!("Path escapes workspace: {}", resolved.display());
        }
        Ok(resolved)
    }

    fn write_raw_rpc_message(&self, message: &Value) -> Result<()> {
        if let Some(parent) = self.raw_event_log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let entry = json!({
            "receivedAtMs": current_time_millis(),
            "runId": self.run_id,
            "message": message,
        });
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.raw_event_log_path)?;
        serde_json::to_writer(&mut file, &entry)?;
        file.write_all(b"\n")?;
        Ok(())
    }
}

fn build_write_file_change(
    workspace: &std::path::Path,
    target: &std::path::Path,
    content: &str,
) -> ToolFileChange {
    let path = target
        .strip_prefix(workspace)
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| target.display().to_string());
    let existing = fs::read_to_string(target);
    let kind = if target.exists() {
        ToolFileChangeKind::Modified
    } else {
        ToolFileChangeKind::Added
    };

    match existing {
        Ok(before) => {
            let diff = simple_unified_diff(&path, Some(&before), Some(content));
            let (diff, truncated) = truncate_for_tool_file_change(&diff);
            ToolFileChange {
                path,
                old_path: None,
                kind,
                status: ToolFileChangeStatus::Completed,
                diff: Some(diff),
                content: None,
                binary: false,
                truncated,
                message: None,
            }
        }
        Err(err) if target.exists() => ToolFileChange {
            path,
            old_path: None,
            kind,
            status: ToolFileChangeStatus::Unavailable,
            diff: None,
            content: None,
            binary: true,
            truncated: false,
            message: Some(format!("Text diff unavailable: {err}")),
        },
        Err(_) => {
            let diff = simple_unified_diff(&path, None, Some(content));
            let (diff, truncated) = truncate_for_tool_file_change(&diff);
            ToolFileChange {
                path,
                old_path: None,
                kind,
                status: ToolFileChangeStatus::Completed,
                diff: Some(diff),
                content: None,
                binary: false,
                truncated,
                message: None,
            }
        }
    }
}

fn extract_tool_file_changes(update: &Value, tool_status: &str) -> Vec<ToolFileChange> {
    let status = tool_status_to_file_change_status(tool_status);
    let candidates = [
        update.pointer("/fileChange"),
        update.pointer("/file_change"),
        update.pointer("/fileChanges"),
        update.pointer("/file_changes"),
        update.pointer("/changes"),
        update.pointer("/content"),
        update.pointer("/content/fileChange"),
        update.pointer("/content/file_change"),
        update.pointer("/content/fileChanges"),
        update.pointer("/content/file_changes"),
        update.pointer("/content/changes"),
    ];

    let changes = candidates
        .iter()
        .flatten()
        .flat_map(|candidate| {
            if let Some(changes) = candidate.as_array() {
                changes
                    .iter()
                    .filter_map(|change| map_tool_file_change(change, status.clone()))
                    .collect::<Vec<_>>()
            } else {
                map_tool_file_change(candidate, status.clone())
                    .into_iter()
                    .collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();

    dedupe_tool_file_changes(changes)
}

fn map_tool_file_change(
    change: &Value,
    default_status: ToolFileChangeStatus,
) -> Option<ToolFileChange> {
    let path = change
        .get("path")
        .or_else(|| change.get("filePath"))
        .or_else(|| change.get("file_path"))
        .or_else(|| change.get("fileName"))
        .or_else(|| change.get("file_name"))
        .or_else(|| change.get("file"))
        .and_then(Value::as_str)?
        .to_string();
    let patch = change.get("patch").and_then(Value::as_str);
    let text_diff = build_text_content_diff(&path, change);
    let diff = change
        .get("diff")
        .or_else(|| change.get("unifiedDiff"))
        .or_else(|| change.get("unified_diff"))
        .and_then(Value::as_str)
        .or(patch)
        .map(str::to_string)
        .filter(|diff| looks_like_unified_diff(diff))
        .or_else(|| text_diff.as_ref().map(|(diff, _)| diff.clone()));
    let content = change
        .get("content")
        .or_else(|| change.get("text"))
        .and_then(Value::as_str)
        .or_else(|| if diff.is_none() { patch } else { None })
        .map(str::to_string);
    let truncated = change
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || text_diff
            .as_ref()
            .map(|(_, truncated)| *truncated)
            .unwrap_or(false);

    Some(ToolFileChange {
        path,
        old_path: change
            .get("oldPath")
            .or_else(|| change.get("old_path"))
            .and_then(Value::as_str)
            .map(str::to_string),
        kind: parse_tool_file_change_kind(change),
        status: change
            .get("status")
            .and_then(Value::as_str)
            .map(tool_status_to_file_change_status)
            .unwrap_or(default_status),
        diff,
        content,
        binary: change
            .get("binary")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        truncated,
        message: change
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn build_text_content_diff(path: &str, change: &Value) -> Option<(String, bool)> {
    if change.get("type").and_then(Value::as_str) != Some("diff") {
        return None;
    }

    let before_field = nullable_string_field(change, "oldText")
        .or_else(|| nullable_string_field(change, "old_text"));
    let after_field = nullable_string_field(change, "newText")
        .or_else(|| nullable_string_field(change, "new_text"));

    if before_field.is_none() && after_field.is_none() {
        return None;
    }

    let diff = simple_unified_diff(path, before_field.flatten(), after_field.flatten());
    Some(truncate_for_tool_file_change(&diff))
}

fn nullable_string_field<'a>(value: &'a Value, key: &str) -> Option<Option<&'a str>> {
    match value.get(key) {
        Some(Value::String(text)) => Some(Some(text.as_str())),
        Some(Value::Null) => Some(None),
        _ => None,
    }
}

fn parse_tool_file_change_kind(change: &Value) -> ToolFileChangeKind {
    let kind = parse_file_change_kind(
        change
            .get("kind")
            .or_else(|| change.get("changeType"))
            .or_else(|| change.get("change_type"))
            .or_else(|| change.pointer("/_meta/kind"))
            .or_else(|| change.get("type"))
            .and_then(Value::as_str),
    );
    if kind != ToolFileChangeKind::Unknown {
        return kind;
    }

    if change.get("type").and_then(Value::as_str) != Some("diff") {
        return ToolFileChangeKind::Unknown;
    }

    let before = nullable_string_field(change, "oldText")
        .or_else(|| nullable_string_field(change, "old_text"));
    let after = nullable_string_field(change, "newText")
        .or_else(|| nullable_string_field(change, "new_text"));

    match (before, after) {
        (Some(None), Some(Some(_))) => ToolFileChangeKind::Added,
        (Some(Some(_)), Some(None)) => ToolFileChangeKind::Deleted,
        (Some(Some(_)), Some(Some(_))) => ToolFileChangeKind::Modified,
        _ => ToolFileChangeKind::Unknown,
    }
}

fn dedupe_tool_file_changes(changes: Vec<ToolFileChange>) -> Vec<ToolFileChange> {
    let mut deduped: Vec<ToolFileChange> = Vec::new();
    for change in changes {
        if deduped
            .iter()
            .any(|existing| existing.path == change.path && existing.kind == change.kind)
        {
            continue;
        }
        deduped.push(change);
    }
    deduped
}

fn looks_like_unified_diff(diff: &str) -> bool {
    diff.contains("@@") || diff.starts_with("--- ") || diff.starts_with("diff --git")
}

fn parse_file_change_kind(kind: Option<&str>) -> ToolFileChangeKind {
    match kind.unwrap_or("").to_ascii_lowercase().as_str() {
        "add" | "added" | "create" | "created" | "new" => ToolFileChangeKind::Added,
        "update" | "modify" | "modified" | "edit" | "edited" => ToolFileChangeKind::Modified,
        "delete" | "deleted" | "remove" | "removed" | "del" => ToolFileChangeKind::Deleted,
        "rename" | "renamed" => ToolFileChangeKind::Renamed,
        _ => ToolFileChangeKind::Unknown,
    }
}

fn tool_status_to_file_change_status(status: &str) -> ToolFileChangeStatus {
    match status {
        "completed" => ToolFileChangeStatus::Completed,
        "failed" => ToolFileChangeStatus::Failed,
        "unavailable" => ToolFileChangeStatus::Unavailable,
        _ => ToolFileChangeStatus::InProgress,
    }
}

pub fn lifecycle(status: LifecycleStatus, message: impl Into<String>) -> RunEvent {
    RunEvent::Lifecycle {
        status,
        message: message.into(),
    }
}

fn extract_tool_call_id(update: &Value) -> Option<String> {
    [
        "/toolCallId",
        "/tool_call_id",
        "/id",
        "/callId",
        "/toolCall/toolCallId",
        "/toolCall/tool_call_id",
        "/toolCall/id",
        "/content/toolCallId",
        "/content/tool_call_id",
        "/content/id",
    ]
    .iter()
    .find_map(|path| {
        update
            .pointer(path)
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn current_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        infrastructure::permission_broker::PermissionBroker, ports::event_sink::RunEventSink,
    };
    use std::sync::{Arc, Mutex as StdMutex};

    #[derive(Clone, Default)]
    struct CapturingSink {
        events: Arc<StdMutex<Vec<(String, RunEvent)>>>,
    }

    impl CapturingSink {
        fn events(&self) -> Vec<(String, RunEvent)> {
            self.events.lock().expect("events lock").clone()
        }
    }

    impl RunEventSink for CapturingSink {
        fn emit(&self, run_id: &str, event: RunEvent) {
            self.events
                .lock()
                .expect("events lock")
                .push((run_id.to_string(), event));
        }
    }

    #[tokio::test]
    async fn tool_call_update_preserves_matching_tool_call_id_and_locations() {
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            PathBuf::from("/tmp/workspace"),
            true,
            PermissionBroker::default(),
            sink.clone(),
        );
        let tool_call_id = "call_bDJJJUTgrC12AVAT23JZpXZu";
        let path = "/tmp/workspace/apps/desktop/package.json";

        client
            .tool_update(&json!({
                "sessionUpdate": "tool_call",
                "toolCallId": tool_call_id,
                "title": "Read package.json",
                "kind": "read",
                "status": "in_progress",
                "locations": [{"path": path}]
            }))
            .await;
        client
            .tool_update(&json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": tool_call_id,
                "status": "completed"
            }))
            .await;

        let events = sink.events();
        assert_eq!(events.len(), 2);

        match &events[0].1 {
            RunEvent::Tool {
                tool_call_id,
                status,
                title,
                locations,
                file_changes,
            } => {
                assert_eq!(
                    tool_call_id.as_deref(),
                    Some("call_bDJJJUTgrC12AVAT23JZpXZu")
                );
                assert_eq!(status, "in_progress");
                assert_eq!(title, "Read package.json");
                assert_eq!(locations, &[path.to_string()]);
                assert!(file_changes.is_empty());
            }
            other => panic!("unexpected event: {other:?}"),
        }

        match &events[1].1 {
            RunEvent::Tool {
                tool_call_id,
                status,
                title,
                locations,
                file_changes,
            } => {
                assert_eq!(
                    tool_call_id.as_deref(),
                    Some("call_bDJJJUTgrC12AVAT23JZpXZu")
                );
                assert_eq!(status, "completed");
                assert_eq!(title, "id=call_bDJJJUTgrC12AVAT23JZpXZu");
                assert_eq!(locations, &[path.to_string()]);
                assert!(file_changes.is_empty());
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn write_text_file_emits_added_file_change() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().canonicalize().expect("canonical tempdir");
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            workspace,
            true,
            PermissionBroker::default(),
            sink.clone(),
        );

        client
            .write_text_file(json!({"path": "new.txt", "content": "hello\n"}))
            .await
            .expect("write text file");

        let events = sink.events();
        match &events[0].1 {
            RunEvent::Tool { file_changes, .. } => {
                assert_eq!(file_changes.len(), 1);
                assert_eq!(file_changes[0].path, "new.txt");
                assert_eq!(file_changes[0].kind, ToolFileChangeKind::Added);
                assert_eq!(file_changes[0].status, ToolFileChangeStatus::Completed);
                assert!(
                    file_changes[0]
                        .diff
                        .as_deref()
                        .unwrap_or("")
                        .contains("+hello")
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn write_text_file_emits_modified_file_change() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().canonicalize().expect("canonical tempdir");
        fs::write(temp.path().join("existing.txt"), "old\n").expect("seed file");
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            workspace,
            true,
            PermissionBroker::default(),
            sink.clone(),
        );

        client
            .write_text_file(json!({"path": "existing.txt", "content": "new\n"}))
            .await
            .expect("write text file");

        let events = sink.events();
        match &events[0].1 {
            RunEvent::Tool { file_changes, .. } => {
                assert_eq!(file_changes.len(), 1);
                assert_eq!(file_changes[0].kind, ToolFileChangeKind::Modified);
                let diff = file_changes[0].diff.as_deref().unwrap_or("");
                assert!(diff.contains("-old"));
                assert!(diff.contains("+new"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn tool_update_maps_codex_style_file_changes() {
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            PathBuf::from("/tmp/workspace"),
            true,
            PermissionBroker::default(),
            sink.clone(),
        );

        client
            .tool_update(&json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "title": "Edit",
                "status": "completed",
                "changes": [{
                    "path": "src/app.ts",
                    "kind": "update",
                    "diff": "@@ -1 +1 @@\n-old\n+new"
                }]
            }))
            .await;

        let events = sink.events();
        match &events[0].1 {
            RunEvent::Tool { file_changes, .. } => {
                assert_eq!(file_changes.len(), 1);
                assert_eq!(file_changes[0].path, "src/app.ts");
                assert_eq!(file_changes[0].kind, ToolFileChangeKind::Modified);
                assert_eq!(file_changes[0].status, ToolFileChangeStatus::Completed);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn tool_update_maps_file_path_and_patch_file_changes() {
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            PathBuf::from("/tmp/workspace"),
            true,
            PermissionBroker::default(),
            sink.clone(),
        );

        client
            .tool_update(&json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "title": "Editing files",
                "status": "completed",
                "fileChanges": [{
                    "filePath": "src/app.ts",
                    "type": "edit",
                    "patch": "@@ -1 +1 @@\n-old\n+new"
                }]
            }))
            .await;

        let events = sink.events();
        match &events[0].1 {
            RunEvent::Tool { file_changes, .. } => {
                assert_eq!(file_changes.len(), 1);
                assert_eq!(file_changes[0].path, "src/app.ts");
                assert_eq!(file_changes[0].kind, ToolFileChangeKind::Modified);
                assert_eq!(
                    file_changes[0].diff.as_deref(),
                    Some("@@ -1 +1 @@\n-old\n+new")
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn tool_update_maps_singular_file_change() {
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            PathBuf::from("/tmp/workspace"),
            true,
            PermissionBroker::default(),
            sink.clone(),
        );

        client
            .tool_update(&json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "title": "Editing files",
                "status": "completed",
                "fileChange": {
                    "path": "src/app.ts",
                    "kind": "modified",
                    "diff": "@@ -1 +1 @@\n-old\n+new"
                }
            }))
            .await;

        let events = sink.events();
        match &events[0].1 {
            RunEvent::Tool { file_changes, .. } => {
                assert_eq!(file_changes.len(), 1);
                assert_eq!(file_changes[0].path, "src/app.ts");
                assert_eq!(file_changes[0].kind, ToolFileChangeKind::Modified);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn tool_update_maps_content_array_file_changes() {
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            PathBuf::from("/tmp/workspace"),
            true,
            PermissionBroker::default(),
            sink.clone(),
        );

        client
            .tool_update(&json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "tool-1",
                "title": "Editing files",
                "status": "completed",
                "content": [{
                    "file_path": "src/app.ts",
                    "change_type": "modified",
                    "unified_diff": "@@ -1 +1 @@\n-old\n+new"
                }]
            }))
            .await;

        let events = sink.events();
        match &events[0].1 {
            RunEvent::Tool { file_changes, .. } => {
                assert_eq!(file_changes.len(), 1);
                assert_eq!(file_changes[0].path, "src/app.ts");
                assert_eq!(file_changes[0].kind, ToolFileChangeKind::Modified);
                assert_eq!(file_changes[0].status, ToolFileChangeStatus::Completed);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn tool_update_maps_acp_diff_content_blocks() {
        let sink = CapturingSink::default();
        let client = AcpClient::new(
            "run-1".to_string(),
            PathBuf::from("/tmp/workspace"),
            true,
            PermissionBroker::default(),
            sink.clone(),
        );

        client
            .tool_update(&json!({
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-1",
                "title": "Editing files",
                "status": "completed",
                "content": [{
                    "type": "diff",
                    "path": "/tmp/workspace/src/lib.rs",
                    "oldText": null,
                    "newText": "pub mod app;\n",
                    "_meta": {"kind": "add"}
                }, {
                    "type": "diff",
                    "path": "/tmp/workspace/src/main.rs",
                    "oldText": "fn main() {}\n",
                    "newText": "fn main() {\n    println!(\"hi\");\n}\n"
                }]
            }))
            .await;

        let events = sink.events();
        match &events[0].1 {
            RunEvent::Tool { file_changes, .. } => {
                assert_eq!(file_changes.len(), 2);
                assert_eq!(file_changes[0].path, "/tmp/workspace/src/lib.rs");
                assert_eq!(file_changes[0].kind, ToolFileChangeKind::Added);
                let added_diff = file_changes[0].diff.as_deref().unwrap_or("");
                assert!(added_diff.contains("+++ b//tmp/workspace/src/lib.rs"));
                assert!(added_diff.contains("+pub mod app;"));

                assert_eq!(file_changes[1].kind, ToolFileChangeKind::Modified);
                let modified_diff = file_changes[1].diff.as_deref().unwrap_or("");
                assert!(modified_diff.contains("-fn main() {}"));
                assert!(modified_diff.contains("+    println!(\"hi\");"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
