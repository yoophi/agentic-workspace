//! 044 OCR 구현 리뷰(host H1·core H1): MCP 도구 호출이 비우기 입구에서 거절되면 도구 결과의 `structuredContent.code`가
//! fault 코드(`draining`)여야 한다. `internalError`로 뭉개면 agent가 "서버가 비우는 중(다시 시도 가능)"과 버그를 구별하지
//! 못한다. 실제 host 조립 + 실제 `/mcp` 끝점, coordinator run은 권한 대기로 바빠 서버가 `drainingWait`에 머문다.

use std::{path::Path, sync::Arc, time::Duration};

use serde_json::{Value, json};
use workbench_core::{
    application::server_control::ServerControl,
    testing::scripted_run_engine::{RunScript, ScriptedRunEngine},
};
use workbench_host::{
    assembly::{HostAssembly, HostOptions, assemble},
    lifecycle::{
        calls::call,
        client::request,
        identity::OwnerIdentity,
        monitor::{MonitorOptions, run_until_stopped},
    },
    mcp::{AW_MCP_RUN_ID_ENV, AW_MCP_TOKEN_ENV},
};

struct Server {
    rt: tokio::runtime::Runtime,
    host: Option<HostAssembly>,
    control: Arc<ServerControl>,
    base: String,
    owner: String,
    engine: Arc<ScriptedRunEngine>,
    _dir: tempfile::TempDir,
    work: String,
}

impl Server {
    fn start() -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let work = std::fs::canonicalize(work)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut adapters =
            workbench_core::application::workbench_runtime::RuntimeAdapters::production();
        // 첫 turn이 권한을 기다리며 응답하지 않는다 → coordinator가 바빠 비우기가 끝나지 않는다.
        let engine = Arc::new(ScriptedRunEngine::new(RunScript {
            permission_id: Some("p1".into()),
            ..RunScript::default()
        }));
        adapters.run_engine = Some(engine.clone());
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
            _dir: dir,
            work,
        }
    }

    fn owner(&self, operation: &str, input: Value) -> Value {
        let command = !matches!(operation, "server.status" | "orchestration.get");
        call(&self.base, &self.owner, None, operation, input, command)
            .unwrap_or_else(|error| panic!("{operation}: {error:?}"))
    }

    fn mcp_token(&self, run: &str) -> String {
        let requests = self.engine.start_requests.lock().unwrap();
        requests
            .iter()
            .filter_map(|request| request.agent_env.as_ref())
            .find(|env| env.get(AW_MCP_RUN_ID_ENV).map(String::as_str) == Some(run))
            .and_then(|env| env.get(AW_MCP_TOKEN_ENV).cloned())
            .unwrap_or_else(|| panic!("no MCP token for {run}"))
    }

    /// 실제 `/mcp` `tools/call`. 도구 결과(`result`) 전체를 돌려준다.
    fn tool(&self, run: &str, name: &str, arguments: Value) -> Value {
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
        body["result"].clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(host) = self.host.take() {
            self.rt.block_on(host.shutdown());
        }
    }
}

fn git_init(dir: &str) {
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(Path::new(dir))
        .status()
        .expect("git");
    assert!(status.success());
}

#[test]
fn a_new_work_tool_call_during_the_drain_reports_the_draining_code() {
    let server = Server::start();
    git_init(&server.work);
    let bench = server.owner("bench.open", json!({ "workingDirectory": server.work }))["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = server.owner(
        "orchestration.bootstrap",
        json!({ "benchId": bench, "worktreePath": server.work }),
    );
    server.owner(
        "run.start",
        json!({ "benchId": bench, "request": { "goal": "g", "agentId": "codex", "runId": "coord" } }),
    );
    server.owner(
        "orchestration.bindCoordinator",
        json!({ "benchId": bench, "request": {
            "requestId": "bind-1", "panelId": "main-agent-run", "runId": "coord",
            "state": "active", "expectedRevision": session["revision"] } }),
    );
    let until = std::time::Instant::now() + Duration::from_secs(20);
    while server.control.work_gate().busy_run_count("coord") == 0 {
        assert!(
            std::time::Instant::now() < until,
            "the coordinator never became busy"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let draining = server.owner("server.stop", json!({ "mode": "wait" }));
    assert_eq!(draining["state"], "drainingWait", "{draining}");

    // 자식 task 만들기는 새 작업(N) — 비우는 중 거절된다.
    let result = server.tool(
        "coord",
        "aw_create_child_task",
        json!({
            "requestId": "c0", "title": "task c0",
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read the repo", "expectedResult": "summary"
        }),
    );
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"]["code"], "draining", "{result}");
    assert_eq!(result["structuredContent"]["retryable"], true, "{result}");
    let status = server.owner("server.status", json!({}));
    assert_eq!(status["state"], "drainingWait", "{status}");
}
