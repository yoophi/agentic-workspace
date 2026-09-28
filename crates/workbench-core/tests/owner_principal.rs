//! 044 T027(contracts/server-lifecycle.md §4, 설계 리뷰 D2): 소유자 주체(`local:owner`)는 작업대 소유 판정을 두 지점에서
//! 우회한다 — 작업대 레지스트리(`resolve`·`admit`·`close_as`)와 이벤트 스트림 구독 판정. 창·agent 주체는 여전히 자기
//! 작업대만 다룬다. 소유자는 agent 전용 operation(호출자 run이 필요한 것)에는 우회를 받지 않고, 소유자 전용
//! operation은 창·agent 주체가 부를 수 없다.

#![allow(clippy::result_large_err)]

mod support;

use serde_json::{json, Value};
use support::{command_request, scripted_run_engine::RunScript, BenchHarness};
use workbench_protocol::{
    AuthenticatedPrincipal, FaultCode, OperationId, StreamCursor, Subscription, Workbench,
    WorkbenchFault,
};

fn window(label: &str) -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop_window(label, "inc-1")
}

fn subscribe(
    h: &BenchHarness,
    principal: AuthenticatedPrincipal,
    stream_id: String,
) -> Result<workbench_protocol::EventStream, WorkbenchFault> {
    h.rt.runtime.events(
        principal,
        Subscription {
            cursors: vec![StreamCursor {
                stream_id,
                epoch: h.rt.runtime.epoch().into(),
                after_sequence: 0,
            }],
        },
    )
}

fn code(result: Result<Value, WorkbenchFault>) -> FaultCode {
    result.expect_err("must be refused").code
}

async fn window_bench_with_run(h: &BenchHarness, label: &str, run: &str) -> String {
    let opened = h
        .call(
            &window(label),
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await
        .expect("bench.open");
    let bench = opened["benchId"].as_str().unwrap().to_owned();
    h.call(
        &window(label),
        OperationId::RunStart,
        json!({ "benchId": bench, "request": { "goal": "g", "agentId": "codex", "runId": run } }),
    )
    .await
    .expect("run.start");
    bench
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_owner_lists_subscribes_cancels_and_closes_any_bench() {
    let h = BenchHarness::new(RunScript::default());
    let bench = window_bench_with_run(&h, "a", "r1").await;
    let owner = AuthenticatedPrincipal::owner();

    let listed = h
        .call(&owner, OperationId::BenchList, json!({}))
        .await
        .expect("bench.list");
    let benches = listed.as_array().unwrap();
    assert_eq!(benches.len(), 1, "{listed}");
    assert_eq!(benches[0]["benchId"], bench);
    assert_eq!(benches[0]["owner"], "desktop:window:a:inc-1");
    assert_eq!(benches[0]["runs"][0]["runId"], "r1");

    for stream in [
        format!("exchange:{bench}"),
        format!("bench:{bench}"),
        "run:r1".to_owned(),
    ] {
        assert!(
            subscribe(&h, owner.clone(), stream.clone()).is_ok(),
            "owner subscribes {stream}"
        );
    }

    h.call(
        &owner,
        OperationId::RunCancel,
        json!({ "benchId": bench, "runId": "r1" }),
    )
    .await
    .expect("owner cancels a run on a window's bench");
    let closed = h
        .call(&owner, OperationId::BenchClose, json!({ "benchId": bench }))
        .await
        .expect("owner closes a window's bench");
    assert_eq!(closed["closed"], true, "{closed}");
    assert!(h.rt.runtime.benches().registry.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn window_and_agent_principals_still_only_reach_their_own_benches() {
    let h = BenchHarness::new(RunScript::default());
    let bench = window_bench_with_run(&h, "a", "r1").await;
    let other = window("b");

    let listed = h
        .call(&other, OperationId::BenchList, json!({}))
        .await
        .expect("bench.list for another window");
    assert_eq!(listed, json!([]), "another window sees none of a's benches");
    let own = h
        .call(&window("a"), OperationId::BenchList, json!({}))
        .await
        .expect("bench.list for the opening window");
    assert_eq!(own.as_array().unwrap().len(), 1);

    assert_eq!(
        code(
            h.call(
                &other,
                OperationId::RunCancel,
                json!({ "benchId": bench, "runId": "r1" })
            )
            .await
        ),
        FaultCode::Forbidden
    );
    assert_eq!(
        code(
            h.call(&other, OperationId::BenchClose, json!({ "benchId": bench }))
                .await
        ),
        FaultCode::Forbidden
    );
    for stream in [format!("bench:{bench}"), "run:r1".to_owned()] {
        assert!(
            subscribe(&h, other.clone(), stream.clone()).is_err(),
            "{stream}"
        );
        assert!(
            subscribe(&h, AuthenticatedPrincipal::agent("r1"), stream.clone()).is_err(),
            "{stream}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_owner_does_not_bypass_agent_only_operations() {
    let h = BenchHarness::new(RunScript::default());
    window_bench_with_run(&h, "a", "r1").await;
    let owner = AuthenticatedPrincipal::owner();
    assert_eq!(
        code(
            h.call(
                &owner,
                OperationId::OrchestrationGetOwnTask,
                json!({ "runId": "r1", "arguments": {} })
            )
            .await
        ),
        FaultCode::Forbidden
    );
    assert_eq!(
        code(
            h.call(
                &owner,
                OperationId::ExchangeGetForRun,
                json!({ "runId": "r1", "requestId": "x" })
            )
            .await
        ),
        FaultCode::Forbidden
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_only_operations_are_forbidden_for_window_and_agent_principals() {
    let h = BenchHarness::new(RunScript::default());
    let inputs = [
        (OperationId::ServerStatus, json!({})),
        (
            OperationId::LeaseAcquire,
            json!({ "clientKind": "desktop", "clientId": "c" }),
        ),
        (OperationId::LeaseRenew, json!({ "leaseId": "l" })),
        (OperationId::LeaseRelease, json!({ "leaseId": "l" })),
        (
            OperationId::DesktopIssueWindowToken,
            json!({ "label": "a", "incarnation": "i", "origin": "tauri://localhost" }),
        ),
        (
            OperationId::DesktopRetireWindow,
            json!({ "label": "a", "incarnation": "i", "closeBench": true }),
        ),
    ];
    for principal in [window("a"), AuthenticatedPrincipal::agent("r1")] {
        for (operation, input) in &inputs {
            assert_eq!(
                code(h.call(&principal, *operation, input.clone()).await),
                FaultCode::Forbidden,
                "{operation} as {}",
                principal.subject
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leases_are_acquired_renewed_and_released_by_the_owner() {
    let h = BenchHarness::new(RunScript::default());
    let owner = AuthenticatedPrincipal::owner();
    let lease = h
        .call(
            &owner,
            OperationId::LeaseAcquire,
            json!({ "clientKind": "desktop", "clientId": "app-1" }),
        )
        .await
        .expect("lease.acquire");
    let id = lease["leaseId"].as_str().unwrap().to_owned();
    assert_eq!(lease["ttlSeconds"], 30);
    assert_eq!(h.rt.runtime.server_control().leases().count(), 1);

    let renewed = h
        .call(&owner, OperationId::LeaseRenew, json!({ "leaseId": id }))
        .await
        .expect("lease.renew");
    assert_eq!(renewed["ttlSeconds"], 30);
    assert_eq!(
        code(
            h.call(
                &owner,
                OperationId::LeaseRenew,
                json!({ "leaseId": "nope" })
            )
            .await
        ),
        FaultCode::NotFound
    );

    let released = h
        .call(&owner, OperationId::LeaseRelease, json!({ "leaseId": id }))
        .await
        .expect("lease.release");
    assert_eq!(released["released"], true);
    let again = h
        .call(&owner, OperationId::LeaseRelease, json!({ "leaseId": id }))
        .await
        .expect("lease.release twice");
    assert_eq!(again["released"], false);
    assert_eq!(h.rt.runtime.server_control().leases().count(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lost_lease_acquire_response_replays_the_original_lease() {
    let h = BenchHarness::new(RunScript::default());
    let call = || {
        h.rt.runtime.call(
            AuthenticatedPrincipal::owner(),
            command_request(
                OperationId::LeaseAcquire,
                "same-lease-command",
                json!({ "clientKind": "desktop", "clientId": "app-retry" }),
            ),
        )
    };
    let first = call().await.unwrap().output().cloned().unwrap();
    let replay = call().await.unwrap().output().cloned().unwrap();
    assert_eq!(replay, first);
    assert_eq!(h.rt.runtime.server_control().leases().count(), 1);
}

/// T026 응답 모양 고정(T041에서 갱신): 모든 필드를 파생한다 — `notYetDerived`는 빈 배열이고, 파생 수는 `null`이 아니라
/// 실제 값이다. 쉬는 세션(idle run)은 활동 작업이 아니므로 알려진 수가 모두 0이면 `blocks_stop`이 거짓이다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_status_derives_every_field() {
    let h = BenchHarness::new(RunScript::default());
    let owner = AuthenticatedPrincipal::owner();
    let bench = window_bench_with_run(&h, "a", "r1").await;
    h.call(
        &owner,
        OperationId::LeaseAcquire,
        json!({ "clientKind": "test", "clientId": "t" }),
    )
    .await
    .expect("lease.acquire");

    let status = h
        .call(&owner, OperationId::ServerStatus, json!({}))
        .await
        .expect("server.status");
    assert_eq!(status["state"], "serving");
    assert_eq!(status["serverEpoch"], h.rt.runtime.epoch());
    assert_eq!(status["leases"], 1);
    assert!(status["activeWork"]["busyRuns"].is_u64());
    assert!(status["activeWork"]["acceptedCalls"].is_u64());
    assert!(status["activeWork"]["reservations"].is_u64());

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
    assert_eq!(status["unresolvedOperations"], 0, "{status}");
    assert_eq!(status["undeliverableExchanges"], json!([]), "{status}");
    assert_eq!(status["failedExchangeDeliveries"], json!([]), "{status}");
    assert!(
        status.get("idleSince").is_none(),
        "a lease is held, so the server is not idle: {status}"
    );

    // 살아 있는 run은 bench.list와 같은 상태로 idleRuns에 들어간다.
    let listed = h
        .call(&owner, OperationId::BenchList, json!({}))
        .await
        .expect("bench.list");
    let idle = listed
        .as_array()
        .unwrap()
        .iter()
        .filter(|summary| summary["benchId"] == bench.as_str())
        .flat_map(|summary| summary["runs"].as_array().unwrap().clone())
        .filter(|run| run["state"] == "idle")
        .count();
    assert_eq!(status["idleRuns"], idle as u64);

    let typed: workbench_protocol::operations::server::ServerStatusOutput =
        serde_json::from_value(status).expect("status deserializes");
    assert!(
        !typed.active_work.blocks_stop(),
        "an idle session is not active work: {:?}",
        typed.active_work
    );
}
