//! 043 T043(SC-004d 서버 측 근거): 교환 prompt의 run 전송은 멱등성 키 `exchange-delivery:<requestId>`를 쓴다. 원장이 없는 새
//! 창(새로고침)에서 같은 교환이 다시 라우팅돼 같은 키로 다시 보내져도, 같은 세대에서는 agent 엔진에 한 번만 전달된다.
//! 대조: 키가 다르면 두 번 전달된다(시험이 우연히 통과하지 않음을 보인다).

mod support;

use std::sync::atomic::Ordering;

use serde_json::json;
use support::{scripted_run_engine::RunScript, BenchHarness};
use workbench_protocol::OperationId;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_same_exchange_delivery_key_reaches_the_agent_once() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let prompt = json!({ "benchId": bench, "runId": "r1", "prompt": "peer message" });

    let first = h
        .keyed(OperationId::RunSendPrompt, "exchange-delivery:x-1", prompt.clone())
        .await
        .expect("first delivery");
    let again = h
        .keyed(OperationId::RunSendPrompt, "exchange-delivery:x-1", prompt.clone())
        .await
        .expect("same key replays the stored result");
    assert_eq!(first, again);
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), 1, "agent received the exchange once");

    h.keyed(OperationId::RunSendPrompt, "exchange-delivery:x-2", prompt)
        .await
        .expect("another exchange");
    assert_eq!(h.engine.prompts.load(Ordering::SeqCst), 2, "a different key is a different delivery");
}
