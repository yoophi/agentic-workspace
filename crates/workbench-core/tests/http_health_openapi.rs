//! 042 US5: `/health/live`는 인증 없이 정보 없이 답하고, `/health/ready`·`/openapi.json`은 인증이 필요하며, 인증 뒤
//! 계약 문서는 커밋된 `workbench.openapi.json`과 같다.

use std::sync::Arc;

use serde_json::{json, Value};
use support::{
    http_harness::{Harness, TOKEN_DESKTOP},
    TestRuntime,
};
use workbench_protocol::Workbench;

mod support;

async fn get(harness: &Harness, path: &str, token: Option<&str>) -> reqwest::Response {
    let mut builder = harness.client().get(format!("{}{path}", harness.base()));
    if let Some(token) = token {
        builder = builder.bearer_auth(token);
    }
    builder.send().await.unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_and_contract_endpoints() {
    let rt = TestRuntime::new();
    let harness = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;

    let live = get(&harness, "/health/live", None).await;
    assert_eq!(live.status(), 200);
    let body: Value = live.json().await.unwrap();
    assert_eq!(
        body,
        json!({ "status": "live" }),
        "live reveals nothing else"
    );

    assert_eq!(get(&harness, "/health/ready", None).await.status(), 401);
    assert_eq!(get(&harness, "/openapi.json", None).await.status(), 401);

    let ready = get(&harness, "/health/ready", Some(TOKEN_DESKTOP)).await;
    assert_eq!(ready.status(), 200);
    let body: Value = ready.json().await.unwrap();
    assert_eq!(body["ready"], json!(true));
    assert!(body["serverEpoch"].is_string());

    let document = get(&harness, "/openapi.json", Some(TOKEN_DESKTOP)).await;
    assert_eq!(document.status(), 200);
    let served = document.text().await.unwrap();
    let committed = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../workbench-protocol/openapi/workbench.openapi.json"),
    )
    .unwrap();
    let served: Value = serde_json::from_str(&served).unwrap();
    let committed: Value = serde_json::from_str(&committed).unwrap();
    assert_eq!(
        served, committed,
        "served contract equals the committed file"
    );
}
