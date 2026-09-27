//! 042 research R13 증거: 네트워크 공개 전에 중단 판정 증거가 없던 영속 operation 5개
//! (`project.update`·`project.delete`·`savedPrompt.update`·`goal.update`·`goal.clear`)를 세 중단 지점에서 멈추고
//! 재시작한다. 판정 규칙(038 research R6): 삭제형은 종료 상태 reconciler가 있어 저장 뒤 중단이면 applied, 저장 전
//! 중단이면 unknown. 수정형은 관찰로 적용 여부를 알 수 없어 항상 unknown. 어느 경우든 재시작이 변경을 다시
//! 실행하지 않고, 같은 키 재요청은 applied면 저장된 결과, unknown이면 `conflict`(`outcome: unknown`, 재시도 불가).

mod support;

use serde_json::{json, Value};
use support::{command_request, create_request, TestRuntime};
use workbench_core::{
    application::workbench_runtime::CrashPoint, ports::operation_ledger::LedgerState,
};
use workbench_protocol::{CallRequest, FaultCode, OperationId, Outcome};

const POINTS: [CrashPoint; 3] = [
    CrashPoint::AfterPending,
    CrashPoint::AfterJsonSave,
    CrashPoint::BeforeApplied,
];
const WT: &str = "/repo/wt";

struct Case {
    name: &'static str,
    /// 저장 뒤 중단이면 applied로 판정되는가(종료 상태 reconciler 등록 여부).
    reconciled: bool,
    /// 대상 준비. 변경 요청을 돌려준다.
    prepare:
        fn(&TestRuntime) -> std::pin::Pin<Box<dyn std::future::Future<Output = CallRequest> + '_>>,
    /// 관찰 대상 저장소 스냅숏.
    observe: fn(&TestRuntime) -> Vec<Value>,
}

fn count(rt: &TestRuntime, state: LedgerState) -> usize {
    rt.runtime.ledger().count_by_state(state).unwrap()
}

fn project_dir(rt: &TestRuntime) -> String {
    let dir = rt.dir.path().join("project-wt");
    std::fs::create_dir_all(&dir).unwrap();
    dir.to_string_lossy().into_owned()
}

async fn project_id(rt: &TestRuntime) -> String {
    let reply = rt
        .call(create_request("setup-project", "Before", &project_dir(rt)))
        .await
        .unwrap();
    reply.output().unwrap()["id"].as_str().unwrap().to_owned()
}

async fn saved_prompt_id(rt: &TestRuntime) -> String {
    let reply = rt
        .call(command_request(
            OperationId::SavedPromptCreate,
            "setup-prompt",
            json!({ "label": "Before", "prompt": "keep going" }),
        ))
        .await
        .unwrap();
    reply.output().unwrap()["id"].as_str().unwrap().to_owned()
}

async fn goal(rt: &TestRuntime) {
    rt.call(command_request(
        OperationId::GoalCreate,
        "setup-goal",
        json!({ "workingDirectory": WT, "objective": "Before", "tokenBudget": 1000 }),
    ))
    .await
    .unwrap();
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "project.update",
            reconciled: false,
            prepare: |rt| {
                Box::pin(async move {
                    let id = project_id(rt).await;
                    command_request(
                        OperationId::ProjectUpdate,
                        "change",
                        json!({ "id": id, "name": "After", "workingDirectory": project_dir(rt) }),
                    )
                })
            },
            observe: |rt| rt.projects(),
        },
        Case {
            name: "project.delete",
            reconciled: true,
            prepare: |rt| {
                Box::pin(async move {
                    let id = project_id(rt).await;
                    command_request(OperationId::ProjectDelete, "change", json!({ "id": id }))
                })
            },
            observe: |rt| rt.projects(),
        },
        Case {
            name: "savedPrompt.update",
            reconciled: false,
            prepare: |rt| {
                Box::pin(async move {
                    let id = saved_prompt_id(rt).await;
                    command_request(
                        OperationId::SavedPromptUpdate,
                        "change",
                        json!({ "id": id, "label": "After", "prompt": "changed" }),
                    )
                })
            },
            observe: |rt| rt.saved_prompts(),
        },
        Case {
            name: "goal.update",
            reconciled: false,
            prepare: |rt| {
                Box::pin(async move {
                    goal(rt).await;
                    command_request(
                        OperationId::GoalUpdate,
                        "change",
                        json!({ "workingDirectory": WT, "objective": "After" }),
                    )
                })
            },
            observe: |rt| rt.goals(),
        },
        Case {
            name: "goal.clear",
            reconciled: true,
            prepare: |rt| {
                Box::pin(async move {
                    goal(rt).await;
                    command_request(
                        OperationId::GoalClear,
                        "change",
                        json!({ "workingDirectory": WT }),
                    )
                })
            },
            observe: |rt| rt.goals(),
        },
    ]
}

async fn check(case: &Case, point: CrashPoint) {
    let label = format!("{} @ {point:?}", case.name);
    let rt = TestRuntime::new();
    let request = (case.prepare)(&rt).await;
    let before = (case.observe)(&rt);
    let applied_before = count(&rt, LedgerState::Applied);

    rt.runtime.hooks().set_crash_point(Some(point));
    let fault = rt.call(request.clone()).await.unwrap_err();
    assert!(fault.message.contains("crash injected"), "{label}: {fault}");
    let saved = point != CrashPoint::AfterPending;
    let at_crash = (case.observe)(&rt);
    if saved {
        assert_ne!(at_crash, before, "{label}: the store write happened");
    } else {
        assert_eq!(at_crash, before, "{label}: nothing was written");
    }
    assert_eq!(count(&rt, LedgerState::Pending), 1, "{label}");

    let rt = rt.restart();
    // 재시작은 변경을 다시 실행하지 않는다 — 저장소는 중단 시점 그대로.
    assert_eq!(
        (case.observe)(&rt),
        at_crash,
        "{label}: no automatic re-execution"
    );
    assert_eq!(
        count(&rt, LedgerState::Pending),
        0,
        "{label}: pending resolved"
    );
    let expect_applied = case.reconciled && saved;
    if expect_applied {
        assert_eq!(
            count(&rt, LedgerState::Applied),
            applied_before + 1,
            "{label}"
        );
        assert_eq!(count(&rt, LedgerState::Unknown), 0, "{label}");
        let replay = rt.call(request).await;
        assert!(replay.is_ok(), "{label}: stored result replays: {replay:?}");
    } else {
        assert_eq!(count(&rt, LedgerState::Unknown), 1, "{label}");
        assert_eq!(count(&rt, LedgerState::Applied), applied_before, "{label}");
        let fault = rt.call(request).await.unwrap_err();
        assert_eq!(fault.code, FaultCode::Conflict, "{label}");
        assert_eq!(fault.outcome, Outcome::Unknown, "{label}");
        assert!(!fault.retryable, "{label}");
    }
    assert_eq!(
        (case.observe)(&rt),
        at_crash,
        "{label}: same-key request did not apply"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persistent_updates_and_deletes_have_restart_verdicts_at_every_crash_point() {
    let cases = cases();
    for case in &cases {
        for point in POINTS {
            check(case, point).await;
        }
    }
}
