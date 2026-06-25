use serde_json::Value;

use crate::domain::events::{PlanEntry, RunEvent};

/// Outcome of mapping a `session/update` JSON-RPC payload.
///
/// The ACP protocol mixes stateless updates (message chunks, plans,
/// usage) with stateful ones (tool call transitions that depend on
/// earlier tool events). The stateless mapping is captured here so it
/// can be unit-tested without the rest of the ACP client; stateful
/// tool updates are forwarded back to the caller as a raw `Value` for
/// session-aware handling.
pub enum MappedSessionUpdate {
    /// A ready-to-emit `RunEvent`.
    Event(RunEvent),
    /// A `tool_call` or `tool_call_update` payload; the caller tracks
    /// tool identity / locations / dedupe signature state.
    Tool(Value),
    /// Update contained no actionable data (e.g. an agent message
    /// chunk without text). The caller should emit nothing.
    Ignored,
}

/// Map a `session/update` params JSON into either a direct run event,
/// a tool payload that requires stateful handling, or a no-op.
pub fn map_session_update(params: &Value) -> MappedSessionUpdate {
    let Some(update) = params.get("update") else {
        return MappedSessionUpdate::Event(RunEvent::Raw {
            method: "session/update".into(),
            payload: params.clone(),
        });
    };

    let kind = update
        .get("sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("session/update");

    match kind {
        "agent_message_chunk" => update
            .pointer("/content/text")
            .and_then(Value::as_str)
            .map(|text| MappedSessionUpdate::Event(RunEvent::AgentMessage { text: text.into() }))
            .unwrap_or(MappedSessionUpdate::Ignored),
        "agent_thought_chunk" => update
            .pointer("/content/text")
            .and_then(Value::as_str)
            .map(|text| MappedSessionUpdate::Event(RunEvent::Thought { text: text.into() }))
            .unwrap_or(MappedSessionUpdate::Ignored),
        "plan" => {
            let entries = update
                .get("entries")
                .and_then(Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| PlanEntry {
                            status: entry
                                .get("status")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            content: entry
                                .get("content")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            MappedSessionUpdate::Event(RunEvent::Plan { entries })
        }
        "tool_call" | "tool_call_update" => MappedSessionUpdate::Tool(update.clone()),
        "usage_update" => {
            let used = update
                .get("used")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let size = update
                .get("size")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            MappedSessionUpdate::Event(RunEvent::Usage { used, size })
        }
        other => MappedSessionUpdate::Event(RunEvent::Raw {
            method: other.to_string(),
            payload: update.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expect_event(mapped: MappedSessionUpdate) -> RunEvent {
        match mapped {
            MappedSessionUpdate::Event(event) => event,
            MappedSessionUpdate::Tool(_) => panic!("expected Event, got Tool"),
            MappedSessionUpdate::Ignored => panic!("expected Event, got Ignored"),
        }
    }

    #[test]
    fn missing_update_field_falls_through_as_raw_session_update() {
        let params = json!({"sessionId": "abc"});
        let event = expect_event(map_session_update(&params));
        match event {
            RunEvent::Raw { method, payload } => {
                assert_eq!(method, "session/update");
                assert_eq!(payload, params);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn agent_message_chunk_with_text_maps_to_agent_message_event() {
        let params = json!({
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": {"text": "hello"}
            }
        });
        match expect_event(map_session_update(&params)) {
            RunEvent::AgentMessage { text } => assert_eq!(text, "hello"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn agent_message_chunk_without_text_is_ignored() {
        let params = json!({
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": {}
            }
        });
        assert!(matches!(
            map_session_update(&params),
            MappedSessionUpdate::Ignored
        ));
    }

    #[test]
    fn agent_thought_chunk_with_text_maps_to_thought_event() {
        let params = json!({
            "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": {"text": "planning"}
            }
        });
        match expect_event(map_session_update(&params)) {
            RunEvent::Thought { text } => assert_eq!(text, "planning"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn plan_maps_to_plan_event_with_entries() {
        let params = json!({
            "update": {
                "sessionUpdate": "plan",
                "entries": [
                    {"status": "pending", "content": "step 1"},
                    {"status": "completed", "content": "step 0"}
                ]
            }
        });
        match expect_event(map_session_update(&params)) {
            RunEvent::Plan { entries } => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].status, "pending");
                assert_eq!(entries[0].content, "step 1");
                assert_eq!(entries[1].status, "completed");
                assert_eq!(entries[1].content, "step 0");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn plan_without_entries_maps_to_empty_plan() {
        let params = json!({"update": {"sessionUpdate": "plan"}});
        match expect_event(map_session_update(&params)) {
            RunEvent::Plan { entries } => assert!(entries.is_empty()),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn usage_update_maps_to_usage_event() {
        let params = json!({
            "update": {
                "sessionUpdate": "usage_update",
                "used": 1200,
                "size": 16_000
            }
        });
        match expect_event(map_session_update(&params)) {
            RunEvent::Usage { used, size } => {
                assert_eq!(used, 1200);
                assert_eq!(size, 16_000);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn usage_update_with_missing_fields_uses_defaults() {
        let params = json!({"update": {"sessionUpdate": "usage_update"}});
        match expect_event(map_session_update(&params)) {
            RunEvent::Usage { used, size } => {
                assert_eq!(used, 0);
                assert_eq!(size, 0);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn tool_call_and_tool_call_update_require_stateful_handling() {
        let tool_call = json!({
            "update": {
                "sessionUpdate": "tool_call",
                "status": "pending",
                "title": "Read"
            }
        });
        match map_session_update(&tool_call) {
            MappedSessionUpdate::Tool(payload) => {
                assert_eq!(
                    payload.get("sessionUpdate").and_then(|v| v.as_str()),
                    Some("tool_call")
                );
            }
            other => panic!("unexpected mapping: {other:?}"),
        }

        let tool_call_update = json!({
            "update": {
                "sessionUpdate": "tool_call_update",
                "status": "completed"
            }
        });
        assert!(matches!(
            map_session_update(&tool_call_update),
            MappedSessionUpdate::Tool(_)
        ));
    }

    #[test]
    fn unknown_kind_falls_through_as_raw_event_preserving_kind() {
        let params = json!({
            "update": {
                "sessionUpdate": "mysterious_event",
                "extra": 42
            }
        });
        match expect_event(map_session_update(&params)) {
            RunEvent::Raw { method, payload } => {
                assert_eq!(method, "mysterious_event");
                assert_eq!(payload.get("extra").and_then(|v| v.as_i64()), Some(42));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn missing_session_update_kind_falls_back_to_session_update_marker() {
        let params = json!({"update": {}});
        match expect_event(map_session_update(&params)) {
            RunEvent::Raw { method, .. } => assert_eq!(method, "session/update"),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    impl std::fmt::Debug for MappedSessionUpdate {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MappedSessionUpdate::Event(event) => write!(f, "Event({event:?})"),
                MappedSessionUpdate::Tool(value) => write!(f, "Tool({value})"),
                MappedSessionUpdate::Ignored => write!(f, "Ignored"),
            }
        }
    }
}
