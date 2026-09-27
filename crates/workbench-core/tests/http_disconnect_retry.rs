//! 042 research R17 공개 게이트: 받아들인 호출은 연결과 무관하게 서버 소유 task에서 끝까지 실행된다.
//! (1) 효과 진행 중 연결을 끊고 같은 서버·작업대에 같은 키로 재시도 → 저장된 결과, 효과 1회(세 경로).
//! (2) 종료 수명(사용자 검토 추가): 연결 단절 → 종료 신호 → `serve`는 지연 효과 완료 뒤에만 반환(경고 간격보다
//!     긴 지연), 멱등 기록 존재. (3) 종료 신호 뒤 새 호출은 `503 unavailable`, 효과 없음.

#![allow(clippy::result_large_err)]

mod support;

use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use serde_json::{json, Value};
use support::{
    command_request, create_request,
    http_harness::{Harness, HarnessOptions, TOKEN_DESKTOP},
    scripted_run_engine::RunScript,
    uuid_key, BenchHarness, TestRuntime,
};
use workbench_core::application::workbench_runtime::CrashPoint;
use workbench_protocol::{AuthenticatedPrincipal, FaultCode, OperationId, Workbench};

const DELAY_MS: u64 = 600;
const ENTRY_WAIT: Duration = Duration::from_secs(10);

/// 효과는 곧바로 일어나고 호출은 그 뒤 `DELAY_MS` 동안 끝나지 않는다: 연결 단절이 효과 뒤·멱등 기록 전에 온다.
fn delayed() -> RunScript {
    RunScript {
        prompt_settle_ms: DELAY_MS,
        ..RunScript::default()
    }
}

async fn orchestration_revision(h: &BenchHarness, bench: &str) -> u64 {
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::OrchestrationGet,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap()["revision"]
        .as_u64()
        .unwrap()
}

async fn bound_orchestration(h: &BenchHarness) -> (String, u64) {
    let bench = h.open().await;
    let session = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": bench, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    h.start(&bench, "main-run").await.unwrap();
    let bound = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::OrchestrationBindCoordinator,
            json!({ "benchId": bench, "request": {
                "requestId": uuid::Uuid::new_v4().to_string(), "panelId": "main-agent-run",
                "runId": "main-run", "state": "active",
                "expectedRevision": session["revision"] } }),
        )
        .await
        .unwrap();
    (bench, bound["revision"].as_u64().unwrap())
}

fn output(reply: workbench_protocol::CallReply) -> Value {
    reply.output().cloned().unwrap_or(Value::Null)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disconnected_prompt_retry_applies_once() {
    let h = BenchHarness::new(delayed());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    let before = h.engine.prompts.load(Ordering::SeqCst);
    let request = command_request(
        OperationId::RunSendPrompt,
        &uuid_key(),
        json!({ "benchId": bench, "runId": "r1", "prompt": "hello-disconnect" }),
    );
    // 이 요청의 효과가 난 뒤·결과 기록 전(settle 구간)에 연결을 끊는다.
    harness
        .send_then_disconnect(
            TOKEN_DESKTOP,
            &request,
            h.engine
                .wait_applied(|l| l == "prompt:r1:hello-disconnect", ENTRY_WAIT),
        )
        .await;
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), before + 1);
    // 원 호출이 아직 진행 중일 때 같은 키로 재시도 → 끝날 때까지 기다려 저장된 결과.
    let retried = harness.call(Some(TOKEN_DESKTOP), &request).await;
    assert!(retried.is_ok(), "{retried:?}");
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), before + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disconnected_orchestration_write_retry_applies_once() {
    let h = BenchHarness::new(delayed());
    let (bench, revision) = bound_orchestration(&h).await;
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    let before = h.engine.prompts.load(Ordering::SeqCst);
    let request = command_request(
        OperationId::OrchestrationDelegateGoal,
        &uuid_key(),
        json!({ "benchId": bench, "request": {
            "requestId": "goal-1", "goal": "goal-042-disconnect", "expectedRevision": revision } }),
    );
    harness
        .send_then_disconnect(
            TOKEN_DESKTOP,
            &request,
            h.engine.wait_applied(
                |l| l.starts_with("prompt:main-run:") && l.contains("goal-042-disconnect"),
                ENTRY_WAIT,
            ),
        )
        .await;
    let retried = harness.call(Some(TOKEN_DESKTOP), &request).await;
    let retried = output(retried.expect("retry returns the stored result"));
    assert!(retried["rootTaskId"].is_string(), "{retried}");
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), before + 1);
    assert_eq!(
        orchestration_revision(&h, &bench).await,
        revision + 1,
        "the workspace file changed exactly once"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disconnected_run_start_retry_starts_once() {
    let h = BenchHarness::new(RunScript {
        start_settle_ms: DELAY_MS,
        ..RunScript::default()
    });
    let bench = h.open().await;
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    let request = command_request(
        OperationId::RunStart,
        &uuid_key(),
        json!({ "benchId": bench, "request": { "goal": "g", "agentId": "codex", "runId": "r1" } }),
    );
    harness
        .send_then_disconnect(
            TOKEN_DESKTOP,
            &request,
            h.engine.wait_applied(|l| l == "start:r1", ENTRY_WAIT),
        )
        .await;
    // run.start의 진행 중 재시도는 오늘 in-process와 같이 retryable conflict(`outcome: unknown`)다 — 클라이언트는
    // 같은 키로 다시 시도해 저장된 결과를 받는다.
    let mut in_progress = 0;
    let retried = loop {
        match harness.call(Some(TOKEN_DESKTOP), &request).await {
            Err(fault) if fault.code == FaultCode::Conflict && fault.retryable => {
                in_progress += 1;
                assert!(in_progress < 100, "never settled");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            other => break other,
        }
    };
    assert!(retried.is_ok(), "{retried:?}");
    assert!(
        in_progress > 0,
        "the first retry overlapped the running start"
    );
    assert_eq!(h.engine.starts.load(Ordering::SeqCst), 1);
    assert_eq!(h.engine.run_count(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_drains_a_disconnected_call_before_returning() {
    let h = BenchHarness::new(delayed());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let mut harness = Harness::spawn_with(
        h.rt.runtime.clone() as Arc<dyn Workbench>,
        HarnessOptions {
            // 경고 간격(50ms)보다 지연(600ms)이 길다: drain은 경고만 내고 조기 반환하면 안 된다.
            drain_warn_after: Duration::from_millis(50),
            ..HarnessOptions::default()
        },
    )
    .await;
    let before = h.engine.prompts.load(Ordering::SeqCst);
    let key = uuid_key();
    let request = command_request(
        OperationId::RunSendPrompt,
        &key,
        json!({ "benchId": bench, "runId": "r1", "prompt": "hello-shutdown" }),
    );
    // 효과가 난 뒤·결과 기록 전 구간 진입을 확인한 그 지점에서 연결을 끊고 곧바로 종료 신호.
    harness
        .send_then_disconnect(
            TOKEN_DESKTOP,
            &request,
            h.engine
                .wait_applied(|l| l == "prompt:r1:hello-shutdown", ENTRY_WAIT),
        )
        .await;
    assert_eq!(harness.calls.active(), 1, "the call was accepted");
    let served = harness.begin_shutdown();
    // 남은 settle(약 600ms)이 경고 간격(50ms)의 여러 배다 — 그 사이 serve가 끝나면 조기 반환이다.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !served.is_finished(),
        "serve returned while an accepted call was running"
    );
    served.await.expect("serve task").expect("serve");
    // serve 반환 시점에 호출과 멱등 기록이 끝나 있다.
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), before + 1);
    assert_eq!(harness.calls.active(), 0);
    let replay =
        h.rt.runtime
            .call(AuthenticatedPrincipal::desktop(), request)
            .await;
    assert!(
        replay.is_ok(),
        "same key returns the stored result: {replay:?}"
    );
    assert_eq!(
        h.engine.prompts.load(Ordering::SeqCst),
        before + 1,
        "replay did not apply again"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn calls_after_the_shutdown_signal_are_rejected_without_effect() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    let before = h.engine.prompts.load(Ordering::SeqCst);
    harness.calls.close();
    let rejected = harness
        .call(
            Some(TOKEN_DESKTOP),
            &command_request(
                OperationId::RunSendPrompt,
                &uuid_key(),
                json!({ "benchId": bench, "runId": "r1", "prompt": "late" }),
            ),
        )
        .await
        .expect_err("closing server rejects new calls");
    assert_eq!(rejected.code, FaultCode::Unavailable);
    assert_eq!(
        rejected.message,
        workbench_server::drain::MESSAGE_SHUTTING_DOWN
    );
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), before);
}

/// 영속 ledger 경로(intent-first, `spawn_blocking`): JSON 저장 뒤·ledger 확정 전 구간에서 연결을 끊고 같은 키로
/// 재시도 → 진행 중이면 retryable conflict, 끝나면 저장된 결과. 프로젝트는 하나만 생긴다. 14개 영속 operation이
/// 이 경로를 공유한다(`application/intent_first.rs`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disconnected_ledger_write_retry_applies_once() {
    let rt = TestRuntime::new();
    let dir = rt.dir.path().join("wt");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.to_string_lossy().into_owned();
    rt.runtime.hooks().set_pause(Some((
        CrashPoint::AfterJsonSave,
        Duration::from_millis(DELAY_MS),
    )));
    let harness = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let key = uuid_key();
    let request = create_request(&key, "Disconnected", &dir);
    harness
        .send_then_disconnect(
            TOKEN_DESKTOP,
            &request,
            rt.runtime.hooks().wait_paused(&key, ENTRY_WAIT),
        )
        .await;
    assert_eq!(
        rt.projects().len(),
        1,
        "the JSON write happened before the disconnect"
    );
    let mut in_progress = 0;
    let retried = loop {
        match harness.call(Some(TOKEN_DESKTOP), &request).await {
            Err(fault) if fault.code == FaultCode::Conflict && fault.retryable => {
                in_progress += 1;
                assert!(in_progress < 100, "never settled");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            other => break other,
        }
    };
    let retried = output(retried.expect("retry returns the stored result"));
    assert!(
        in_progress > 0,
        "the first retry overlapped the running write"
    );
    let projects = rt.projects();
    assert_eq!(projects.len(), 1, "created once");
    assert_eq!(retried["id"], projects[0]["id"]);
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

/// agent 주체 변경(`Scope::RunOwner`, 15개 공유): coordinator가 자식 과제를 만들고 자식 run이 기동된 뒤·결과
/// 기록 전에 연결이 끊긴다 → 같은 키 재시도 → 저장된 결과, 자식은 한 번만 기동.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn disconnected_agent_child_creation_retry_starts_one_child() {
    let h = BenchHarness::new(RunScript {
        start_settle_ms: DELAY_MS,
        ..RunScript::default()
    });
    git_init(&h.dir);
    let (_bench, _) = {
        let bench = h.open().await;
        let session = h
            .call(
                &AuthenticatedPrincipal::desktop(),
                OperationId::OrchestrationBootstrap,
                json!({ "benchId": bench, "worktreePath": h.dir }),
            )
            .await
            .unwrap();
        h.start(&bench, "coord").await.unwrap();
        let bound = h
            .call(
                &AuthenticatedPrincipal::desktop(),
                OperationId::OrchestrationBindCoordinator,
                json!({ "benchId": bench, "request": {
                    "requestId": "bind-1", "panelId": "main-agent-run", "runId": "coord",
                    "state": "active", "expectedRevision": session["revision"] } }),
            )
            .await
            .unwrap();
        (bench, bound)
    };
    let harness = Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await;
    let agent = AuthenticatedPrincipal::agent("coord");
    let token = Harness::token_string(&agent);
    let starts = h.engine.starts.load(Ordering::SeqCst);
    let request = command_request(
        OperationId::OrchestrationCreateChildTask,
        &uuid_key(),
        json!({ "runId": "coord", "arguments": {
            "requestId": "child-req", "title": "task",
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read the repo", "expectedResult": "summary" } }),
    );
    let child = harness
        .send_then_disconnect(
            &token,
            &request,
            h.engine.wait_applied(
                |l| l.starts_with("start:") && l != "start:coord",
                ENTRY_WAIT,
            ),
        )
        .await;
    let retried = output(
        harness
            .call(Some(&token), &request)
            .await
            .expect("retry returns the stored result"),
    );
    assert_eq!(
        format!("start:{}", retried["runId"].as_str().unwrap()),
        child,
        "the stored result names the child started by the original request"
    );
    assert_eq!(
        h.engine.starts.load(Ordering::SeqCst),
        starts + 1,
        "one child"
    );
}
