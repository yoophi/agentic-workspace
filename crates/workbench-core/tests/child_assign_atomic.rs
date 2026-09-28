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
    LaunchPoint, LaunchProbe, ReservePoint, ReserveProbe,
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
    let probe: ReserveProbe = Arc::new(move |point| {
        let _ = reached_tx.send(point);
        if point == ReservePoint::BeforeCommit {
            // blocking 스레드에서 불린다: 시험이 abort를 마칠 때까지 커밋을 붙잡는다.
            let _ = release_rx.lock().unwrap().recv();
        }
    });
    h.rt.runtime.orchestration().set_reserve_probe(Some(probe));
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
        Some(ReservePoint::BeforeCommit)
    );
    assigning.abort();
    assert!(assigning.await.unwrap_err().is_cancelled());
    release_tx.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(WAIT, reached.recv()).await.unwrap(),
        Some(ReservePoint::AfterCommit),
        "the reservation committed after the abort"
    );
    h.rt.runtime.orchestration().set_reserve_probe(None);
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
