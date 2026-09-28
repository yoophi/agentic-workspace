//! 044 T040 (research R7·R14, contracts/drain-classification.md): 비우기·정지 중 Workbench 호출 입구 판정.
//! - `draining`: 새 작업(N)은 입력과 상관없이 `draining`(`notApplied`)으로 거절한다. 조회(Q)·끝내는 제어(C)는 입구에서
//!   막지 않는다(다른 까닭의 fault는 있을 수 있다). 이어 가기(K)는 입구가 조건을 본다.
//! - `stopping`: 모든 새 호출을 `unavailable`로 거절한다(HTTP 서버는 503).
//! - 대기 task 배정(K): 비우기 시작 전에 만든 준비(Ready) task면 받는다. 준비 task가 아니면 `draining`.
//!
//! 표는 `OperationId::ALL`을 돈다 — 분류가 바뀌거나 새 operation이 생기면 이 시험이 같이 본다.

#![allow(clippy::result_large_err)]

mod support;

use std::sync::Arc;

use serde_json::{json, Value};
use support::{command_request, query_request, scripted_run_engine::RunScript, BenchHarness};
use workbench_core::application::{
    authorization::required_scopes,
    drain::{drain_class, DrainClass},
    work_gate::DrainMode,
};
use workbench_protocol::{
    operations::spec_for, AuthenticatedPrincipal, FaultCode, OperationId, OperationKind, Outcome,
    Workbench, WorkbenchFault,
};

/// operation을 부를 수 있는 주체: 소유자 scope로 되면 소유자, 아니면 agent(agent 전용 도구).
fn principal_for(operation: OperationId) -> AuthenticatedPrincipal {
    let owner = AuthenticatedPrincipal::owner();
    if required_scopes(operation)
        .iter()
        .all(|scope| owner.has_scope(*scope))
    {
        owner
    } else {
        AuthenticatedPrincipal::agent("r-entry")
    }
}

async fn raw_call(
    h: &BenchHarness,
    operation: OperationId,
    input: Value,
) -> Result<Value, WorkbenchFault> {
    let request = if matches!(spec_for(operation).kind, OperationKind::Query) {
        query_request(operation, input)
    } else {
        command_request(operation, &format!("k-{operation}"), input)
    };
    h.rt.runtime
        .call(principal_for(operation), request)
        .await
        .map(|reply| reply.output().cloned().unwrap_or(Value::Null))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn while_draining_new_work_is_refused_at_the_entry_and_queries_and_controls_are_not() {
    let h = BenchHarness::new(RunScript::default());
    h.rt.runtime.work_gate().begin_drain(DrainMode::Wait);
    let mut checked = 0;
    for operation in OperationId::ALL {
        let class = drain_class(operation);
        if class == DrainClass::Continuation {
            continue; // 조건부: 아래 시험들이 양쪽을 본다.
        }
        let result = raw_call(&h, operation, json!({})).await;
        match class {
            DrainClass::NewWork => {
                let fault =
                    result.expect_err(&format!("{operation} must be refused while draining"));
                assert_eq!(
                    (fault.code, fault.outcome),
                    (FaultCode::Draining, Outcome::NotApplied),
                    "{operation}: {fault:?}"
                );
            }
            DrainClass::Query | DrainClass::Control => {
                if let Err(fault) = &result {
                    assert_ne!(
                        fault.code,
                        FaultCode::Draining,
                        "{operation} must pass the drain entry: {fault:?}"
                    );
                }
            }
            DrainClass::Continuation => unreachable!(),
        }
        checked += 1;
    }
    assert_eq!(
        checked,
        OperationId::ALL.len() - 2,
        "every non-continuation operation was judged"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn while_stopping_every_new_call_is_unavailable() {
    let h = BenchHarness::new(RunScript::default());
    h.rt.runtime.work_gate().force_stop();
    for operation in [
        OperationId::ProjectList,
        OperationId::ServerStatus,
        OperationId::ProjectCreate,
        OperationId::RunCancel,
    ] {
        let fault = raw_call(&h, operation, json!({}))
            .await
            .expect_err(&format!("{operation} while stopping"));
        assert_eq!(fault.code, FaultCode::Unavailable, "{operation}: {fault:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_plain_prompt_is_new_work_while_draining() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    h.rt.runtime.work_gate().begin_drain(DrainMode::Wait);
    let fault = h
        .keyed(
            OperationId::RunSendPrompt,
            "k-plain",
            json!({"benchId": bench, "runId": "r1", "prompt": "more"}),
        )
        .await
        .expect_err("a prompt without continuation is new work");
    assert_eq!(
        (fault.code, fault.outcome),
        (FaultCode::Draining, Outcome::NotApplied)
    );
}

// --- 대기 task 배정(K) ---

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

/// 동시 한도 1: 첫 과제는 보고로 끝났고 둘째는 대기(ready). (h, bench, 끝난 task, 대기 task).
async fn running_and_ready() -> (Arc<BenchHarness>, String, String, String) {
    let h = Arc::new(BenchHarness::with(
        |a| a.orchestration.max_concurrent_children = 1,
        RunScript::default(),
    ));
    git_init(&h.dir);
    let bench = h.open().await;
    let boot = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": bench, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    h.start(&bench, "coord").await.unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::OrchestrationBindCoordinator,
        json!({ "benchId": bench, "request": { "requestId": "bind-1", "panelId": "main-agent-run", "runId": "coord", "state": "active", "expectedRevision": boot["revision"] } }),
    )
    .await
    .unwrap();
    let create = |key: &'static str| {
        let h = Arc::clone(&h);
        async move {
            tool(&h, "coord", OperationId::OrchestrationCreateChildTask, json!({
                "requestId": key, "title": format!("task {key}"),
                "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
                "objective": "read", "expectedResult": "summary"
            }))
            .await
            .unwrap()
        }
    };
    let first = create("c0").await;
    let second = create("c1").await;
    let third = create("c2").await;
    assert_eq!(second["queued"], true);
    tool(
        &h,
        first["runId"].as_str().unwrap(),
        OperationId::OrchestrationReportResult,
        json!({ "requestId": "r0", "summary": "done" }),
    )
    .await
    .unwrap();
    // 첫째는 끝났다(대기 task가 아니다). 둘째·셋째는 아직 시작하지 않은 대기 task.
    let _ = third;
    (
        h,
        bench,
        first["taskId"].as_str().unwrap().to_owned(),
        second["taskId"].as_str().unwrap().to_owned(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ready_task_made_before_the_drain_can_still_be_assigned() {
    let (h, _bench, _other, ready) = running_and_ready().await;
    h.rt.runtime.work_gate().begin_drain(DrainMode::Wait);
    let assigned = tool(
        &h,
        "coord",
        OperationId::OrchestrationAssignChildTask,
        json!({ "taskId": ready, "requestId": "a-ready" }),
    )
    .await
    .expect("K: a ready task created before the drain is a continuation");
    assert!(
        assigned["runId"].is_string() || assigned["queued"] == true,
        "{assigned}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn assigning_a_task_that_is_not_waiting_is_new_work_while_draining() {
    let (h, _bench, finished, _ready) = running_and_ready().await;
    h.rt.runtime.work_gate().begin_drain(DrainMode::Wait);
    let before = h.engine.launches.load(std::sync::atomic::Ordering::SeqCst);
    for (task_id, key) in [
        (finished.as_str(), "a-finished"),
        ("no-such-task", "a-unknown"),
    ] {
        let fault = tool(
            &h,
            "coord",
            OperationId::OrchestrationAssignChildTask,
            json!({ "taskId": task_id, "requestId": key }),
        )
        .await
        .expect_err("a task that is not a waiting task is not a continuation");
        assert_eq!(
            (fault.code, fault.outcome),
            (FaultCode::Draining, Outcome::NotApplied),
            "{task_id}: {fault:?}"
        );
    }
    let launches = h.engine.launches.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(launches, before, "no launch for a refused assignment");
}
