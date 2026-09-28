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
        Self::start_in(tempfile::tempdir().unwrap(), configure, engine)
    }

    /// 같은 데이터 디렉터리(`dir/data`)와 작업 디렉터리(`dir/work`)로 host를 조립한다.
    fn start_in(
        dir: tempfile::TempDir,
        configure: impl FnOnce(&mut RuntimeAdapters),
        engine: Option<RunScript>,
    ) -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
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
        options.owner = Some(identity.identity().clone());
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
            "server.status"
                | "run.replay"
                | "bench.list"
                | "orchestration.get"
                | "orchestration.listRecoverable"
                | "orchestration.listTasks"
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

    fn mcp_base(&self) -> String {
        let mcp = self.host.as_ref().unwrap().mcp.base_url().to_owned();
        mcp.strip_suffix("/mcp").unwrap().to_owned()
    }

    /// 실제 MCP HTTP 끝점으로 도구를 부른다. 성공이면 구조화 결과, 도구 실패면 `Err(결과)`.
    fn tool(&self, run: &str, name: &str, arguments: Value) -> Result<Value, Value> {
        mcp_call(&self.mcp_base(), &self.mcp_token(run), name, arguments)
    }

    /// host 전체 종료(`HostAssembly::shutdown` — 독립 서버의 종료 경로와 같음: HTTP·MCP 비우기 → 작업대 닫기) 뒤
    /// 런타임·저장소를 모두 버리고, **같은 데이터 디렉터리**로 새 host를 조립한다(새 소유자 신원·새 엔진).
    fn restart(
        mut self,
        configure: impl FnOnce(&mut RuntimeAdapters),
        engine: Option<RunScript>,
    ) -> Self {
        let host = self.host.take().expect("host");
        self.rt.block_on(host.shutdown());
        drop(host);
        let dir = std::mem::replace(&mut self.dir, tempfile::tempdir().unwrap());
        drop(self);
        Self::start_in(dir, configure, engine)
    }

    /// 현재 파생 결과(정지 판정과 같은 `ServerControl::derive`).
    fn derived(&self) -> workbench_core::application::server_control::DerivedWork {
        self.rt.block_on(self.control.derive())
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

/// MCP `tools/call`(agent가 하는 호출과 같은 경로). blocking — 비동기 문맥에서는 `spawn_blocking`으로 부른다.
fn mcp_call(base: &str, token: &str, name: &str, arguments: Value) -> Result<Value, Value> {
    let (status, body) = request(
        base,
        "POST",
        "/mcp",
        Some(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": name, "arguments": arguments } })),
        Some(token),
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

/// coordinator의 **실제 turn**(엔진 `send_and_wait` — 알림 전달이 여는 A-turn) 안에서 도구를 부르는 가짜 agent.
/// ACP agent는 turn 안에서만 MCP 도구를 부른다. `assign`이 있으면 `release` 뒤 그 turn 안에서 대기 task를 배정한다(K).
/// 없으면 turn이 배정 없이 끝난다 — 재배정 주체가 없는 경로.
struct CoordinatorTurn {
    armed: Arc<std::sync::atomic::AtomicBool>,
    entered: Arc<std::sync::atomic::AtomicUsize>,
    release: Arc<tokio::sync::Semaphore>,
    assigned: Arc<std::sync::Mutex<Option<Result<Value, Value>>>>,
}

impl CoordinatorTurn {
    fn install(server: &Server, assign: Option<String>) -> Self {
        use std::sync::atomic::Ordering;
        let turn = Self {
            armed: Arc::default(),
            entered: Arc::default(),
            release: Arc::new(tokio::sync::Semaphore::new(0)),
            assigned: Arc::default(),
        };
        let (armed, entered, release, assigned) = (
            Arc::clone(&turn.armed),
            Arc::clone(&turn.entered),
            Arc::clone(&turn.release),
            Arc::clone(&turn.assigned),
        );
        let (base, token) = (server.mcp_base(), server.mcp_token("coord"));
        let engine = server.engine.as_ref().expect("scripted engine");
        *engine.turn_hook.lock().unwrap() = Some(Arc::new(move |run: String| {
            let (armed, entered, release, assigned) = (
                Arc::clone(&armed),
                Arc::clone(&entered),
                Arc::clone(&release),
                Arc::clone(&assigned),
            );
            let (base, token, assign) = (base.clone(), token.clone(), assign.clone());
            Box::pin(async move {
                if run != "coord" || !armed.swap(false, Ordering::SeqCst) {
                    return;
                }
                entered.fetch_add(1, Ordering::SeqCst);
                let Some(task) = assign else { return };
                release.acquire().await.expect("release").forget();
                let result = tokio::task::spawn_blocking(move || {
                    mcp_call(
                        &base,
                        &token,
                        "aw_assign_child_task",
                        json!({ "taskId": task, "requestId": "a1" }),
                    )
                })
                .await
                .expect("assign call");
                *assigned.lock().unwrap() = Some(result);
            })
        }));
        turn
    }

    /// 다음 coordinator turn을 이 가짜 agent가 맡는다.
    fn arm(&self) {
        self.armed.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn entered(&self) -> usize {
        self.entered.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn release(&self) {
        self.release.add_permits(1);
    }

    fn assigned(&self) -> Option<Result<Value, Value>> {
        self.assigned.lock().unwrap().clone()
    }
}

/// 동시 상한 1: 첫 task 실행, 둘째 대기(Ready).
fn two_tasks(server: &Server) -> (Value, Value) {
    let first = server
        .tool("coord", "aw_create_child_task", create_args("c0"))
        .expect("first");
    let second = server
        .tool("coord", "aw_create_child_task", create_args("c1"))
        .expect("second");
    assert_eq!(second["queued"], true, "{second}");
    (first, second)
}

/// (d) 실제 K 경로: 첫 task 결과 → 알림이 coordinator turn을 연다 → **그 turn 안에서** coordinator가 둘째를 MCP로
/// 배정(K) → 결과 → 멈춤. turn이 진행 중인 동안(배정 전) 대기 task는 배정될 수 있으므로 활동 작업이다.
#[test]
fn a_waiting_task_is_assigned_as_a_continuation_and_the_server_stops() {
    let server = Server::start(
        |adapters| adapters.orchestration.max_concurrent_children = 1,
        Some(RunScript::default()),
    );
    coordinator(&server);
    let (first, second) = two_tasks(&server);
    let second_id = second["taskId"].as_str().unwrap().to_owned();
    let turn = CoordinatorTurn::install(&server, Some(second_id.clone()));
    server.wait_stop();
    turn.arm();
    let reported = server
        .tool(
            first["runId"].as_str().unwrap(),
            "aw_report_result",
            json!({ "requestId": "r0", "summary": "done" }),
        )
        .expect("first result");
    assert_eq!(reported["nextReadyTaskId"], second["taskId"], "{reported}");
    server.wait_until("the coordinator's notification turn", || {
        turn.entered() == 1
    });
    // coordinator turn 진행 중(A-turn), 배정 전: 대기 task는 배정 가능 → 활동 작업(queued), 보류 아님.
    let derived = server.derived();
    assert_eq!(derived.queued_tasks, 1, "{derived:?}");
    assert!(derived.deferred_tasks.is_empty(), "{derived:?}");
    server.assert_not_stopping("the coordinator's turn can still assign the waiting task");

    turn.release();
    server.wait_until("the in-turn assignment", || turn.assigned().is_some());
    let assigned = turn.assigned().unwrap();
    let assigned = assigned.expect("the continuation assignment is accepted during the drain");
    assert_eq!(assigned["taskId"], json!(second_id), "{assigned}");
    let run = assigned["runId"].as_str().expect("assigned run").to_owned();
    server.assert_not_stopping("the assigned task is running");
    let result = server.tool(
        &run,
        "aw_report_result",
        json!({ "requestId": "r1", "summary": "done" }),
    );
    assert!(
        server.stops_within(STOP_BOUND),
        "the server did not stop after the waiting task ran ({result:?})"
    );
    assert!(result.is_ok(), "{result:?}");
}

/// (d 대조) 재배정 주체가 없는 경로: coordinator의 알림 turn이 **배정 없이** 끝난다(바쁜 coordinator·미전달 알림
/// 없음) → 대기 task는 보류(`deferredTasks`)로 보고되고 서버가 멈춘다. 이어서 host 전체 종료 → 같은 데이터로 새
/// host → 복구 목록에 **같은 task id·Ready** → 복구·coordinator 재결합 → 배정되어 새 run으로 실행된다.
#[test]
fn a_waiting_task_without_an_assigner_is_deferred_and_reassigned_after_a_restart() {
    let limit = |adapters: &mut RuntimeAdapters| adapters.orchestration.max_concurrent_children = 1;
    let server = Server::start(limit, Some(RunScript::default()));
    coordinator(&server);
    let (first, second) = two_tasks(&server);
    let second_id = second["taskId"].as_str().unwrap().to_owned();
    let turn = CoordinatorTurn::install(&server, None);
    server.wait_stop();
    turn.arm();
    server
        .tool(
            first["runId"].as_str().unwrap(),
            "aw_report_result",
            json!({ "requestId": "r0", "summary": "done" }),
        )
        .expect("first result");
    assert!(
        server.stops_within(STOP_BOUND),
        "the server stops when nobody can assign the waiting task ({:?})",
        server.derived()
    );
    assert_eq!(
        turn.entered(),
        1,
        "the coordinator had its notification turn and did not assign"
    );
    // 정지 판정 뒤의 파생(관문은 `stopping`에 멈춰 있고 작업대는 host 종료 전까지 열려 있다) — 멈춘 근거다. 멈추기 전
    // 표본은 쓰지 않는다(마지막 표본과 감시 루프의 판정 사이에 상태가 바뀔 수 있다).
    let last = server.derived();
    assert_eq!(last.deferred_tasks, vec![second_id.clone()], "{last:?}");
    assert_eq!(last.queued_tasks, 0, "{last:?}");
    assert_eq!(last.pending_notifications, 0, "{last:?}");

    let server = server.restart(limit, Some(RunScript::default()));
    let bench = server.open_bench();
    let recoverable = server.owner(
        "orchestration.listRecoverable",
        json!({ "benchId": bench, "worktreePath": server.work }),
    );
    let tasks = recoverable[0]["tasks"]
        .as_array()
        .expect("recoverable tasks");
    let task = tasks
        .iter()
        .find(|task| task["id"] == json!(second_id))
        .unwrap_or_else(|| panic!("the deferred task is recoverable: {recoverable}"));
    assert_eq!(task["status"], "ready", "{task}");
    assert_eq!(task["startedAt"], Value::Null, "{task}");

    // 복구: 작업 영역을 다시 묶고 새 coordinator run을 결합한 뒤 복구(scheduler에 Ready task 반영)한다.
    let session = server.owner(
        "orchestration.bootstrap",
        json!({ "benchId": bench, "worktreePath": server.work,
            "resumeWorkspaceId": recoverable[0]["id"] }),
    );
    assert_eq!(
        session["id"], recoverable[0]["id"],
        "the recoverable session is resumed"
    );
    server.start_run(&bench, "coord2", json!({}));
    // 새 Main run은 명시적 coordinator 인계로 결합한다(이전 세대 run `coord`는 서버와 함께 끝났다).
    let handed = server.owner(
        "orchestration.handoffCoordinator",
        json!({ "benchId": bench, "request": {
            "requestId": "handoff-1", "successorRunId": "coord2", "summary": "resume after restart",
            "confirmed": true, "expectedRevision": session["revision"] } }),
    );
    assert_ne!(
        handed["activeCoordinatorGenerationId"], session["activeCoordinatorGenerationId"],
        "{handed}"
    );
    server.owner("orchestration.recover", json!({ "benchId": bench }));
    let assigned = server
        .tool(
            "coord2",
            "aw_assign_child_task",
            json!({ "taskId": second_id, "requestId": "a2" }),
        )
        .expect("the deferred task is assigned by the new coordinator");
    assert_eq!(assigned["taskId"], json!(second_id), "{assigned}");
    assert_eq!(
        assigned["queued"],
        Value::Null,
        "a free slot starts the task: {assigned}"
    );
    let run = assigned["runId"].as_str().expect("assigned run").to_owned();
    let reported = server
        .tool(
            &run,
            "aw_report_result",
            json!({ "requestId": "r1", "summary": "done after restart" }),
        )
        .expect("the reassigned task reports its result");
    assert_eq!(reported["report"]["taskId"], json!(second_id), "{reported}");
    let tasks = server.owner("orchestration.get", json!({ "benchId": bench }))["tasks"].clone();
    let task = tasks
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["id"] == json!(second_id))
        .cloned()
        .expect("task");
    assert_eq!(task["status"], "completed", "{task}");
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
