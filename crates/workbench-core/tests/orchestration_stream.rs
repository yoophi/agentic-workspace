//! 041 US3: orchestration 스트림은 작업 영역의 **현재 묶임**마다 하나(`orchestration:<bindingId>`)다. 그 작업대를 연
//! 주체만 구독하고, 작업대를 닫으면 구독자에게 `Gap(evicted)`, 다른 작업대가 재개하면 새 스트림이 열린다. run
//! 스트림은 소유 작업대(또는 그 run을 기록한 묶인 작업 영역의 작업대)를 연 주체만 구독한다(research R17). 창 전달은
//! 이벤트마다 그 작업대에 한 번이다.

#![allow(clippy::result_large_err)]

mod support;

use std::time::Duration;

use futures_util::StreamExt;
use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_core::ports::desktop_bridge::DesktopDelivery;
use workbench_protocol::{
    AuthenticatedPrincipal, EventItem, EventStream, FaultCode, GapReason, OperationId,
    StreamCursor, Subscription, Workbench, WorkbenchFault,
};

fn desktop() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop()
}

fn subscribe(
    h: &BenchHarness,
    principal: AuthenticatedPrincipal,
    stream_id: &str,
) -> Result<EventStream, WorkbenchFault> {
    h.rt.runtime.events(
        principal,
        Subscription {
            cursors: vec![StreamCursor {
                stream_id: stream_id.to_owned(),
                epoch: h.rt.runtime.epoch().into(),
                after_sequence: 0,
            }],
        },
    )
}

fn rejected(result: Result<EventStream, WorkbenchFault>) -> (FaultCode, String) {
    match result {
        Ok(_) => panic!("subscription must be rejected"),
        Err(fault) => (fault.code, fault.message),
    }
}

/// 다음 gap(앞선 이벤트는 건너뛴다 — cursor 0 구독은 journal을 먼저 재생한다).
async fn next_gap(stream: &mut EventStream) -> workbench_protocol::GapNotice {
    loop {
        if let EventItem::Gap { gap } = next(stream).await {
            return gap;
        }
    }
}

async fn next(stream: &mut EventStream) -> EventItem {
    tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("item in time")
        .expect("stream open")
}

async fn bootstrap(h: &BenchHarness, bench: &str, resume: Option<&str>) -> Value {
    let mut input = json!({ "benchId": bench, "worktreePath": h.dir });
    if let Some(resume) = resume {
        input["resumeWorkspaceId"] = json!(resume);
    }
    h.call(&desktop(), OperationId::OrchestrationBootstrap, input)
        .await
        .unwrap()
}

async fn bind_main(h: &BenchHarness, bench: &str, run: &str, revision: &Value) -> Value {
    h.call(
        &desktop(),
        OperationId::OrchestrationBindCoordinator,
        json!({ "benchId": bench, "request": {
            "requestId": uuid::Uuid::new_v4().to_string(), "panelId": "main-agent-run",
            "runId": run, "state": "active", "expectedRevision": revision } }),
    )
    .await
    .unwrap()
}

fn orchestration_deliveries(h: &BenchHarness, bench: &str) -> usize {
    h.desktop
        .deliveries_for(bench)
        .iter()
        .filter(|delivery| matches!(delivery, DesktopDelivery::Orchestration { .. }))
        .count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bound_stream_is_sequenced_and_owned_by_the_bench_principal() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let session = bootstrap(&h, &a, None).await;
    let stream_id = session["eventStreamId"].as_str().unwrap().to_owned();
    let mut stream = subscribe(&h, desktop(), &stream_id).unwrap();

    // 다른 주체·agent는 구독할 수 없고, 모르는 묶임은 notFound.
    assert_eq!(
        rejected(subscribe(
            &h,
            AuthenticatedPrincipal::test_as("desktop2"),
            &stream_id
        )),
        (
            FaultCode::Forbidden,
            "bench belongs to another principal.".to_owned()
        )
    );
    h.start(&a, "main-run").await.unwrap();
    assert_eq!(
        rejected(subscribe(
            &h,
            AuthenticatedPrincipal::agent("main-run"),
            &stream_id
        ))
        .0,
        FaultCode::Forbidden
    );
    assert_eq!(
        rejected(subscribe(&h, desktop(), "orchestration:nope")),
        (
            FaultCode::NotFound,
            "orchestration stream not found.".to_owned()
        )
    );

    // cursor 0 구독은 묶인 뒤 발행된 bootstrap 이벤트부터 재생한다.
    let EventItem::Event { event: replayed } = next(&mut stream).await else {
        panic!("replayed bootstrap event expected")
    };
    assert_eq!(replayed.sequence, 1);
    assert_eq!(replayed.body["revision"], session["revision"]);

    let before = orchestration_deliveries(&h, &a);
    let bound = bind_main(&h, &a, "main-run", &session["revision"]).await;
    let EventItem::Event { event } = next(&mut stream).await else {
        panic!("event expected")
    };
    assert_eq!(event.stream_id, stream_id);
    assert_eq!(event.body["workspaceId"], session["id"]);
    assert_eq!(event.body["revision"], bound["revision"]);
    let first = event.sequence;
    h.call(
        &desktop(),
        OperationId::OrchestrationDelegateGoal,
        json!({ "benchId": a, "request": {
            "requestId": "goal-1", "goal": "Summarize the repo",
            "expectedRevision": bound["revision"] } }),
    )
    .await
    .unwrap();
    // 변경마다 창 전달은 정확히 한 번(스트림 발행과 같은 순간).
    let EventItem::Event { event: second } = next(&mut stream).await else {
        panic!("event expected")
    };
    assert_eq!(second.sequence, first + 1);
    assert_eq!(orchestration_deliveries(&h, &a) - before, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_the_bench_evicts_the_stream_and_rebinding_opens_a_new_one() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let session = bootstrap(&h, &a, None).await;
    h.start(&a, "main-a").await.unwrap();
    let bound = bind_main(&h, &a, "main-a", &session["revision"]).await;
    let old_stream = session["eventStreamId"].as_str().unwrap().to_owned();
    let mut stream = subscribe(&h, desktop(), &old_stream).unwrap();

    h.close(&a).await.unwrap();
    assert_eq!(next_gap(&mut stream).await.reason, GapReason::Evicted);
    // 닫힌 묶임을 cursor 0으로 구독하면 곧바로 Gap(evicted).
    let mut late = subscribe(&h, desktop(), &old_stream).unwrap();
    assert!(
        matches!(next(&mut late).await, EventItem::Gap { gap } if gap.reason == GapReason::Evicted)
    );
    let delivered_after_close = orchestration_deliveries(&h, &a);

    let b = h.open().await;
    let resumed = bootstrap(&h, &b, bound["id"].as_str()).await;
    let new_stream = resumed["eventStreamId"].as_str().unwrap().to_owned();
    assert_ne!(new_stream, old_stream);
    let mut fresh = subscribe(&h, desktop(), &new_stream).unwrap();
    h.call(
        &desktop(),
        OperationId::OrchestrationRecover,
        json!({ "benchId": b }),
    )
    .await
    .unwrap();
    let EventItem::Event { event } = next(&mut fresh).await else {
        panic!("event expected")
    };
    assert_eq!(event.body["workspaceId"], resumed["id"]);
    // 닫힌 작업대로는 더 전달하지 않는다.
    assert_eq!(orchestration_deliveries(&h, &a), delivered_after_close);
    assert!(orchestration_deliveries(&h, &b) >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_streams_follow_bench_ownership_and_recovered_workspaces() {
    let h = BenchHarness::new(RunScript::default());
    let a = h.open().await;
    let session = bootstrap(&h, &a, None).await;
    h.start(&a, "coord-a").await.unwrap();
    let bound = bind_main(&h, &a, "coord-a", &session["revision"]).await;
    assert!(subscribe(&h, desktop(), "run:coord-a").is_ok());

    // 다른 주체가 연 작업대의 run은 구독할 수 없다.
    let other = AuthenticatedPrincipal::test_as("p2");
    let their_bench = h
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
    h.call(
        &other,
        OperationId::RunStart,
        json!({ "benchId": their_bench, "request": { "goal": "g", "agentId": "codex", "runId": "theirs" } }),
    )
    .await
    .unwrap();
    assert_eq!(
        rejected(subscribe(&h, desktop(), "run:theirs")),
        (
            FaultCode::Forbidden,
            "run is owned by another bench.".to_owned()
        )
    );
    assert!(subscribe(&h, other.clone(), "run:theirs").is_ok());
    // 기동된 적 없는 run id는 기다릴 수 없다.
    assert_eq!(
        rejected(subscribe(&h, desktop(), "run:never")),
        (FaultCode::NotFound, "run stream not found.".to_owned())
    );

    // 작업대 A를 닫고 p2가 그 작업 영역을 재개하면, p2는 기록된 coordinator run을 구독·재생할 수 있다.
    h.close(&a).await.unwrap();
    assert_eq!(
        rejected(subscribe(&h, other.clone(), "run:coord-a")).0,
        FaultCode::Forbidden
    );
    h.call(
        &other,
        OperationId::OrchestrationBootstrap,
        json!({ "benchId": their_bench, "worktreePath": h.dir, "resumeWorkspaceId": bound["id"] }),
    )
    .await
    .unwrap();
    assert!(subscribe(&h, other.clone(), "run:coord-a").is_ok());
    let replay = h
        .call(
            &other,
            OperationId::RunReplay,
            json!({ "benchId": their_bench, "runId": "coord-a", "afterSequence": 0 }),
        )
        .await
        .unwrap();
    assert_eq!(replay["runId"], "coord-a");
}
