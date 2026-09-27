//! 043 T043(사용자 검토): 교환 prompt의 멱등성 키 `exchange-delivery:<requestId>`가 **실제 agent 프로세스 경계**에서 prompt를
//! 한 번만 보내게 하는지. `exchange_delivery_once.rs`는 core → `RunEngine` 호출 수(가짜 엔진 카운터)의 근거이고, 이 시험은
//! 실제 `AcpRunEngine`(acp-agent-core runner)과 최소 ACP agent 프로세스(`support/agents/fake_acp_permission_agent.py`,
//! 받은 prompt마다 응답 전에 `prompt:<id>` 기록)로 agent가 받은 prompt 수를 센다.
//!
//! 같은 키로 두 번 보낸 뒤, 다른 키(x-2)로 세 번째를 보내고 **x-2 본문의 prompt**가 agent에서 끝나기를 기다린다(agent가
//! `prompt-text:<id>:<본문>`과 `end_turn:<id>`를 남긴다). 한 세션의 prompt는 차례로 처리되므로 x-2가 끝났으면 앞선 전송은
//! 모두 처리된 뒤다 — 그때 x-1 본문을 받은 횟수가 1이어야 한다. 시간 대기로 "나중에도 안 온다"를 추정하지 않는다.

use std::time::Duration;

use serde_json::{json, Value};
use support::{command_request, uuid_key, TestRuntime};
use workbench_core::application::workbench_runtime::RuntimeAdapters;
use workbench_protocol::{AuthenticatedPrincipal, OperationId, Workbench};

mod support;

fn agent_command(log: &std::path::Path) -> String {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/agents/fake_acp_permission_agent.py");
    format!("python3 {} --log {}", script.display(), log.display())
}

fn agent_log(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// 본문에 `needle`이 든 prompt의 id들.
fn prompt_ids_with(lines: &[String], needle: &str) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix("prompt-text:"))
        .filter(|rest| rest.contains(needle))
        .map(|rest| rest.split(':').next().unwrap_or_default().to_owned())
        .collect()
}

/// 본문에 `needle`이 든 prompt가 agent에서 끝날(`end_turn:<id>`) 때까지(조건 확인 폴링, 상한 10초).
async fn wait_until_finished(path: &std::path::Path, needle: &str) -> Vec<String> {
    for _ in 0..500 {
        let lines = agent_log(path);
        let ids = prompt_ids_with(&lines, needle);
        if ids
            .iter()
            .any(|id| lines.iter().any(|line| line == &format!("end_turn:{id}")))
        {
            return lines;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("agent did not finish the prompt with {needle:?}: {:?}", agent_log(path));
}

async fn call(rt: &TestRuntime, operation: OperationId, key: &str, input: Value) -> Value {
    rt.runtime
        .call(AuthenticatedPrincipal::desktop(), command_request(operation, key, input))
        .await
        .unwrap_or_else(|fault| panic!("{operation:?}: {fault:?}"))
        .output()
        .cloned()
        .unwrap_or(Value::Null)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_same_exchange_delivery_key_reaches_the_acp_agent_process_once() {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let log = rt.dir.path().join("agent.log");
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work).unwrap().to_string_lossy().into_owned();
    let bench = call(&rt, OperationId::BenchOpen, &uuid_key(), json!({ "workingDirectory": work })).await["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    call(
        &rt,
        OperationId::RunStart,
        &uuid_key(),
        json!({ "benchId": bench, "request": {
            "goal": "first", "agentId": "fake-acp", "agentCommand": agent_command(&log),
            "cwd": work, "runId": "r1", "autoAllow": true } }),
    )
    .await;
    wait_until_finished(&log, "first").await; // 시작 목표 prompt

    let exchange = json!({ "benchId": bench, "runId": "r1", "prompt": "peer message x-1" });
    call(&rt, OperationId::RunSendPrompt, "exchange-delivery:x-1", exchange.clone()).await;
    wait_until_finished(&log, "peer message x-1").await;
    // 새로고침 뒤 원장 없이 다시 라우팅된 같은 교환: 같은 키.
    call(&rt, OperationId::RunSendPrompt, "exchange-delivery:x-1", exchange).await;
    // 장벽: 다른 교환 x-2가 끝나면 그 앞의 전송은 모두 처리됐다.
    call(
        &rt,
        OperationId::RunSendPrompt,
        "exchange-delivery:x-2",
        json!({ "benchId": bench, "runId": "r1", "prompt": "peer message x-2" }),
    )
    .await;
    let lines = wait_until_finished(&log, "peer message x-2").await;
    assert_eq!(prompt_ids_with(&lines, "peer message x-1").len(), 1, "x-1 reached the agent once: {lines:?}");
    assert_eq!(prompt_ids_with(&lines, "peer message x-2").len(), 1, "{lines:?}");
    let prompts = lines.iter().filter(|line| line.starts_with("prompt:")).count();
    assert_eq!(prompts, 3, "agent prompts (goal, x-1 once, x-2): {lines:?}");
    rt.runtime.close_all_benches().await;
}
