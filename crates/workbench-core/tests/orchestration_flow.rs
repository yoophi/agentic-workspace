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
    for round in 0..200 {
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
        let (first, second) = (spawn(a.clone()), spawn(b.clone()));
        let results = [first.await.unwrap(), second.await.unwrap()];
        let bound = results.iter().filter(|result| result.is_ok()).count();
        assert_eq!(bound, 1, "round {round}: {results:?}");
        // 작업대 상한(256) 안에서 200회를 돌도록 회차마다 닫는다.
        h.close(&a).await.unwrap();
        h.close(&b).await.unwrap();
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

/// research R18 + 설계 리뷰: 작업대를 닫고(run 종료) 다른 작업대에서 같은 run id를 다시 쓰면, 복구 가능한 작업
/// 영역이 그 id를 기록하고 있어 재생 권한·역할이 다른 run을 가리키게 된다. 그래서 끝난 run의 id는 다시 쓸 수
/// 없다. 복구한 작업대는 원래 run의 기록만 재생한다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ended_run_ids_cannot_be_reused_across_close_and_recover() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let first = with_work(&h, &a, &h.dir, "coord-1").await;
    let id = first["id"].as_str().unwrap().to_owned();
    h.close(&a).await.unwrap();

    // 다른 작업대가 같은 id로 run을 띄울 수 없다(작업 영역이 기록 중).
    let b = h.open().await;
    let reused = h.start(&b, "coord-1").await.unwrap_err();
    assert_eq!(
        (reused.code, reused.message.as_str()),
        (FaultCode::Conflict, "duplicate run id: coord-1")
    );

    // 작업 영역과 무관한 일반 run도 끝난 뒤에는 같은 id를 다시 쓸 수 없다(hub journal이 남아 있다).
    h.start(&b, "plain-1").await.unwrap();
    h.close(&b).await.unwrap();
    let c = h.open().await;
    let plain = h.start(&c, "plain-1").await.unwrap_err();
    assert_eq!(plain.message, "duplicate run id: plain-1");

    // 복구한 작업대는 원래 coordinator run의 기록을 재생한다(노드 run 허용), 다른 작업대의 일반 run은 거절.
    let resumed = bootstrap(&h, &c, &h.dir, Some(&id)).await.unwrap();
    assert_eq!(resumed["id"], id.as_str());
    let replay = h
        .call(
            &desktop(),
            OperationId::RunReplay,
            json!({ "benchId": c, "runId": "coord-1", "afterSequence": 0 }),
        )
        .await
        .unwrap();
    assert_eq!(replay["runId"], "coord-1");
    let foreign = h
        .call(
            &desktop(),
            OperationId::RunReplay,
            json!({ "benchId": c, "runId": "plain-1", "afterSequence": 0 }),
        )
        .await
        .unwrap_err();
    assert_eq!(foreign.code, FaultCode::Forbidden);
}

/// research R17: 다른 작업대가 기동 중인(소유 등록됐지만 아직 발행 전) run은 재생·구독할 수 없다. 응답 모양이
/// 아니라 소유 기록으로 판정하므로 빈 기록이어도 거절된다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn live_unpublished_run_of_another_bench_is_not_replayable() {
    let h = Arc::new(BenchHarness::new(RunScript {
        start_delay_ms: 600,
        ..RunScript::default()
    }));
    let a = h.open().await;
    let other = AuthenticatedPrincipal::test_as("p2");
    let b = h
        .call(
            &other,
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await
        .unwrap()["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    let start = {
        let (h, other, b) = (Arc::clone(&h), other.clone(), b.clone());
        tokio::spawn(async move {
            h.call(
                &other,
                OperationId::RunStart,
                json!({ "benchId": b, "request": { "goal": "g", "agentId": "codex", "runId": "pending" } }),
            )
            .await
        })
    };
    // 기동 지연 중: 소유는 등록됐고 발행은 아직 없다.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while h.rt.runtime.events_hub().run_owner("pending").is_none() {
        assert!(std::time::Instant::now() < deadline, "claim never recorded");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(!h.rt.runtime.events_hub().has_run_history("pending"));
    let refused = h
        .call(
            &desktop(),
            OperationId::RunReplay,
            json!({ "benchId": a, "runId": "pending", "afterSequence": 0 }),
        )
        .await
        .unwrap_err();
    assert_eq!(
        (refused.code, refused.message.as_str()),
        (FaultCode::Forbidden, "run is owned by another bench.")
    );
    let subscribe = |principal: AuthenticatedPrincipal| {
        workbench_protocol::Workbench::events(
            h.rt.runtime.as_ref(),
            principal,
            workbench_protocol::Subscription {
                cursors: vec![workbench_protocol::StreamCursor {
                    stream_id: "run:pending".into(),
                    epoch: h.rt.runtime.epoch().into(),
                    after_sequence: 0,
                }],
            },
        )
    };
    assert_eq!(
        subscribe(desktop()).err().map(|f| f.code),
        Some(FaultCode::Forbidden)
    );
    // 소유 주체는 발행 전에도 기다릴 수 있다.
    assert!(subscribe(other.clone()).is_ok());
    start.await.unwrap().unwrap();
    let own = h
        .call(
            &other,
            OperationId::RunReplay,
            json!({ "benchId": b, "runId": "pending", "afterSequence": 0 }),
        )
        .await
        .unwrap();
    assert!(!own["events"].as_array().unwrap().is_empty());
}

/// 보관 한도로 실제 제거된 run(hub 제거 표식)은 내용이 없으므로 소유와 관계없이 오늘의 Evicted 형태를 돌려준다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retention_evicted_run_replays_as_evicted_for_any_bench() {
    let h = BenchHarness::with(
        |adapters| adapters.event_limits.max_retained_runs = 1,
        RunScript::default(),
    );
    let owner = h.open().await;
    let viewer = h.open().await;
    h.start(&owner, "old").await.unwrap();
    h.engine
        .finish("old", &h.rt.runtime.benches().run_sink(&owner));
    h.start(&owner, "new").await.unwrap();
    h.engine
        .finish("new", &h.rt.runtime.benches().run_sink(&owner));
    assert!(h.rt.runtime.events_hub().is_evicted("run:old"));
    assert_eq!(h.rt.runtime.events_hub().run_owner("old"), None);

    let replay = h
        .call(
            &desktop(),
            OperationId::RunReplay,
            json!({ "benchId": viewer, "runId": "old", "afterSequence": 0 }),
        )
        .await
        .unwrap();
    assert_eq!(
        (
            &replay["terminal"],
            &replay["gapDetected"],
            replay["events"].as_array().unwrap().len()
        ),
        (&json!(true), &json!(true), 0)
    );
    // 아직 보관 중인 run은 여전히 소유 작업대만.
    let kept = h
        .call(
            &desktop(),
            OperationId::RunReplay,
            json!({ "benchId": viewer, "runId": "new", "afterSequence": 0 }),
        )
        .await
        .unwrap_err();
    assert_eq!(kept.code, FaultCode::Forbidden);
}

async fn start_main(h: &BenchHarness, bench: &str, run: &str) -> Result<Value, WorkbenchFault> {
    h.call(
        &desktop(),
        OperationId::RunStart,
        json!({ "benchId": bench, "panelId": "main-agent-run",
                "request": { "goal": "g", "agentId": "codex", "runId": run } }),
    )
    .await
}

/// 화면은 Main run을 띄우기 **전에** 계획한 run id로 먼저 묶는다(`onBeforeRunStart`). 흔적 없는 계획 id는 묶을 때
/// 이 작업대 소유로 claim되고, 이어서 같은 작업대가 그 id로 Main run을 띄울 수 있다(research R18 계획 id).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn main_runs_are_prebound_with_a_planned_id_then_launched() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;

    // 작업 영역이 없으면 Main run을 띄울 수 없다(오늘 문구).
    let none = start_main(&h, &a, "main-1").await.unwrap_err();
    assert_eq!(
        (none.code, none.message.as_str()),
        (
            FaultCode::PreconditionFailed,
            "Main Coordinator workspace is unavailable."
        )
    );
    let session = bootstrap(&h, &a, &h.dir, None).await.unwrap();
    let unbound = start_main(&h, &a, "main-1").await.unwrap_err();
    assert_eq!(
        unbound.message,
        "Main Coordinator generation must be bound before launch."
    );

    let bound = bind_main(&h, &a, "main-1", session["revision"].as_u64().unwrap())
        .await
        .unwrap();
    assert!(bound["activeCoordinatorGenerationId"].is_string());
    let mismatch = start_main(&h, &a, "other").await.unwrap_err();
    assert_eq!(
        mismatch.message,
        "Main Coordinator generation does not match the run being launched."
    );
    start_main(&h, &a, "main-1").await.unwrap();

    // 교대도 계획 id로 먼저 한다.
    let handed = h
        .call(
            &desktop(),
            OperationId::OrchestrationHandoffCoordinator,
            json!({ "benchId": a, "request": {
                "requestId": "handoff-1", "successorRunId": "main-2", "summary": "next",
                "confirmed": true, "expectedRevision": get(&h, &a).await["revision"] } }),
        )
        .await
        .unwrap();
    assert_ne!(
        handed["activeCoordinatorGenerationId"],
        bound["activeCoordinatorGenerationId"]
    );
    start_main(&h, &a, "main-2").await.unwrap();
}

/// 계획 id는 claim한 작업대의 것이다: 다른 작업대는 그 id로 run을 띄우거나 묶을 수 없고, 다른 작업대가 쓰는
/// id(살아 있거나 끝난 run)를 계획 id로 가로챌 수도 없다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn planned_run_ids_belong_to_the_claiming_bench() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let b = h.open().await;
    let dir_b = extra_dir(&h, "wt-b");
    let session_a = bootstrap(&h, &a, &h.dir, None).await.unwrap();
    let session_b = bootstrap(&h, &b, &dir_b, None).await.unwrap();
    bind_main(&h, &a, "planned-a", session_a["revision"].as_u64().unwrap())
        .await
        .unwrap();

    let started = h.start(&b, "planned-a").await.unwrap_err();
    assert_eq!(started.message, "duplicate run id: planned-a");
    let rebound = bind_main(&h, &b, "planned-a", session_b["revision"].as_u64().unwrap())
        .await
        .unwrap_err();
    assert_eq!(rebound.code, FaultCode::Forbidden);

    // b가 띄운 run(살아 있음)과 끝난 run은 a의 계획 id가 될 수 없다.
    h.start(&b, "live-b").await.unwrap();
    let revision = get(&h, &a).await["revision"].as_u64().unwrap();
    let taken = h
        .call(
            &desktop(),
            OperationId::OrchestrationHandoffCoordinator,
            json!({ "benchId": a, "request": {
                "requestId": "handoff-x", "successorRunId": "live-b", "summary": "x",
                "confirmed": true, "expectedRevision": revision } }),
        )
        .await
        .unwrap_err();
    assert_eq!(
        (taken.code, taken.message.as_str()),
        (FaultCode::Forbidden, "run is owned by another bench.")
    );
    h.engine
        .finish("live-b", &h.rt.runtime.benches().run_sink(&b));
    let ended = h
        .call(
            &desktop(),
            OperationId::OrchestrationHandoffCoordinator,
            json!({ "benchId": a, "request": {
                "requestId": "handoff-y", "successorRunId": "live-b", "summary": "y",
                "confirmed": true, "expectedRevision": revision } }),
        )
        .await
        .unwrap_err();
    assert_eq!(ended.code, FaultCode::Forbidden);
}

/// 사용자 점검: 작업대 A가 계획 id를 claim한 직후(저장 전)에 작업대 B가 같은 id로 run을 띄우는 경합. 매 회차 정확히
/// 한쪽만 성공하고, hub 소유 기록은 이긴 쪽이며 발행이 그 기록을 뒤집지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn prebind_and_foreign_start_race_leaves_one_consistent_owner() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    for round in 0..60 {
        let (a, b) = (h.open().await, h.open().await);
        let dir = extra_dir(&h, &format!("race-{round}"));
        let session = bootstrap(&h, &a, &dir, None).await.unwrap();
        let run = format!("planned-{round}");
        let bind = {
            let (h, a, run) = (Arc::clone(&h), a.clone(), run.clone());
            let revision = session["revision"].as_u64().unwrap();
            tokio::spawn(async move { bind_main(&h, &a, &run, revision).await })
        };
        let start = {
            let (h, b, run) = (Arc::clone(&h), b.clone(), run.clone());
            tokio::spawn(async move { h.start(&b, &run).await })
        };
        let (bound, started) = (bind.await.unwrap(), start.await.unwrap());
        assert!(
            bound.is_ok() != started.is_ok(),
            "round {round}: exactly one wins: bind={bound:?} start={started:?}"
        );
        let owner = h.rt.runtime.events_hub().run_owner(&run);
        let winner = if bound.is_ok() { &a } else { &b };
        assert_eq!(owner.as_deref(), Some(winner.as_str()), "round {round}");
        h.close(&a).await.unwrap();
        h.close(&b).await.unwrap();
    }
}

/// 실패한 묶기는 새로 만든 계획 id claim을 되돌린다 — 그 id를 다른 작업대가 쓸 수 있다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_prebind_releases_its_claim() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let session = bootstrap(&h, &a, &h.dir, None).await.unwrap();
    let stale = session["revision"].as_u64().unwrap() + 7;
    let failed = bind_main(&h, &a, "planned-x", stale).await.unwrap_err();
    assert_eq!(failed.code, FaultCode::Conflict);
    assert_eq!(h.rt.runtime.events_hub().run_owner("planned-x"), None);
    let b = h.open().await;
    h.start(&b, "planned-x").await.unwrap();
}

/// 같은 작업대의 정상 흐름: 계획 id로 묶고 띄우다 엔진이 거절해도(동시 실행 한도) claim은 남아, 자리가 나면 같은
/// id로 다시 띄울 수 있다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn prebound_main_start_can_be_retried_after_an_engine_failure() {
    let h = BenchHarness::new(RunScript {
        max_runs: Some(1),
        ..RunScript::default()
    });
    let a = h.open().await;
    h.start(&a, "filler").await.unwrap();
    let session = bootstrap(&h, &a, &h.dir, None).await.unwrap();
    bind_main(&h, &a, "main-r", session["revision"].as_u64().unwrap())
        .await
        .unwrap();
    let limited = start_main(&h, &a, "main-r").await.unwrap_err();
    assert_eq!(limited.code, FaultCode::RateLimited);
    assert!(
        h.desktop
            .revoked
            .lock()
            .unwrap()
            .contains(&"main-r".to_owned()),
        "a capability issued before a failed launch is revoked"
    );
    assert_eq!(
        h.rt.runtime.events_hub().run_owner("main-r").as_deref(),
        Some(a.as_str())
    );
    h.engine
        .finish("filler", &h.rt.runtime.benches().run_sink(&a));
    assert!(
        h.desktop
            .revoked
            .lock()
            .unwrap()
            .contains(&"filler".to_owned()),
        "a terminal run capability is revoked without waiting for bench close"
    );
    start_main(&h, &a, "main-r").await.unwrap();
}

/// 041 Codex 리뷰 C4: 작업대 닫기의 복구 가능 전환 저장이 실패해도(디스크 오류) 묶임은 풀린다. `bench.close`는
/// 계약대로 `closed: true`(작업대 닫힘·run 취소는 사실)이고 실패는 로그로 남는다. 실패 직후 다른 작업대가 복구 목록에서
/// 그 작업 영역을 보고, 디스크 오류가 사라지면 서버 재시작 없이 재개한다.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failed_release_write_still_leaves_the_workspace_resumable() {
    use std::os::unix::fs::PermissionsExt;
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let first = with_work(&h, &a, &h.dir, "main-a").await;
    let id = first["id"].as_str().unwrap().to_owned();
    let data_dir = h.rt.paths.app_data_dir().to_path_buf();
    let original = std::fs::metadata(&data_dir).unwrap().permissions();
    // 저장은 같은 디렉터리의 임시 파일 + rename이다 — 디렉터리 쓰기를 막아 실패시킨다.
    std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    let closed = h.close(&a).await;
    let b = h.open().await;
    let listed = h
        .call(
            &desktop(),
            OperationId::OrchestrationListRecoverable,
            json!({ "benchId": b, "worktreePath": h.dir }),
        )
        .await;
    std::fs::set_permissions(&data_dir, original).unwrap();
    assert_eq!(closed.unwrap()["closed"], true);
    let listed = listed.unwrap();
    assert!(
        listed
            .as_array()
            .unwrap()
            .iter()
            .any(|session| session["id"] == id.as_str()),
        "recoverable right after the failed write: {listed}"
    );
    let resumed = bootstrap(&h, &b, &h.dir, Some(&id)).await.unwrap();
    assert_eq!(resumed["id"], id.as_str());
    assert_eq!(get(&h, &b).await["id"], id.as_str());
}
