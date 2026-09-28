//! 044 T016(research R2): MCP 연결 주입은 창과 무관하다. 데스크톱 창이 없는 작업대에서 시작한 run도 run에 묶인 MCP
//! 토큰·끝점·run id와 MCP 서버 설정·안내문을 받는다. 주입은 host 조립의 `McpLaunchDecorator`가 한다.

use std::sync::Arc;

use serde_json::json;
use workbench_core::{
    application::workbench_runtime::RuntimeAdapters,
    testing::scripted_run_engine::{RunScript, ScriptedRunEngine},
};
use workbench_host::{
    assembly::{HostOptions, assemble},
    mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV, AW_MCP_URL_ENV},
};
use workbench_protocol::{
    AuthenticatedPrincipal, CallRequest, IdempotencyKey, OperationId, Workbench,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_started_on_a_bench_without_a_desktop_window_gets_its_mcp_connection() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work).unwrap();
    let engine = Arc::new(ScriptedRunEngine::new(RunScript::default()));
    let mut adapters = RuntimeAdapters::production();
    adapters.run_engine = Some(engine.clone());
    let host = assemble(HostOptions::new(
        dir.path().to_path_buf(),
        adapters,
        "test",
        tokio::runtime::Handle::current(),
    ))
    .unwrap();

    // 데스크톱 창이 없는 주체(창 label 표에 없음)로 작업대를 열고 run을 시작한다.
    let principal = AuthenticatedPrincipal::desktop_window("no-window", "inc-1");
    let call = |operation, input| {
        let mut request = CallRequest::query(operation, input);
        request.idempotency_key =
            Some(IdempotencyKey::new(uuid::Uuid::new_v4().to_string()).unwrap());
        request
    };
    let opened = host
        .runtime
        .call(
            principal.clone(),
            call(OperationId::BenchOpen, json!({ "workingDirectory": work })),
        )
        .await
        .expect("bench.open");
    let bench = opened.output().unwrap()["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    host.runtime
        .call(
            principal,
            call(
                OperationId::RunStart,
                json!({ "benchId": bench, "request": { "goal": "g", "agentId": "codex", "runId": "run-1" } }),
            ),
        )
        .await
        .expect("run.start");

    let request = engine.start_requests.lock().unwrap()[0].clone();
    let env = request.agent_env.expect("MCP env injected");
    assert_eq!(
        env.get(AW_MCP_URL_ENV),
        Some(&host.mcp.base_url().to_owned())
    );
    assert_eq!(
        env.get(AW_MCP_RUN_ID_ENV).map(String::as_str),
        Some("run-1")
    );
    let token = env.get(AW_MCP_TOKEN_ENV).expect("run token");
    assert_eq!(
        host.mcp
            .capability_registry()
            .resolve(token)
            .map(|p| p.run_id),
        Some("run-1".to_owned()),
        "the token is bound to this run"
    );
    assert_eq!(
        request.mcp_servers.len(),
        1,
        "the MCP server config is attached"
    );
    assert!(
        request.goal.ends_with("User request:\ng"),
        "instructions precede the goal"
    );
    host.shutdown().await;
}
