//! 043 T004: 창별 데스크톱 주체(`desktop:window:<label>:<incarnation>`)의 격리. 작업대는 연 창의 주체에 묶이고,
//! 다른 창·같은 label의 새 incarnation은 그 작업대의 run·교환·orchestration을 조작하거나 구독할 수 없다.
//! 판정은 서버(core)의 기존 작업대 소유 규칙이 한다 — 창 주체는 subject만 나눈다.

#![allow(clippy::result_large_err)]

mod support;

use serde_json::{json, Value};
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_protocol::{
    AuthenticatedPrincipal, FaultCode, OperationId, StreamCursor, Subscription, Workbench,
    WorkbenchFault,
};

fn window(label: &str, incarnation: &str) -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::desktop_window(label, incarnation)
}

async fn open_as(h: &BenchHarness, principal: &AuthenticatedPrincipal) -> String {
    let output = h
        .call(
            principal,
            OperationId::BenchOpen,
            json!({ "workingDirectory": h.dir }),
        )
        .await
        .expect("bench.open");
    output["benchId"].as_str().unwrap().to_owned()
}

fn forbidden(result: Result<Value, WorkbenchFault>, what: &str) {
    match result {
        Ok(value) => panic!("{what} must be rejected, got {value}"),
        Err(fault) => assert_eq!(
            fault.code,
            FaultCode::Forbidden,
            "{what}: {}",
            fault.message
        ),
    }
}

fn subscribe(
    h: &BenchHarness,
    principal: &AuthenticatedPrincipal,
    stream_id: String,
) -> Result<workbench_protocol::EventStream, WorkbenchFault> {
    h.rt.runtime.events(
        principal.clone(),
        Subscription {
            cursors: vec![StreamCursor {
                stream_id,
                epoch: h.rt.runtime.epoch().into(),
                after_sequence: 0,
            }],
        },
    )
}

#[test]
fn window_principal_subject_carries_label_and_incarnation_with_full_desktop_scopes() {
    let principal = window("session-1", "inc-7");
    assert_eq!(principal.subject.as_str(), "desktop:window:session-1:inc-7");
    assert_eq!(principal.kind, workbench_protocol::PrincipalKind::Desktop);
    assert_eq!(principal.scopes, AuthenticatedPrincipal::desktop().scopes);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn another_window_and_a_reopened_label_cannot_touch_the_bench() {
    let h = BenchHarness::new(RunScript::default());
    let owner = window("session-1", "inc-1");
    let bench = open_as(&h, &owner).await;
    h.call(
        &owner,
        OperationId::RunStart,
        json!({ "benchId": bench, "request": { "goal": "g", "agentId": "codex", "runId": "r1" } }),
    )
    .await
    .expect("owner starts a run");
    h.call(
        &owner,
        OperationId::ExchangeSyncWorkspace,
        json!({"benchId": bench, "request": {
            "worktreePath": h.dir, "revision": 1, "focusedPanelId": "main",
            "panels": [{"panelId": "main", "title": "Main", "runId": "r1", "status": "running"}]}}),
    )
    .await
    .expect("owner syncs workspace");
    // 주인 창이 bootstrap한 실제 묶임의 스트림. 있는 스트림이라 거절 사유가 "없음"이 될 수 없다.
    let session = h
        .call(
            &owner,
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": bench, "worktreePath": h.dir }),
        )
        .await
        .expect("owner bootstraps orchestration");
    let orchestration_stream = session["eventStreamId"]
        .as_str()
        .expect("bound session carries its stream id")
        .to_owned();
    assert!(orchestration_stream.starts_with("orchestration:"));
    let owned_streams = [
        "run:r1".to_owned(),
        format!("exchange:{bench}"),
        format!("bench:{bench}"),
        orchestration_stream.clone(),
    ];
    // 대조: 주인 창은 네 스트림 모두 구독할 수 있다(거절이 스트림 부재 때문이 아님을 보인다).
    for stream in &owned_streams {
        assert!(
            subscribe(&h, &owner, stream.clone()).is_ok(),
            "owner must subscribe {stream}"
        );
    }

    // 다른 창과, 같은 label로 다시 연 창(새 incarnation) — 둘 다 작업대 id를 알아도 거절된다.
    for intruder in [window("session-2", "inc-1"), window("session-1", "inc-2")] {
        let who = intruder.subject.as_str().to_owned();
        let calls: Vec<(OperationId, Value)> = vec![
            (
                OperationId::RunStart,
                json!({ "benchId": bench, "request": { "goal": "g", "agentId": "codex", "runId": "rx" } }),
            ),
            (
                OperationId::RunSendPrompt,
                json!({ "benchId": bench, "runId": "r1", "prompt": "hi" }),
            ),
            (
                OperationId::RunCancel,
                json!({ "benchId": bench, "runId": "r1" }),
            ),
            (
                OperationId::RunReplay,
                json!({ "benchId": bench, "runId": "r1", "afterSequence": 0 }),
            ),
            (
                OperationId::ExchangeSyncWorkspace,
                json!({"benchId": bench, "request": {
                    "worktreePath": h.dir, "revision": 2, "focusedPanelId": "main", "panels": []}}),
            ),
            (OperationId::ExchangeList, json!({ "benchId": bench })),
            (
                OperationId::ExchangeSend,
                json!({ "benchId": bench, "request": {
                    "requestId": "x1", "sourcePanelId": "main", "targetPanelId": "main",
                    "message": "m", "delivery": "queue" } }),
            ),
            (OperationId::OrchestrationGet, json!({ "benchId": bench })),
            (
                OperationId::OrchestrationBootstrap,
                json!({ "benchId": bench, "worktreePath": h.dir }),
            ),
            (OperationId::BenchClose, json!({ "benchId": bench })),
        ];
        for (operation, input) in calls {
            forbidden(
                h.call(&intruder, operation, input).await,
                &format!("{who} {}", operation.as_str()),
            );
        }
        for stream in &owned_streams {
            match subscribe(&h, &intruder, stream.clone()) {
                Ok(_) => panic!("{who} must not subscribe {stream}"),
                Err(fault) => {
                    // run 스트림은 run의 작업대 소유로 판정한다(문구가 다르다). 나머지는 작업대 소유 문구.
                    let expected = if stream.starts_with("run:") {
                        "run is owned by another bench."
                    } else {
                        "bench belongs to another principal."
                    };
                    assert_eq!(
                        (fault.code, fault.message.as_str()),
                        (FaultCode::Forbidden, expected),
                        "{who} {stream}"
                    )
                }
            }
        }
    }

    // 주인 창은 거절 시도 뒤에도 계속 쓸 수 있다.
    for stream in &owned_streams {
        assert!(
            subscribe(&h, &owner, stream.clone()).is_ok(),
            "owner {stream}"
        );
    }
    assert!(h
        .call(
            &owner,
            OperationId::ExchangeList,
            json!({ "benchId": bench })
        )
        .await
        .is_ok());
}
