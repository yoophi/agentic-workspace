//! 041 US1: orchestration 작업 영역은 작업대에 묶이고(창 label 없음), 다른 주체·다른 작업대는 조작할 수 없으며,
//! 작업대를 닫으면 복구 가능이 되어 다른 작업대가 재개할 수 있다. 작업 영역에 넣는 run은 그 작업대가 소유한
//! 살아 있는 run이어야 한다(research R18). 복구 가능한 작업 영역 하나를 두 작업대가 동시에 재개하면 하나만
//! 성공한다(R3). (재개 없이 bootstrap하면 오늘처럼 작업대마다 작업 영역이 따로 생긴다.)

#![allow(clippy::result_large_err)]

mod support;

use std::sync::Arc;

use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_protocol::{AuthenticatedPrincipal, FaultCode, OperationId, WorkbenchFault};

fn desktop() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop()
}

async fn bootstrap(
    h: &BenchHarness,
    bench: &str,
    dir: &str,
    resume: Option<&str>,
) -> Result<Value, WorkbenchFault> {
    let mut input = json!({ "benchId": bench, "worktreePath": dir });
    if let Some(resume) = resume {
        input["resumeWorkspaceId"] = json!(resume);
    }
    h.call(&desktop(), OperationId::OrchestrationBootstrap, input)
        .await
}

async fn get(h: &BenchHarness, bench: &str) -> Value {
    h.call(
        &desktop(),
        OperationId::OrchestrationGet,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap()
}

async fn bind_main(
    h: &BenchHarness,
    bench: &str,
    run: &str,
    revision: u64,
) -> Result<Value, WorkbenchFault> {
    h.call(
        &desktop(),
        OperationId::OrchestrationBindCoordinator,
        json!({ "benchId": bench, "request": {
            "requestId": uuid::Uuid::new_v4().to_string(), "panelId": "main-agent-run",
            "runId": run, "state": "active", "expectedRevision": revision } }),
    )
    .await
}

async fn extra_bench(h: &BenchHarness) -> String {
    h.open().await
}

/// 복구 목록에는 실제 작업이 있는 작업 영역만 나온다(오늘 규칙) — coordinator 연결로 세대를 만든다.
async fn with_work(h: &BenchHarness, bench: &str, dir: &str, run: &str) -> Value {
    let session = bootstrap(h, bench, dir, None).await.unwrap();
    h.start(bench, run).await.unwrap();
    bind_main(h, bench, run, session["revision"].as_u64().unwrap())
        .await
        .unwrap()
}

fn extra_dir(h: &BenchHarness, name: &str) -> String {
    let dir = std::path::Path::new(&h.dir).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(dir)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspaces_are_bound_to_benches_and_isolated() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let session = bootstrap(&h, &a, &h.dir, None).await.unwrap();
    assert!(
        session.get("boundWindowLabel").is_none(),
        "no window label in the contract"
    );
    assert!(session["eventStreamId"]
        .as_str()
        .unwrap()
        .starts_with("orchestration:"));
    assert_eq!(get(&h, &a).await["id"], session["id"]);

    // 다른 주체는 이 작업대를 쓸 수 없다.
    let other = h
        .call(
            &AuthenticatedPrincipal::test_as("desktop2"),
            OperationId::OrchestrationGet,
            json!({ "benchId": a }),
        )
        .await
        .unwrap_err();
    assert_eq!(
        (other.code, other.message.as_str()),
        (FaultCode::Forbidden, "bench belongs to another principal.")
    );

    // 같은 주체의 다른 작업대는 이 작업 영역을 보지 못하고, 재개 없이 bootstrap하면 자기 작업 영역을 따로 만든다.
    let b = h.open().await;
    assert_eq!(get(&h, &b).await, Value::Null);
    let own = bootstrap(&h, &b, &h.dir, None).await.unwrap();
    assert_ne!(own["id"], session["id"]);
    // 이미 묶인 작업 영역을 재개할 수는 없다(오늘 문구).
    let taken = bootstrap(&h, &extra_bench(&h).await, &h.dir, session["id"].as_str())
        .await
        .unwrap_err();
    assert_eq!(
        taken.message,
        "The workspace cannot be bound to this window."
    );
    assert_eq!(
        taken.details.as_ref().unwrap()["orchestrationError"]["code"],
        "scopeMismatch"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coordinator_binding_requires_a_run_owned_by_the_bench() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let b = h.open().await;
    h.start(&b, "foreign").await.unwrap();
    let session = bootstrap(&h, &a, &h.dir, None).await.unwrap();
    let revision = session["revision"].as_u64().unwrap();

    let refused = bind_main(&h, &a, "foreign", revision).await.unwrap_err();
    assert_eq!(
        (refused.code, refused.message.as_str()),
        (FaultCode::Forbidden, "run is owned by another bench.")
    );
    assert_eq!(
        get(&h, &a).await["activeCoordinatorGenerationId"],
        Value::Null
    );

    // 그 run의 기록도 이 작업대로는 재생할 수 없다.
    let replay = h
        .call(
            &desktop(),
            OperationId::RunReplay,
            json!({ "benchId": a, "runId": "foreign", "afterSequence": 0 }),
        )
        .await
        .unwrap_err();
    assert_eq!(replay.code, FaultCode::Forbidden);

    h.start(&a, "main-run").await.unwrap();
    let bound = bind_main(&h, &a, "main-run", revision).await.unwrap();
    assert!(bound["activeCoordinatorGenerationId"].is_string());
    let own = h
        .call(
            &desktop(),
            OperationId::RunReplay,
            json!({ "benchId": a, "runId": "main-run", "afterSequence": 0 }),
        )
        .await
        .unwrap();
    assert_eq!(own["runId"], "main-run");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_a_bench_makes_the_workspace_recoverable_by_another_bench() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let first = with_work(&h, &a, &h.dir, "main-a").await;
    let id = first["id"].as_str().unwrap().to_owned();
    h.close(&a).await.unwrap();

    let b = h.open().await;
    let recoverable = h
        .call(
            &desktop(),
            OperationId::OrchestrationListRecoverable,
            json!({ "benchId": b, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    let listed = recoverable.as_array().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], id.as_str());
    assert_eq!(listed[0]["eventStreamId"], Value::Null);

    let resumed = bootstrap(&h, &b, &h.dir, Some(&id)).await.unwrap();
    assert_eq!(resumed["id"], id.as_str());
    assert_ne!(
        resumed["eventStreamId"], first["eventStreamId"],
        "new binding, new stream"
    );
    assert_eq!(get(&h, &b).await["id"], id.as_str());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_resumes_of_one_workspace_bind_exactly_once() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    for round in 0..30 {
        let dir = extra_dir(&h, &format!("wt-{round}"));
        let owner = h.open().await;
        let id = with_work(&h, &owner, &dir, &format!("main-{round}")).await["id"]
            .as_str()
            .unwrap()
            .to_owned();
        h.close(&owner).await.unwrap();
        let (a, b) = (h.open().await, h.open().await);
        let spawn = |bench: String| {
            let (h, dir, id) = (Arc::clone(&h), dir.clone(), id.clone());
            tokio::spawn(async move { bootstrap(&h, &bench, &dir, Some(&id)).await })
        };
        let (first, second) = (spawn(a), spawn(b));
        let results = [first.await.unwrap(), second.await.unwrap()];
        let bound = results.iter().filter(|result| result.is_ok()).count();
        assert_eq!(bound, 1, "round {round}: {results:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delegating_a_goal_prompts_the_coordinator_run() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let session = bootstrap(&h, &a, &h.dir, None).await.unwrap();
    h.start(&a, "main-run").await.unwrap();
    let bound = bind_main(&h, &a, "main-run", session["revision"].as_u64().unwrap())
        .await
        .unwrap();
    let before = h.engine.prompts.load(std::sync::atomic::Ordering::SeqCst);
    let outcome = h
        .call(
            &desktop(),
            OperationId::OrchestrationDelegateGoal,
            json!({ "benchId": a, "request": {
                "requestId": "goal-1", "goal": "Summarize the repo",
                "expectedRevision": bound["revision"] } }),
        )
        .await
        .unwrap();
    assert!(outcome["rootTaskId"].is_string());
    assert_eq!(
        h.engine.prompts.load(std::sync::atomic::Ordering::SeqCst),
        before + 1
    );
}
