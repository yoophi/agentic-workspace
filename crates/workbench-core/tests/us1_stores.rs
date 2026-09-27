//! 038 US1: 저장 단위 4개에 대한 동시성(SC-004), GC 뒤 revision 연속성(research R5), 손상 복구가 lock 안에서만
//! 일어남(R14). 037의 `concurrency.rs`·`revision_retention.rs`·`recovery_under_lock.rs`는 손대지 않는다.

mod support;

use std::{collections::BTreeSet, fs, sync::Arc};

use serde_json::{json, Value};
use support::{command_request, query_request, TestRuntime};
use workbench_core::ports::operation_ledger::{LedgerState, OperationLedger};
use workbench_protocol::{CallRequest, FaultCode, OperationId};

const N: usize = 20;

fn saved_prompt_create(key: &str, label: &str) -> CallRequest {
    command_request(
        OperationId::SavedPromptCreate,
        key,
        json!({ "label": label, "prompt": "p" }),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn twenty_saved_prompt_creates_with_distinct_keys_all_apply() {
    let rt = Arc::new(TestRuntime::new());
    let tasks: Vec<_> = (0..N)
        .map(|index| {
            let rt = Arc::clone(&rt);
            tokio::spawn(async move {
                rt.call(saved_prompt_create(
                    &format!("k{index}"),
                    &format!("P{index}"),
                ))
                .await
            })
        })
        .collect();
    let mut revisions = BTreeSet::new();
    for task in tasks {
        let reply = task.await.unwrap().expect("each create succeeds");
        revisions.insert(reply.revision().unwrap());
    }
    assert_eq!(revisions, (1..=N as u64).collect::<BTreeSet<_>>());
    assert_eq!(rt.saved_prompts().len(), N);
    assert_eq!(
        rt.runtime.coordinator().revision_of("saved-prompts"),
        N as u64
    );
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Applied)
            .unwrap(),
        N
    );
}

/// 같은 worktree에 동시에 진행을 기록해도 한 번에 하나만 적용되고 어느 기록도 사라지지 않는다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn twenty_goal_progress_records_on_one_worktree_serialize() {
    let rt = Arc::new(TestRuntime::new());
    rt.call(command_request(
        OperationId::GoalCreate,
        "g",
        json!({ "workingDirectory": "/repo/wt", "objective": "Ship" }),
    ))
    .await
    .unwrap();
    let tasks: Vec<_> = (1..=N)
        .map(|index| {
            let rt = Arc::clone(&rt);
            tokio::spawn(async move {
                rt.call(command_request(
                    OperationId::GoalRecordProgress,
                    &format!("p{index}"),
                    json!({ "workingDirectory": "/repo/wt", "tokensUsed": index * 10, "timeUsedSeconds": 1 }),
                ))
                .await
            })
        })
        .collect();
    for task in tasks {
        task.await.unwrap().expect("each record succeeds");
    }
    let goal = &rt.goals()[0];
    assert_eq!(goal["tokensUsed"], (N * 10) as u64, "max 규칙");
    assert_eq!(
        goal["timeUsedSeconds"], N as u64,
        "누적 규칙 — 사라진 기록 없음"
    );
    assert_eq!(rt.runtime.coordinator().revision_of("goals"), N as u64 + 1);
}

struct Store {
    aggregate: &'static str,
    mutation: fn(&str, usize) -> CallRequest,
    query: CallRequest,
    file: fn(&TestRuntime) -> std::path::PathBuf,
}

fn stores() -> Vec<Store> {
    vec![
        Store {
            aggregate: "projects",
            mutation: |key, index| {
                command_request(
                    OperationId::ProjectCreate,
                    key,
                    json!({ "name": format!("P{index}"), "workingDirectory": "/tmp/p" }),
                )
            },
            query: query_request(OperationId::ProjectList, json!({})),
            file: |rt| rt.paths.projects_file(),
        },
        Store {
            aggregate: "saved-prompts",
            mutation: |key, index| saved_prompt_create(key, &format!("L{index}")),
            query: query_request(OperationId::SavedPromptList, json!({})),
            file: |rt| rt.paths.saved_prompts_file(),
        },
        Store {
            aggregate: "goals",
            mutation: |key, index| {
                command_request(
                    OperationId::GoalCreate,
                    key,
                    json!({ "workingDirectory": format!("/repo/wt{index}"), "objective": "Ship" }),
                )
            },
            query: query_request(
                OperationId::GoalGet,
                json!({ "workingDirectory": "/repo/wt1" }),
            ),
            file: |rt| rt.paths.goals_file(),
        },
        Store {
            aggregate: "agent-run-settings",
            mutation: |key, index| {
                command_request(
                    OperationId::AgentRunSettingsSave,
                    key,
                    json!({ "settings": { "workingDirectory": format!("/repo/wt{index}") } }),
                )
            },
            query: query_request(
                OperationId::AgentRunSettingsGet,
                json!({ "workingDirectory": "/repo/wt1" }),
            ),
            file: |rt| rt.paths.agent_run_settings_file(),
        },
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revision_survives_gc_and_restart_for_every_store() {
    for store in stores() {
        let rt = TestRuntime::new();
        for index in 1..=2 {
            let reply = rt
                .call((store.mutation)(&format!("k{index}"), index))
                .await
                .unwrap();
            assert_eq!(reply.revision(), Some(index as u64), "{}", store.aggregate);
        }
        let ledger = rt.runtime.ledger();
        ledger
            .set_expires_at_for_all(chrono::Utc::now() - chrono::Duration::hours(1))
            .unwrap();
        assert_eq!(ledger.gc_expired(chrono::Utc::now()).unwrap(), 2);

        let rt = rt.restart();
        assert_eq!(
            rt.runtime.coordinator().revision_of(store.aggregate),
            2,
            "{}: GC·재시작 뒤에도 revision 유지",
            store.aggregate
        );
        let reply = rt.call((store.mutation)("k3", 3)).await.unwrap();
        assert_eq!(reply.revision(), Some(3), "{}", store.aggregate);

        let mut stale = (store.mutation)("k4", 4);
        stale.expected_revision = Some(2);
        let fault = rt.call(stale).await.unwrap_err();
        assert_eq!(
            fault.code,
            FaultCode::PreconditionFailed,
            "{}",
            store.aggregate
        );
        assert_eq!(fault.details.unwrap()["currentRevision"], 3);
    }
}

/// 조회는 파일을 쓰지 않고, 손상 복구는 lock 안에서 `.bak`으로만 일어난다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn corrupt_store_is_recovered_from_backup_under_lock_for_every_store() {
    for store in stores() {
        let rt = TestRuntime::new();
        rt.call((store.mutation)("k1", 1)).await.unwrap();
        rt.call((store.mutation)("k2", 2)).await.unwrap(); // .bak = 첫 번째 상태
        let path = (store.file)(&rt);
        fs::write(&path, b"{ this is not json").unwrap();

        let reply = rt.call(store.query.clone()).await.unwrap();
        let output = reply.output().unwrap().clone();
        match output {
            Value::Array(items) => assert_eq!(items.len(), 1, "{}: .bak 내용", store.aggregate),
            Value::Object(_) => {}
            other => panic!("{}: unexpected output {other}", store.aggregate),
        }
        let restored: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            restored.as_array().unwrap().len(),
            1,
            "{}: primary가 .bak으로 복구됨",
            store.aggregate
        );
    }
}
