//! 044 T041 (research R7·R9·R10·R14, contracts/server-lifecycle.md §5·§6, data-model ActiveWork): 정지 판정의 core 부분.
//! - `server.status`는 모든 필드를 파생한다(`notYetDerived` 빈 배열).
//! - ledger `unknown`은 활동 작업이 **아니다**: `unresolvedOperations`에만 보이고 wait·유휴 정지를 막지 않는다.
//!   (반대로 파생하지 않은 `null`만 모르는 활동으로 본다 — `ActiveWorkDto::blocks_stop` 단위 시험.)
//! - `default`: 활동 작업이 있으면 `conflict` + `details.activeWork`. `wait`: 비운 뒤 활동이 0이 되면 정지.
//!   `force`: 작업대를 모두 닫고 정지. 임대 획득은 유휴 비우기를 서빙으로 되돌린다.

#![allow(clippy::result_large_err)]

mod support;

use std::time::Duration;

use serde_json::{json, Value};
use support::{command_request, query_request, scripted_run_engine::RunScript, BenchHarness};
use workbench_core::{
    application::work_gate::{DrainMode, GateState},
    ports::operation_ledger::{LedgerKey, LedgerState, NewLedgerEntry, OperationLedger},
};
use workbench_protocol::{
    operations::server::ActiveWorkDto, AuthenticatedPrincipal, FaultCode, IdempotencyKey,
    OperationId, PrincipalKind, RequestId, Workbench, WorkbenchFault, CONTRACT_REVISION,
};

async fn owner(
    h: &BenchHarness,
    operation: OperationId,
    input: Value,
) -> Result<Value, WorkbenchFault> {
    let request = if operation == OperationId::ServerStatus {
        query_request(operation, input)
    } else {
        command_request(operation, &support::uuid_key(), input)
    };
    h.rt.runtime
        .call(AuthenticatedPrincipal::owner(), request)
        .await
        .map(|reply| reply.output().cloned().unwrap_or(Value::Null))
}

async fn status(h: &BenchHarness) -> Value {
    owner(h, OperationId::ServerStatus, json!({}))
        .await
        .unwrap()
}

fn active_work(status: &Value) -> ActiveWorkDto {
    serde_json::from_value(status["activeWork"].clone()).unwrap()
}

async fn stop(h: &BenchHarness, mode: &str) -> Result<Value, WorkbenchFault> {
    owner(h, OperationId::ServerStop, json!({ "mode": mode })).await
}

/// 이전 세대에서 적용 여부를 알 수 없게 끝난 변경(수정형은 재시작 판정이 항상 `unknown`)을 만든 harness.
fn with_unknown_record() -> BenchHarness {
    let h = BenchHarness::new(RunScript::default());
    h.rt.runtime
        .ledger()
        .begin(NewLedgerEntry {
            key: LedgerKey {
                principal_kind: PrincipalKind::Desktop,
                operation: OperationId::SavedPromptUpdate,
                contract_revision: CONTRACT_REVISION,
                idempotency_key: IdempotencyKey::new("interrupted-update").unwrap(),
            },
            input_fingerprint: "fp".into(),
            aggregate: "saved_prompts".into(),
            reserved_resource_id: None,
            request_id: RequestId::new("r-interrupted").unwrap(),
        })
        .unwrap();
    let h = h.restart();
    assert_eq!(
        h.rt.runtime
            .ledger()
            .count_by_state(LedgerState::Unknown)
            .unwrap(),
        1
    );
    assert_eq!(
        h.rt.runtime
            .ledger()
            .count_by_state(LedgerState::Pending)
            .unwrap(),
        0
    );
    h
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_derives_every_field() {
    let h = BenchHarness::new(RunScript::default());
    let status = status(&h).await;
    assert_eq!(status["notYetDerived"], json!([]), "{status}");
    for field in [
        "orchestrationTasks",
        "queuedTasks",
        "pendingExchanges",
        "pendingNotifications",
        "pendingOperations",
    ] {
        assert_eq!(status["activeWork"][field], 0, "{field}: {status}");
    }
    assert_eq!(status["unresolvedOperations"], 0);
    assert_eq!(status["undeliverableExchanges"], json!([]));
    assert_eq!(status["failedExchangeDeliveries"], json!([]));
    assert!(!active_work(&status).blocks_stop(), "{status}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_unknown_ledger_records_are_reported_but_do_not_block_a_wait_stop() {
    let h = with_unknown_record();
    let status = status(&h).await;
    assert_eq!(status["unresolvedOperations"], 1, "{status}");
    assert_eq!(status["activeWork"]["pendingOperations"], 0, "{status}");
    assert!(
        !active_work(&status).blocks_stop(),
        "unknown is not active work: {status}"
    );
    let stopped = stop(&h, "wait").await.unwrap();
    assert_eq!(stopped["state"], "stopping", "{stopped}");
    assert_eq!(h.rt.runtime.work_gate().state(), GateState::Stopping);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_unknown_ledger_records_do_not_block_an_idle_stop() {
    let h = with_unknown_record();
    let control = h.rt.runtime.server_control();
    assert!(
        control.tick(Duration::ZERO).await,
        "idle with only unknown records stops"
    );
    assert_eq!(h.rt.runtime.work_gate().state(), GateState::Stopping);
    assert!(*control.stopped().borrow());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_stop_is_refused_with_the_blockers_and_wait_stops_once_the_turn_ends() {
    let h = BenchHarness::new(RunScript {
        permission_id: Some("p1".into()),
        ..RunScript::default()
    });
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let fault = stop(&h, "default")
        .await
        .expect_err("a busy run blocks the default stop");
    assert_eq!(fault.code, FaultCode::Conflict, "{fault:?}");
    let blockers: ActiveWorkDto =
        serde_json::from_value(fault.details.as_ref().unwrap()["activeWork"].clone()).unwrap();
    assert_eq!(blockers.busy_runs, 1, "{blockers:?}");
    assert_eq!(
        h.rt.runtime.work_gate().state(),
        GateState::Serving,
        "a refused default stop changes nothing"
    );

    let draining = stop(&h, "wait").await.unwrap();
    assert_eq!(draining["state"], "drainingWait");
    let control = h.rt.runtime.server_control();
    assert!(
        !control.tick(Duration::from_secs(600)).await,
        "the permission wait is still a busy run"
    );
    // 권한 응답(C)은 비우기 중에도 받는다. turn이 끝나면 세션이 살아 있어도 멈춘다.
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::RunRespondPermission,
        json!({"benchId": bench, "runId": "r1", "permissionId": "p1", "optionId": "allow"}),
    )
    .await
    .unwrap();
    assert!(
        control.tick(Duration::from_secs(600)).await,
        "no busy run left"
    );
    assert_eq!(
        h.engine.run_count(),
        1,
        "the idle session was alive when the stop was decided"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn force_closes_every_bench_and_stops() {
    let h = BenchHarness::new(RunScript {
        permission_id: Some("p1".into()),
        ..RunScript::default()
    });
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let stopped = stop(&h, "force").await.unwrap();
    assert_eq!(stopped["state"], "stopping");
    assert_eq!(h.engine.runs_owned_by(&bench), 0, "force cancels the runs");
    assert!(h.rt.runtime.benches().registry.open_benches().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lease_keeps_the_server_serving_and_cancels_an_idle_drain() {
    let h = BenchHarness::new(RunScript::default());
    let control = h.rt.runtime.server_control();
    let lease = owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "app"}),
    )
    .await
    .unwrap();
    assert!(!control.tick(Duration::ZERO).await, "a lease is not idle");
    assert_eq!(h.rt.runtime.work_gate().state(), GateState::Serving);
    assert!(status(&h).await.get("idleSince").is_none());

    owner(
        &h,
        OperationId::LeaseRelease,
        json!({"leaseId": lease["leaseId"]}),
    )
    .await
    .unwrap();
    h.rt.runtime.work_gate().begin_drain(DrainMode::Idle);
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "cli", "clientId": "c"}),
    )
    .await
    .unwrap();
    assert_eq!(
        h.rt.runtime.work_gate().state(),
        GateState::Serving,
        "a lease returns an idle drain to serving"
    );
    h.rt.runtime.work_gate().begin_drain(DrainMode::Wait);
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "cli", "clientId": "c2"}),
    )
    .await
    .unwrap();
    assert_eq!(
        h.rt.runtime.work_gate().state(),
        GateState::Draining(DrainMode::Wait),
        "a wait drain does not return"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_clock_starts_only_when_quiet_and_stops_after_the_timeout() {
    let h = BenchHarness::new(RunScript::default());
    let control = h.rt.runtime.server_control();
    assert!(!control.tick(Duration::from_secs(600)).await);
    let status = status(&h).await;
    assert!(
        status["idleSince"].is_string(),
        "the idle clock started: {status}"
    );
    assert_eq!(
        h.rt.runtime.work_gate().state(),
        GateState::Serving,
        "not before the timeout"
    );
    assert!(
        control.tick(Duration::ZERO).await,
        "idle for at least the timeout → stops"
    );
}

// --- 미소비 교환: 데스크톱 임대가 있을 때만 활동 작업(R7 검증 8) ---

async fn accepted_exchange(h: &BenchHarness) -> String {
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    h.start(&bench, "r2").await.unwrap();
    let desktop = AuthenticatedPrincipal::desktop();
    h.call(
        &desktop,
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
    h.call(&desktop, OperationId::ExchangeSend, json!({"benchId": bench, "request": {
        "requestId": "q1", "sourcePanelId": "main", "sourceRunId": "r1",
        "targetPanelId": "extra", "targetRunId": "r2", "message": "hello peer", "delivery": "queue"}})).await.unwrap();
    h.call(
        &desktop,
        OperationId::ExchangeAcknowledge,
        json!({"benchId": bench, "request": {
        "requestId": "q1", "targetPanelId": "extra", "outcome": "delivered", "reason": null}}),
    )
    .await
    .unwrap();
    bench
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unconsumed_exchange_without_a_desktop_lease_is_undeliverable_and_does_not_block() {
    let h = BenchHarness::new(RunScript::default());
    accepted_exchange(&h).await;
    let status = status(&h).await;
    assert_eq!(status["undeliverableExchanges"], json!(["q1"]), "{status}");
    assert_eq!(status["activeWork"]["pendingExchanges"], 0);
    let stopped = stop(&h, "wait").await.unwrap();
    assert_eq!(
        stopped["state"], "stopping",
        "nobody could deliver it: {stopped}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unconsumed_exchange_with_a_desktop_lease_blocks_until_it_is_delivered() {
    let h = BenchHarness::new(RunScript::default());
    let bench = accepted_exchange(&h).await;
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "app"}),
    )
    .await
    .unwrap();
    let status = status(&h).await;
    assert_eq!(status["activeWork"]["pendingExchanges"], 1, "{status}");
    assert_eq!(status["undeliverableExchanges"], json!([]));
    assert_eq!(stop(&h, "wait").await.unwrap()["state"], "drainingWait");
    let control = h.rt.runtime.server_control();
    assert!(
        !control.tick(Duration::ZERO).await,
        "the desktop can still deliver it"
    );
    // K: 확인된 교환의 전달 prompt(continuation)는 비우기 중에도 받는다.
    h.keyed(
        OperationId::RunSendPrompt,
        "exchange-delivery:q1",
        json!({"benchId": bench, "runId": "r2", "prompt": "hello peer",
        "continuation": {"exchangeRequestId": "q1"}}),
    )
    .await
    .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !control.tick(Duration::ZERO).await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "never stopped after the delivery"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

// 044 메인 세션 검토(사용자 검토 반영): 비우기 전에 만든 준비(Ready) task는 coordinator agent만 `assignChildTask`(K)로
// 배정한다(coordinator 역할의 agent 도구 — 소유자·데스크톱·CLI는 부를 수 없다). coordinator는 turn 안에서만 배정하고,
// turn은 바쁜 실행이나 미전달 알림으로만 생긴다(비우는 중 사용자 prompt는 N). 그래서 준비 task는 **coordinator가 살아
// 있고, 바쁘거나 전달할 알림이 있을 때만** 활동으로 센다. 아니면 `deferredTasks`로 보고한다. task는 저장돼 있어 복구할
// 수 있다. 데스크톱 임대만으로는 배정이 일어나지 않는다(교환과 다른 점).

use workbench_core::application::orchestration::notification_dispatcher::{
    DispatchAction, DispatchPoint, DispatchProbe,
};

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

/// 동시 한도 1에서 첫 task가 결과를 보고해 둘째 task가 배정 전 준비(Ready) 상태가 된 harness. coordinator run `coord`는
/// 살아 있지만 turn이 없다. `probe`가 있으면 보고 전에 전달기에 설치한다(첫 poll에서 붙잡기).
async fn with_ready_task(probe: Option<DispatchProbe>) -> (BenchHarness, String, String) {
    let h = BenchHarness::with(
        |adapters| adapters.orchestration.max_concurrent_children = 1,
        RunScript::default(),
    );
    git_init(&h.dir);
    let bench = h.open().await;
    let desktop = AuthenticatedPrincipal::desktop();
    let bootstrapped = h
        .call(
            &desktop,
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": bench, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    h.start(&bench, "coord").await.unwrap();
    h.call(
        &desktop,
        OperationId::OrchestrationBindCoordinator,
        json!({ "benchId": bench, "request": {
            "requestId": "bind-1", "panelId": "main-agent-run", "runId": "coord",
            "state": "active", "expectedRevision": bootstrapped["revision"] } }),
    )
    .await
    .unwrap();
    let agent = |run: &str| AuthenticatedPrincipal::agent(run);
    let create = |key: &str| {
        json!({ "runId": "coord", "arguments": {
            "requestId": key, "title": format!("task {key}"),
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read", "expectedResult": "summary" } })
    };
    let first = h
        .call(
            &agent("coord"),
            OperationId::OrchestrationCreateChildTask,
            create("c0"),
        )
        .await
        .unwrap();
    let second = h
        .call(
            &agent("coord"),
            OperationId::OrchestrationCreateChildTask,
            create("c1"),
        )
        .await
        .unwrap();
    assert_eq!(second["queued"], true, "{second}");
    if let Some(probe) = probe {
        h.rt.runtime.orchestration().set_dispatch_probe(Some(probe));
    }
    let first_run = first["runId"].as_str().unwrap().to_owned();
    h.call(
        &agent(&first_run),
        OperationId::OrchestrationReportResult,
        json!({ "runId": first_run, "arguments": { "requestId": "r0", "summary": "done" } }),
    )
    .await
    .unwrap();
    let task_id = second["taskId"].as_str().unwrap().to_owned();
    (h, bench, task_id)
}

/// 첫 task의 결과 보고가 만든 coordinator 알림 전달(진짜 활동 작업)이 끝나기를 제한 시간 안에서 기다린다.
async fn settled(h: &BenchHarness) -> Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let current = status(h).await;
        if current["activeWork"]["pendingNotifications"] == 0
            && current["activeWork"]["reservations"] == 0
        {
            return current;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the coordinator notification settles: {current}"
        );
        tokio::task::yield_now().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unassignable_ready_task_is_deferred_and_does_not_hold_the_idle_stop() {
    let (h, _bench, task_id) = with_ready_task(None).await;
    let status = settled(&h).await;
    assert_eq!(
        status["activeWork"]["queuedTasks"], 0,
        "nobody can assign it: {status}"
    );
    assert_eq!(status["deferredTasks"], json!([task_id]), "{status}");
    assert!(
        h.rt.runtime.server_control().tick(Duration::ZERO).await,
        "idle stop proceeds"
    );
}

/// 반례(앞선 "데스크톱 임대가 있으면 셈" 조건): 비우는 중 데스크톱은 새 prompt(N)를 못 보내 배정을 일으킬 수 없다.
/// 임대만으로 세면 wait-stop이 영원히 끝나지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_desktop_lease_alone_does_not_make_a_ready_task_assignable() {
    let (h, _bench, task_id) = with_ready_task(None).await;
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "app"}),
    )
    .await
    .unwrap();
    let status = settled(&h).await;
    assert_eq!(status["activeWork"]["queuedTasks"], 0, "{status}");
    assert_eq!(status["deferredTasks"], json!([task_id]), "{status}");
    stop(&h, "wait").await.unwrap();
    // host 감시처럼 판정을 되풀이한다(비우기 진입의 알림 한 바퀴가 첫 판정의 활동 세대를 바꿀 수 있다).
    let control = h.rt.runtime.server_control();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !control.tick(Duration::ZERO).await {
        assert!(
            std::time::Instant::now() < deadline,
            "wait completes: {}",
            self::status(&h).await
        );
        tokio::task::yield_now().await;
    }
    assert_eq!(h.rt.runtime.work_gate().state(), GateState::Stopping);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ready_task_counts_while_its_coordinator_is_busy() {
    let (h, _bench, _task) = with_ready_task(None).await;
    settled(&h).await;
    let _turn =
        h.rt.runtime
            .work_gate()
            .reserve(
                workbench_core::application::work_gate::ReservationKind::Turn,
                Some("coord"),
            )
            .unwrap();
    let status = status(&h).await;
    assert_eq!(
        status["activeWork"]["queuedTasks"], 1,
        "the coordinator can still assign it: {status}"
    );
    assert_eq!(status["deferredTasks"], json!([]), "{status}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_ready_task_counts_while_a_notification_to_its_coordinator_is_undelivered() {
    let (reached_tx, mut reached) = tokio::sync::mpsc::unbounded_channel();
    let release = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
    let gate = std::sync::Arc::clone(&release);
    let probe: DispatchProbe = std::sync::Arc::new(move |at| {
        let (tx, gate) = (reached_tx.clone(), std::sync::Arc::clone(&gate));
        Box::pin(async move {
            if at == DispatchPoint::FirstPoll {
                let _ = tx.send(());
                gate.acquire().await.expect("probe gate").forget();
            }
            DispatchAction::Continue
        })
    });
    let (h, _bench, _task) = with_ready_task(Some(probe)).await;
    tokio::time::timeout(Duration::from_secs(10), reached.recv())
        .await
        .expect("dispatcher held")
        .unwrap();
    let status = status(&h).await;
    assert!(
        status["activeWork"]["pendingNotifications"]
            .as_u64()
            .unwrap()
            >= 1,
        "{status}"
    );
    assert_eq!(
        status["activeWork"]["queuedTasks"], 1,
        "the notification will wake the coordinator: {status}"
    );
    release.add_permits(64);
}

/// 저장 지속성만: deferred task는 저장돼 있어, 작업대가 닫히고 런타임이 다시 떠도 복구 목록에서 보인다. 실제 정지(감시
/// 루프) → host 전체 종료 → 같은 데이터로 새 host → 복구·재배정은 `workbench-host` `wait_stop.rs`
/// `a_waiting_task_without_an_assigner_is_deferred_and_reassigned_after_a_restart`가 본다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_deferred_task_survives_the_stop_and_is_recoverable_after_restart() {
    let (h, _bench, task_id) = with_ready_task(None).await;
    let status = settled(&h).await;
    assert_eq!(status["deferredTasks"], json!([task_id]), "{status}");
    // 시험 harness의 `restart`는 관문(adapters)을 이어 쓰므로 정지 상태를 거치지 않고 닫기 → 새 런타임으로 저장
    // 지속성만 본다(실제 정지 뒤 복구는 위 host 시험).
    h.rt.runtime.close_all_benches().await;
    let h = h.restart();
    let reopened = h.open().await;
    let recoverable = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::OrchestrationListRecoverable,
            json!({ "benchId": reopened, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    assert!(
        recoverable.to_string().contains(&task_id),
        "the deferred task is recoverable: {recoverable}"
    );
}
