use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEventEnvelope {
    pub run_id: String,
    pub event: RunEvent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum RunEvent {
    Lifecycle {
        status: LifecycleStatus,
        message: String,
    },
    AgentMessage {
        text: String,
    },
    Thought {
        text: String,
    },
    Plan {
        entries: Vec<PlanEntry>,
    },
    Tool {
        tool_call_id: Option<String>,
        status: String,
        title: String,
        locations: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        file_changes: Vec<ToolFileChange>,
    },
    Usage {
        used: i64,
        size: i64,
    },
    SessionInfo {
        thread_status: Option<String>,
        title: Option<String>,
        updated_at: Option<String>,
    },
    Permission {
        permission_id: Option<String>,
        title: String,
        input: Option<Value>,
        options: Vec<PermissionOption>,
        selected: Option<String>,
        requires_response: bool,
    },
    FileSystem {
        operation: String,
        path: String,
    },
    Terminal {
        operation: String,
        terminal_id: Option<String>,
        message: String,
    },
    Diagnostic {
        message: String,
    },
    RalphLoop {
        iteration: usize,
        max_iterations: usize,
        status: RalphLoopStatus,
    },
    Raw {
        method: String,
        payload: Value,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleStatus {
    Started,
    Initialized,
    SessionCreated,
    PromptSent,
    PromptCompleted,
    SteerPending,
    SteerAccepted,
    SteerRejected,
    Cancelled,
    Completed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RalphLoopStatus {
    Started,
    Completed,
    Failed,
    Stopped,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    pub status: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolFileChange {
    pub path: String,
    pub old_path: Option<String>,
    pub kind: ToolFileChangeKind,
    pub status: ToolFileChangeStatus,
    pub diff: Option<String>,
    pub content: Option<String>,
    pub binary: bool,
    pub truncated: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ToolFileChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ToolFileChangeStatus {
    InProgress,
    Completed,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    pub name: String,
    pub kind: String,
    pub option_id: String,
}

#[cfg(test)]
mod tests {
    use super::RunEvent;
    use serde_json::json;

    #[test]
    fn tool_event_serializes_tool_call_id_as_camel_case() {
        let event = RunEvent::Tool {
            tool_call_id: Some("call_123".to_string()),
            status: "completed".to_string(),
            title: "Read package.json".to_string(),
            locations: vec!["/tmp/package.json".to_string()],
            file_changes: Vec::new(),
        };

        let value = serde_json::to_value(event).expect("serialize event");

        assert_eq!(
            value,
            json!({
                "type": "tool",
                "toolCallId": "call_123",
                "status": "completed",
                "title": "Read package.json",
                "locations": ["/tmp/package.json"]
            })
        );
    }

    #[test]
    fn tool_event_serializes_file_changes_as_camel_case() {
        let event = RunEvent::Tool {
            tool_call_id: Some("call_123".to_string()),
            status: "completed".to_string(),
            title: "Write app.ts".to_string(),
            locations: vec!["app.ts".to_string()],
            file_changes: vec![super::ToolFileChange {
                path: "app.ts".to_string(),
                old_path: None,
                kind: super::ToolFileChangeKind::Modified,
                status: super::ToolFileChangeStatus::Completed,
                diff: Some("@@ -1 +1 @@\n-old\n+new".to_string()),
                content: None,
                binary: false,
                truncated: false,
                message: None,
            }],
        };

        let value = serde_json::to_value(event).expect("serialize event");

        assert_eq!(
            value,
            json!({
                "type": "tool",
                "toolCallId": "call_123",
                "status": "completed",
                "title": "Write app.ts",
                "locations": ["app.ts"],
                "fileChanges": [{
                    "path": "app.ts",
                    "oldPath": null,
                    "kind": "modified",
                    "status": "completed",
                    "diff": "@@ -1 +1 @@\n-old\n+new",
                    "content": null,
                    "binary": false,
                    "truncated": false,
                    "message": null
                }]
            })
        );
    }

    #[test]
    fn ralph_loop_event_serializes_iteration_fields_as_camel_case() {
        let event = RunEvent::RalphLoop {
            iteration: 2,
            max_iterations: 5,
            status: super::RalphLoopStatus::Completed,
        };

        let value = serde_json::to_value(event).expect("serialize event");

        assert_eq!(
            value,
            json!({
                "type": "ralphLoop",
                "iteration": 2,
                "maxIterations": 5,
                "status": "completed"
            })
        );
    }
}
