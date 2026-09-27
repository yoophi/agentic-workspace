//! 044 T042(research R7 검증 3·4, R14): **실제 경로 wait-stop 시험**. 실제 host 조립(런타임 + HTTP 어댑터 + MCP)과
//! host 감시 루프(`monitor::run_until_stopped`)를 쓰고, 정지 요청·권한 응답·교환 전달·도구 호출은 HTTP(`/v1/calls`,
//! `/mcp`)로 보낸다. 각 경우 `server.stop{wait}`을 요청한 뒤 실제 경로로 작업을 끝내고 서버가 `stopping`에 드는지 본다.
//!
//! - (a) 권한 대기 run(가짜 ACP agent) → 권한 응답(C) → turn 완료 → **세션은 살아 있어도** 멈춤.
//! - (b) 대상 run 바쁨(가짜 ACP agent, turn gate) → 교환 요청 → 확인이 전송보다 먼저 → turn 뒤 `run.sendPrompt`
//!   (`continuation`, K) → 엔진 대기열로 전달·소비 → 멈춤. 데스크톱 임대가 있어 미소비 교환이 활동 작업이다.
//! - (c) 자식 task 보고·결과(MCP) → 알림 전달기가 coordinator에 전달 → task 종결 → 멈춤.
//! - (d) 동시 상한 1, task 둘 → 첫 task 결과 → coordinator가 둘째를 MCP로 배정(K) → 결과 → 멈춤.
//! - (e) 자식이 바쁠 때(권한 대기) coordinator가 보낸 대기 자식 명령(queue) → 비우기 뒤 전달 → 자식 결과 → 멈춤.
//!
//! (c)–(e)는 가짜 **엔진**(`ScriptedRunEngine`)을 쓴다: 가짜 ACP agent는 MCP 도구를 부르지 못한다. 대신 시험이 run에
//! 주입된 MCP 토큰(엔진이 받은 시작 요청의 환경)으로 실제 MCP HTTP 끝점을 부른다 — agent가 하는 호출과 같은 경로다.
//!
//! 대조 변이(구현 증거 기록): 해당 C·K를 N으로 바꾸거나 (a)를 세션 수로 세면 각 시험이 [`STOP_BOUND`] 안에 멈추지 않아
//! 실패한다. "멈춤"은 상한 있는 조건 대기이고, "멈추지 않음"은 상한 동안의 단정이다(고정 sleep 동기화 없음).

use std::{path::Path, sync::Arc, time::Duration};

use serde_json::{Value, json};
use workbench_core::{
    application::{
        server_control::ServerControl, work_gate::GateState, workbench_runtime::RuntimeAdapters,
    },
    testing::scripted_run_engine::{RunScript, ScriptedRunEngine},
};
use workbench_host::{
    assembly::{HostAssembly, HostOptions, assemble},
    lifecycle::{
        calls::{CallError, call},
        client::request,
        identity::OwnerIdentity,
        monitor::{MonitorOptions, run_until_stopped},
    },
    mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV},
};

/// 작업이 끝난 뒤 멈추기까지의 상한.
const STOP_BOUND: Duration = Duration::from_secs(15);
/// "아직 멈추지 않음"을 단정하는 구간.
const NOT_YET: Duration = Duration::from_millis(600);
const DEADLINE: Duration = Duration::from_secs(20);

fn repo_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

struct Server {
    rt: tokio::runtime::Runtime,
    host: Option<HostAssembly>,
    control: Arc<ServerControl>,
    base: String,
    owner: String,
    engine: Option<Arc<ScriptedRunEngine>>,
    dir: tempfile::TempDir,
    work: String,
}

impl Server {
    fn start(configure: impl FnOnce(&mut RuntimeAdapters), engine: Option<RunScript>) -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let work = std::fs::canonicalize(work)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut adapters = RuntimeAdapters::production();
        let engine = engine.map(|script| Arc::new(ScriptedRunEngine::new(script)));
        if let Some(engine) = &engine {
            adapters.run_engine = Some(engine.clone());
        }
        configure(&mut adapters);
        let identity = OwnerIdentity::generate();
        let mut options = HostOptions::new(
            dir.path().join("data"),
            adapters,
            "test",
            rt.handle().clone(),
        );
        options.owner = Some(identity.clone());
        let host = assemble(options).expect("assembly");
        let control = Arc::clone(host.runtime.server_control());
        rt.spawn(run_until_stopped(
            Arc::clone(&control),
            MonitorOptions {
                idle_timeout: Duration::from_secs(3600),
                tick: Duration::from_millis(20),
            },
        ));
        let base = host.http.as_ref().unwrap().base_url().to_owned();
        Self {
            rt,
            host: Some(host),
            control,
            base,
            owner: identity.token().to_owned(),
            engine,
            dir,
            work,
        }
    }

    fn try_owner(&self, operation: &str, input: Value) -> Result<Value, CallError> {
        let command = !matches!(
            operation,
            "server.status" | "run.replay" | "bench.list" | "orchestration.get"
        );
        call(&self.base, &self.owner, None, operation, input, command)
    }

    fn owner(&self, operation: &str, input: Value) -> Value {
        self.try_owner(operation, input)
            .unwrap_or_else(|error| panic!("{operation}: {error:?}"))
    }

    fn open_bench(&self) -> String {
        self.owner("bench.open", json!({ "workingDirectory": self.work }))["benchId"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn wait_stop(&self) {
        let draining = self.owner("server.stop", json!({ "mode": "wait" }));
        assert_eq!(draining["state"], "drainingWait", "{draining}");
    }

    fn stopped(&self) -> bool {
        *self.control.stopped().borrow()
    }

    /// 상한 안에 `stopping`에 드는지(조건 대기).
    fn stops_within(&self, bound: Duration) -> bool {
        let until = std::time::Instant::now() + bound;
        while std::time::Instant::now() < until {
            if self.stopped() {
                assert_eq!(self.control.work_gate().state(), GateState::Stopping);
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn assert_not_stopping(&self, why: &str) {
        assert!(!self.stops_within(NOT_YET), "stopped although {why}");
    }

    fn wait_until(&self, what: &str, mut ready: impl FnMut() -> bool) {
        let until = std::time::Instant::now() + DEADLINE;
        while std::time::Instant::now() < until {
            if ready() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for {what}");
    }

    fn busy_runs(&self) -> u64 {
        self.owner("server.status", json!({}))["activeWork"]["busyRuns"]
            .as_u64()
            .unwrap()
    }

    /// run에 주입된 MCP 토큰(엔진이 받은 시작 요청의 환경).
    fn mcp_token(&self, run: &str) -> String {
        let engine = self.engine.as_ref().expect("scripted engine");
        let requests = engine.start_requests.lock().unwrap();
        requests
            .iter()
            .filter_map(|request| request.agent_env.as_ref())
            .find(|env| env.get(AW_MCP_RUN_ID_ENV).map(String::as_str) == Some(run))
            .and_then(|env| env.get(AW_MCP_TOKEN_ENV).cloned())
            .unwrap_or_else(|| panic!("no MCP token for {run}"))
    }

    /// 실제 MCP HTTP 끝점으로 도구를 부른다. 성공이면 구조화 결과, 도구 실패면 `Err(결과)`.
    fn tool(&self, run: &str, name: &str, arguments: Value) -> Result<Value, Value> {
        let mcp = self.host.as_ref().unwrap().mcp.base_url().to_owned();
        let base = mcp.strip_suffix("/mcp").unwrap();
        let (status, body) = request(
            base,
            "POST",
            "/mcp",
            Some(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": name, "arguments": arguments } })),
            Some(&self.mcp_token(run)),
        )
        .expect("MCP reachable");
        assert_eq!(status, 200, "{body}");
        let result = body["result"].clone();
        if result["isError"] == json!(false) {
            Ok(result["structuredContent"].clone())
        } else {
            Err(result)
        }
    }

    fn start_run(&self, bench: &str, run: &str, extra: Value) {
        let mut request = json!({ "goal": "g", "agentId": "codex", "runId": run });
        if let (Some(request), Some(extra)) = (request.as_object_mut(), extra.as_object()) {
            request.extend(extra.clone());
        }
        self.owner("run.start", json!({ "benchId": bench, "request": request }));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(host) = self.host.take() {
            self.rt.block_on(host.shutdown());
        }
    }
}

fn fake_agent(log: &Path, extra: &str) -> String {
    format!(
        "python3 {} --log {} {extra}",
        repo_path("crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py")
            .display(),
        log.display()
    )
}

/// replay에서 아직 응답하지 않은 권한 요청 id를 찾는다.
fn permission_id(server: &Server, bench: &str, run: &str) -> Option<String> {
    let replay = server.owner(
        "run.replay",
        json!({ "benchId": bench, "runId": run, "afterSequence": 0 }),
    );
    replay["events"].as_array()?.iter().rev().find_map(|item| {
        let event = &item["event"];
        (event["type"] == "permission" && event["requiresResponse"] == true)
            .then(|| event["permissionId"].as_str().map(str::to_owned))
            .flatten()
    })
}

// (a) --------------------------------------------------------------------------------------------------------------

#[test]
fn a_permission_wait_ends_by_the_response_and_the_server_stops_with_the_session_alive() {
    let server = Server::start(|_| {}, None);
    let bench = server.open_bench();
    let log = server.dir.path().join("agent-a.log");
    server.start_run(
        &bench,
        "r1",
        json!({ "agentId": "fake-acp", "agentCommand": fake_agent(&log, ""), "cwd": server.work }),
    );
    let mut permission = None;
    server.wait_until("the permission request", || {
        permission = permission_id(&server, &bench, "r1");
        permission.is_some()
    });
    assert_eq!(server.busy_runs(), 1, "a permission wait is a busy run");
    server.wait_stop();
    server.assert_not_stopping("the run waits for a permission response");

    // 권한 응답(C)은 비우기 중에도 받는다.
    server.owner(
        "run.respondPermission",
        json!({ "benchId": bench, "runId": "r1", "permissionId": permission, "optionId": "allow" }),
    );
    assert!(
        server.stops_within(STOP_BOUND),
        "the server did not stop after the turn ended"
    );
    let running = server
        .rt
        .block_on(server.control.benches().engine.active_owner_of("r1"));
    assert_eq!(
        running.as_deref(),
        Some(bench.as_str()),
        "the idle session was still alive when the server stopped"
    );
}

// (b) --------------------------------------------------------------------------------------------------------------

#[test]
fn an_exchange_acknowledged_before_the_send_is_delivered_after_the_turn_and_the_server_stops() {
    let server = Server::start(|_| {}, None);
    server.owner(
        "lease.acquire",
        json!({ "clientKind": "desktop", "clientId": "app" }),
    );
    let bench = server.open_bench();
    let gate = server.dir.path().join("r2-end-turn");
    let log1 = server.dir.path().join("agent-r1.log");
    let log2 = server.dir.path().join("agent-r2.log");
    let agent = |log: &Path, extra: &str| json!({ "agentId": "fake-acp", "agentCommand": fake_agent(log, extra), "cwd": server.work, "autoAllow": true });
    server.start_run(&bench, "r1", agent(&log1, ""));
    server.start_run(
        &bench,
        "r2",
        agent(&log2, &format!("--end-turn-gate {}", gate.display())),
    );
    server.wait_until("r2 busy in its first turn", || {
        server.control.work_gate().busy_run_count("r2") > 0
    });
    server.owner(
        "exchange.syncWorkspace",
        json!({ "benchId": bench, "request": {
        "worktreePath": server.work, "revision": 1, "focusedPanelId": "main",
        "panels": [
            {"panelId": "main", "title": "Main", "runId": "r1", "status": "running"},
            {"panelId": "extra", "title": "Extra", "runId": "r2", "status": "running"}
        ]}}),
    );
    server.owner(
        "exchange.send",
        json!({ "benchId": bench, "request": {
            "requestId": "q1", "sourcePanelId": "main", "sourceRunId": "r1",
            "targetPanelId": "extra", "targetRunId": "r2",
            "message": "hello peer", "delivery": "queue"}}),
    );
    // 043 원장처럼 확인이 전송보다 먼저다.
    server.owner(
        "exchange.acknowledge",
        json!({ "benchId": bench, "request": {
            "requestId": "q1", "targetPanelId": "extra", "outcome": "delivered", "reason": null}}),
    );
    server.wait_stop();
    server.assert_not_stopping("r2 is busy and the exchange is not delivered");

    std::fs::write(&gate, b"").unwrap();
    server.wait_until("r2 to finish its turn", || {
        server.control.work_gate().busy_run_count("r2") == 0
    });
    server.assert_not_stopping("the exchange prompt is not consumed yet (desktop lease held)");
    // turn 뒤 패널 대기열이 continuation으로 보낸다(K). 결과는 멈춤 판정 뒤에 단정한다 — 대조 변이(N)에서는 전달이
    // 거절되어 "상한 안에 멈추지 않음"으로 실패해야 한다.
    // 043 계약: 전달 키는 `exchange-delivery:<requestId>`.
    let delivered = request(
        &server.base,
        "POST",
        "/v1/calls",
        Some(&json!({
            "protocolVersion": workbench_protocol::PROTOCOL_VERSION,
            "operation": "run.sendPrompt",
            "requestId": "req-delivery-q1",
            "idempotencyKey": "exchange-delivery:q1",
            "input": { "benchId": bench, "runId": "r2", "prompt": "hello peer",
                "continuation": { "exchangeRequestId": "q1" } },
        })),
        Some(&server.owner),
    )
    .map(|(status, body)| (status, body["code"].clone()));
    assert!(
        server.stops_within(STOP_BOUND),
        "the server did not stop after the exchange delivery ({delivered:?})"
    );
    assert_eq!(
        delivered.as_ref().map(|(status, _)| *status),
        Ok(200),
        "{delivered:?}"
    );
    let lines = std::fs::read_to_string(&log2).unwrap_or_default();
    assert!(
        lines.contains("hello peer"),
        "the exchange prompt reached the agent: {lines}"
    );
}

// (c)–(e): 가짜 엔진 + 실제 MCP HTTP ----------------------------------------------------------------------------------

fn git_init(dir: &str) {
    for args in [
        vec!["init", "-q"],
        vec![
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    ] {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }
}

/// coordinator(`coord`)가 묶인 orchestration 작업대.
fn coordinator(server: &Server) -> String {
    git_init(&server.work);
    let bench = server.open_bench();
    let session = server.owner(
        "orchestration.bootstrap",
        json!({ "benchId": bench, "worktreePath": server.work }),
    );
    server.start_run(&bench, "coord", json!({}));
    server.owner(
        "orchestration.bindCoordinator",
        json!({ "benchId": bench, "request": {
            "requestId": "bind-1", "panelId": "main-agent-run", "runId": "coord",
            "state": "active", "expectedRevision": session["revision"] } }),
    );
    bench
}

fn create_args(key: &str) -> Value {
    json!({
        "requestId": key, "title": format!("task {key}"),
        "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
        "objective": "read the repo", "expectedResult": "summary"
    })
}

#[test]
fn a_child_result_over_mcp_is_delivered_to_the_coordinator_and_the_server_stops() {
    let server = Server::start(|_| {}, Some(RunScript::default()));
    coordinator(&server);
    let child = server
        .tool("coord", "aw_create_child_task", create_args("c0"))
        .expect("create child");
    let child_run = child["runId"].as_str().unwrap().to_owned();
    server.wait_stop();
    server.assert_not_stopping("the child task is running");

    let reported = server.tool(
        &child_run,
        "aw_report_result",
        json!({ "requestId": "r0", "summary": "done" }),
    );
    assert!(
        server.stops_within(STOP_BOUND),
        "the server did not stop after the child result ({reported:?})"
    );
    assert!(reported.is_ok(), "{reported:?}");
    let applied = server.engine.as_ref().unwrap().applied();
    assert!(
        applied
            .iter()
            .any(|label| label.starts_with("prompt:coord:")),
        "the notification reached the coordinator: {applied:?}"
    );
}

#[test]
fn a_waiting_task_is_assigned_as_a_continuation_and_the_server_stops() {
    let server = Server::start(
        |adapters| adapters.orchestration.max_concurrent_children = 1,
        Some(RunScript::default()),
    );
    coordinator(&server);
    let first = server
        .tool("coord", "aw_create_child_task", create_args("c0"))
        .expect("first");
    let second = server
        .tool("coord", "aw_create_child_task", create_args("c1"))
        .expect("second");
    assert_eq!(second["queued"], true, "{second}");
    server.wait_stop();
    let reported = server
        .tool(
            first["runId"].as_str().unwrap(),
            "aw_report_result",
            json!({ "requestId": "r0", "summary": "done" }),
        )
        .expect("first result");
    assert_eq!(reported["nextReadyTaskId"], second["taskId"], "{reported}");
    server.assert_not_stopping("a waiting task created before the drain remains");

    // coordinator가 대기 task를 배정한다(K). 결과는 멈춤 판정 뒤에 단정한다.
    let assigned = server.tool(
        "coord",
        "aw_assign_child_task",
        json!({ "taskId": second["taskId"], "requestId": "a1" }),
    );
    if let Ok(assigned) = &assigned
        && let Some(run) = assigned["runId"].as_str()
    {
        let _ = server.tool(
            run,
            "aw_report_result",
            json!({ "requestId": "r1", "summary": "done" }),
        );
    }
    assert!(
        server.stops_within(STOP_BOUND),
        "the server did not stop after the waiting task ran ({assigned:?})"
    );
    assert!(assigned.is_ok(), "{assigned:?}");
}

#[test]
fn a_queued_child_command_is_delivered_after_the_drain_and_the_server_stops() {
    let server = Server::start(
        |_| {},
        Some(RunScript {
            permission_id: Some("p1".into()),
            ..RunScript::default()
        }),
    );
    let bench = coordinator(&server);
    server.owner(
        "run.respondPermission",
        json!({ "benchId": bench, "runId": "coord", "permissionId": "p1", "optionId": "allow" }),
    );
    let child = server
        .tool("coord", "aw_create_child_task", create_args("c0"))
        .expect("create child");
    let child_run = child["runId"].as_str().unwrap().to_owned();
    // 자식 첫 turn이 권한을 기다리는 동안(바쁨) coordinator가 대기 명령을 보낸다(비우기 전, N이 아직 허용될 때).
    server.wait_until("the child to be busy", || {
        server.control.work_gate().busy_run_count(&child_run) > 0
    });
    server
        .tool(
            "coord",
            "aw_send_child_message",
            json!({ "taskId": child["taskId"], "requestId": "m1", "message": "also check the tests" }),
        )
        .expect("queued child command");
    server.wait_stop();
    server.assert_not_stopping("the child is busy and its task is running");

    server.owner(
        "run.respondPermission",
        json!({ "benchId": bench, "runId": child_run, "permissionId": "p1", "optionId": "allow" }),
    );
    server.wait_until("the queued command to reach the child", || {
        server
            .engine
            .as_ref()
            .unwrap()
            .applied()
            .iter()
            .any(|label| {
                label.starts_with(&format!("prompt:{child_run}:"))
                    && label.contains("also check the tests")
            })
    });
    let reported = server.tool(
        &child_run,
        "aw_report_result",
        json!({ "requestId": "r0", "summary": "done" }),
    );
    assert!(
        server.stops_within(STOP_BOUND),
        "the server did not stop after the queued command and the result ({reported:?})"
    );
    assert!(reported.is_ok(), "{reported:?}");
}
