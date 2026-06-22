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
    domain::events::{LifecycleStatus, RunEvent},
    infrastructure::acp::{
        permission_flow,
        session_update_mapper::{MappedSessionUpdate, map_session_update},
        terminal::TerminalHandler,
        transport::RpcPeer,
        util::{
            clean_tool_title, expand_tilde, extract_locations, normalize_path, select_lines,
            string_param,
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
    last_tool_signature: Mutex<Option<(Option<String>, String, String, Vec<String>)>>,
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
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, content)?;
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
        infrastructure::permission_broker::PermissionBroker,
        ports::event_sink::RunEventSink,
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
            } => {
                assert_eq!(tool_call_id.as_deref(), Some("call_bDJJJUTgrC12AVAT23JZpXZu"));
                assert_eq!(status, "in_progress");
                assert_eq!(title, "Read package.json");
                assert_eq!(locations, &[path.to_string()]);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        match &events[1].1 {
            RunEvent::Tool {
                tool_call_id,
                status,
                title,
                locations,
            } => {
                assert_eq!(tool_call_id.as_deref(), Some("call_bDJJJUTgrC12AVAT23JZpXZu"));
                assert_eq!(status, "completed");
                assert_eq!(title, "id=call_bDJJJUTgrC12AVAT23JZpXZu");
                assert_eq!(locations, &[path.to_string()]);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
