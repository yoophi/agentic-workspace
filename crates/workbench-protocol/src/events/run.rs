//! `run.event.v1` 본문: acp-agent-core `domain::events::RunEvent`의 미러(039, research R8). serde 속성은 원본과 같다 —
//! `type` 태그, variant 이름·필드 camelCase, `Tool.fileChanges`는 비면 생략. hub는 원본 `RunEvent`를 그대로
//! 직렬화해 싣고, 이 타입은 스키마 생성과 wire 동일성 테스트(core `application/event_dto.rs`)에만 쓴다.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RunEventDto {
    #[serde(rename_all = "camelCase")]
    Lifecycle {
        status: LifecycleStatusDto,
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    AgentMessage { text: String },
    #[serde(rename_all = "camelCase")]
    Thought { text: String },
    #[serde(rename_all = "camelCase")]
    Plan { entries: Vec<PlanEntryDto> },
    #[serde(rename_all = "camelCase")]
    Tool {
        tool_call_id: Option<String>,
        status: String,
        title: String,
        locations: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        file_changes: Vec<ToolFileChangeDto>,
    },
    #[serde(rename_all = "camelCase")]
    Usage { used: i64, size: i64 },
    #[serde(rename_all = "camelCase")]
    SessionInfo {
        thread_status: Option<String>,
        title: Option<String>,
        updated_at: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Permission {
        permission_id: Option<String>,
        title: String,
        #[schema(value_type = Option<Object>)]
        input: Option<Value>,
        options: Vec<PermissionOptionDto>,
        selected: Option<String>,
        requires_response: bool,
    },
    #[serde(rename_all = "camelCase")]
    FileSystem { operation: String, path: String },
    #[serde(rename_all = "camelCase")]
    Terminal {
        operation: String,
        terminal_id: Option<String>,
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    Diagnostic { message: String },
    #[serde(rename_all = "camelCase")]
    RalphLoop {
        iteration: u64,
        max_iterations: u64,
        status: RalphLoopStatusDto,
    },
    #[serde(rename_all = "camelCase")]
    Raw {
        method: String,
        #[schema(value_type = Object)]
        payload: Value,
    },
    #[serde(rename_all = "camelCase")]
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleStatusDto {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum RalphLoopStatusDto {
    Started,
    Completed,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntryDto {
    pub status: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolFileChangeDto {
    pub path: String,
    pub old_path: Option<String>,
    pub kind: ToolFileChangeKindDto,
    pub status: ToolFileChangeStatusDto,
    pub diff: Option<String>,
    pub content: Option<String>,
    pub binary: bool,
    pub truncated: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ToolFileChangeKindDto {
    Added,
    Modified,
    Deleted,
    Renamed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ToolFileChangeStatusDto {
    InProgress,
    Completed,
    Failed,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOptionDto {
    pub name: String,
    pub kind: String,
    pub option_id: String,
}
