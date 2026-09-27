//! 이벤트 본문 wire 동일성(039, research R8). hub는 원본 타입을 serde로 직렬화해 싣는다 — protocol DTO는 스키마
//! 생성용 미러이므로, 원본 JSON이 DTO로 읽히고 다시 **같은 JSON**으로 쓰이는지 모든 variant에 대해 고정한다.

#[cfg(test)]
mod tests {
    use acp_agent_core::domain::events::{
        LifecycleStatus, PermissionOption, PlanEntry, RalphLoopStatus, RunEvent, ToolFileChange,
        ToolFileChangeKind, ToolFileChangeStatus,
    };
    use serde_json::json;
    use workbench_protocol::events::run::RunEventDto;

    fn assert_roundtrip(label: &str, event: &RunEvent) {
        let domain = serde_json::to_value(event).expect("domain serializes");
        let dto: RunEventDto = serde_json::from_value(domain.clone()).unwrap_or_else(|error| {
            panic!("{label}: dto cannot read domain JSON: {error}\n{domain}")
        });
        let back = serde_json::to_value(&dto).expect("dto serializes");
        assert_eq!(back, domain, "{label}: wire mismatch");
    }

    fn file_change(kind: ToolFileChangeKind, status: ToolFileChangeStatus) -> ToolFileChange {
        ToolFileChange {
            path: "a.ts".into(),
            old_path: Some("b.ts".into()),
            kind,
            status,
            diff: Some("@@".into()),
            content: None,
            binary: false,
            truncated: true,
            message: Some("m".into()),
        }
    }

    #[test]
    fn run_event_wire_parity_for_every_variant() {
        for status in [
            LifecycleStatus::Started,
            LifecycleStatus::Initialized,
            LifecycleStatus::SessionCreated,
            LifecycleStatus::PromptSent,
            LifecycleStatus::PromptCompleted,
            LifecycleStatus::SteerPending,
            LifecycleStatus::SteerAccepted,
            LifecycleStatus::SteerRejected,
            LifecycleStatus::Cancelled,
            LifecycleStatus::Completed,
        ] {
            assert_roundtrip(
                "lifecycle",
                &RunEvent::Lifecycle {
                    status,
                    message: "m".into(),
                },
            );
        }
        let events = vec![
            RunEvent::AgentMessage { text: "hi".into() },
            RunEvent::Thought { text: "t".into() },
            RunEvent::Plan {
                entries: vec![PlanEntry {
                    status: "pending".into(),
                    content: "c".into(),
                }],
            },
            RunEvent::Tool {
                tool_call_id: None,
                status: "running".into(),
                title: "t".into(),
                locations: vec![],
                file_changes: vec![],
            },
            RunEvent::Tool {
                tool_call_id: Some("call".into()),
                status: "completed".into(),
                title: "t".into(),
                locations: vec!["a.ts".into()],
                file_changes: vec![
                    file_change(ToolFileChangeKind::Added, ToolFileChangeStatus::InProgress),
                    file_change(
                        ToolFileChangeKind::Modified,
                        ToolFileChangeStatus::Completed,
                    ),
                    file_change(ToolFileChangeKind::Deleted, ToolFileChangeStatus::Failed),
                    file_change(
                        ToolFileChangeKind::Renamed,
                        ToolFileChangeStatus::Unavailable,
                    ),
                    file_change(ToolFileChangeKind::Unknown, ToolFileChangeStatus::Completed),
                ],
            },
            RunEvent::Usage { used: 10, size: -1 },
            RunEvent::SessionInfo {
                thread_status: None,
                title: Some("t".into()),
                updated_at: None,
            },
            RunEvent::Permission {
                permission_id: Some("p".into()),
                title: "allow?".into(),
                input: Some(json!({"cmd": "ls", "n": [1, 2]})),
                options: vec![PermissionOption {
                    name: "Allow".into(),
                    kind: "allow_once".into(),
                    option_id: "o1".into(),
                }],
                selected: None,
                requires_response: true,
            },
            RunEvent::Permission {
                permission_id: None,
                title: "t".into(),
                input: None,
                options: vec![],
                selected: Some("o1".into()),
                requires_response: false,
            },
            RunEvent::FileSystem {
                operation: "read".into(),
                path: "/a".into(),
            },
            RunEvent::Terminal {
                operation: "output".into(),
                terminal_id: Some("t1".into()),
                message: "x".into(),
            },
            RunEvent::Diagnostic {
                message: "stderr".into(),
            },
            RunEvent::Raw {
                method: "m".into(),
                payload: json!({"a": null}),
            },
            RunEvent::Error {
                message: "boom".into(),
            },
        ];
        for event in &events {
            assert_roundtrip("event", event);
        }
        for status in [
            RalphLoopStatus::Started,
            RalphLoopStatus::Completed,
            RalphLoopStatus::Failed,
            RalphLoopStatus::Stopped,
        ] {
            assert_roundtrip(
                "ralph",
                &RunEvent::RalphLoop {
                    iteration: 2,
                    max_iterations: 5,
                    status,
                },
            );
        }
    }

    #[test]
    fn worktree_changed_wire_parity_for_every_kind() {
        use crate::infrastructure::fs::worktree_watcher::{
            WorktreeChangeKind, WorktreeChangedEvent,
        };
        use workbench_protocol::events::worktree::WorktreeChangedDto;

        for kind in [WorktreeChangeKind::File, WorktreeChangeKind::Git] {
            let domain = serde_json::to_value(WorktreeChangedEvent {
                working_directory: "/repo".into(),
                changed_path: "/repo/src/a.ts".into(),
                kind,
            })
            .expect("domain serializes");
            let dto: WorktreeChangedDto =
                serde_json::from_value(domain.clone()).expect("dto reads domain JSON");
            assert_eq!(serde_json::to_value(&dto).unwrap(), domain, "{kind:?}");
        }
    }
}
