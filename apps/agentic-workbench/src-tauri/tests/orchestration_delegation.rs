use std::{
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
};

use agentic_workbench_lib::{
    application::orchestration_service::{
        BindMainRunRequest, CreateChildTaskRequest, MainRunBindingState, OrchestrationService,
        ReportTaskRequest,
    },
    domain::agent_orchestration::{
        AgentRoleProfile, MAIN_AGENT_NODE_ID, OrchestrationError, OrchestrationSession,
        TaskReportType, TaskStatus,
    },
    ports::{
        orchestration_event_sink::{OrchestrationEvent, OrchestrationEventSink},
        orchestration_repository::OrchestrationRepository,
    },
};
use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    process::Command,
};

#[derive(Clone, Default)]
struct MemoryRepository(Arc<Mutex<Vec<OrchestrationSession>>>);

impl OrchestrationRepository for MemoryRepository {
    fn load_sessions(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn save_sessions(&self, sessions: &[OrchestrationSession]) -> Result<(), OrchestrationError> {
        *self.0.lock().unwrap() = sessions.to_vec();
        Ok(())
    }
}

#[derive(Clone, Default)]
struct NoopSink;

impl OrchestrationEventSink for NoopSink {
    fn emit(
        &self,
        _window_label: &str,
        _event: OrchestrationEvent,
    ) -> Result<(), OrchestrationError> {
        Ok(())
    }
}

#[test]
fn delegates_three_direct_children_and_collects_structured_results() {
    let service = OrchestrationService::new(MemoryRepository::default(), NoopSink);
    let workspace = service.bootstrap("/repo", "window-1", None).unwrap();
    let workspace = service
        .bind_main_run(
            "window-1",
            BindMainRunRequest {
                request_id: "bind-main".into(),
                panel_id: MAIN_AGENT_NODE_ID.into(),
                run_id: "run-main".into(),
                state: MainRunBindingState::Active,
                expected_revision: workspace.revision,
            },
        )
        .unwrap();
    let generation_id = workspace.active_coordinator_generation_id.unwrap();

    for (index, role) in ["Researcher", "Reviewer", "Tester"].into_iter().enumerate() {
        let outcome = service
            .create_child_task(
                "window-1",
                &generation_id,
                CreateChildTaskRequest {
                    request_id: format!("create-{index}"),
                    title: format!("{role} task"),
                    role: AgentRoleProfile::new(
                        role.to_lowercase(),
                        role,
                        "독립 관점 조사",
                        "구조화 결과",
                    )
                    .unwrap(),
                    objective: format!("{role} 관점으로 조사한다."),
                    constraints: vec!["read-only".into()],
                    expected_result: "summary와 findings".into(),
                    dependency_task_ids: vec![],
                    preferred_node_id: None,
                },
            )
            .unwrap();
        let run_id = format!("run-child-{index}");
        service
            .bind_child_run("window-1", &outcome.task_id, &outcome.node_id, &run_id)
            .unwrap();
        service
            .report_task(
                "window-1",
                ReportTaskRequest {
                    request_id: format!("result-{index}"),
                    task_id: outcome.task_id,
                    reporter_node_id: outcome.node_id,
                    reporter_run_id: run_id,
                    report_type: TaskReportType::Result,
                    progress_percent: Some(100),
                    summary: format!("{role} result"),
                    findings: vec![],
                    artifact_refs: vec![],
                    unresolved: vec![],
                    confidence: Some(0.9),
                },
            )
            .unwrap();
    }

    let snapshot = service.get_for_window("window-1").unwrap().unwrap();
    assert_eq!(snapshot.nodes.len(), 4);
    assert!(
        snapshot
            .nodes
            .iter()
            .filter(|node| node.id != MAIN_AGENT_NODE_ID)
            .all(|node| node.parent_node_id.as_deref() == Some(MAIN_AGENT_NODE_ID))
    );
    assert_eq!(
        snapshot
            .tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Completed)
            .count(),
        3
    );
    assert_eq!(snapshot.reports.len(), 3);
    assert_eq!(snapshot.coordinator_notifications.len(), 3);
    assert!(
        snapshot
            .coordinator_notifications
            .iter()
            .zip(snapshot.reports.iter())
            .all(|(notification, report)| notification.report_id == report.id)
    );
}

#[derive(Clone, Default)]
struct SmokeMcpState {
    calls: Arc<Mutex<Vec<Value>>>,
}

async fn smoke_mcp_handler(
    State(state): State<SmokeMcpState>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Json<Value> {
    assert_eq!(
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        Some("Bearer smoke-token")
    );
    state.calls.lock().unwrap().push(request.clone());
    Json(json!({
        "jsonrpc": "2.0",
        "id": request["id"],
        "result": {
            "structuredContent": {
                "accepted": true
            }
        }
    }))
}

async fn send_rpc(stdin: &mut tokio::process::ChildStdin, id: u64, method: &str, params: Value) {
    let request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    stdin
        .write_all(format!("{request}\n").as_bytes())
        .await
        .unwrap();
}

async fn read_response(
    stdout: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    expected_id: u64,
) -> Value {
    loop {
        let line = tokio::time::timeout(std::time::Duration::from_secs(5), stdout.next_line())
            .await
            .expect("smoke ACP response timed out")
            .expect("smoke ACP stdout failed")
            .expect("smoke ACP stdout closed");
        let message: Value = serde_json::from_str(&line).unwrap();
        if message["id"] == expected_id {
            return message;
        }
    }
}

fn smoke_agent_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/acp-orchestration-smoke-agent.mjs")
}

#[tokio::test]
async fn real_acp_smoke_worker_requests_parent_input_then_reports_result() {
    let state = SmokeMcpState::default();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(
        axum::serve(
            listener,
            Router::new()
                .route("/", post(smoke_mcp_handler))
                .with_state(state.clone()),
        )
        .into_future(),
    );

    let mut child = Command::new("node")
        .arg(smoke_agent_script())
        .env("AW_MCP_URL", format!("http://{address}/"))
        .env("AW_MCP_TOKEN", "smoke-token")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("real smoke ACP worker should start");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();

    send_rpc(&mut stdin, 1, "initialize", json!({})).await;
    assert_eq!(
        read_response(&mut stdout, 1).await["result"]["agentInfo"]["name"],
        "orchestration-smoke-agent"
    );
    send_rpc(&mut stdin, 2, "session/new", json!({})).await;
    let session = read_response(&mut stdout, 2).await["result"]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();

    send_rpc(
        &mut stdin,
        3,
        "session/prompt",
        json!({
            "sessionId": session,
            "prompt": "Role: Reviewer\n[input] 정책을 검토한다."
        }),
    )
    .await;
    assert_eq!(
        read_response(&mut stdout, 3).await["result"]["stopReason"],
        "end_turn"
    );

    send_rpc(
        &mut stdin,
        4,
        "session/prompt",
        json!({
            "sessionId": session,
            "prompt": "strict"
        }),
    )
    .await;
    assert_eq!(
        read_response(&mut stdout, 4).await["result"]["stopReason"],
        "end_turn"
    );

    let calls = state.calls.lock().unwrap().clone();
    let tool_names = calls
        .iter()
        .filter_map(|call| call["params"]["name"].as_str())
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"aw_request_parent_input"));
    assert!(tool_names.contains(&"aw_report_result"));
    assert_eq!(
        calls
            .iter()
            .find(|call| call["params"]["name"] == "aw_report_result")
            .unwrap()["params"]["arguments"]["summary"],
        "Reviewer가 부모 입력을 받아 완료한 결정적 구조화 결과입니다."
    );

    child.kill().await.unwrap();
    server.abort();
}
