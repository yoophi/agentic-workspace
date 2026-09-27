//! 042 Codex 재리뷰: HTTP로 받아들인 `run.cancelAndSend`가 교체 prompt의 **사용자 권한 응답**을 기다리는 동안 종료하면,
//! 수락이 닫혀 `run.respondPermission`·`run.cancel`이 더는 들어올 수 없으므로 drain이 영원히 기다린다. 실제
//! `AcpRunEngine`(acp-agent-core runner·`permission_flow`)과 최소 ACP agent(`support/agents/fake_acp_permission_agent.py`)로
//! 재현한다. 종료 순서: 수락 차단 → 진행 중 작업 해제(작업대 닫기 = 소유 run 취소 = 권한 대기 제거) → 받아들인 호출
//! 끝(terminal 결과) → drain.

use std::{sync::Arc, time::Duration};

use serde_json::{json, Value};
use support::{
    command_request,
    http_harness::{Harness, HarnessOptions, TOKEN_DESKTOP},
    uuid_key, TestRuntime,
};
use workbench_core::application::workbench_runtime::RuntimeAdapters;
use workbench_protocol::{AuthenticatedPrincipal, OperationId, Workbench};

mod support;

fn agent_command() -> String {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/agents/fake_acp_permission_agent.py");
    format!("python3 {}", script.display())
}

async fn desktop(rt: &TestRuntime, operation: OperationId, input: Value) -> Value {
    rt.runtime
        .call(
            AuthenticatedPrincipal::desktop(),
            command_request(operation, &uuid_key(), input),
        )
        .await
        .unwrap_or_else(|fault| panic!("{operation:?}: {fault:?}"))
        .output()
        .cloned()
        .unwrap_or(Value::Null)
}

/// run r1의 기록에 사용자 응답을 기다리는 권한 요청이 `count`개 쌓일 때까지(상태를 확인하며 기다린다).
async fn wait_for_permission_requests(rt: &TestRuntime, count: usize) {
    for _ in 0..500 {
        let replay = rt.runtime.events_hub().replay_run("r1", 0);
        let asked = replay
            .events
            .iter()
            .filter(|event| {
                event
                    .event
                    .to_string()
                    .contains("\"requiresResponse\":true")
            })
            .count();
        if asked >= count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the agent did not ask for permission {count} time(s)");
}

struct Waiting {
    rt: TestRuntime,
    bench: String,
    request: workbench_protocol::CallRequest,
    harness: Harness,
    call: tokio::task::JoinHandle<
        Option<Result<workbench_protocol::CallReply, workbench_protocol::WorkbenchFault>>,
    >,
}

/// run을 띄워 첫 prompt가 권한을 기다리게 하고, HTTP로 `run.cancelAndSend`를 받아들여 교체 prompt도 권한을 기다리게 한다.
async fn cancel_and_send_waiting_for_permission() -> Waiting {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let bench = desktop(
        &rt,
        OperationId::BenchOpen,
        json!({ "workingDirectory": work }),
    )
    .await["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    desktop(
        &rt,
        OperationId::RunStart,
        json!({ "benchId": bench, "request": {
            "goal": "first", "agentId": "fake-acp", "agentCommand": agent_command(),
            "cwd": work, "runId": "r1", "permissionMode": "default" } }),
    )
    .await;
    wait_for_permission_requests(&rt, 1).await;
    let harness = Harness::spawn_with(
        rt.runtime.clone() as Arc<dyn Workbench>,
        HarnessOptions {
            drain_warn_after: Duration::from_millis(200),
            ..HarnessOptions::default()
        },
    )
    .await;
    let request = command_request(
        OperationId::RunCancelAndSend,
        &uuid_key(),
        json!({ "benchId": bench, "runId": "r1", "prompt": "replacement" }),
    );
    let call = {
        let client = harness.client().clone();
        let url = harness.url();
        let request = request.clone();
        tokio::spawn(async move {
            // 유예 뒤 서버가 연결을 닫으면 응답 없이 끝난다(수정 전 순서의 정리 단계).
            let response = client
                .post(url)
                .bearer_auth(TOKEN_DESKTOP)
                .json(&request)
                .send()
                .await
                .ok()?;
            Some(support::http_harness::read_reply(response).await)
        })
    };
    wait_for_permission_requests(&rt, 2).await;
    assert_eq!(
        harness.calls.active(),
        1,
        "cancelAndSend was accepted and is waiting"
    );
    Waiting {
        rt,
        bench,
        request,
        harness,
        call,
    }
}

/// 수정 전 순서(수락만 닫고 곧바로 drain)는 끝나지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exit_without_releasing_work_waits_forever() {
    let Waiting {
        rt,
        bench,
        request: _,
        mut harness,
        call,
    } = cancel_and_send_waiting_for_permission().await;
    harness.calls.close();
    let served = harness.begin_shutdown();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        !served.is_finished(),
        "drain returned while the call still waited for a permission answer"
    );
    // 정리: 작업을 풀면 끝난다.
    desktop(&rt, OperationId::BenchClose, json!({ "benchId": bench })).await;
    tokio::time::timeout(Duration::from_secs(15), served)
        .await
        .expect("released")
        .unwrap()
        .unwrap();
    let _ = call.await;
}

/// 수정 뒤 순서: 수락 차단 → 작업대 닫기(소유 run 취소, 권한 대기 제거) → 받아들인 호출 끝 → drain 완료.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn exit_releases_permission_waits_then_drains() {
    let Waiting {
        rt,
        bench: _,
        request,
        harness,
        call,
    } = cancel_and_send_waiting_for_permission().await;
    harness.calls.close();
    // AW 종료 경로와 같은 해제: 연 주체와 무관하게 열린 작업대를 모두 닫는다.
    assert_eq!(rt.runtime.close_all_benches().await, 1);
    let calls = harness.calls.clone();
    tokio::time::timeout(Duration::from_secs(15), harness.shutdown())
        .await
        .expect("exit finished after releasing the permission wait");
    assert_eq!(
        calls.active(),
        0,
        "the accepted call reached a terminal result"
    );
    let reply = tokio::time::timeout(Duration::from_secs(5), call)
        .await
        .expect("client got an answer")
        .unwrap()
        .expect("the connection stayed open until the call ended");
    let fault = reply.expect_err("the cancelled run fails the replacement prompt");
    eprintln!("terminal fault: {:?} {}", fault.code, fault.message);
    assert_eq!(
        rt.runtime.run_engine().owner_of("r1").await,
        None,
        "the run is gone"
    );
    // 멱등 처리: 세대 멱등은 성공만 기록하고, 작업대 닫기가 그 작업대 범위의 기록을 지운다(계약). 그래서 끝난 뒤 같은
    // 키 재시도는 작업대가 없어 `notFound`이고 교체 prompt를 다시 보내지 않는다. 영속 ledger 기록은 없다(세대 범위).
    let retry = rt
        .runtime
        .call(AuthenticatedPrincipal::desktop(), request)
        .await
        .expect_err("the bench is gone");
    assert_eq!(
        retry.code,
        workbench_protocol::FaultCode::NotFound,
        "{retry:?}"
    );
    assert_eq!(rt.runtime.benches().open_count(), 0, "no bench left open");
}

/// 서버 종료의 작업 해제는 연 주체와 무관하다: 데스크톱과 다른 주체(HTTP 클라이언트)가 연 작업대의 run도 취소된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn closing_all_benches_covers_every_owner() {
    use support::{scripted_run_engine::RunScript, BenchHarness};
    let h = BenchHarness::new(RunScript::default());
    let mine = h.open().await;
    h.start(&mine, "r-desktop").await.unwrap();
    let other = h
        .call(
            &AuthenticatedPrincipal::test_as("desktop2"),
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await
        .unwrap()["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    h.call(
        &AuthenticatedPrincipal::test_as("desktop2"),
        OperationId::RunStart,
        json!({ "benchId": other, "request": { "goal": "g", "agentId": "codex", "runId": "r-other" } }),
    )
    .await
    .unwrap();
    assert_eq!(h.engine.run_count(), 2);
    assert_eq!(h.rt.runtime.close_all_benches().await, 2);
    assert_eq!(h.engine.run_count(), 0, "every owner's runs were cancelled");
    assert_eq!(h.rt.runtime.close_all_benches().await, 0, "idempotent");
}
