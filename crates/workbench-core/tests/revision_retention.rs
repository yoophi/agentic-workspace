//! Codex 리뷰 반영(research R5): TTL GC가 ledger 결과를 모두 지우고 재시작해도 revision은 되돌아가지 않고,
//! 이전 revision으로 보낸 `expectedRevision`은 거절된다.

mod support;

use support::{create_request, TestRuntime};
use workbench_core::ports::operation_ledger::{LedgerState, OperationLedger};
use workbench_protocol::FaultCode;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revision_survives_gc_and_restart() {
    let rt = TestRuntime::new();
    for index in 1..=3 {
        let reply = rt
            .call(create_request(
                &format!("k{index}"),
                &format!("P{index}"),
                "/tmp",
            ))
            .await
            .unwrap();
        assert_eq!(reply.revision(), Some(index));
    }
    assert_eq!(rt.runtime.coordinator().revision(), 3);

    // 모든 결과를 만료시키고 GC
    let ledger = rt.runtime.ledger();
    assert_eq!(
        ledger
            .set_expires_at_for_all(chrono::Utc::now() - chrono::Duration::hours(1))
            .unwrap(),
        3
    );
    assert_eq!(ledger.gc_expired(chrono::Utc::now()).unwrap(), 3);
    assert_eq!(ledger.count_by_state(LedgerState::Applied).unwrap(), 0);
    assert_eq!(
        ledger.current_revision("projects").unwrap(),
        3,
        "aggregate_revision은 GC와 무관"
    );

    // 재시작 뒤에도 revision이 이어진다.
    let rt = rt.restart();
    assert_eq!(rt.runtime.coordinator().revision(), 3);
    assert_eq!(rt.projects().len(), 3);

    let reply = rt.call(create_request("k4", "P4", "/tmp")).await.unwrap();
    assert_eq!(reply.revision(), Some(4), "GC 뒤 첫 변경은 3 다음인 4");

    // 예전 revision으로 보낸 요청은 stale
    let mut stale = create_request("k5", "P5", "/tmp");
    stale.expected_revision = Some(3);
    let fault = rt.call(stale).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::PreconditionFailed);
    assert_eq!(fault.details.unwrap()["currentRevision"], 4);
    assert_eq!(rt.projects().len(), 4);
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
            .count_by_state(LedgerState::Failed)
            .unwrap(),
        1
    );

    // 같은 키로 다시 보내면 저장된 실패가 그대로 돌아온다(재실행 없음).
    let mut again = create_request("k5", "P5", "/tmp");
    again.expected_revision = Some(3);
    let fault = rt.call(again).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::PreconditionFailed);
    assert_eq!(rt.projects().len(), 4);
}
