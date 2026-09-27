//! 042 T027: agent `tools/call` → MCP 어댑터 → `Workbench.call` 경로의 재시도 식별. 실제 `WorkbenchRuntime`
//! (가짜 run 엔진)으로 잰다. 같은 `requestId`의 재전송·동시 전송·연결 단절 뒤 재전송은 효과가 한 번이고 원 요청의
//! 결과를 받는다. `requestId`가 없는 도구(제목)는 자연 멱등에 기댄다.

use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use workbench_core::{
    application::workbench_runtime::{RuntimeAdapters, WorkbenchRuntime},
    infrastructure::data_paths::DataPaths,
    testing::scripted_run_engine::{RunScript, ScriptedRunEngine},
};
use workbench_protocol::{
    AuthenticatedPrincipal, CallRequest, IdempotencyKey, OperationId, Workbench,
};

use super::{
    agent_exchange_tool::SEND_MESSAGE_TO_AGENT_TOOL,
    capability_registry::CapabilityPrincipal,
    handle_tool_call,
    orchestration_tool::{COLLECT_CHILD_RESULTS_TOOL, CREATE_CHILD_TASK_TOOL, REPORT_RESULT_TOOL},
    title_tool::SET_WINDOW_TITLE_TOOL,
};

struct Env {
    _dir: tempfile::TempDir,
    work: String,
    runtime: Arc<WorkbenchRuntime>,
    engine: Arc<ScriptedRunEngine>,
    bench: String,
}

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

async fn desktop(
    env_runtime: &Arc<WorkbenchRuntime>,
    operation: OperationId,
    input: Value,
) -> Value {
    let mut request = CallRequest::query(operation, input);
    if !matches!(
        workbench_protocol::operations::spec_for(operation).kind,
        workbench_protocol::OperationKind::Query
    ) {
        request.idempotency_key = Some(IdempotencyKey::random());
    }
    env_runtime
        .call(AuthenticatedPrincipal::desktop(), request)
        .await
        .unwrap_or_else(|fault| panic!("{operation:?}: {fault:?}"))
        .output()
        .cloned()
        .unwrap_or(Value::Null)
}

async fn env(script: RunScript) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    git_init(&work);
    let engine = Arc::new(ScriptedRunEngine::new(script));
    let mut adapters = RuntimeAdapters::production();
    adapters.run_engine = Some(engine.clone());
    let runtime =
        WorkbenchRuntime::bootstrap_with(DataPaths::new(dir.path().join("data")), adapters)
            .unwrap();
    let bench = desktop(
        &runtime,
        OperationId::BenchOpen,
        json!({ "workingDirectory": work }),
    )
    .await["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    Env {
        _dir: dir,
        work,
        runtime,
        engine,
        bench,
    }
}

impl Env {
    async fn start(&self, run: &str) {
        desktop(
            &self.runtime,
            OperationId::RunStart,
            json!({ "benchId": self.bench, "request": { "goal": "g", "agentId": "codex", "runId": run } }),
        )
        .await;
    }

    async fn coordinator(&self) {
        let session = desktop(
            &self.runtime,
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": self.bench, "worktreePath": self.work }),
        )
        .await;
        self.start("coord").await;
        desktop(
            &self.runtime,
            OperationId::OrchestrationBindCoordinator,
            json!({ "benchId": self.bench, "request": {
                "requestId": "bind-1", "panelId": "main-agent-run", "runId": "coord",
                "state": "active", "expectedRevision": session["revision"] } }),
        )
        .await;
    }

    async fn tool(&self, run: &str, name: &str, arguments: Value) -> Value {
        handle_tool_call(
            &self.runtime,
            &CapabilityPrincipal::run(run),
            Some(json!({ "name": name, "arguments": arguments })),
        )
        .await
    }
}

fn create_args(request_id: &str) -> Value {
    json!({
        "requestId": request_id, "title": "task",
        "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
        "objective": "read the repo", "expectedResult": "summary"
    })
}

fn structured(result: &Value) -> &Value {
    assert_eq!(result["isError"], json!(false), "tool failed: {result}");
    &result["structuredContent"]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resent_tool_call_with_the_same_request_id_applies_once() {
    let env = env(RunScript::default()).await;
    env.coordinator().await;
    let starts = env.engine.starts.load(Ordering::SeqCst);
    let first = env
        .tool("coord", CREATE_CHILD_TASK_TOOL, create_args("child-1"))
        .await;
    let again = env
        .tool("coord", CREATE_CHILD_TASK_TOOL, create_args("child-1"))
        .await;
    assert_eq!(structured(&again)["runId"], structured(&first)["runId"]);
    assert_eq!(structured(&again)["taskId"], structured(&first)["taskId"]);
    assert_eq!(
        env.engine.starts.load(Ordering::SeqCst),
        starts + 1,
        "one child"
    );
    // 다른 requestId는 다른 요청이다.
    let other = env
        .tool("coord", CREATE_CHILD_TASK_TOOL, create_args("child-2"))
        .await;
    assert_ne!(structured(&other)["taskId"], structured(&first)["taskId"]);
    assert_eq!(env.engine.starts.load(Ordering::SeqCst), starts + 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_identical_tool_calls_start_one_child() {
    let env = env(RunScript {
        start_settle_ms: 300,
        ..RunScript::default()
    })
    .await;
    env.coordinator().await;
    let starts = env.engine.starts.load(Ordering::SeqCst);
    let (a, b) = tokio::join!(
        env.tool("coord", CREATE_CHILD_TASK_TOOL, create_args("child-1")),
        env.tool("coord", CREATE_CHILD_TASK_TOOL, create_args("child-1")),
    );
    assert_eq!(structured(&a)["runId"], structured(&b)["runId"]);
    assert_eq!(
        env.engine.starts.load(Ordering::SeqCst),
        starts + 1,
        "one child"
    );
}

/// 연결 단절: `tools/call` handler가 자식 기동 뒤·결과 기록 전에 사라진다(운영 handler와 같은 `spawn_accepted`).
/// agent가 같은 요청을 다시 보내면 원 요청이 띄운 자식을 결과로 받는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tool_call_resent_after_a_disconnect_returns_the_original_child() {
    let env = env(RunScript {
        start_settle_ms: 400,
        ..RunScript::default()
    })
    .await;
    env.coordinator().await;
    let starts = env.engine.starts.load(Ordering::SeqCst);
    let calls = Arc::new(workbench_server::drain::DetachedCalls::default());
    let caller = {
        let (runtime, calls) = (env.runtime.clone(), calls.clone());
        tokio::spawn(async move {
            let guard = calls.accept().unwrap();
            workbench_server::drain::spawn_accepted(guard, async move {
                handle_tool_call(
                    &runtime,
                    &CapabilityPrincipal::run("coord"),
                    Some(json!({ "name": CREATE_CHILD_TASK_TOOL, "arguments": create_args("child-1") })),
                )
                .await
            })
            .await
        })
    };
    let child = env
        .engine
        .wait_applied(
            |l| l.starts_with("start:") && l != "start:coord",
            Duration::from_secs(10),
        )
        .await;
    caller.abort();
    let retried = env
        .tool("coord", CREATE_CHILD_TASK_TOOL, create_args("child-1"))
        .await;
    assert_eq!(
        format!("start:{}", structured(&retried)["runId"].as_str().unwrap()),
        child
    );
    assert_eq!(
        env.engine.starts.load(Ordering::SeqCst),
        starts + 1,
        "one child"
    );
}

/// 수집은 보고서를 수집됨으로 표시한다: 같은 requestId로 다시 보내면 첫 수집 결과를 다시 받는다(응답을 잃은
/// agent가 결과를 잃지 않는다). requestId 없는 수집은 재시도를 식별할 수 없다(계약에 명시).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resent_collection_with_a_request_id_returns_the_same_reports() {
    let env = env(RunScript::default()).await;
    env.coordinator().await;
    let created = env
        .tool("coord", CREATE_CHILD_TASK_TOOL, create_args("child-1"))
        .await;
    let child = structured(&created)["runId"].as_str().unwrap().to_owned();
    let reported = env
        .tool(
            &child,
            REPORT_RESULT_TOOL,
            json!({ "requestId": "result-1", "summary": "done", "confidence": 0.9 }),
        )
        .await;
    structured(&reported);
    let first = env
        .tool(
            "coord",
            COLLECT_CHILD_RESULTS_TOOL,
            json!({ "requestId": "collect-1" }),
        )
        .await;
    let reports = structured(&first)["reports"].clone();
    assert_eq!(reports.as_array().unwrap().len(), 1, "{first}");
    let again = env
        .tool(
            "coord",
            COLLECT_CHILD_RESULTS_TOOL,
            json!({ "requestId": "collect-1" }),
        )
        .await;
    assert_eq!(
        structured(&again)["reports"],
        reports,
        "same collection replayed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resent_exchange_message_is_delivered_once() {
    let env = env(RunScript::default()).await;
    env.start("r1").await;
    env.start("r2").await;
    desktop(
        &env.runtime,
        OperationId::ExchangeSyncWorkspace,
        json!({"benchId": env.bench, "request": {
        "worktreePath": env.work, "revision": 1, "focusedPanelId": "main",
        "panels": [
            {"panelId": "main", "title": "Main", "runId": "r1", "status": "running"},
            {"panelId": "extra", "title": "Extra", "runId": "r2", "status": "running"}
        ]}}),
    )
    .await;
    let args = json!({ "runId": "r1", "requestId": "msg-1", "targetPanelId": "extra",
        "targetRunId": "r2", "message": "hello", "delivery": "send" });
    let (a, b) = tokio::join!(
        env.tool("r1", SEND_MESSAGE_TO_AGENT_TOOL, args.clone()),
        env.tool("r1", SEND_MESSAGE_TO_AGENT_TOOL, args.clone()),
    );
    let c = env.tool("r1", SEND_MESSAGE_TO_AGENT_TOOL, args).await;
    for result in [&a, &b, &c] {
        assert_eq!(result["isError"], json!(false), "{result}");
    }
    assert_eq!(structured(&b), structured(&a));
    assert_eq!(structured(&c), structured(&a));
    let exchanges = desktop(
        &env.runtime,
        OperationId::ExchangeList,
        json!({ "benchId": env.bench }),
    )
    .await;
    let count = exchanges
        .as_array()
        .or_else(|| exchanges["exchanges"].as_array())
        .map(Vec::len)
        .unwrap_or_else(|| panic!("exchange list shape: {exchanges}"));
    assert_eq!(count, 1, "one exchange: {exchanges}");
}

/// 제목 도구에는 requestId가 없다: 재전송은 새 요청이지만 같은 제목을 다시 적용할 뿐이다(자연 멱등).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn resent_title_request_converges_to_the_same_title() {
    let env = env(RunScript::default()).await;
    env.start("r1").await;
    let args = json!({ "runId": "r1", "title": "Reading the repo" });
    let first = env.tool("r1", SET_WINDOW_TITLE_TOOL, args.clone()).await;
    let again = env.tool("r1", SET_WINDOW_TITLE_TOOL, args).await;
    assert_eq!(first["isError"], json!(false), "{first}");
    assert_eq!(again["structuredContent"], first["structuredContent"]);
}
