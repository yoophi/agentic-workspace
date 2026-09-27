//! 044 T039 (research R14 표 6'·6'', Codex 재검토 3 F2·4 G1·5 G2): coordinator 알림 전달 예약.
//! - F2: 자식 보고가 돌아간 뒤 전달기가 처음 돌기 전에도 N-notify 예약이 활동으로 남아 wait-stop 판정이 멈추지 않는다.
//!   전달이 끝나 결과가 저장되면 예약이 풀려 멈출 수 있다.
//! - 재시도 가능 실패 뒤 서버가 외부 계기 없이 다시 전달한다(backoff).
//! - G1: `Dispatching{attemptId}` 저장 직후 abort·결과 저장 실패 → 시도의 guard가 회수해 외부 복구 호출 없이 재전달.
//! - G2: 결과 transaction 직전에 회수 한 바퀴를 돌려도 살아 있는 시도는 되돌리지 않는다(변경·재전달 0). 같은 지점
//!   abort는 회수되어 재전달 1회.
//!
//! "wait-stop이 끝나지 않음"은 작업 관문 판정(`WorkGate::try_stop`, 파생 활동 0으로 넘김)으로 단정한다. host의 정지
//! 상태 기계·저장소 파생 활동(미전달 알림 수)은 T041이다.

#![allow(clippy::result_large_err)]

mod support;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use tokio::sync::{mpsc, Semaphore};
use workbench_core::application::orchestration::notification_dispatcher::{
    DispatchAction, DispatchPoint, DispatchProbe,
};
use workbench_protocol::{AuthenticatedPrincipal, OperationId, WorkbenchFault};

const WAIT: Duration = Duration::from_secs(10);

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

struct Fixture {
    h: Arc<BenchHarness>,
    bench: String,
    child_run: String,
}

/// coordinator를 묶고 자식 과제 하나를 띄운다(자식 run은 쉬는 중).
async fn fixture() -> Fixture {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
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
    let created = tool(
        &h,
        "coord",
        OperationId::OrchestrationCreateChildTask,
        json!({
            "requestId": "c1", "title": "task c1",
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read the repo", "expectedResult": "summary"
        }),
    )
    .await
    .unwrap();
    let child_run = created["runId"].as_str().unwrap().to_owned();
    Fixture {
        h,
        bench,
        child_run,
    }
}

/// 자식이 결과를 보고한다(보고 호출은 곧바로 돌아가고 전달기는 뒤에서 돈다). 보고 id를 돌려준다.
async fn report(f: &Fixture) -> String {
    let reported = tool(
        &f.h,
        &f.child_run,
        OperationId::OrchestrationReportResult,
        json!({ "requestId": "r1", "summary": "done" }),
    )
    .await
    .unwrap();
    reported["report"]["id"].as_str().unwrap().to_owned()
}

async fn notification(f: &Fixture, report_id: &str) -> Value {
    let session =
        f.h.call(
            &desktop(),
            OperationId::OrchestrationGet,
            json!({ "benchId": f.bench }),
        )
        .await
        .unwrap();
    session["coordinatorNotifications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["reportId"] == report_id)
        .cloned()
        .expect("notification stored")
}

/// coordinator에 실제로 보낸 알림 prompt 수(보고 id가 들어간 `prompt:coord:` 표지).
fn coordinator_prompts(f: &Fixture, report_id: &str) -> usize {
    f.h.engine
        .applied()
        .iter()
        .filter(|label| label.starts_with("prompt:coord:") && label.contains(report_id))
        .count()
}

async fn eventually<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        if check().await {
            return;
        }
        assert!(tokio::time::Instant::now() < deadline, "never: {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

async fn wait_status(f: &Fixture, report_id: &str, status: &str) -> Value {
    let mut last = Value::Null;
    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        let current = notification(f, report_id).await;
        if current["status"] == status {
            return current;
        }
        last = current;
        assert!(
            tokio::time::Instant::now() < deadline,
            "notification never {status}: {last}"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// 전달기 지점 probe: `point`의 첫 도착에서 알리고, 허가를 얻을 때까지 멈춘 뒤 `action`을 돌려준다. 이후 도착은 그대로
/// 지나간다(회수 뒤 재전달이 다시 멈추지 않도록).
struct Pause {
    reached: mpsc::UnboundedReceiver<()>,
    release: Arc<Semaphore>,
}

fn pause_once(f: &Fixture, point: DispatchPoint, action: DispatchAction) -> Pause {
    let (tx, reached) = mpsc::unbounded_channel();
    let release = Arc::new(Semaphore::new(0));
    let hits = Arc::new(AtomicUsize::new(0));
    let gate = Arc::clone(&release);
    let probe: DispatchProbe = Arc::new(move |at| {
        let (tx, gate, hits) = (tx.clone(), Arc::clone(&gate), Arc::clone(&hits));
        Box::pin(async move {
            if at != point || hits.fetch_add(1, Ordering::SeqCst) != 0 {
                return DispatchAction::Continue;
            }
            let _ = tx.send(());
            gate.acquire().await.expect("probe gate").forget();
            action
        })
    });
    f.h.rt
        .runtime
        .orchestration()
        .set_dispatch_probe(Some(probe));
    Pause { reached, release }
}

async fn reached(pause: &mut Pause, what: &str) {
    tokio::time::timeout(WAIT, pause.reached.recv())
        .await
        .unwrap_or_else(|_| panic!("dispatcher reached {what}"))
        .expect("probe alive");
}

/// F2: 전달기 첫 poll을 막은 채 보고 호출이 돌아가고 자식 turn도 없다 → 활동에 N-notify가 남아 정지 판정이 거절된다
/// → 풀어 주면 알림이 전달·저장되고 예약이 풀려 판정이 통과한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_stored_notification_keeps_the_server_busy_until_it_is_delivered() {
    let f = fixture().await;
    let mut pause = pause_once(&f, DispatchPoint::FirstPoll, DispatchAction::Continue);
    let report_id = report(&f).await;
    reached(&mut pause, "its first poll").await;
    let gate = Arc::clone(f.h.rt.runtime.work_gate());
    assert_eq!(
        gate.busy_run_count(&f.child_run),
        0,
        "the child turn is over"
    );
    assert_eq!(gate.active_work().notifications, 1, "N-notify is held");
    assert!(
        !gate.try_stop(|| 0),
        "wait-stop must not stop before the notification is delivered"
    );
    pause.release.add_permits(1);
    let delivered = wait_status(&f, &report_id, "delivered").await;
    assert_eq!(delivered["attemptCount"], 1);
    assert_eq!(coordinator_prompts(&f, &report_id), 1);
    eventually("all reservations released", || {
        let gate = Arc::clone(&gate);
        async move { gate.reservation_total() == 0 }
    })
    .await;
    assert!(
        gate.try_stop(|| 0),
        "stops once the notification is delivered"
    );
}

/// 재시도 가능 실패(coordinator 전달 실패 1회) → 서버가 외부 계기 없이 backoff 뒤 다시 전달한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_retryable_failure_is_redelivered_by_the_server() {
    let f = fixture().await;
    f.h.engine.fail_send_and_wait.store(1, Ordering::SeqCst);
    let report_id = report(&f).await;
    let delivered = wait_status(&f, &report_id, "delivered").await;
    assert_eq!(delivered["attemptCount"], 2, "{delivered}");
    assert_eq!(coordinator_prompts(&f, &report_id), 1);
    let gate = Arc::clone(f.h.rt.runtime.work_gate());
    eventually("all reservations released", || {
        let gate = Arc::clone(&gate);
        async move { gate.reservation_total() == 0 }
    })
    .await;
}

/// G1: `Dispatching{attemptId}` 저장 직후 전달 future abort → guard가 회수(`Failed(retryable)`)해 재전달. coordinator에
/// 보낸 prompt는 1회(abort 전에는 보내지 않았다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_attempt_aborted_right_after_dispatching_is_reclaimed_and_redelivered() {
    let f = fixture().await;
    let mut pause = pause_once(
        &f,
        DispatchPoint::AfterDispatchingSaved,
        DispatchAction::Continue,
    );
    let report_id = report(&f).await;
    reached(&mut pause, "the saved Dispatching").await;
    let dispatching = notification(&f, &report_id).await;
    assert_eq!(dispatching["status"], "dispatching");
    assert!(dispatching["attemptId"].is_string(), "{dispatching}");
    f.h.rt
        .runtime
        .orchestration()
        .last_notification_pass()
        .expect("a notification pass was spawned")
        .abort();
    let delivered = wait_status(&f, &report_id, "delivered").await;
    assert_eq!(delivered["attemptCount"], 2, "{delivered}");
    assert_ne!(delivered["attemptId"], dispatching["attemptId"]);
    assert_eq!(coordinator_prompts(&f, &report_id), 1);
}

/// G1: 결과 저장이 실패하면(prompt는 보냈다) 그 시도를 회수해 재전달한다. 외부 복구 호출 없음.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_failed_result_save_is_reclaimed_and_redelivered() {
    let f = fixture().await;
    let mut pause = pause_once(
        &f,
        DispatchPoint::BeforeResultSave,
        DispatchAction::FailResultSave,
    );
    let report_id = report(&f).await;
    reached(&mut pause, "the result transaction").await;
    pause.release.add_permits(1);
    let delivered = wait_status(&f, &report_id, "delivered").await;
    assert_eq!(delivered["attemptCount"], 2, "{delivered}");
    assert_eq!(coordinator_prompts(&f, &report_id), 2);
}

/// G2: `send_and_wait`가 돌아간 뒤·결과 transaction 직전에 회수 한 바퀴를 돌린다 → 살아 있는 시도라 변경 0, 재전달 0.
/// 풀어 주면 그 시도가 결과를 저장한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn reclaiming_before_the_result_transaction_leaves_a_live_attempt_alone() {
    let f = fixture().await;
    let mut pause = pause_once(
        &f,
        DispatchPoint::BeforeResultSave,
        DispatchAction::Continue,
    );
    let report_id = report(&f).await;
    reached(&mut pause, "the result transaction").await;
    let before = notification(&f, &report_id).await;
    assert_eq!(before["status"], "dispatching");
    assert_eq!(coordinator_prompts(&f, &report_id), 1);
    let reclaimed =
        f.h.rt
            .runtime
            .orchestration()
            .reclaim_notifications(&f.bench)
            .await
            .unwrap();
    assert_eq!(reclaimed, 0, "a live attempt is not reclaimed");
    assert_eq!(notification(&f, &report_id).await, before, "no change");
    pause.release.add_permits(1);
    let delivered = wait_status(&f, &report_id, "delivered").await;
    assert_eq!(delivered["attemptCount"], 1);
    assert_eq!(delivered["attemptId"], before["attemptId"]);
    assert_eq!(coordinator_prompts(&f, &report_id), 1, "no redelivery");
}

/// G2: 같은 지점(결과 transaction 직전)에서 abort → 회수되어 재전달 1회.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_before_the_result_transaction_is_reclaimed_and_redelivered_once() {
    let f = fixture().await;
    let mut pause = pause_once(
        &f,
        DispatchPoint::BeforeResultSave,
        DispatchAction::Continue,
    );
    let report_id = report(&f).await;
    reached(&mut pause, "the result transaction").await;
    f.h.rt
        .runtime
        .orchestration()
        .last_notification_pass()
        .expect("a notification pass was spawned")
        .abort();
    let delivered = wait_status(&f, &report_id, "delivered").await;
    assert_eq!(delivered["attemptCount"], 2, "{delivered}");
    assert_eq!(coordinator_prompts(&f, &report_id), 2, "redelivered once");
    let gate = Arc::clone(f.h.rt.runtime.work_gate());
    eventually("all reservations released", || {
        let gate = Arc::clone(&gate);
        async move { gate.reservation_total() == 0 }
    })
    .await;
}
