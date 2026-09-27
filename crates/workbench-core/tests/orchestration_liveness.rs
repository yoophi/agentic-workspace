//! 041 liveness ③④와 서버 안 후처리(research R2·R8). ③ `cancelTask`가 엔진 취소 → 종료 처리를 같은 task에서 inline
//! 호출해도 교착이 없다. ④ 자식 기동(엔진 지연) 중 작업대를 닫아도 제한 시간 안에 닫기가 끝나고 남은 run이 없다.
//! 읽기 전용 자식이 worktree를 바꾸고 끝나면 과제가 실패하고, 동시 실행 한도를 넘은 과제는 대기했다가 자리가 나면
//! 진행된다. (①②는 `orchestration_agent.rs`, ⑤는 `orchestration_flow.rs`.)

#![allow(clippy::result_large_err)]

mod support;

use std::{sync::Arc, time::Duration};

use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::application::workbench_runtime::RuntimeAdapters;
use workbench_protocol::{AuthenticatedPrincipal, OperationId, WorkbenchFault};

fn desktop() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop()
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

async fn fixture(
    configure: impl FnOnce(&mut RuntimeAdapters),
    script: RunScript,
) -> (Arc<BenchHarness>, String) {
    let h = Arc::new(BenchHarness::with(configure, script));
    git_init(&h.dir);
    let bench = h.open().await;
    let session = h
        .call(
            &desktop(),
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": bench, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    h.start(&bench, "coord").await.unwrap();
    h.call(
        &desktop(),
        OperationId::OrchestrationBindCoordinator,
        json!({ "benchId": bench, "request": {
            "requestId": "bind-1", "panelId": "main-agent-run", "runId": "coord",
            "state": "active", "expectedRevision": session["revision"] } }),
    )
    .await
    .unwrap();
    (h, bench)
}

async fn tool(
    h: &BenchHarness,
    run: &str,
    operation: OperationId,
    arguments: Value,
) -> Result<Value, WorkbenchFault> {
    h.call(
        &AuthenticatedPrincipal::agent(run),
        operation,
        json!({ "runId": run, "arguments": arguments }),
    )
    .await
}

async fn create_child(h: &BenchHarness, key: &str) -> Value {
    tool(
        h,
        "coord",
        OperationId::OrchestrationCreateChildTask,
        json!({
            "requestId": key, "title": format!("task {key}"),
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read the repo", "expectedResult": "summary"
        }),
    )
    .await
    .unwrap()
}

async fn session(h: &BenchHarness, bench: &str) -> Value {
    h.call(
        &desktop(),
        OperationId::OrchestrationGet,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap()
}

fn task<'a>(session: &'a Value, task_id: &str) -> &'a Value {
    session["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["id"] == task_id)
        .unwrap()
}

/// liveness ③: 살아 있는 자식의 과제 취소 — 가짜 엔진은 취소 안에서 종료 이벤트를 inline으로 낸다(종료 hook이 같은
/// task에서 불린다). 제한 시간 안에 끝나고 과제는 취소된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_a_live_child_with_inline_termination_does_not_deadlock() {
    let (h, bench) = fixture(|_| {}, RunScript::default()).await;
    let created = create_child(&h, "c1").await;
    let task_id = created["taskId"].as_str().unwrap().to_owned();
    let run_id = created["runId"].as_str().unwrap().to_owned();
    let revision = task(&session(&h, &bench).await, &task_id)["revision"].clone();
    let cancelled = tokio::time::timeout(
        Duration::from_secs(10),
        h.call(
            &desktop(),
            OperationId::OrchestrationCancelTask,
            json!({ "benchId": bench, "request": {
                "requestId": "cancel-1", "taskId": task_id, "expectedRevision": revision } }),
        ),
    )
    .await
    .expect("cancel finished in time")
    .unwrap();
    assert_eq!(task(&cancelled, &task_id)["status"], "cancelled");
    assert_eq!(
        h.rt.runtime.run_engine().active_owner_of(&run_id).await,
        None
    );
}

/// liveness ④: 자식 기동이 엔진에서 지연되는 동안 작업대를 닫는다. 닫기는 기동이 끝나기를 기다린 뒤(입장권) 소유
/// run을 모두 취소하고 제한 시간 안에 끝난다. 작업 영역은 복구 가능이 된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn closing_the_bench_during_a_slow_child_launch_finishes_without_leftover_runs() {
    let (h, bench) = fixture(
        |_| {},
        RunScript {
            start_delay_ms: 400,
            ..RunScript::default()
        },
    )
    .await;
    let launch = {
        let h = Arc::clone(&h);
        tokio::spawn(async move {
            tool(
                &h,
                "coord",
                OperationId::OrchestrationCreateChildTask,
                json!({
                    "requestId": "slow", "title": "slow",
                    "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
                    "objective": "read", "expectedResult": "summary"
                }),
            )
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    tokio::time::timeout(Duration::from_secs(10), h.close(&bench))
        .await
        .expect("close finished in time")
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(10), launch)
        .await
        .expect("launch returned");
    assert_eq!(
        h.engine.runs_owned_by(&bench),
        0,
        "no run survives the close"
    );
    let other = h.open().await;
    let recoverable = h
        .call(
            &desktop(),
            OperationId::OrchestrationListRecoverable,
            json!({ "benchId": other, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    assert_eq!(recoverable.as_array().unwrap().len(), 1);
}

/// 읽기 전용 자식이 worktree를 바꾼 채 끝나면(종료 hook) 과제가 실패한다(오늘 사유·문구). 조건부 갱신은 blocking
/// pool에서 돌므로 잠시 기다린다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_child_that_changes_the_worktree_fails_its_task() {
    let (h, bench) = fixture(|_| {}, RunScript::default()).await;
    let created = create_child(&h, "c1").await;
    let task_id = created["taskId"].as_str().unwrap().to_owned();
    let run_id = created["runId"].as_str().unwrap().to_owned();
    std::fs::write(std::path::Path::new(&h.dir).join("changed.txt"), "oops").unwrap();
    h.engine
        .finish(&run_id, &h.rt.runtime.benches().run_sink(&bench));
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let current = session(&h, &bench).await;
        let task = task(&current, &task_id);
        if task["status"] == "failed" {
            assert_eq!(task["failure"]["code"], "readOnlyViolation");
            assert_eq!(
                task["failure"]["message"],
                "The read-only child changed the worktree; changes were preserved for review."
            );
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "task never failed: {task}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// 동시 실행 한도(자식 1)를 넘은 과제는 대기하고, 앞 과제가 결과를 보고하면 다음 과제가 준비되어 coordinator가
/// 배정하면 진행된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tasks_over_the_concurrency_limit_wait_then_proceed() {
    let (h, _bench) = fixture(
        |adapters| adapters.orchestration.max_concurrent_children = 1,
        RunScript::default(),
    )
    .await;
    let first = create_child(&h, "c1").await;
    let second = create_child(&h, "c2").await;
    assert_eq!(second["queued"], true, "{second}");
    let reported = tool(
        &h,
        first["runId"].as_str().unwrap(),
        OperationId::OrchestrationReportResult,
        json!({ "requestId": "r1", "summary": "done" }),
    )
    .await
    .unwrap();
    assert_eq!(reported["nextReadyTaskId"], second["taskId"]);
    let assigned = tool(
        &h,
        "coord",
        OperationId::OrchestrationAssignChildTask,
        json!({ "taskId": second["taskId"] }),
    )
    .await
    .unwrap();
    assert!(assigned["runId"].is_string(), "{assigned}");
}
