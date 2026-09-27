//! 이벤트 계약(039): 스트림 종류, 스키마 registry, WebSocket 프레임.
//!
//! `system.describe.eventSchemas`, OpenAPI `EventBySchema`, TS `EventMap`이 모두 `EVENT_SCHEMAS`를 읽는다.
//! 분류(상태 복원용/알림용)는 ADR `crates/workbench-core/docs/adr/0003`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    fault::WorkbenchFault,
    principal::Scope,
    workbench::{EventEnvelope, GapNotice, StreamCursor},
};

/// 상태 복원용(보관·replay) 또는 알림용(보관 없음, 구독 시작 이후만).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum EventClass {
    State,
    Notification,
}

/// 스트림 식별자 `<kind>:<key>`의 kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum StreamKind {
    Run,
    Worktree,
    /// 2b 예약.
    Orchestration,
    /// 2b 예약.
    Exchange,
}

impl StreamKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StreamKind::Run => "run",
            StreamKind::Worktree => "worktree",
            StreamKind::Orchestration => "orchestration",
            StreamKind::Exchange => "exchange",
        }
    }

    /// 039에서 구독을 여는 kind인지.
    pub fn is_subscribable(self) -> bool {
        matches!(self, StreamKind::Run | StreamKind::Worktree)
    }

    pub fn class(self) -> EventClass {
        match self {
            StreamKind::Worktree => EventClass::Notification,
            StreamKind::Run | StreamKind::Orchestration | StreamKind::Exchange => EventClass::State,
        }
    }

    pub fn required_scope(self) -> Scope {
        match self {
            StreamKind::Run => Scope::RunRead,
            StreamKind::Worktree => Scope::WorktreeRead,
            // 2b에서 정한다. 구독이 열리기 전까지 쓰이지 않는다.
            StreamKind::Orchestration | StreamKind::Exchange => Scope::RunRead,
        }
    }

    pub fn stream_id(self, key: &str) -> String {
        format!("{}:{key}", self.as_str())
    }
}

/// `<kind>:<key>` 해석. key는 비어 있지 않아야 한다(worktree key의 `:`는 허용 — 첫 `:`에서만 자른다).
pub fn parse_stream_id(stream_id: &str) -> Option<(StreamKind, &str)> {
    let (kind, key) = stream_id.split_once(':')?;
    if key.is_empty() {
        return None;
    }
    let kind = match kind {
        "run" => StreamKind::Run,
        "worktree" => StreamKind::Worktree,
        "orchestration" => StreamKind::Orchestration,
        "exchange" => StreamKind::Exchange,
        _ => return None,
    };
    Some((kind, key))
}

/// 이벤트 스키마 한 종류의 정적 계약.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSchemaSpec {
    pub schema: &'static str,
    pub stream_kind: StreamKind,
}

impl EventSchemaSpec {
    pub fn class(&self) -> EventClass {
        self.stream_kind.class()
    }
}

pub const RUN_EVENT_V1: &str = "run.event.v1";
pub const WORKTREE_CHANGED_V1: &str = "worktree.changed.v1";
pub const ORCHESTRATION_WORKSPACE_UPDATED_V1: &str = "orchestration.workspaceUpdated.v1";
pub const EXCHANGE_REQUESTED_V1: &str = "exchange.requested.v1";
pub const EXCHANGE_STATUS_V1: &str = "exchange.status.v1";

/// 계약 순서. describe·OpenAPI가 이 순서를 따른다.
pub const EVENT_SCHEMAS: [EventSchemaSpec; 5] = [
    EventSchemaSpec {
        schema: RUN_EVENT_V1,
        stream_kind: StreamKind::Run,
    },
    EventSchemaSpec {
        schema: WORKTREE_CHANGED_V1,
        stream_kind: StreamKind::Worktree,
    },
    EventSchemaSpec {
        schema: ORCHESTRATION_WORKSPACE_UPDATED_V1,
        stream_kind: StreamKind::Orchestration,
    },
    EventSchemaSpec {
        schema: EXCHANGE_REQUESTED_V1,
        stream_kind: StreamKind::Exchange,
    },
    EventSchemaSpec {
        schema: EXCHANGE_STATUS_V1,
        stream_kind: StreamKind::Exchange,
    },
];

/// `system.describe.eventSchemas`의 항목. 구독 가능하고 principal에게 허용된 것만 나온다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventSchemaDescriptor {
    pub schema: String,
    pub stream_kind: StreamKind,
    pub class: EventClass,
    pub required_scopes: Vec<Scope>,
}

impl From<&EventSchemaSpec> for EventSchemaDescriptor {
    fn from(spec: &EventSchemaSpec) -> Self {
        Self {
            schema: spec.schema.to_owned(),
            stream_kind: spec.stream_kind,
            class: spec.class(),
            required_scopes: vec![spec.stream_kind.required_scope()],
        }
    }
}

/// 테스트 WebSocket(039)과 3단계 운영 WebSocket이 공유하는 프레임.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EventFrame {
    /// server → client, 연결 직후 한 번.
    #[serde(rename_all = "camelCase")]
    Hello {
        protocol_version: u16,
        epoch: String,
    },
    /// client → server, 첫 프레임 한 번.
    Subscribe {
        cursors: Vec<StreamCursor>,
    },
    Event {
        event: EventEnvelope,
    },
    Gap {
        #[serde(flatten)]
        gap: GapNotice,
    },
    /// 구독 거절. 뒤이어 연결을 닫는다.
    Fault {
        fault: WorkbenchFault,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::workbench::GapReason;

    #[test]
    fn stream_ids_parse_and_reject_unknown_or_empty() {
        assert_eq!(parse_stream_id("run:r1"), Some((StreamKind::Run, "r1")));
        assert_eq!(
            parse_stream_id("worktree:/a/b:c"),
            Some((StreamKind::Worktree, "/a/b:c"))
        );
        assert_eq!(parse_stream_id("run:"), None);
        assert_eq!(parse_stream_id("nope:x"), None);
        assert_eq!(parse_stream_id("run"), None);
        assert!(StreamKind::Run.is_subscribable());
        assert!(!StreamKind::Orchestration.is_subscribable());
    }

    #[test]
    fn schemas_classify_and_frames_are_tagged() {
        let worktree = EVENT_SCHEMAS
            .iter()
            .find(|spec| spec.schema == WORKTREE_CHANGED_V1)
            .unwrap();
        assert_eq!(worktree.class(), EventClass::Notification);
        let gap = EventFrame::Gap {
            gap: GapNotice {
                stream_id: "run:r1".into(),
                epoch: "e".into(),
                reason: GapReason::Evicted,
                first_sequence: None,
                last_sequence: None,
            },
        };
        assert_eq!(
            serde_json::to_value(&gap).unwrap(),
            json!({"type": "gap", "streamId": "run:r1", "epoch": "e", "reason": "evicted"})
        );
        let hello: EventFrame =
            serde_json::from_value(json!({"type": "hello", "protocolVersion": 1, "epoch": "e"}))
                .unwrap();
        assert_eq!(
            hello,
            EventFrame::Hello {
                protocol_version: 1,
                epoch: "e".into()
            }
        );
    }
}
