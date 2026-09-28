//! 044 T038 (research R14 표 4·5·5'·6, 시작 장벽): 대기 task 배정의 원자성.
//! - E3: 서로 다른 키로 같은 task를 동시에 100번 배정해도 run은 하나다(저장소 RMW 비교 후 변경 + 기동 단일 비행).
//! - E4·F1: 시작 장벽 지점마다(registry 예약 전·spawn 뒤 attach 전·attach 뒤 전이 전·전이 뒤 장벽 전) 취소하면 취소가
//!   성공하고 launcher·prompt 실행이 0이며 예약이 남지 않는다. 같은 지점에서 배정 future를 abort해도 실행 0·예약 0이다.
//! - 등록(바인딩) 뒤 취소는 실제 run을 취소한다. `bind_child_run`은 취소된 task를 거절한다.
//!
//! 지점 멈춤은 결정적 gate다(시간 지연 없음). "spawn 뒤·attach 전"은 엔진 내부 지점이라 스크립트 엔진의 준비 hook으로
//! 모사한다(운영 `AcpRunEngine`의 같은 지점은 acp-agent-core `start_gate` 단위 시험이 덮는다).

#![allow(clippy::result_large_err)]

mod support;

use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use tokio::sync::{mpsc, Semaphore};
use workbench_core::application::orchestration::runtime::{
    LaunchPoint, LaunchProbe, StorePoint, StoreProbe,
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

fn node_of<'a>(session: &'a Value, task_id: &str) -> &'a Value {
    let node_id = task(session, task_id)["assignedNodeId"].clone();
    session["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == node_id)
        .unwrap()
}

/// 동시 한도 1: 첫 과제는 실행되고 둘째 과제는 대기한다. 첫 과제가 결과를 보고하면 둘째 과제가 scheduler 자리를 얻어
/// **아직 기동하지 않은 준비(Ready) task**가 된다. 그 task id를 돌려준다.
async fn ready_task() -> (Arc<BenchHarness>, String, String) {
    let (h, bench, task_id, _) = ready_task_with_baseline().await;
    (h, bench, task_id)
}

/// `ready_task`와 같고, 준비가 끝난 시점의 효과 표지 수(첫 과제의 기동 등 이전 효과를 빼고 세기 위해)를 함께 돌려준다.
async fn ready_task_with_baseline() -> (Arc<BenchHarness>, String, String, Baseline) {
    let h = Arc::new(BenchHarness::with(
        |adapters| adapters.orchestration.max_concurrent_children = 1,
        RunScript::default(),
    ));
    git_init(&h.dir);
    let bench = h.open().await;
    let bootstrapped = h
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
            "state": "active", "expectedRevision": bootstrapped["revision"] } }),
    )
    .await
    .unwrap();
    let create = |key: &'static str| {
        let h = Arc::clone(&h);
        async move {
            tool(
                &h,
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
    };
    let first = create("c0").await;
    let second = create("c1").await;
    assert_eq!(second["queued"], true, "{second}");
    let reported = tool(
        &h,
        first["runId"].as_str().unwrap(),
        OperationId::OrchestrationReportResult,
        json!({ "requestId": "r0", "summary": "done" }),
    )
    .await
    .unwrap();
    assert_eq!(reported["nextReadyTaskId"], second["taskId"]);
    let task_id = second["taskId"].as_str().unwrap().to_owned();
    let current = session(&h, &bench).await;
    assert_eq!(task(&current, &task_id)["status"], "ready");
    assert!(node_of(&current, &task_id)["currentRunId"].is_null());
    let baseline = Baseline {
        applied: h.engine.applied().len(),
        launches: h.engine.launches.load(Ordering::SeqCst),
        runs: h.engine.run_count(),
    };
    (h, bench, task_id, baseline)
}

#[derive(Clone, Copy)]
struct Baseline {
    applied: usize,
    launches: usize,
    /// 살아 있는 run 수(coordinator·첫 과제의 run).
    runs: usize,
}

async fn assign(h: &BenchHarness, task_id: &str, key: &str) -> Result<Value, WorkbenchFault> {
    tool(
        h,
        "coord",
        OperationId::OrchestrationAssignChildTask,
        json!({ "taskId": task_id, "requestId": key }),
    )
    .await
}

async fn cancel(h: &BenchHarness, task_id: &str, key: &str) -> Result<Value, WorkbenchFault> {
    tool(
        h,
        "coord",
        OperationId::OrchestrationCancelChildTask,
        json!({ "taskId": task_id, "requestId": key }),
    )
    .await
}

/// coordinator가 아닌 run의 실행 표지(launcher 실행·prompt).
fn child_executions(h: &BenchHarness, baseline: Baseline) -> Vec<String> {
    h.engine
        .applied()
        .into_iter()
        .skip(baseline.applied)
        .filter(|label| {
            label.starts_with("launch:")
                || (label.starts_with("prompt:") && !label.starts_with("prompt:coord:"))
        })
        .collect()
}

/// 조건이 참이 될 때까지 기다린다(비동기 정리 — abort 뒤 guard가 예약한 저장소 복구·닫힌 장벽의 run 정리 — 를 관찰).
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

/// 지점 멈춤: `point`에 들어오면 알리고 허가를 얻을 때까지 멈춘다.
struct Pause {
    reached: mpsc::UnboundedReceiver<()>,
    release: Arc<Semaphore>,
}

fn pause_at(h: &BenchHarness, point: LaunchPoint) -> Pause {
    let (tx, reached) = mpsc::unbounded_channel();
    let release = Arc::new(Semaphore::new(0));
    let gate = Arc::clone(&release);
    let probe: LaunchProbe = Arc::new(move |at| {
        let tx = tx.clone();
        let gate = Arc::clone(&gate);
        Box::pin(async move {
            if at == point {
                let _ = tx.send(());
                gate.acquire().await.expect("probe gate").forget();
            }
        })
    });
    h.rt.runtime.orchestration().set_launch_probe(Some(probe));
    Pause { reached, release }
}

/// 엔진 준비 안(spawn 뒤·attach 전) 멈춤.
fn pause_in_engine(h: &BenchHarness) -> Pause {
    let (tx, reached) = mpsc::unbounded_channel();
    let release = Arc::new(Semaphore::new(0));
    let gate = Arc::clone(&release);
    *h.engine.prepare_hook.lock().unwrap() = Some(Arc::new(move |_run| {
        let tx = tx.clone();
        let gate = Arc::clone(&gate);
        Box::pin(async move {
            let _ = tx.send(());
            gate.acquire().await.expect("prepare gate").forget();
        })
    }));
    Pause { reached, release }
}

#[derive(Clone, Copy, Debug)]
enum Point {
    BeforeReserve,
    AfterSpawnBeforeAttach,
    AfterAttachBeforeTransition,
    AfterTransitionBeforeGate,
}

const POINTS: [Point; 4] = [
    Point::BeforeReserve,
    Point::AfterSpawnBeforeAttach,
    Point::AfterAttachBeforeTransition,
    Point::AfterTransitionBeforeGate,
];

fn pause_for(h: &BenchHarness, point: Point) -> Pause {
    match point {
        Point::BeforeReserve => pause_at(h, LaunchPoint::BeforePrepare),
        Point::AfterSpawnBeforeAttach => pause_in_engine(h),
        Point::AfterAttachBeforeTransition => pause_at(h, LaunchPoint::AfterPrepare),
        Point::AfterTransitionBeforeGate => pause_at(h, LaunchPoint::AfterRegister),
    }
}

/// 배정 뒤 끝 상태: 실행 0, 관문 예약 0, coordinator 말고 살아 있는 run 0, 노드 예약 해제.
async fn assert_nothing_ran(
    h: &BenchHarness,
    bench: &str,
    task_id: &str,
    baseline: Baseline,
    label: &str,
) {
    assert_eq!(
        child_executions(h, baseline),
        Vec::<String>::new(),
        "{label}: no launcher or prompt execution"
    );
    let gate = Arc::clone(h.rt.runtime.work_gate());
    eventually(&format!("{label}: gate reservations released"), || {
        let gate = Arc::clone(&gate);
        async move { gate.reservation_total() == 0 }
    })
    .await;
    eventually(&format!("{label}: prepared run gone"), || async {
        h.engine.run_count() == baseline.runs
    })
    .await;
    eventually(&format!("{label}: node reservation released"), || async {
        node_of(&session(h, bench).await, task_id)["currentRunId"].is_null()
    })
    .await;
    assert_eq!(
        child_executions(h, baseline),
        Vec::<String>::new(),
        "{label}: still no execution after cleanup"
    );
    assert_eq!(
        h.engine.launches.load(Ordering::SeqCst),
        baseline.launches,
        "{label}"
    );
}

/// E3: 서로 다른 키로 같은 준비 task를 동시에 100번 배정 → 엔진 기동 1회, 모든 응답이 같은 run, 노드의 run도 그것.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn one_hundred_concurrent_assigns_start_exactly_one_run() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let starts_before = h.engine.starts.load(Ordering::SeqCst);
    let barrier = Arc::new(tokio::sync::Barrier::new(100));
    let mut calls = Vec::new();
    for i in 0..100 {
        let (h, task_id, barrier) = (Arc::clone(&h), task_id.clone(), Arc::clone(&barrier));
        calls.push(tokio::spawn(async move {
            barrier.wait().await;
            assign(&h, &task_id, &format!("assign-{i}")).await
        }));
    }
    let mut runs = std::collections::BTreeSet::new();
    for call in calls {
        let value = call.await.unwrap().expect("assign accepted");
        runs.insert(value["runId"].as_str().expect("runId").to_owned());
    }
    assert_eq!(
        h.engine.starts.load(Ordering::SeqCst) - starts_before,
        1,
        "exactly one engine start"
    );
    assert_eq!(runs.len(), 1, "every assign reports the same run: {runs:?}");
    let current = session(&h, &bench).await;
    assert_eq!(
        node_of(&current, &task_id)["currentRunId"].as_str(),
        runs.iter().next().map(String::as_str)
    );
    assert_eq!(task(&current, &task_id)["status"], "running");
    assert_eq!(h.engine.run_count(), baseline.runs + 1, "one new child run");
}

/// 배정·취소 경합(반복): 취소가 성공하면(task `cancelled`) 그 task의 run은 살아 있지 않다. 관문 예약은 남지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn assign_racing_cancel_never_leaves_a_live_run_for_a_cancelled_task() {
    for round in 0..20 {
        let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let assigning = {
            let (h, task_id, barrier) = (Arc::clone(&h), task_id.clone(), Arc::clone(&barrier));
            tokio::spawn(async move {
                barrier.wait().await;
                assign(&h, &task_id, "assign").await
            })
        };
        let cancelling = {
            let (h, task_id, barrier) = (Arc::clone(&h), task_id.clone(), Arc::clone(&barrier));
            tokio::spawn(async move {
                barrier.wait().await;
                cancel(&h, &task_id, "cancel").await
            })
        };
        let _ = assigning.await.unwrap();
        let cancelled = cancelling.await.unwrap();
        let current = session(&h, &bench).await;
        let status = task(&current, &task_id)["status"].clone();
        if cancelled.is_ok() {
            assert_eq!(status, "cancelled", "round {round}: {cancelled:?}");
            eventually(&format!("round {round}: no live child run"), || async {
                h.engine.run_count() == baseline.runs
            })
            .await;
        }
        let gate = Arc::clone(h.rt.runtime.work_gate());
        eventually(
            &format!("round {round}: gate reservations released"),
            || {
                let gate = Arc::clone(&gate);
                async move { gate.reservation_total() == 0 }
            },
        )
        .await;
    }
}

/// F1·E4: 시작 장벽 지점마다 멈춘 채 취소 → 취소 성공 → 풀어 준다 → 실행 0·예약 0·task `cancelled`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_at_each_start_barrier_point_prevents_any_execution() {
    for point in POINTS {
        let label = format!("{point:?}");
        let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
        let mut pause = pause_for(&h, point);
        let assigning = {
            let (h, task_id) = (Arc::clone(&h), task_id.clone());
            tokio::spawn(async move { assign(&h, &task_id, "assign").await })
        };
        tokio::time::timeout(WAIT, pause.reached.recv())
            .await
            .unwrap_or_else(|_| panic!("{label}: assign reached the point"));
        let cancelled = cancel(&h, &task_id, "cancel").await;
        assert!(cancelled.is_ok(), "{label}: cancel accepted: {cancelled:?}");
        assert_eq!(
            task(&session(&h, &bench).await, &task_id)["status"],
            "cancelled",
            "{label}"
        );
        pause.release.add_permits(1);
        let assigned = tokio::time::timeout(WAIT, assigning)
            .await
            .unwrap_or_else(|_| panic!("{label}: assign returned"))
            .unwrap();
        assert!(
            assigned
                .as_ref()
                .map_or(true, |value| value["runId"].is_null()
                    || value["executionStatus"] != "active"),
            "{label}: a cancelled assign does not report an active run: {assigned:?}"
        );
        assert_nothing_ran(&h, &bench, &task_id, baseline, &label).await;
        assert_eq!(
            task(&session(&h, &bench).await, &task_id)["status"],
            "cancelled",
            "{label}"
        );
    }
}

/// F1: 같은 지점들에서 배정 future를 abort → 실행 0·예약 0(장벽이 닫힌 채 drop, guard가 저장소 예약을 되돌림).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_the_assign_at_each_start_barrier_point_leaks_nothing() {
    for point in POINTS {
        let label = format!("abort {point:?}");
        let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
        let mut pause = pause_for(&h, point);
        // 배정 future 자체를 abort하려고 handler를 거치지 않고 orchestration 런타임을 직접 부른다(HTTP 연결 단절 등으로
        // 호출 future가 drop되는 경우와 같다).
        let assigning = {
            let (h, bench, task_id) = (Arc::clone(&h), bench.clone(), task_id.clone());
            tokio::spawn(async move {
                h.rt.runtime
                    .orchestration()
                    .launch_task_for_ui(&bench, &task_id)
                    .await
            })
        };
        tokio::time::timeout(WAIT, pause.reached.recv())
            .await
            .unwrap_or_else(|_| panic!("{label}: assign reached the point"));
        assigning.abort();
        assert!(assigning.await.unwrap_err().is_cancelled(), "{label}");
        assert_nothing_ran(&h, &bench, &task_id, baseline, &label).await;
        let current = session(&h, &bench).await;
        assert_eq!(
            task(&current, &task_id)["status"],
            "ready",
            "{label}: an aborted assign leaves the task assignable"
        );
        drop(pause);
    }
}

/// Codex r6 medium: 저장소 예약(`reserve_child_run`)은 blocking 작업이라 배정 future가 그 await 중에 abort돼도 끝까지
/// 커밋한다. 커밋 직전에 blocking 작업을 붙잡고 → 배정 future를 abort하고 → 커밋을 끝내게 한 뒤: 예정 run id가 노드에서
/// 풀리고 scheduler 자리가 반납돼, 다시 배정하면 실제 run이 기동한다(`alreadyAssigned`로 영원히 막히지 않는다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_the_assign_while_the_store_reservation_commits_rolls_it_back() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let (reached_tx, mut reached) = mpsc::unbounded_channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let release_rx = std::sync::Mutex::new(release_rx);
    let probe: StoreProbe = Arc::new(move |point| {
        let _ = reached_tx.send(point);
        if point == StorePoint::ReserveBeforeCommit {
            // blocking 스레드에서 불린다: 시험이 abort를 마칠 때까지 커밋을 붙잡는다.
            let _ = release_rx.lock().unwrap().recv_timeout(WAIT);
        }
    });
    h.rt.runtime.orchestration().set_store_probe(Some(probe));
    let assigning = {
        let (h, bench, task_id) = (Arc::clone(&h), bench.clone(), task_id.clone());
        tokio::spawn(async move {
            h.rt.runtime
                .orchestration()
                .launch_task_for_ui(&bench, &task_id)
                .await
        })
    };
    assert_eq!(
        tokio::time::timeout(WAIT, reached.recv()).await.unwrap(),
        Some(StorePoint::ReserveBeforeCommit)
    );
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    release_tx.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(WAIT, reached.recv()).await.unwrap(),
        Some(StorePoint::ReserveAfterCommit),
        "the reservation committed after the abort"
    );
    h.rt.runtime.orchestration().set_store_probe(None);
    eventually("the planned run id is released from the node", || async {
        node_of(&session(&h, &bench).await, &task_id)["currentRunId"].is_null()
    })
    .await;
    let scheduler_free = || async {
        h.rt.runtime
            .orchestration()
            .scheduler()
            .active_count()
            .unwrap()
            == 0
    };
    eventually("the scheduler slot is returned", scheduler_free).await;
    assert_eq!(child_executions(&h, baseline), Vec::<String>::new());
    let current = session(&h, &bench).await;
    assert_eq!(task(&current, &task_id)["status"], "ready");

    let assigned = assign(&h, &task_id, "reassign").await.unwrap();
    assert!(
        assigned.get("alreadyAssigned").is_none(),
        "a leaked reservation blocks reassignment: {assigned}"
    );
    assert_eq!(assigned["executionStatus"], "active", "{assigned}");
    let run_id = assigned["runId"].as_str().unwrap().to_owned();
    assert_eq!(
        node_of(&session(&h, &bench).await, &task_id)["currentRunId"],
        run_id.as_str()
    );
    assert!(
        child_executions(&h, baseline)
            .iter()
            .any(|label| label.starts_with("launch:")),
        "a real child run started: {:?}",
        child_executions(&h, baseline)
    );
}

/// 등록(바인딩) 뒤 취소는 실제 run을 취소한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_after_registration_cancels_the_real_run() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let assigned = assign(&h, &task_id, "assign").await.unwrap();
    let run_id = assigned["runId"].as_str().unwrap().to_owned();
    assert_eq!(
        h.rt.runtime
            .run_engine()
            .active_owner_of(&run_id)
            .await
            .as_deref(),
        Some(bench.as_str())
    );
    assert_eq!(
        h.engine.launches.load(Ordering::SeqCst) - baseline.launches,
        1
    );
    cancel(&h, &task_id, "cancel").await.unwrap();
    assert_eq!(
        task(&session(&h, &bench).await, &task_id)["status"],
        "cancelled"
    );
    assert_eq!(
        h.rt.runtime.run_engine().active_owner_of(&run_id).await,
        None
    );
    let gate = Arc::clone(h.rt.runtime.work_gate());
    eventually("gate reservations released", || {
        let gate = Arc::clone(&gate);
        async move { gate.reservation_total() == 0 }
    })
    .await;
}

/// 표 6: `bind_child_run`은 취소된 task를 거절한다(노드 run을 바꾸지 않는다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn binding_a_run_to_a_cancelled_task_is_refused() {
    let (h, bench, task_id) = ready_task().await;
    cancel(&h, &task_id, "cancel").await.unwrap();
    let current = session(&h, &bench).await;
    assert_eq!(task(&current, &task_id)["status"], "cancelled");
    let node_id = node_of(&current, &task_id)["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let refused =
        h.rt.runtime
            .orchestration()
            .service()
            .bind_child_run(&bench, &task_id, &node_id, "late-run");
    assert!(refused.is_err(), "bind refused: {refused:?}");
    let after = session(&h, &bench).await;
    assert!(node_of(&after, &task_id)["currentRunId"].is_null());
    assert_eq!(task(&after, &task_id)["status"], "cancelled");
}

/// 저장소 지점 멈춤(blocking 스레드에서 동기로 불림): `point`에 닿으면 알리고, 시험이 풀 때까지 그 커밋 단계를 붙잡는다.
/// 다른 지점은 알리기만 한다.
struct StorePause {
    reached: mpsc::UnboundedReceiver<StorePoint>,
    release: std::sync::mpsc::Sender<()>,
}

fn pause_store_at(h: &BenchHarness, point: StorePoint) -> StorePause {
    let (reached_tx, reached) = mpsc::unbounded_channel();
    let (release, release_rx) = std::sync::mpsc::channel::<()>();
    let release_rx = std::sync::Mutex::new(release_rx);
    let probe: StoreProbe = Arc::new(move |at| {
        let _ = reached_tx.send(at);
        if at == point {
            let _ = release_rx.lock().unwrap().recv_timeout(WAIT);
        }
    });
    h.rt.runtime.orchestration().set_store_probe(Some(probe));
    StorePause { reached, release }
}

impl StorePause {
    async fn wait_for(&mut self, point: StorePoint) {
        loop {
            let at = tokio::time::timeout(WAIT, self.reached.recv())
                .await
                .unwrap_or_else(|_| panic!("never reached {point:?}"))
                .expect("store probe alive");
            if at == point {
                return;
            }
        }
    }
}

type LaunchJoin = tokio::task::JoinHandle<
    workbench_core::application::orchestration::runtime::OrchestrationResult<
        workbench_core::domain::agent_orchestration::OrchestrationSession,
    >,
>;

/// 배정 future를 직접 띄운다(abort 대상 — HTTP 연결 단절 등으로 호출 future가 drop되는 경우와 같다).
fn spawn_launch(h: &Arc<BenchHarness>, bench: &str, task_id: &str) -> LaunchJoin {
    let (h, bench, task_id) = (Arc::clone(h), bench.to_owned(), task_id.to_owned());
    tokio::spawn(async move {
        h.rt.runtime
            .orchestration()
            .launch_task_for_ui(&bench, &task_id)
            .await
    })
}

/// abort 뒤 공통 끝 상태: 새 run 0(준비·실행한 run은 취소됨), 노드 run 해제, 관문 예약 0, scheduler 자리 반납, 정지 판정이
/// 실행 중 task로 막히지 않음(`orchestration_tasks == 0`), task는 `expected_status`.
async fn assert_launch_undone(
    h: &BenchHarness,
    bench: &str,
    task_id: &str,
    baseline: Baseline,
    expected_status: &str,
    label: &str,
) {
    eventually(&format!("{label}: the launched run is gone"), || async {
        h.engine.run_count() == baseline.runs
    })
    .await;
    eventually(&format!("{label}: node run released"), || async {
        node_of(&session(h, bench).await, task_id)["currentRunId"].is_null()
    })
    .await;
    let gate = Arc::clone(h.rt.runtime.work_gate());
    eventually(&format!("{label}: gate reservations released"), || {
        let gate = Arc::clone(&gate);
        async move { gate.reservation_total() == 0 }
    })
    .await;
    eventually(
        &format!("{label}: the scheduler slot is returned"),
        || async {
            h.rt.runtime
                .orchestration()
                .scheduler()
                .active_count()
                .unwrap()
                == 0
        },
    )
    .await;
    let current = session(h, bench).await;
    assert_eq!(
        task(&current, task_id)["status"],
        expected_status,
        "{label}: task and node agree (no run, no running task)"
    );
    assert!(
        node_of(&current, task_id)["currentRunId"].is_null(),
        "{label}: no late write re-binds the cancelled run"
    );
    assert_eq!(
        h.rt.runtime
            .server_control()
            .derive()
            .await
            .orchestration_tasks,
        0,
        "{label}: no running task blocks a wait stop"
    );
}

/// 재배정이 실제 run을 기동한다(`alreadyAssigned`로 막히지 않는다).
async fn assert_reassign_starts_a_real_run(
    h: &BenchHarness,
    bench: &str,
    task_id: &str,
    baseline: Baseline,
    label: &str,
) {
    let assigned = assign(h, task_id, &format!("reassign-{label}"))
        .await
        .unwrap();
    assert!(
        assigned.get("alreadyAssigned").is_none(),
        "{label}: a stale launch blocks reassignment: {assigned}"
    );
    assert_eq!(assigned["executionStatus"], "active", "{label}: {assigned}");
    let run_id = assigned["runId"].as_str().unwrap().to_owned();
    let current = session(h, bench).await;
    assert_eq!(node_of(&current, task_id)["currentRunId"], run_id.as_str());
    assert_eq!(task(&current, task_id)["status"], "running");
    let launched = format!("launch:{run_id}");
    h.engine.wait_applied(|label| label == launched, WAIT).await;
    assert_eq!(
        h.engine.run_count(),
        baseline.runs + 1,
        "{label}: exactly one live child run"
    );
}

/// Codex r7 (a): 바인딩 커밋 **직전**에 배정 future를 abort. 바인딩은 blocking 작업이라 커밋을 이어 가므로, 정리는 그 결과를
/// 기다린 뒤 run을 취소하고 task·노드를 한 트랜잭션에서 되돌려야 한다(늦은 바인딩이 취소된 run id를 다시 적지 않는다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_the_assign_just_before_the_bind_commit_undoes_the_launch() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let mut pause = pause_store_at(&h, StorePoint::BindBeforeCommit);
    let assigning = spawn_launch(&h, &bench, &task_id);
    pause.wait_for(StorePoint::BindBeforeCommit).await;
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    pause.release.send(()).unwrap();
    pause.wait_for(StorePoint::BindAfterCommit).await;
    h.rt.runtime.orchestration().set_store_probe(None);
    assert_launch_undone(&h, &bench, &task_id, baseline, "ready", "bind-before").await;
    assert_reassign_starts_a_real_run(&h, &bench, &task_id, baseline, "bind-before").await;
}

/// Codex r7 (b): 바인딩 커밋 **직후**·결과를 받기 전에 abort. task는 Running·노드는 새 run으로 커밋됐지만 배정은 끝나지 않았다
/// — 정리가 run을 취소하고 task를 다시 배정할 수 있는 상태로 되돌린다(Running인데 run 없음을 남기지 않는다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_the_assign_right_after_the_bind_commit_undoes_the_launch() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let mut pause = pause_store_at(&h, StorePoint::BindAfterCommit);
    let assigning = spawn_launch(&h, &bench, &task_id);
    pause.wait_for(StorePoint::BindAfterCommit).await;
    assert_eq!(
        task(&session(&h, &bench).await, &task_id)["status"],
        "running",
        "the bind committed"
    );
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    pause.release.send(()).unwrap();
    h.rt.runtime.orchestration().set_store_probe(None);
    assert_launch_undone(&h, &bench, &task_id, baseline, "ready", "bind-after").await;
    assert_reassign_starts_a_real_run(&h, &bench, &task_id, baseline, "bind-after").await;
}

/// Codex r7 (c): 실패 정리가 엔진 취소를 기다리는 동안 abort. 등록 전 취소로 기동이 실패하면 정리는 준비한 run을 취소하고
/// 노드 예약을 푼다 — 그 await 중에 배정 future가 drop돼도 정리는 끝까지 간다(준비한 run·노드 예약이 남지 않는다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_the_assign_while_failure_cleanup_cancels_the_engine_run_finishes_it() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let mut launch_pause = pause_at(&h, LaunchPoint::AfterPrepare);
    let (cancel_tx, mut cancel_reached) = mpsc::unbounded_channel();
    let cancel_release = Arc::new(Semaphore::new(0));
    {
        let gate = Arc::clone(&cancel_release);
        *h.engine.cancel_hook.lock().unwrap() = Some(Arc::new(move |_run| {
            let (tx, gate) = (cancel_tx.clone(), Arc::clone(&gate));
            Box::pin(async move {
                let _ = tx.send(());
                gate.acquire().await.expect("cancel gate").forget();
            })
        }));
    }
    let assigning = spawn_launch(&h, &bench, &task_id);
    tokio::time::timeout(WAIT, launch_pause.reached.recv())
        .await
        .expect("assign reached AfterPrepare");
    cancel(&h, &task_id, "cancel").await.unwrap();
    launch_pause.release.add_permits(1);
    tokio::time::timeout(WAIT, cancel_reached.recv())
        .await
        .expect("failure cleanup is cancelling the prepared run");
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    cancel_release.add_permits(1);
    assert_launch_undone(&h, &bench, &task_id, baseline, "cancelled", "fail-cancel").await;
    *h.engine.cancel_hook.lock().unwrap() = None;
    h.rt.runtime.orchestration().set_launch_probe(None);
    // 취소된 task는 다시 배정하지 않는다: 반납된 자리(동시 한도 1)로 새 과제가 실제 run을 기동한다.
    let created = tool(
        &h,
        "coord",
        OperationId::OrchestrationCreateChildTask,
        json!({
            "requestId": "c-after", "title": "after",
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read again", "expectedResult": "summary"
        }),
    )
    .await
    .unwrap();
    assert_eq!(created["executionStatus"], "active", "{created}");
    let launched = format!("launch:{}", created["runId"].as_str().unwrap());
    h.engine.wait_applied(|label| label == launched, WAIT).await;
}

/// Codex r7 (d): 실패 정리가 저장소 예약 해제를 기다리는 동안 abort. 엔진 준비가 실패하면 정리는 노드 예약을 풀고 호출자는
/// scheduler 자리를 반납한다 — 배정 future가 그 사이 drop되면 호출자가 없으므로 정리가 자리를 반납해야 한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_the_assign_while_failure_cleanup_releases_the_reservation_finishes_it() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    h.engine.fail_next_start.store(true, Ordering::SeqCst);
    let mut pause = pause_store_at(&h, StorePoint::RevertBeforeCommit);
    let assigning = spawn_launch(&h, &bench, &task_id);
    pause.wait_for(StorePoint::RevertBeforeCommit).await;
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    pause.release.send(()).unwrap();
    h.rt.runtime.orchestration().set_store_probe(None);
    assert_launch_undone(&h, &bench, &task_id, baseline, "ready", "fail-release").await;
    assert_eq!(child_executions(&h, baseline), Vec::<String>::new());
    assert_reassign_starts_a_real_run(&h, &bench, &task_id, baseline, "fail-release").await;
}

/// Codex r7 관련 전이: 되돌리기가 끝나기 전의 새 배정은 곧 취소될 예정 run id(`alreadyAssigned` 또는 `Started`)를 받지 않고
/// 재시도 가능하게 거절된다. 되돌리기가 끝나면 같은 배정이 실제 run을 기동한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_assign_during_a_rollback_is_refused_retryably_instead_of_getting_the_doomed_run() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let mut pause = pause_store_at(&h, StorePoint::BindAfterCommit);
    let assigning = spawn_launch(&h, &bench, &task_id);
    pause.wait_for(StorePoint::BindAfterCommit).await;
    let doomed = node_of(&session(&h, &bench).await, &task_id)["currentRunId"]
        .as_str()
        .unwrap()
        .to_owned();
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    // 바인딩 커밋은 아직 붙잡혀 있다: 되돌리기가 그 결과를 기다리는 중이다.
    // 상한(대기 상한, 기대값 아님): 되돌리기 중 배정은 붙잡힌 커밋을 기다리지 않고 곧바로 답해야 한다.
    let during = tokio::time::timeout(WAIT, assign(&h, &task_id, "during-rollback"))
        .await
        .expect("the assign during the rollback answers without waiting for the held commit")
        .expect("the assign call itself is answered");
    assert!(
        during["runId"].is_null(),
        "an assign during the rollback got a run: {during} (doomed {doomed})"
    );
    assert_eq!(during["launch"]["status"], "failed", "{during}");
    assert_eq!(during["launch"]["code"], "launchRollingBack", "{during}");
    assert_eq!(during["launch"]["retryable"], true, "{during}");
    assert_eq!(
        h.rt.runtime
            .orchestration()
            .scheduler()
            .active_count()
            .unwrap(),
        1,
        "the refused assign does not return the slot the rollback still holds"
    );
    pause.release.send(()).unwrap();
    h.rt.runtime.orchestration().set_store_probe(None);
    assert_launch_undone(&h, &bench, &task_id, baseline, "ready", "during-rollback").await;
    assert_reassign_starts_a_real_run(&h, &bench, &task_id, baseline, "during-rollback").await;
}

/// 동시 한도 1에서 새 과제를 만든다(자리를 이미 누가 쥐고 있으면 대기열에 든다).
async fn create_extra(h: &BenchHarness, key: &str) -> Value {
    tool(
        h,
        "coord",
        OperationId::OrchestrationCreateChildTask,
        json!({
            "requestId": key, "title": format!("extra {key}"),
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read again", "expectedResult": "summary"
        }),
    )
    .await
    .unwrap()
}

/// Codex r8 (medium): 같은 task의 자리를 여러 배정 시도가 함께 쥔다. 되돌리는 중인 앞 기동 A가 있을 때 새 배정 B가 자리
/// 보유를 얻고 작업 영역을 읽기 전에 멈춘다 → A의 되돌리기가 끝난다(A는 자기 보유만 놓는다) → B가 재개해 실제 run을
/// 기동한다. B의 run은 자리를 쥔 채 실행되고, 동시 한도(1)는 넘지 않는다 — 새 과제는 대기열에 든다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_reassign_holding_the_slot_across_a_rollback_keeps_its_slot() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let scheduler = h.rt.runtime.orchestration().scheduler().clone();
    let mut store = pause_store_at(&h, StorePoint::BindAfterCommit);
    let a = spawn_launch(&h, &bench, &task_id);
    store.wait_for(StorePoint::BindAfterCommit).await;
    a.abort();
    assert!(a.await.unwrap_err().is_cancelled());
    let mut b_pause = pause_at(&h, LaunchPoint::BeforeAssignSnapshot);
    let b = {
        let (h, task_id) = (Arc::clone(&h), task_id.clone());
        tokio::spawn(async move { assign(&h, &task_id, "b-across-rollback").await })
    };
    tokio::time::timeout(WAIT, b_pause.reached.recv())
        .await
        .expect("B holds the slot and is about to read the workspace");
    assert_eq!(
        scheduler.hold_count(&task_id),
        2,
        "A (rolling back) and B hold it"
    );
    store.release.send(()).unwrap();
    h.rt.runtime.orchestration().set_store_probe(None);
    // A의 정리가 끝났다: run 취소·노드 해제, 그리고 A가 자기 보유를 놓았다(보유가 2 미만).
    eventually("A's rollback finished", || async {
        h.engine.run_count() == baseline.runs
            && node_of(&session(&h, &bench).await, &task_id)["currentRunId"].is_null()
            && scheduler.hold_count(&task_id) < 2
    })
    .await;
    assert_eq!(scheduler.hold_count(&task_id), 1, "only B's hold is left");
    assert_eq!(
        scheduler.active_count().unwrap(),
        1,
        "A's cleanup kept the slot B still holds"
    );
    b_pause.release.add_permits(1);
    let assigned = tokio::time::timeout(WAIT, b)
        .await
        .expect("B answers")
        .unwrap()
        .unwrap();
    assert_eq!(assigned["executionStatus"], "active", "{assigned}");
    let run_id = assigned["runId"].as_str().unwrap().to_owned();
    let launched = format!("launch:{run_id}");
    h.engine.wait_applied(|label| label == launched, WAIT).await;
    h.rt.runtime.orchestration().set_launch_probe(None);
    assert_eq!(h.engine.run_count(), baseline.runs + 1);
    assert_eq!(
        scheduler.active_count().unwrap(),
        1,
        "B's run owns the slot"
    );
    assert_eq!(
        scheduler.hold_count(&task_id),
        0,
        "B's hold became the running slot"
    );
    let extra = create_extra(&h, "extra-after-b").await;
    assert_eq!(
        extra["queued"], true,
        "the concurrency limit (1) holds while B runs: {extra}"
    );
    assert_eq!(scheduler.active_count().unwrap(), 1);
}

/// Codex r8 (medium): 되돌리기가 저장소에 저장되지 못하면 완료로 보지 않는다. 바인딩 커밋 뒤 abort된 기동의 되돌리기 커밋을
/// 실패시킨다: 엔진 run은 취소됐지만 저장소는 그 run을 가리킨다. 그 동안 (1) 단일 비행 자리는 "되돌리는 중"으로 남아 새
/// 배정은 죽은 run id(`alreadyAssigned`)가 아니라 재시도 가능한 `launchRollingBack`을 받고, (2) 이 시도의 scheduler 보유가
/// 남으며, (3) 정리 미완료가 활동 작업으로 보여 `default` 정지가 거절되고 감시 바퀴는 멈추지 않는다. 저장소가 회복되면
/// 감시 바퀴의 재시도가 정리를 끝내고 재배정이 실제 run을 기동한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_rollback_that_cannot_be_stored_is_kept_and_retried_until_it_succeeds() {
    use workbench_core::application::server_control::StopOutcome;
    use workbench_protocol::operations::server::StopModeDto;
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let orchestration = h.rt.runtime.orchestration();
    orchestration.fail_next_reverts(1_000);
    let mut store = pause_store_at(&h, StorePoint::BindAfterCommit);
    let a = spawn_launch(&h, &bench, &task_id);
    store.wait_for(StorePoint::BindAfterCommit).await;
    a.abort();
    assert!(a.await.unwrap_err().is_cancelled());
    store.release.send(()).unwrap();
    orchestration.set_store_probe(None);
    eventually("the unstored rollback is kept for a retry", || async {
        orchestration.pending_revert_tasks() == vec![task_id.clone()]
            && h.engine.run_count() == baseline.runs
    })
    .await;
    let stored = session(&h, &bench).await;
    let doomed = node_of(&stored, &task_id)["currentRunId"]
        .as_str()
        .expect("the store still points at the cancelled run")
        .to_owned();
    assert_eq!(task(&stored, &task_id)["status"], "running");

    let during = tokio::time::timeout(WAIT, assign(&h, &task_id, "during-unstored"))
        .await
        .expect("answers")
        .expect("answered");
    assert!(
        during.get("alreadyAssigned").is_none(),
        "{during} (doomed {doomed})"
    );
    assert!(during["runId"].is_null(), "{during}");
    assert_eq!(during["launch"]["code"], "launchRollingBack", "{during}");
    assert_eq!(during["launch"]["retryable"], true, "{during}");
    let scheduler = orchestration.scheduler();
    assert_eq!(
        scheduler.active_count().unwrap(),
        1,
        "the rolling-back hold is kept"
    );
    let control = h.rt.runtime.server_control();
    for round in 0..3 {
        assert!(
            !control.tick(Duration::from_secs(600)).await,
            "round {round}: the retry fails again"
        );
        assert_eq!(orchestration.pending_revert_tasks(), vec![task_id.clone()]);
    }
    match control.request_stop(StopModeDto::Default).await {
        StopOutcome::Blocked(active) => assert!(
            active.orchestration_tasks.is_some_and(|count| count >= 1),
            "the unfinished cleanup is active work: {active:?}"
        ),
        other => panic!("an unfinished rollback must block the default stop: {other:?}"),
    }

    orchestration.fail_next_reverts(0);
    assert!(!control.tick(Duration::from_secs(600)).await);
    assert!(
        orchestration.pending_revert_tasks().is_empty(),
        "the watch-loop retry stored the rollback"
    );
    assert_launch_undone(&h, &bench, &task_id, baseline, "ready", "unstored").await;
    assert_reassign_starts_a_real_run(&h, &bench, &task_id, baseline, "unstored").await;
}

/// Codex r8 + 사용자 요구: 저장되지 않은 되돌리기가 남은 채 서버가 멈추면(메모리의 재시도 목록은 사라진다) 저장소에는
/// 예약만 된(`Starting`) 노드가 죽은 run을 가리킨다. 같은 저장소로 다시 시작해 작업 영역을 복구(`recover`)하면 그 예약을
/// 되돌린다 — 다음 기동은 죽은 run id를 돌려받지 않고 실제 run을 기동한다. 실패 정리(`fail`) 경로: 엔진 준비 실패 뒤
/// 되돌리기 커밋 실패.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unstored_rollback_left_by_a_stop_is_undone_by_the_restart_recovery() {
    let (h, bench, task_id, _baseline) = ready_task_with_baseline().await;
    let orchestration = h.rt.runtime.orchestration();
    orchestration.fail_next_reverts(1_000);
    h.engine.fail_next_start.store(true, Ordering::SeqCst);
    let failed = orchestration.launch_task_for_ui(&bench, &task_id).await;
    assert!(failed.is_err(), "the launch failed: {failed:?}");
    assert_eq!(orchestration.pending_revert_tasks(), vec![task_id.clone()]);
    let stored = session(&h, &bench).await;
    let doomed = node_of(&stored, &task_id)["currentRunId"]
        .as_str()
        .expect("the reservation is still stored")
        .to_owned();
    assert_eq!(node_of(&stored, &task_id)["executionStatus"], "starting");
    let workspace_id = stored["id"].as_str().unwrap().to_owned();
    // 정리가 저장되지 않은 동안은 활동 작업이다: task는 `Ready`라 task 수로는 막히지 않지만 정지는 거절된다.
    {
        use workbench_core::application::server_control::StopOutcome;
        use workbench_protocol::operations::server::StopModeDto;
        match h
            .rt
            .runtime
            .server_control()
            .request_stop(StopModeDto::Default)
            .await
        {
            StopOutcome::Blocked(active) => assert!(
                active.orchestration_tasks.is_some_and(|count| count >= 1),
                "{active:?}"
            ),
            other => panic!("an unstored rollback must block the default stop: {other:?}"),
        }
    }

    // 멈춤: 작업대를 닫고(강제 정지와 같음) 같은 저장소로 새 런타임을 조립한다(재시도 목록은 메모리라 사라진다).
    h.rt.runtime.close_all_benches().await;
    let h = Arc::try_unwrap(h).ok().expect("sole harness owner");
    let h = h.restart_runtime(
        |adapters| adapters.orchestration.max_concurrent_children = 1,
        RunScript::default(),
    );
    let bench = h.open().await;
    let desktop = desktop();
    h.call(
        &desktop,
        OperationId::OrchestrationBootstrap,
        json!({ "benchId": bench, "worktreePath": h.dir, "resumeWorkspaceId": workspace_id }),
    )
    .await
    .unwrap();
    let orchestration = h.rt.runtime.orchestration();
    assert!(
        orchestration.pending_revert_tasks().is_empty(),
        "a new process"
    );
    h.call(
        &desktop,
        OperationId::OrchestrationRecover,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap();
    let recovered = session(&h, &bench).await;
    assert!(
        node_of(&recovered, &task_id)["currentRunId"].is_null(),
        "the recovery undid the stale reservation of {doomed}: {recovered}"
    );
    assert_eq!(task(&recovered, &task_id)["status"], "ready");

    orchestration
        .launch_task_for_ui(&bench, &task_id)
        .await
        .expect("the task launches again");
    let relaunched = session(&h, &bench).await;
    let run_id = node_of(&relaunched, &task_id)["currentRunId"]
        .as_str()
        .expect("a new run")
        .to_owned();
    assert_ne!(run_id, doomed, "not the dead run");
    assert_eq!(task(&relaunched, &task_id)["status"], "running");
    let launched = format!("launch:{run_id}");
    h.engine.wait_applied(|label| label == launched, WAIT).await;
}

/// 실패 정리(`fail`) 경로로 저장되지 않은 되돌리기 하나를 남긴다: 엔진 준비 실패 → 되돌리기 커밋 실패. task는 `Ready`이고
/// 노드 예약(`Starting`)은 죽은 run을 가리킨다.
async fn leave_an_unstored_rollback(h: &BenchHarness, bench: &str, task_id: &str) -> String {
    let orchestration = h.rt.runtime.orchestration();
    orchestration.fail_next_reverts(1);
    h.engine.fail_next_start.store(true, Ordering::SeqCst);
    let failed = orchestration.launch_task_for_ui(bench, task_id).await;
    assert!(failed.is_err(), "the launch failed: {failed:?}");
    assert_eq!(
        orchestration.pending_revert_tasks(),
        vec![task_id.to_owned()]
    );
    let stored = session(h, bench).await;
    assert_eq!(task(&stored, task_id)["status"], "ready");
    node_of(&stored, task_id)["currentRunId"]
        .as_str()
        .expect("the reservation is still stored")
        .to_owned()
}

/// Codex r9 (a): 되돌리기 **재시도가 진행 중인 동안**에도 정리는 활동 작업이다. 재시도가 저장소 커밋 직전에 멈춘 사이의
/// `default`·`wait` 정지는 멈추지 않고, 그 재시도가 다시 저장에 실패해도 활동으로 남으며, 저장된 뒤에야 wait가 멈춘다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_rollback_retry_in_flight_stays_active_work_until_it_is_stored() {
    use workbench_core::application::{server_control::StopOutcome, work_gate::GateState};
    use workbench_protocol::operations::server::StopModeDto;
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let orchestration = Arc::clone(h.rt.runtime.orchestration());
    let control = Arc::clone(h.rt.runtime.server_control());
    leave_an_unstored_rollback(&h, &bench, &task_id).await;
    // 작업대를 닫아 coordinator run과 그 밖의 활동을 없앤다 — 남은 활동은 이 정리뿐이다(작업 영역 id로 정리한다).
    h.rt.runtime.close_all_benches().await;

    // 재시도 1: 커밋 직전에 멈춘다. 그 사이의 정지 판정은 정리를 활동으로 본다.
    orchestration.fail_next_reverts(1);
    let mut store = pause_store_at(&h, StorePoint::RevertBeforeCommit);
    let retry = {
        let orchestration = Arc::clone(&orchestration);
        tokio::spawn(async move { orchestration.retry_pending_reverts().await })
    };
    store.wait_for(StorePoint::RevertBeforeCommit).await;
    assert_eq!(
        orchestration.pending_revert_tasks(),
        vec![task_id.clone()],
        "a retry in flight is still visible"
    );
    match control.request_stop(StopModeDto::Default).await {
        StopOutcome::Blocked(active) => assert!(
            active.orchestration_tasks.is_some_and(|count| count >= 1),
            "the cleanup in flight is active work: {active:?}"
        ),
        other => panic!("a rollback retry in flight must block the default stop: {other:?}"),
    }
    let waited = control.request_stop(StopModeDto::Wait).await;
    assert!(
        matches!(waited, StopOutcome::Draining),
        "wait drains while the retry is in flight: {waited:?}"
    );
    // 재시도가 다시 저장에 실패한다: 정리는 목록에 남고(다시 시도), wait는 멈추지 않는다.
    store.release.send(()).unwrap();
    tokio::time::timeout(WAIT, retry)
        .await
        .expect("the retry returns")
        .expect("the retry task");
    assert_eq!(orchestration.pending_revert_tasks(), vec![task_id.clone()]);
    orchestration.set_store_probe(None);
    orchestration.fail_next_reverts(1);
    assert!(
        !control.tick(Duration::from_secs(600)).await,
        "the failed retry keeps the wait stop from finishing"
    );
    assert_ne!(h.rt.runtime.work_gate().state(), GateState::Stopping);
    assert_eq!(orchestration.pending_revert_tasks(), vec![task_id.clone()]);
    // 저장소가 회복된다: 다음 바퀴의 재시도가 정리를 끝내고 wait가 멈춘다.
    let mut stopped = false;
    for _ in 0..3 {
        if control.tick(Duration::from_secs(600)).await {
            stopped = true;
            break;
        }
    }
    assert!(stopped, "the wait stop finishes once the cleanup is stored");
    assert!(orchestration.pending_revert_tasks().is_empty());
    assert_eq!(orchestration.scheduler().active_count().unwrap(), 0);
    let _ = baseline;
}

/// Codex r9 (b): 되돌리기 재시도 future가 저장소 커밋 중에 abort돼도 정리 책임은 사라지지 않는다 — 커밋이 끝나면 단일
/// 비행 자리와 scheduler 보유를 정리하고, 재배정이 실제 run을 기동한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn aborting_a_rollback_retry_mid_commit_still_finishes_the_cleanup() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let orchestration = Arc::clone(h.rt.runtime.orchestration());
    leave_an_unstored_rollback(&h, &bench, &task_id).await;
    let mut store = pause_store_at(&h, StorePoint::RevertBeforeCommit);
    let retry = {
        let orchestration = Arc::clone(&orchestration);
        tokio::spawn(async move { orchestration.retry_pending_reverts().await })
    };
    store.wait_for(StorePoint::RevertBeforeCommit).await;
    retry.abort();
    assert!(retry.await.unwrap_err().is_cancelled());
    assert_eq!(
        orchestration.pending_revert_tasks(),
        vec![task_id.clone()],
        "the aborted retry still owns the cleanup"
    );
    store.release.send(()).unwrap();
    orchestration.set_store_probe(None);
    eventually("the aborted retry's commit settles the cleanup", || async {
        orchestration.pending_revert_tasks().is_empty()
    })
    .await;
    assert_launch_undone(&h, &bench, &task_id, baseline, "ready", "aborted retry").await;
    assert_reassign_starts_a_real_run(&h, &bench, &task_id, baseline, "aborted retry").await;
}

/// Codex r9 (c): 저장되지 않은 되돌리기가 남은 채 작업대를 닫으면(작업 영역의 작업대 묶임이 풀린다) 재시도는 작업대 id가
/// 아니라 작업 영역 id로 정리를 끝낸다. 저장소가 회복되면 목록·보유가 비고, 정지가 막히지 않으며, 같은 작업 영역을 새
/// 작업대로 다시 열어 재배정하면 실제 run이 기동한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unstored_rollback_survives_a_bench_close_and_finishes_by_workspace() {
    let (h, bench, task_id, _baseline) = ready_task_with_baseline().await;
    let orchestration = Arc::clone(h.rt.runtime.orchestration());
    let doomed = leave_an_unstored_rollback(&h, &bench, &task_id).await;
    let workspace_id = session(&h, &bench).await["id"].as_str().unwrap().to_owned();
    orchestration.fail_next_reverts(1_000);
    h.rt.runtime.close_all_benches().await;
    let control = Arc::clone(h.rt.runtime.server_control());
    assert!(!control.tick(Duration::from_secs(600)).await);
    assert_eq!(
        orchestration.pending_revert_tasks(),
        vec![task_id.clone()],
        "while the store fails the cleanup stays (observable)"
    );
    orchestration.fail_next_reverts(0);
    assert!(!control.tick(Duration::from_secs(600)).await);
    assert!(
        orchestration.pending_revert_tasks().is_empty(),
        "the retry finished the cleanup of a closed bench by workspace id"
    );
    assert_eq!(
        orchestration.scheduler().active_count().unwrap(),
        0,
        "the rolling-back hold is released"
    );
    let derived = control.derive().await;
    assert_eq!(derived.pending_launch_reverts, 0, "{derived:?}");
    assert_eq!(
        derived.active_total(),
        0,
        "nothing left blocks a normal stop: {derived:?}"
    );
    // 같은 작업 영역을 새 작업대로 다시 열어 재배정한다(같은 저장소, 새 런타임).
    let h = Arc::try_unwrap(h).ok().expect("sole harness owner");
    let h = h.restart_runtime(
        |adapters| adapters.orchestration.max_concurrent_children = 1,
        RunScript::default(),
    );
    let bench = h.open().await;
    h.call(
        &desktop(),
        OperationId::OrchestrationBootstrap,
        json!({ "benchId": bench, "worktreePath": h.dir, "resumeWorkspaceId": workspace_id }),
    )
    .await
    .unwrap();
    let resumed = session(&h, &bench).await;
    assert!(
        node_of(&resumed, &task_id)["currentRunId"].is_null(),
        "the stale reservation of {doomed} is gone: {resumed}"
    );
    assert_eq!(task(&resumed, &task_id)["status"], "ready");
    let orchestration = h.rt.runtime.orchestration();
    orchestration
        .launch_task_for_ui(&bench, &task_id)
        .await
        .expect("the task launches again");
    let relaunched = session(&h, &bench).await;
    let run_id = node_of(&relaunched, &task_id)["currentRunId"]
        .as_str()
        .expect("a new run")
        .to_owned();
    assert_ne!(run_id, doomed);
    let launched = format!("launch:{run_id}");
    h.engine.wait_applied(|label| label == launched, WAIT).await;
}

/// 작업 영역 복구(`orchestration.recover`)를 한 번 부른다(상한 있음).
async fn recover(h: &BenchHarness, bench: &str) {
    tokio::time::timeout(
        WAIT,
        h.call(
            &desktop(),
            OperationId::OrchestrationRecover,
            json!({ "benchId": bench }),
        ),
    )
    .await
    .expect("the recovery answers")
    .expect("the recovery succeeds");
}

/// 동시 한도(1)가 비었다: 새 과제가 대기열에 들지 않고 곧바로 자리를 얻는다.
async fn assert_capacity_is_free(h: &BenchHarness, key: &str) {
    let extra = create_extra(h, key).await;
    assert_ne!(
        extra["queued"], true,
        "{key}: a leaked slot keeps the concurrency limit full: {extra}"
    );
}

/// Codex r10 (medium): 바인딩 커밋 **직후**·결과 수신 전에 복구가 돌면 task는 `Running`, run은 살아 있다. 복구가 그 자리를
/// 실행 중 자리로 확정하면서 진행 중 기동 A의 보유를 지우면, 이어서 A가 abort돼 정리해도 자리가 남는다(한도 1이면 실행
/// 중 자식이 없는데 다른 task가 계속 대기). 복구는 진행 중 기동의 보유를 보존하고 성공 인계 전에는 실행 중으로 확정하지
/// 않는다 → A의 정리가 끝나면 자리가 비고 새 과제가 곧바로 자리를 얻는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_recovery_between_the_bind_commit_and_an_abort_does_not_leak_the_slot() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let scheduler = h.rt.runtime.orchestration().scheduler().clone();
    let mut pause = pause_store_at(&h, StorePoint::BindAfterCommit);
    let assigning = spawn_launch(&h, &bench, &task_id);
    pause.wait_for(StorePoint::BindAfterCommit).await;
    recover(&h, &bench).await;
    assert_eq!(
        scheduler.hold_count(&task_id),
        1,
        "the recovery keeps the in-flight launch's hold"
    );
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    pause.release.send(()).unwrap();
    h.rt.runtime.orchestration().set_store_probe(None);
    assert_launch_undone(
        &h,
        &bench,
        &task_id,
        baseline,
        "ready",
        "bind-recover-abort",
    )
    .await;
    assert_capacity_is_free(&h, "extra-after-bind-recover-abort").await;
}

/// Codex r10: 노드 예약 커밋 직후(task는 `Ready`, 노드는 `Starting`) 복구가 돌고 A가 abort된다. 복구는 A의 보유를 지우고 task를
/// 대기열에 넣지 않는다(보유를 쥔 자리가 남는다) → A의 정리가 끝나면 자리가 비고 새 과제가 곧바로 자리를 얻는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_recovery_between_the_reservation_and_an_abort_keeps_the_hold_then_frees_the_slot() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let scheduler = h.rt.runtime.orchestration().scheduler().clone();
    let mut pause = pause_store_at(&h, StorePoint::ReserveAfterCommit);
    let assigning = spawn_launch(&h, &bench, &task_id);
    pause.wait_for(StorePoint::ReserveAfterCommit).await;
    recover(&h, &bench).await;
    assert_eq!(
        scheduler.hold_count(&task_id),
        1,
        "the recovery keeps the in-flight launch's hold"
    );
    assert_eq!(
        scheduler.active_count().unwrap(),
        1,
        "the in-flight launch still counts against the limit"
    );
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    pause.release.send(()).unwrap();
    h.rt.runtime.orchestration().set_store_probe(None);
    assert_launch_undone(
        &h,
        &bench,
        &task_id,
        baseline,
        "ready",
        "reserve-recover-abort",
    )
    .await;
    assert_capacity_is_free(&h, "extra-after-reserve-recover-abort").await;
}

/// Codex r10: 노드 예약 커밋 직후 복구가 돌고 A가 **성공**한다. 복구가 A의 보유를 지우면 A의 성공 인계(`transfer`)가 자리를
/// 찾지 못해 실행 중 run이 자리 없이 돈다(한도 초과). 보유가 보존되면 A의 run이 자리를 쥐고 한도(1)가 지켜진다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_recovery_crossing_a_successful_launch_keeps_the_concurrency_limit() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let scheduler = h.rt.runtime.orchestration().scheduler().clone();
    let mut pause = pause_store_at(&h, StorePoint::ReserveAfterCommit);
    let assigning = spawn_launch(&h, &bench, &task_id);
    pause.wait_for(StorePoint::ReserveAfterCommit).await;
    recover(&h, &bench).await;
    pause.release.send(()).unwrap();
    h.rt.runtime.orchestration().set_store_probe(None);
    tokio::time::timeout(WAIT, assigning)
        .await
        .expect("the launch finishes")
        .unwrap()
        .expect("the launch succeeds");
    let run_id = node_of(&session(&h, &bench).await, &task_id)["currentRunId"]
        .as_str()
        .expect("the launched run is bound")
        .to_owned();
    let launched = format!("launch:{run_id}");
    h.engine.wait_applied(|label| label == launched, WAIT).await;
    assert_eq!(h.engine.run_count(), baseline.runs + 1);
    assert_eq!(
        scheduler.active_count().unwrap(),
        1,
        "the launched run owns the slot"
    );
    assert_eq!(
        scheduler.hold_count(&task_id),
        0,
        "the hold became the running slot"
    );
    let extra = create_extra(&h, "extra-after-recover-success").await;
    assert_eq!(
        extra["queued"], true,
        "the concurrency limit (1) holds while the launched run runs: {extra}"
    );
}

/// Codex r10: 저장되지 않은 되돌리기가 남은 동안 복구가 돌아도 그 정리의 보유는 남는다(정리가 끝날 때까지 자리를 쓴다). 재시도가
/// 정리를 끝내면 자리가 비고, 재배정이 실제 run을 기동한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_recovery_during_an_unstored_rollback_keeps_its_hold_until_the_retry_finishes() {
    let (h, bench, task_id, baseline) = ready_task_with_baseline().await;
    let orchestration = h.rt.runtime.orchestration();
    let scheduler = orchestration.scheduler().clone();
    leave_an_unstored_rollback(&h, &bench, &task_id).await;
    assert_eq!(
        scheduler.hold_count(&task_id),
        1,
        "the pending rollback holds the slot"
    );
    recover(&h, &bench).await;
    assert_eq!(
        scheduler.hold_count(&task_id),
        1,
        "the recovery keeps the pending rollback's hold"
    );
    tokio::time::timeout(WAIT, orchestration.retry_pending_reverts())
        .await
        .expect("the retry finishes");
    assert!(
        orchestration.pending_revert_tasks().is_empty(),
        "the rollback is stored"
    );
    eventually("the slot is free after the rollback", || async {
        scheduler.active_count().unwrap() == 0
    })
    .await;
    assert_reassign_starts_a_real_run(&h, &bench, &task_id, baseline, "recover-pending").await;
}
