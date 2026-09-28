//! 041 US2: agent orchestration operation. 역할은 서버 상태로 판정하고(research R7), principal run과 입력 run이
//! 같아야 한다. `waitChildTasks`는 workspace revision watch로 깨어난다(R12) — 자식 `reportResult`가 대기를 바로
//! 깨우고(liveness ②), 보고와 대기가 동시에 일어나도 알림을 놓치지 않는다. coordinator 알림은 Main 턴이 도구를
//! 부르는 동안에도 교착 없이 끝난다(liveness ①).

#![allow(clippy::result_large_err)]

mod support;

use std::{
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::application::orchestration::runtime::{LaunchPoint, LaunchProbe};
use workbench_protocol::{AuthenticatedPrincipal, FaultCode, OperationId, WorkbenchFault};

/// 044 시작 장벽: 자식 실행(첫 턴 hook 포함)은 장벽이 열린 뒤 따로 돈다. 바인딩 **전에** 첫 턴이 끝나도록 장벽을 연
/// 직후(`AfterOpen`) 지점에서 hook이 끝났다는 허가를 기다린다(오늘 "엔진 등록 직후·노드 기록 전" 순서를 결정적으로 유지).
fn hold_binding_until(h: &BenchHarness, first_turn_done: Arc<tokio::sync::Semaphore>) {
    let probe: LaunchProbe = Arc::new(move |point| {
        let done = Arc::clone(&first_turn_done);
        Box::pin(async move {
            if point == LaunchPoint::AfterOpen {
                done.acquire().await.expect("first turn gate").forget();
            }
        })
    });
    h.rt.runtime.orchestration().set_launch_probe(Some(probe));
}

fn desktop() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop()
}

/// 자식 기동은 작업 트리 지문(git)을 요구한다.
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

struct Fixture {
    h: Arc<BenchHarness>,
    bench: String,
    coordinator: String,
}

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
    Fixture {
        h,
        bench,
        coordinator: "coord".into(),
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

async fn role(h: &BenchHarness, run: &str) -> Value {
    h.call(
        &AuthenticatedPrincipal::agent(run),
        OperationId::OrchestrationGetAgentRole,
        json!({ "runId": run }),
    )
    .await
    .unwrap()
}

/// coordinator가 자식 과제를 만들고 띄운다 → (task id, 자식 run id).
async fn create_child(f: &Fixture, key: &str) -> (String, String) {
    let created = tool(
        &f.h,
        &f.coordinator,
        OperationId::OrchestrationCreateChildTask,
        json!({
            "requestId": key, "title": format!("task {key}"),
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read the repo", "expectedResult": "summary"
        }),
    )
    .await
    .unwrap();
    let run = created["runId"]
        .as_str()
        .unwrap_or_else(|| panic!("child started: {created}"))
        .to_owned();
    (created["taskId"].as_str().unwrap().to_owned(), run)
}

async fn report_result(h: &BenchHarness, run: &str, key: &str) -> Result<Value, WorkbenchFault> {
    tool(
        h,
        run,
        OperationId::OrchestrationReportResult,
        json!({ "requestId": key, "summary": "done", "confidence": 0.9 }),
    )
    .await
}

fn tool_code(fault: &WorkbenchFault) -> &str {
    fault.details.as_ref().unwrap()["toolError"]["code"]
        .as_str()
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn roles_come_from_server_state() {
    let f = fixture().await;
    let coordinator = role(&f.h, &f.coordinator).await;
    assert_eq!(coordinator["role"], "coordinator");
    assert!(coordinator["workspaceId"].is_string());

    let (task, child) = create_child(&f, "c1").await;
    let child_role = role(&f.h, &child).await;
    assert_eq!(
        (child_role["role"].as_str(), child_role["taskId"].as_str()),
        (Some("child"), Some(task.as_str()))
    );
    let own = tool(
        &f.h,
        &child,
        OperationId::OrchestrationGetOwnTask,
        json!({}),
    )
    .await
    .unwrap();
    assert_eq!(own["id"], task.as_str());

    // 자식은 coordinator 도구를, coordinator는 자식 도구를 부를 수 없다.
    let refused = tool(
        &f.h,
        &child,
        OperationId::OrchestrationListChildTasks,
        json!({}),
    )
    .await
    .unwrap_err();
    assert_eq!(
        (refused.code, tool_code(&refused)),
        (FaultCode::Forbidden, "forbiddenActor")
    );
    let refused = report_result(&f.h, &f.coordinator, "r0").await.unwrap_err();
    assert_eq!(tool_code(&refused), "forbiddenActor");

    // 작업 영역과 무관한 run은 역할이 없다.
    f.h.start(&f.bench, "stray").await.unwrap();
    assert_eq!(role(&f.h, "stray").await["role"], Value::Null);
    let stray = tool(
        &f.h,
        "stray",
        OperationId::OrchestrationGetOwnTask,
        json!({}),
    )
    .await
    .unwrap_err();
    assert_eq!(tool_code(&stray), "forbiddenActor");

    // principal run과 입력 run이 다르면 거절(도구 쪽 검사 전에).
    let mismatch =
        f.h.call(
            &AuthenticatedPrincipal::agent(&child),
            OperationId::OrchestrationGetOwnTask,
            json!({ "runId": f.coordinator, "arguments": {} }),
        )
        .await
        .unwrap_err();
    assert_eq!(mismatch.code, FaultCode::Forbidden);
    assert!(mismatch.details.is_none());

    // 작업대를 닫으면 작업 영역이 어느 작업대에도 묶이지 않는다 — 기록된 run이 불러도 도구를 쓸 수 없다.
    f.h.close(&f.bench).await.unwrap();
    let closed = tool(
        &f.h,
        &child,
        OperationId::OrchestrationGetOwnTask,
        json!({}),
    )
    .await
    .unwrap_err();
    assert_eq!(tool_code(&closed), "scopeMismatch");
    assert_eq!(role(&f.h, &child).await["role"], Value::Null);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn previous_generation_coordinator_loses_its_tools() {
    let f = fixture().await;
    f.h.start(&f.bench, "coord-2").await.unwrap();
    let session =
        f.h.call(
            &desktop(),
            OperationId::OrchestrationGet,
            json!({ "benchId": f.bench }),
        )
        .await
        .unwrap();
    f.h.call(
        &desktop(),
        OperationId::OrchestrationHandoffCoordinator,
        json!({ "benchId": f.bench, "request": {
            "requestId": "handoff-1", "successorRunId": "coord-2", "summary": "handoff",
            "confirmed": true, "expectedRevision": session["revision"] } }),
    )
    .await
    .unwrap();
    assert_eq!(role(&f.h, "coord-2").await["role"], "coordinator");
    assert_eq!(role(&f.h, &f.coordinator).await["role"], Value::Null);
    let old = tool(
        &f.h,
        &f.coordinator,
        OperationId::OrchestrationListChildTasks,
        json!({}),
    )
    .await
    .unwrap_err();
    assert_eq!(tool_code(&old), "forbiddenActor");
}

/// liveness ②: 대기 중 자식이 결과를 보고하면 대기가 30초 제한보다 훨씬 빨리 깨어난다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn report_result_wakes_a_waiting_coordinator() {
    let f = fixture().await;
    let (task, child) = create_child(&f, "c1").await;
    let waiter = {
        let (h, run, task) = (Arc::clone(&f.h), f.coordinator.clone(), task.clone());
        tokio::spawn(async move {
            let started = Instant::now();
            let result = tool(
                &h,
                &run,
                OperationId::OrchestrationWaitChildTasks,
                json!({ "taskIds": [task], "timeoutMs": 30000 }),
            )
            .await;
            (result, started.elapsed())
        })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!waiter.is_finished(), "still waiting before the report");
    report_result(&f.h, &child, "r1").await.unwrap();
    let (result, elapsed) = tokio::time::timeout(Duration::from_secs(5), waiter)
        .await
        .expect("woken promptly")
        .unwrap();
    let result = result.unwrap();
    assert_eq!(result["timedOut"], false);
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
}

/// 보고가 대기의 구독 직전·직후 어디에 끼어들어도 놓치지 않는다(구독 → 읽기 순서). 매 회차 대기와 보고를 동시에
/// 출발시키고, 대기가 제한(10초) 전에 끝나는지 본다. 작업 영역의 노드 상한(8) 때문에 6회마다 새 작업대를 쓴다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_report_and_wait_never_miss_the_notification() {
    let mut f = fixture().await;
    for round in 0..42 {
        if round > 0 && round % 6 == 0 {
            f = fixture().await;
        }
        let (task, child) = create_child(&f, &format!("c{round}")).await;
        let wait = {
            let (h, run, task) = (Arc::clone(&f.h), f.coordinator.clone(), task.clone());
            tokio::spawn(async move {
                tool(
                    &h,
                    &run,
                    OperationId::OrchestrationWaitChildTasks,
                    json!({ "taskIds": [task], "timeoutMs": 10000 }),
                )
                .await
            })
        };
        let report = {
            let (h, run) = (Arc::clone(&f.h), child.clone());
            tokio::spawn(async move { report_result(&h, &run, &format!("r{round}")).await })
        };
        report.await.unwrap().unwrap();
        let waited = tokio::time::timeout(Duration::from_secs(10), wait)
            .await
            .unwrap_or_else(|_| panic!("round {round}: wait missed the report"))
            .unwrap()
            .unwrap();
        assert_eq!(waited["timedOut"], false, "round {round}");
    }
}

/// liveness ①: 자식 보고의 coordinator 알림은 Main 턴을 기다린다. 그 턴 안에서 Main이 `collectChildResults`와
/// `waitChildTasks`를 불러도 교착이 없다(저장소 경계·binding mutex를 턴 동안 쥐지 않는다).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn coordinator_turn_can_call_tools_while_a_notification_is_delivered() {
    let f = fixture().await;
    let (task, child) = create_child(&f, "c1").await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    {
        let (h, calls, task) = (Arc::clone(&f.h), Arc::clone(&calls), task.clone());
        *f.h.engine.turn_hook.lock().unwrap() = Some(Arc::new(move |run: String| {
            let (h, calls, task) = (Arc::clone(&h), Arc::clone(&calls), task.clone());
            Box::pin(async move {
                if run != "coord" {
                    return;
                }
                tool(
                    &h,
                    &run,
                    OperationId::OrchestrationCollectChildResults,
                    json!({ "taskIds": [task] }),
                )
                .await
                .unwrap();
                let waited = tool(
                    &h,
                    &run,
                    OperationId::OrchestrationWaitChildTasks,
                    json!({ "taskIds": [task], "timeoutMs": 5000 }),
                )
                .await
                .unwrap();
                assert_eq!(waited["timedOut"], false);
                calls.fetch_add(1, Ordering::SeqCst);
            })
        }));
    }
    tokio::time::timeout(Duration::from_secs(10), report_result(&f.h, &child, "r1"))
        .await
        .expect("no deadlock while the coordinator turn calls tools")
        .unwrap();
    // 알림은 비동기로 전달된다 — 턴이 끝날 때까지 기다린다.
    let deadline = Instant::now() + Duration::from_secs(10);
    while calls.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "notification turn never ran");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// 자식 첫 턴(research R7): 엔진이 run을 등록한 직후, 작업 영역이 그 run을 노드에 기록하기 전에 자식이 도구를
/// 부른다. 기동 중(Launching) 기록으로 자식 역할이 인정되어 `getOwnTask`·`reportProgress`가 허용된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn child_first_turn_tools_are_allowed_while_launching() {
    let f = fixture().await;
    let seen = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let first_turn_done = Arc::new(tokio::sync::Semaphore::new(0));
    hold_binding_until(&f.h, Arc::clone(&first_turn_done));
    {
        let (h, seen, done) = (
            Arc::clone(&f.h),
            Arc::clone(&seen),
            Arc::clone(&first_turn_done),
        );
        *f.h.engine.start_hook.lock().unwrap() = Some(Arc::new(move |run: String| {
            let (h, seen, done) = (Arc::clone(&h), Arc::clone(&seen), Arc::clone(&done));
            Box::pin(async move {
                let own = tool(&h, &run, OperationId::OrchestrationGetOwnTask, json!({}))
                    .await
                    .unwrap();
                let progress = tool(
                    &h,
                    &run,
                    OperationId::OrchestrationReportProgress,
                    json!({ "requestId": "p1", "summary": "started", "progressPercent": 5 }),
                )
                .await
                .unwrap();
                seen.lock().unwrap().extend([own, progress]);
                done.add_permits(1);
            })
        }));
    }
    let (task, _child) = create_child(&f, "c1").await;
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2, "first-turn calls ran");
    assert_eq!(seen[0]["id"], task.as_str());
}

async fn session_of(h: &BenchHarness, bench: &str) -> Value {
    h.call(
        &desktop(),
        OperationId::OrchestrationGet,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap()
}

fn task_status(session: &Value, task: &str) -> Value {
    session["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == task)
        .map(|entry| entry["status"].clone())
        .unwrap()
}

/// 041 Codex 리뷰 C1: 작업대를 닫고 다른 작업대가 작업 영역을 재개해도, 끝난 이전 coordinator·자식 run은 역할을
/// 되찾지 못한다(도구 거절·역할 없음·상태 불변). 닫기는 그 run들의 MCP 토큰도 폐기한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn previous_runs_do_not_regain_roles_after_another_bench_resumes() {
    let f = fixture().await;
    let (_task, child) = create_child(&f, "c1").await;
    let workspace = session_of(&f.h, &f.bench).await["id"].clone();
    f.h.close(&f.bench).await.unwrap();
    let revoked = f.h.desktop.revoked.lock().unwrap().clone();
    assert!(
        revoked.contains(&f.coordinator) && revoked.contains(&child),
        "closing revokes the bench's run tokens: {revoked:?}"
    );

    let b = f.h.open().await;
    f.h.call(
        &desktop(),
        OperationId::OrchestrationBootstrap,
        json!({ "benchId": b, "worktreePath": f.h.dir, "resumeWorkspaceId": workspace }),
    )
    .await
    .unwrap();
    let before = session_of(&f.h, &b).await;

    assert_eq!(role(&f.h, &f.coordinator).await["role"], Value::Null);
    assert_eq!(role(&f.h, &child).await["role"], Value::Null);
    let old_coordinator = tool(
        &f.h,
        &f.coordinator,
        OperationId::OrchestrationCreateChildTask,
        json!({
            "requestId": "after-resume", "title": "x",
            "role": { "name": "R", "responsibility": "r", "expectedOutput": "o" },
            "objective": "o", "expectedResult": "e"
        }),
    )
    .await
    .unwrap_err();
    assert_eq!(tool_code(&old_coordinator), "forbiddenActor");
    let old_child = report_result(&f.h, &child, "late").await.unwrap_err();
    assert_eq!(tool_code(&old_child), "forbiddenActor");
    let after = session_of(&f.h, &b).await;
    assert_eq!(after["revision"], before["revision"], "state unchanged");
    assert_eq!(after["tasks"], before["tasks"]);
}

/// 041 Codex 리뷰 C2: 자식의 첫 턴(엔진이 run을 등록한 직후·기동 결과 저장 전)에 보낸 결과·입력 요청은 예약된
/// 현재 run의 보고로 반영된다 — 과제 상태가 바뀌고 coordinator 알림이 생긴다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn first_turn_result_and_input_request_update_the_task_and_notify() {
    for (tool_op, expected_status, report_type) in [
        (
            OperationId::OrchestrationReportResult,
            "completed",
            "result",
        ),
        (
            OperationId::OrchestrationRequestParentInput,
            "inputRequired",
            "inputRequest",
        ),
    ] {
        let f = fixture().await;
        let first_turn_done = Arc::new(tokio::sync::Semaphore::new(0));
        hold_binding_until(&f.h, Arc::clone(&first_turn_done));
        {
            let (h, done) = (Arc::clone(&f.h), Arc::clone(&first_turn_done));
            *f.h.engine.start_hook.lock().unwrap() = Some(Arc::new(move |run: String| {
                let (h, done) = (Arc::clone(&h), Arc::clone(&done));
                Box::pin(async move {
                    if run == "coord" {
                        return;
                    }
                    tool(
                        &h,
                        &run,
                        tool_op,
                        json!({
                            "requestId": format!("first-{run}"),
                            "summary": "first turn",
                            "question": "which?"
                        }),
                    )
                    .await
                    .unwrap();
                    done.add_permits(1);
                })
            }));
        }
        let request = json!({
            "requestId": "c1", "title": "task c1",
            "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
            "objective": "read the repo", "expectedResult": "summary"
        });
        let created = tool(
            &f.h,
            &f.coordinator,
            OperationId::OrchestrationCreateChildTask,
            request.clone(),
        )
        .await
        .expect("the applied child creation is reported as success");
        let session = session_of(&f.h, &f.bench).await;
        let task = created["taskId"].as_str().expect("created task id");
        let task_entry = session["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == task)
            .expect("the task was committed before its worker started");
        assert_eq!(
            task_status(&session, task),
            expected_status,
            "{report_type}"
        );
        let notifications = session["coordinatorNotifications"].as_array().unwrap();
        assert!(
            notifications
                .iter()
                .any(|entry| entry["taskId"] == task && entry["reportType"] == report_type),
            "{report_type}: {notifications:?}"
        );
        let node = session["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == task_entry["assignedNodeId"])
            .expect("the assigned child node remains available");
        if report_type == "result" {
            assert_eq!(created["status"], "completed");
            assert_eq!(created["executionStatus"], "idle");
            assert_eq!(created["runId"], Value::Null);
            assert_eq!(node["currentRunId"], Value::Null);
            assert_eq!(node["executionStatus"], "idle");

            let repeated = tool(
                &f.h,
                &f.coordinator,
                OperationId::OrchestrationCreateChildTask,
                request,
            )
            .await
            .expect("same-key retry returns the stored completed task");
            assert_eq!(repeated["taskId"], task);
            assert_eq!(repeated["status"], "completed");
            assert_eq!(
                session_of(&f.h, &f.bench).await["tasks"]
                    .as_array()
                    .unwrap()
                    .len(),
                1,
                "same-key retry does not create duplicate work"
            );

            let distinct = tool(
                &f.h,
                &f.coordinator,
                OperationId::OrchestrationCreateChildTask,
                json!({
                    "requestId": "c2", "title": "task c2",
                    "role": { "name": "Reader", "responsibility": "read", "expectedOutput": "notes" },
                    "objective": "read the repo", "expectedResult": "summary"
                }),
            )
            .await
            .expect("a new key intentionally creates a distinct completed task");
            assert_ne!(distinct["taskId"], task);
            assert_eq!(distinct["status"], "completed");
            assert_eq!(
                session_of(&f.h, &f.bench).await["tasks"]
                    .as_array()
                    .unwrap()
                    .len(),
                2,
                "new-key retry semantics remain a new operation"
            );
        } else {
            let child = created["runId"].as_str().expect("child started").to_owned();
            assert_eq!(node["currentRunId"], child);
            assert_eq!(node["executionStatus"], "active");
        }
    }
}
