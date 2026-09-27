//! 042 research R13 증거: 세대 범위 변경과 orchestration 변경을 적용한 뒤 재시작하고 같은 키로 재시도한다.
//! 세대 범위 멱등 기록은 세대와 함께 사라지고 작업대도 없으므로 재시도는 `notFound`(닫기는 종료 상태 멱등이라
//! `closed: false`)이며 효과가 다시 나지 않는다.
//! orchestration 파일은 변경이 한 번만 반영된 채 남는다. `bench.open` 재시도는 새 작업대를 연다(설계 리뷰 D2).
//! 각 재시도는 HTTP(운영 router)로도 한 번 보내 같은 결과를 확인한다.

#![allow(clippy::result_large_err)]

mod support;

use std::sync::{atomic::Ordering, Arc};

use serde_json::{json, Value};
use support::{
    command_request,
    http_harness::{Harness, TOKEN_DESKTOP},
    scripted_run_engine::RunScript,
    BenchHarness,
};
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, CallRequest, FaultCode, OperationId, Workbench,
    WorkbenchFault,
};

fn desktop() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop()
}

/// in-process 재시도 → HTTP 재시도. 두 결과를 함께 돌려준다.
async fn retry_both(
    h: &BenchHarness,
    request: &CallRequest,
) -> (
    Result<CallReply, WorkbenchFault>,
    Result<CallReply, WorkbenchFault>,
) {
    let local = h.rt.runtime.call(desktop(), request.clone()).await;
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    let remote = harness.call(Some(TOKEN_DESKTOP), request).await;
    (local, remote)
}

fn assert_not_found(
    label: &str,
    result: (
        Result<CallReply, WorkbenchFault>,
        Result<CallReply, WorkbenchFault>,
    ),
) {
    let (local, remote) = result;
    let local = local.expect_err(label);
    let remote = remote.expect_err(label);
    assert_eq!(local.code, FaultCode::NotFound, "{label}: {local:?}");
    assert_eq!(remote.code, local.code, "{label}: http parity");
    assert_eq!(remote.message, local.message, "{label}: http parity");
}

fn orchestration_file(h: &BenchHarness) -> Value {
    let path =
        h.rt.paths
            .app_data_dir()
            .join("orchestration-sessions.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("orchestration file"))
        .expect("orchestration json")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn epoch_scoped_commands_are_not_found_after_restart() {
    // bench.close
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    let close = command_request(
        OperationId::BenchClose,
        "close-1",
        json!({ "benchId": bench }),
    );
    h.rt.runtime.call(desktop(), close.clone()).await.unwrap();
    let h = h.restart();
    // 닫기는 종료 상태 멱등이다: 없는 작업대는 `closed: false`, 취소된 run 없음(효과 없음).
    let (local, remote) = retry_both(&h, &close).await;
    let local = local.expect("bench.close").output().cloned().unwrap();
    assert_eq!(local, json!({ "closed": false, "cancelledRuns": [] }));
    assert_eq!(
        remote
            .expect("bench.close over http")
            .output()
            .cloned()
            .unwrap(),
        local,
        "http parity"
    );

    // run.sendPrompt
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let prompt = command_request(
        OperationId::RunSendPrompt,
        "prompt-1",
        json!({ "benchId": bench, "runId": "r1", "prompt": "hello" }),
    );
    h.rt.runtime.call(desktop(), prompt.clone()).await.unwrap();
    let prompts = h.engine.prompts.load(Ordering::SeqCst);
    let h = h.restart();
    assert_not_found("run.sendPrompt", retry_both(&h, &prompt).await);
    assert_eq!(
        h.engine.prompts.load(Ordering::SeqCst),
        prompts,
        "not re-sent"
    );

    // exchange.send
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    h.start(&bench, "r2").await.unwrap();
    h.call(
        &desktop(),
        OperationId::ExchangeSyncWorkspace,
        json!({"benchId": bench, "request": {
        "worktreePath": h.dir, "revision": 1, "focusedPanelId": "main",
        "panels": [
            {"panelId": "main", "title": "Main", "runId": "r1", "status": "running"},
            {"panelId": "extra", "title": "Extra", "runId": "r2", "status": "running"}
        ]}}),
    )
    .await
    .unwrap();
    let send = command_request(
        OperationId::ExchangeSend,
        "send-1",
        json!({"benchId": bench, "request": {
            "requestId": "q1", "sourcePanelId": "main", "sourceRunId": "r1",
            "targetPanelId": "extra", "targetRunId": "r2",
            "message": "hello", "delivery": "send"}}),
    );
    h.rt.runtime.call(desktop(), send.clone()).await.unwrap();
    let prompts = h.engine.prompts.load(Ordering::SeqCst);
    let h = h.restart();
    assert_not_found("exchange.send", retry_both(&h, &send).await);
    assert_eq!(
        h.engine.prompts.load(Ordering::SeqCst),
        prompts,
        "not re-delivered"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn orchestration_commands_apply_once_across_restart() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    let bootstrap = command_request(
        OperationId::OrchestrationBootstrap,
        "boot-1",
        json!({ "benchId": bench, "worktreePath": h.dir }),
    );
    let session =
        h.rt.runtime
            .call(desktop(), bootstrap.clone())
            .await
            .unwrap();
    let revision = session.output().unwrap()["revision"].clone();
    h.start(&bench, "main-run").await.unwrap();
    let bind = command_request(
        OperationId::OrchestrationBindCoordinator,
        "bind-1",
        json!({ "benchId": bench, "request": {
            "requestId": "bind-req", "panelId": "main-agent-run", "runId": "main-run",
            "state": "active", "expectedRevision": revision } }),
    );
    let bound = h.rt.runtime.call(desktop(), bind.clone()).await.unwrap();
    let delegate = command_request(
        OperationId::OrchestrationDelegateGoal,
        "goal-1",
        json!({ "benchId": bench, "request": {
            "requestId": "goal-req", "goal": "Summarize",
            "expectedRevision": bound.output().unwrap()["revision"] } }),
    );
    h.rt.runtime
        .call(desktop(), delegate.clone())
        .await
        .unwrap();
    let prompts = h.engine.prompts.load(Ordering::SeqCst);
    let applied = orchestration_file(&h);

    let h = h.restart();
    // 재시작은 작업 영역을 바꾸지 않는다(작업대가 닫혀 복구 가능이 되는 표시는 재시작 쪽 규칙이므로 비교는 재시도 전후).
    let at_restart = orchestration_file(&h);
    for (label, request) in [
        ("orchestration.bootstrap", &bootstrap),
        ("orchestration.bindCoordinator", &bind),
        ("orchestration.delegateGoal", &delegate),
    ] {
        assert_not_found(label, retry_both(&h, request).await);
        assert_eq!(
            orchestration_file(&h),
            at_restart,
            "{label}: retry did not write"
        );
    }
    assert_eq!(
        h.engine.prompts.load(Ordering::SeqCst),
        prompts,
        "goal not re-delegated"
    );
    // 작업 영역에는 위임이 한 번 기록돼 있다(재시작 전 기록과 작업 목록이 같다).
    let tasks = |doc: &Value| doc.to_string().matches("Summarize").count();
    assert_eq!(tasks(&at_restart), tasks(&applied));
    assert!(tasks(&applied) >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bench_open_retry_after_restart_opens_a_new_bench() {
    let h = BenchHarness::new(RunScript::default());
    let open = command_request(
        OperationId::BenchOpen,
        "open-1",
        json!({ "workingDirectory": h.dir }),
    );
    let first = h.rt.runtime.call(desktop(), open.clone()).await.unwrap();
    let first_id = first.output().unwrap()["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    let h = h.restart();
    let (local, remote) = retry_both(&h, &open).await;
    let second_id = local.unwrap().output().unwrap()["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(second_id, first_id, "a new bench after restart");
    assert_eq!(
        remote.unwrap().output().unwrap()["benchId"],
        json!(second_id),
        "http retry in the same epoch replays the new bench"
    );
    let stale = h.start(&first_id, "late-run").await.unwrap_err();
    assert_eq!(
        stale.code,
        FaultCode::NotFound,
        "the pre-restart bench is gone"
    );
}
