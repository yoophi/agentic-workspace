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

/// Codex 구현 리뷰(high): 감시 루프가 임대 0을 본 뒤 정지 판정에 들어가기 전에 임대가 잡혀 서빙으로 돌아갔다면,
/// 비우기용 정지 판정(`try_stop`)은 서빙 중인 서버를 멈추지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lease_taken_before_the_idle_stop_decision_keeps_the_server_serving() {
    let h = BenchHarness::new(RunScript::default());
    let control = h.rt.runtime.server_control();
    h.rt.runtime.work_gate().begin_drain(DrainMode::Idle);
    owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "app"}),
    )
    .await
    .unwrap();
    assert_eq!(h.rt.runtime.work_gate().state(), GateState::Serving);
    assert!(
        !control.try_stop().await,
        "the drain was cancelled by the lease; its stop decision must not stop the server"
    );
    assert_eq!(h.rt.runtime.work_gate().state(), GateState::Serving);
    assert!(!*control.stopped().borrow());
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
    with_ready_task_failing(probe, 0).await
}

/// `failing_deliveries`: 첫 결과 보고 전에 coordinator 알림 전달(`send_and_wait`)을 그 수만큼 실패시킨다.
async fn with_ready_task_failing(
    probe: Option<DispatchProbe>,
    failing_deliveries: usize,
) -> (BenchHarness, String, String) {
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
    h.engine
        .fail_send_and_wait
        .store(failing_deliveries, std::sync::atomic::Ordering::SeqCst);
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

/// OCR 구현 리뷰(core 2): 비우기가 시작된 뒤 만들어진 대기 task(비우기 전에 받은 호출이 비우기 안에서 끝나 만든 것)는
/// 이어 가기로 배정받지 못한다(`ensure_assign_continues`). 활동으로 세지 않되 **보고에서 빠지지 않고** `deferredTasks`에
/// 든다 — coordinator가 바빠 비우기 전 task는 세는 경우에도.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_task_created_after_the_drain_started_is_reported_as_deferred() {
    use workbench_core::application::{
        orchestration::agent_tools::{handle_tool, RoleLookup, CREATE_CHILD_TASK_TOOL},
        work_gate::{DrainMode, ReservationKind},
    };
    let (h, _bench, before) = with_ready_task(None).await;
    settled(&h).await;
    let gate = h.rt.runtime.work_gate().clone();
    let _turn = gate.reserve(ReservationKind::Turn, Some("coord")).unwrap();
    gate.begin_drain(DrainMode::Wait);
    // 입구를 이미 지난 호출이 비우기 안에서 끝나는 경우: 도구 처리부를 직접 부른다(입구 판정은 비우기 전에 끝났다).
    let orchestration = h.rt.runtime.orchestration().clone();
    let RoleLookup::Role(role) = orchestration.agent_role("coord").await else {
        panic!("coord is the coordinator");
    };
    let created = handle_tool(
        &orchestration,
        "coord",
        role,
        CREATE_CHILD_TASK_TOOL,
        &json!({ "requestId": "c-after", "title": "late", "objective": "read", "expectedResult": "summary",
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" } }),
    )
    .await
    .expect("the late task is created");
    let after = created["taskId"].as_str().unwrap().to_owned();
    let status = status(&h).await;
    assert_eq!(
        status["activeWork"]["queuedTasks"], 1,
        "the pre-drain task can still be assigned by the busy coordinator: {status}"
    );
    let deferred: Vec<String> = serde_json::from_value(status["deferredTasks"].clone()).unwrap();
    assert!(
        deferred.contains(&after),
        "the post-drain task is reported: {status}"
    );
    assert!(!deferred.contains(&before), "{status}");
}

async fn session_of(h: &BenchHarness, bench: &str) -> Value {
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::OrchestrationGet,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap()
}

/// 알림 전달이 계속 실패하는 동안 상태를 지켜본다: 상한 전에는 정지를 막고, 상한을 넘으면 `stalledNotifications`에만
/// 든다. `(알림 id, 상한 전 관측 수)`.
async fn until_stalled(h: &BenchHarness, bench: &str) -> (String, usize) {
    use workbench_core::application::server_control::MAX_NOTIFICATION_ATTEMPTS_FOR_STOP;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut blocked_before_cap = 0;
    loop {
        let current = status(h).await;
        let note = session_of(h, bench).await["coordinatorNotifications"][0].clone();
        let attempts = note["attemptCount"].as_u64().unwrap_or(0) as u32;
        let stalled: Vec<String> =
            serde_json::from_value(current["stalledNotifications"].clone()).unwrap_or_default();
        if stalled.is_empty() {
            if attempts < MAX_NOTIFICATION_ATTEMPTS_FOR_STOP {
                assert!(
                    current["activeWork"]["pendingNotifications"]
                        .as_u64()
                        .unwrap()
                        >= 1,
                    "a notification under the cap blocks the stop: {current} {note}"
                );
                blocked_before_cap += 1;
            }
        } else {
            let id = note["id"].as_str().unwrap().to_owned();
            assert_eq!(stalled, vec![id.clone()]);
            assert!(
                attempts >= MAX_NOTIFICATION_ATTEMPTS_FOR_STOP,
                "stalled only after the cap: {note}"
            );
            assert_eq!(note["status"], "failed", "{note}");
            return (id, blocked_before_cap);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the notification reaches the attempt cap: {current} {note}"
        );
        tokio::task::yield_now().await;
    }
}

/// 유휴 정지가 진행될 때까지 tick한다(배경 재시도의 전달 시도 중에는 그 시도가 활동이다 — 조건 대기).
async fn idle_stops(h: &BenchHarness, why: &str) {
    let control = h.rt.runtime.server_control();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !control.tick(Duration::ZERO).await {
        assert!(
            std::time::Instant::now() < deadline,
            "the idle stop proceeds: {why}"
        );
        tokio::task::yield_now().await;
    }
}

/// OCR 구현 리뷰(core 3): coordinator turn이 계속 실패하면(인증·할당량 등) 재시도 가능한 실패 알림을 영원히 활동으로 세어
/// wait·유휴 정지가 끝나지 않았다. 상한(`MAX_NOTIFICATION_ATTEMPTS_FOR_STOP`) 전에는 정지를 막고, 상한을 넘은 실패 알림은
/// `stalledNotifications`로 보고만 한다. 알림은 전달된 것으로 바뀌지 않고 재시도 가능한 실패로 저장된 채 남는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_notification_that_keeps_failing_stops_blocking_after_the_attempt_cap() {
    let (h, bench, _task) = with_ready_task_failing(None, 10_000).await;
    let (id, blocked_before_cap) = until_stalled(&h, &bench).await;
    assert!(blocked_before_cap > 0, "the pre-cap window was observed");
    idle_stops(&h, &id).await;
    // 정지 뒤에는 호출 입구가 닫혀 있다 — 저장소를 직접 읽는다.
    let session =
        h.rt.runtime
            .orchestration()
            .get(&bench)
            .await
            .unwrap()
            .expect("session");
    let note = session
        .coordinator_notifications
        .iter()
        .find(|note| note.id == id)
        .expect("stored");
    assert_eq!(
        note.status,
        workbench_core::domain::agent_orchestration::CoordinatorNotificationStatus::Failed,
        "kept as a failed notification"
    );
    assert!(
        note.failure
            .as_ref()
            .is_some_and(|failure| failure.retryable),
        "still retryable"
    );
    assert!(note.collected_at.is_none());
}

/// 상한은 재시도를 **기다리는** 실패 알림에만 적용된다. 시도 수가 상한 이상이어도 진행 중인 전달 시도(`Dispatching`,
/// N-notify 예약)는 정지를 막는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_attempt_in_progress_blocks_the_stop_even_past_the_cap() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use workbench_core::application::server_control::MAX_NOTIFICATION_ATTEMPTS_FOR_STOP;
    let hold = std::sync::Arc::new(AtomicBool::new(false));
    let (reached_tx, mut reached) = tokio::sync::mpsc::unbounded_channel();
    let release = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
    let probe: DispatchProbe = {
        let (hold, release) = (
            std::sync::Arc::clone(&hold),
            std::sync::Arc::clone(&release),
        );
        std::sync::Arc::new(move |at| {
            let (hold, release, tx) = (
                std::sync::Arc::clone(&hold),
                std::sync::Arc::clone(&release),
                reached_tx.clone(),
            );
            Box::pin(async move {
                if at == DispatchPoint::AfterDispatchingSaved && hold.swap(false, Ordering::SeqCst)
                {
                    let _ = tx.send(());
                    release.acquire().await.expect("probe gate").forget();
                }
                DispatchAction::Continue
            })
        })
    };
    let (h, bench, _task) = with_ready_task_failing(Some(probe), 10_000).await;
    let (id, _) = until_stalled(&h, &bench).await;
    hold.store(true, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(20), reached.recv())
        .await
        .expect("the next retry attempt is held in flight")
        .unwrap();
    let note = session_of(&h, &bench).await["coordinatorNotifications"][0].clone();
    assert_eq!(note["status"], "dispatching", "{note}");
    assert!(note["attemptCount"].as_u64().unwrap() as u32 > MAX_NOTIFICATION_ATTEMPTS_FOR_STOP);
    let current = status(&h).await;
    assert_eq!(current["stalledNotifications"], json!([]), "{current}");
    assert!(
        current["activeWork"]["pendingNotifications"]
            .as_u64()
            .unwrap()
            >= 1,
        "{current}"
    );
    assert!(
        !h.rt.runtime.server_control().tick(Duration::ZERO).await,
        "an attempt in flight blocks the stop"
    );
    release.add_permits(1);
    // 그 시도도 실패하면 다시 재시도 대기로 돌아가 보고만 된다.
    let (again, _) = until_stalled(&h, &bench).await;
    assert_eq!(again, id);
}

/// 상한을 넘어 보고만 되던 알림도 잃지 않는다: coordinator가 다시 성공하면 배경 재시도가 전달해 `delivered`가 된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stalled_notification_is_delivered_once_the_coordinator_recovers() {
    use std::sync::atomic::Ordering;
    let (h, bench, _task) = with_ready_task_failing(None, 10_000).await;
    let (id, _) = until_stalled(&h, &bench).await;
    h.engine.fail_send_and_wait.store(0, Ordering::SeqCst);
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let note = session_of(&h, &bench).await["coordinatorNotifications"][0].clone();
        if note["status"] == "delivered" {
            assert_eq!(note["id"], json!(id));
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the retry delivers it: {note}"
        );
        tokio::task::yield_now().await;
    }
    assert_eq!(status(&h).await["stalledNotifications"], json!([]));
}

/// 정지 뒤 같은 데이터로 runtime 재조립(같은 시험 프로세스, OS 프로세스 재시작 아님): 보고만 되던 알림은 재시도 가능한
/// 실패로, 그 보고는 결과로 저장돼 있다. 복구 → 새 coordinator 인계 뒤 그 알림은 `superseded`가 되고 **새 coordinator에게
/// 다시 전달되지 않는다**(041 인계 계약). 증명하는 것은 "저장 보존 + 보고 모으기(`orchestration.collectReports`)로 결과를
/// 읽을 수 있음"이다. 자동 재전달은 증명하지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stalled_notification_survives_the_stop_and_its_report_is_collected_after_a_restart() {
    let limit = |adapters: &mut workbench_core::application::workbench_runtime::RuntimeAdapters| {
        adapters.orchestration.max_concurrent_children = 1
    };
    let (h, _bench, _task) = with_ready_task_failing(None, 10_000).await;
    let (id, _) = until_stalled(&h, &_bench).await;
    idle_stops(&h, &id).await;
    h.rt.runtime.close_all_benches().await;
    let h = h.restart_runtime(limit, RunScript::default());
    let bench = h.open().await;
    let desktop = AuthenticatedPrincipal::desktop();
    let recoverable = h
        .call(
            &desktop,
            OperationId::OrchestrationListRecoverable,
            json!({ "benchId": bench, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    let saved = &recoverable[0];
    let note = saved["coordinatorNotifications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|note| note["id"] == json!(id))
        .cloned()
        .expect("the stalled notification is stored");
    assert_eq!(note["status"], "failed", "{note}");
    assert_eq!(note["failure"]["retryable"], true, "{note}");
    let report_task = note["taskId"].as_str().unwrap().to_owned();
    assert!(
        saved["reports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|report| report["taskId"] == json!(report_task)),
        "the report behind it is stored: {saved}"
    );
    let session = h
        .call(
            &desktop,
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": bench, "worktreePath": h.dir, "resumeWorkspaceId": saved["id"] }),
        )
        .await
        .unwrap();
    h.start(&bench, "coord2").await.unwrap();
    h.call(
        &desktop,
        OperationId::OrchestrationHandoffCoordinator,
        json!({ "benchId": bench, "request": {
            "requestId": "handoff-1", "successorRunId": "coord2", "summary": "resume after restart",
            "confirmed": true, "expectedRevision": session["revision"] } }),
    )
    .await
    .unwrap();
    h.call(
        &desktop,
        OperationId::OrchestrationRecover,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap();
    // 결과는 저장돼 읽을 수 있다: 복구 뒤에도 보고 모으기(데스크톱·소유자)가 그 보고를 돌려준다.
    let reports = h
        .call(
            &desktop,
            OperationId::OrchestrationCollectReports,
            json!({ "benchId": bench }),
        )
        .await
        .unwrap();
    assert!(
        reports
            .as_array()
            .unwrap()
            .iter()
            .any(|report| report["taskId"] == json!(report_task) && report["summary"] == "done"),
        "the result is still collectable: {reports}"
    );
    // 한계(041 계약): 새 세대로 인계하면 이전 세대의 전달 안 된 알림은 다음 전달 바퀴에서 `superseded`가 된다. 그 알림은
    // 새 coordinator에게 다시 전달되지 않는다(재전달 아님). 새 coordinator의 `collectChildResults`도 이전 세대의 끝난
    // task를 받지 않는다(활성 세대 한정) — 결과는 보고 모으기로만 읽는다.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let note = session_of(&h, &bench).await["coordinatorNotifications"]
            .as_array()
            .unwrap()
            .iter()
            .find(|note| note["id"] == json!(id))
            .cloned()
            .unwrap();
        if note["status"] == "superseded" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the handoff supersedes it: {note}"
        );
        tokio::task::yield_now().await;
    }
}

/// OCR 구현 리뷰 2차(core 1): coordinator가 바빠 **거절한**(`CoordinatorBusy`) 알림은 시도 수가 상한을 넘어도 보류
/// (`stalledNotifications`)가 아니다 — turn이 끝나면 전달될 수 있는 알림이다. 상한은 회복되지 않는 실패(인증·할당량 등)에만
/// 적용된다. 거절은 probe가 전달기 대신 `accepted: false` 영수증을 만든다(운영 전달기는 대기열에 넣고 기다리므로 거절하지
/// 않는다 — 이 시험은 거절 경로의 계약만 본다).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_busy_decline_past_the_attempt_cap_still_blocks_the_stop_and_is_delivered_later() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use workbench_core::application::server_control::MAX_NOTIFICATION_ATTEMPTS_FOR_STOP;
    let decline = std::sync::Arc::new(AtomicBool::new(true));
    let probe: DispatchProbe = {
        let decline = std::sync::Arc::clone(&decline);
        std::sync::Arc::new(move |at| {
            let decline = std::sync::Arc::clone(&decline);
            Box::pin(async move {
                if at == DispatchPoint::AfterDispatchingSaved && decline.load(Ordering::SeqCst) {
                    DispatchAction::DeclineAsBusy
                } else {
                    DispatchAction::Continue
                }
            })
        })
    };
    let (h, bench, _task) = with_ready_task(Some(probe)).await;
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let note = loop {
        let note = session_of(&h, &bench).await["coordinatorNotifications"][0].clone();
        if note["status"] == "failed"
            && note["attemptCount"].as_u64().unwrap_or(0)
                >= MAX_NOTIFICATION_ATTEMPTS_FOR_STOP as u64
        {
            break note;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "busy declines pass the cap: {note}"
        );
        tokio::task::yield_now().await;
    };
    assert_eq!(note["failure"]["code"], "coordinatorBusy", "{note}");
    let current = status(&h).await;
    assert_eq!(current["stalledNotifications"], json!([]), "{current}");
    assert!(
        current["activeWork"]["pendingNotifications"]
            .as_u64()
            .unwrap()
            >= 1,
        "a busy-declined notification is still active work: {current}"
    );
    assert!(
        !h.rt.runtime.server_control().tick(Duration::ZERO).await,
        "the stop is blocked while the notification can still be delivered"
    );

    decline.store(false, Ordering::SeqCst);
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let note = session_of(&h, &bench).await["coordinatorNotifications"][0].clone();
        if note["status"] == "delivered" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "delivered once the coordinator accepts: {note}"
        );
        tokio::task::yield_now().await;
    }
}

/// OCR 구현 리뷰 2차(core 1, 섞인 이력): 바쁨 거절이 상한만큼 쌓인 뒤 **실제 실패 한 번**이 와도 보류가 아니다 — 상한은
/// 실제 전달 실패 수(`deliveryFailureCount`)로 센다(전체 시도 수 `attemptCount`가 아님). 실제 실패가 상한까지 쌓이면
/// 그때 보류다. 실제 실패 직후 상태는 다음 전달 한 바퀴의 첫 poll을 붙잡아 관찰한다(시간에 기대지 않음).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn busy_declines_do_not_consume_the_real_failure_budget() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use workbench_core::application::server_control::MAX_NOTIFICATION_ATTEMPTS_FOR_STOP;
    let cap = MAX_NOTIFICATION_ATTEMPTS_FOR_STOP as usize;
    let attempts = std::sync::Arc::new(AtomicUsize::new(0));
    let hold_next_poll = std::sync::Arc::new(AtomicBool::new(false));
    let release = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
    let (held_tx, mut held) = tokio::sync::mpsc::unbounded_channel();
    let probe: DispatchProbe = {
        let (attempts, hold_next_poll, release) = (
            std::sync::Arc::clone(&attempts),
            std::sync::Arc::clone(&hold_next_poll),
            std::sync::Arc::clone(&release),
        );
        std::sync::Arc::new(move |at| {
            let (attempts, hold_next_poll, release, held_tx) = (
                std::sync::Arc::clone(&attempts),
                std::sync::Arc::clone(&hold_next_poll),
                std::sync::Arc::clone(&release),
                held_tx.clone(),
            );
            Box::pin(async move {
                match at {
                    DispatchPoint::AfterDispatchingSaved => {
                        let n = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                        if n <= cap {
                            return DispatchAction::DeclineAsBusy;
                        }
                        if n == cap + 1 {
                            // 이 시도는 실제로 실패한다(엔진 주입). 다음 한 바퀴의 첫 poll을 붙잡는다.
                            hold_next_poll.store(true, Ordering::SeqCst);
                        }
                        DispatchAction::Continue
                    }
                    DispatchPoint::FirstPoll if hold_next_poll.swap(false, Ordering::SeqCst) => {
                        let _ = held_tx.send(());
                        release.acquire().await.expect("probe gate").forget();
                        DispatchAction::Continue
                    }
                    _ => DispatchAction::Continue,
                }
            })
        })
    };
    // 실제 전달 한 번만 실패시킨다(바쁨 거절 시도는 엔진을 부르지 않으므로 이 수를 쓰지 않는다).
    let (h, bench, _task) = with_ready_task_failing(Some(probe), 1).await;
    tokio::time::timeout(Duration::from_secs(20), held.recv())
        .await
        .expect("the pass after the real failure is held")
        .unwrap();
    let note = session_of(&h, &bench).await["coordinatorNotifications"][0].clone();
    assert_eq!(note["status"], "failed", "{note}");
    assert!(
        note["attemptCount"].as_u64().unwrap() > cap as u64,
        "{note}"
    );
    assert_eq!(note["deliveryFailureCount"], 1, "{note}");
    assert_ne!(note["failure"]["code"], "coordinatorBusy", "{note}");
    let current = status(&h).await;
    assert_eq!(
        current["stalledNotifications"],
        json!([]),
        "one real failure after busy declines is not stalled: {current}"
    );
    assert!(
        current["activeWork"]["pendingNotifications"]
            .as_u64()
            .unwrap()
            >= 1,
        "{current}"
    );

    // 이제 실제 실패만 계속된다 — 실제 실패가 상한에 이르면 보류다.
    h.engine
        .fail_send_and_wait
        .store(10_000, std::sync::atomic::Ordering::SeqCst);
    release.add_permits(1);
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let current = status(&h).await;
        let stalled: Vec<String> =
            serde_json::from_value(current["stalledNotifications"].clone()).unwrap_or_default();
        let note = session_of(&h, &bench).await["coordinatorNotifications"][0].clone();
        if !stalled.is_empty() {
            assert!(
                note["deliveryFailureCount"].as_u64().unwrap() >= cap as u64,
                "stalled only once real failures reach the cap: {note}"
            );
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "real failures reach the cap: {note}"
        );
        tokio::task::yield_now().await;
    }
}

/// OCR 구현 리뷰 2차(core 2): 멈추는(`stopping`) 서버는 임대를 내주지 않는다 — 거절하고 임대를 남기지 않는다.
/// 한계: 이 시험은 입구 판정(`admit`)이 거절하는 경로를 본다. handler 안의 "넣은 뒤 `stopping`이면 되돌림"은 입구 통과와
/// 넣기 사이의 창을 강제할 수단이 없어 결정적으로 재현하지 못한다(순서: 서빙 복귀 → 넣기 → 서빙 복귀 → `stopping` 확인).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stopping_server_hands_out_no_lease() {
    let h = BenchHarness::new(RunScript::default());
    h.rt.runtime.work_gate().force_stop();
    let refused = owner(
        &h,
        OperationId::LeaseAcquire,
        json!({"clientKind": "desktop", "clientId": "late"}),
    )
    .await
    .expect_err("a stopping server refuses a lease");
    assert_eq!(refused.code, FaultCode::Unavailable, "{refused:?}");
    assert_eq!(
        h.rt.runtime.server_control().leases().count(),
        0,
        "no lease is left behind"
    );
}
