//! SC-004 / FR-010: 동시 요청이 aggregate lock과 ledger unique 제약으로 직렬화된다(research R12).

mod support;

use std::{collections::BTreeSet, sync::Arc};

use support::{create_request, http_harness::Harness, TestRuntime};
use workbench_core::ports::operation_ledger::LedgerState;
use workbench_protocol::{FaultCode, Outcome, Workbench};

const N: usize = 20;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn twenty_distinct_keys_all_apply_with_monotonic_revisions() {
    let rt = Arc::new(TestRuntime::new());
    let tasks: Vec<_> = (0..N)
        .map(|index| {
            let rt = Arc::clone(&rt);
            tokio::spawn(async move {
                rt.call(create_request(
                    &format!("k{index}"),
                    &format!("P{index}"),
                    "/tmp/p",
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
    assert_eq!(rt.projects().len(), N);
    assert_eq!(rt.runtime.coordinator().revision(), N as u64);
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Applied)
            .unwrap(),
        N
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn twenty_same_key_requests_apply_exactly_once() {
    let rt = Arc::new(TestRuntime::new());
    let tasks: Vec<_> = (0..N)
        .map(|_| {
            let rt = Arc::clone(&rt);
            tokio::spawn(async move { rt.call(create_request("same", "Same", "/tmp/same")).await })
        })
        .collect();

    let mut ok_ids = BTreeSet::new();
    let mut conflicts = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(reply) => {
                ok_ids.insert(reply.output().unwrap()["id"].as_str().unwrap().to_owned());
                assert_eq!(reply.revision(), Some(1));
            }
            Err(fault) => {
                assert_eq!(fault.code, FaultCode::Conflict, "{fault}");
                assert_eq!(
                    fault.outcome,
                    Outcome::Unknown,
                    "진행 중 충돌은 unknown outcome"
                );
                assert!(fault.retryable);
                conflicts += 1;
            }
        }
    }
    assert_eq!(ok_ids.len(), 1, "성공 응답은 모두 같은 프로젝트를 가리킨다");
    assert_eq!(rt.projects().len(), 1);
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Applied)
            .unwrap(),
        1
    );
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Pending)
            .unwrap(),
        0
    );
    assert!(conflicts < N, "적어도 하나는 성공한다");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn twenty_distinct_keys_over_http_all_apply() {
    let rt = TestRuntime::new();
    let workbench: Arc<dyn Workbench> = rt.runtime.clone();
    let harness = Arc::new(Harness::spawn(workbench).await);
    let tasks: Vec<_> = (0..N)
        .map(|index| {
            let harness = Arc::clone(&harness);
            tokio::spawn(async move {
                harness
                    .call(
                        Some(support::http_harness::TOKEN_DESKTOP),
                        &create_request(&format!("h{index}"), &format!("H{index}"), "/tmp/h"),
                    )
                    .await
            })
        })
        .collect();
    let mut revisions = BTreeSet::new();
    for task in tasks {
        revisions.insert(
            task.await
                .unwrap()
                .expect("http create")
                .revision()
                .unwrap(),
        );
    }
    assert_eq!(revisions.len(), N);
    assert_eq!(rt.projects().len(), N);
}
