//! 042 US1: 호환성 협상(research R6), 모든 응답의 `AW-Protocol-Version` 헤더, 인증 없음 `401`.

mod support;

use std::sync::Arc;

use serde_json::{json, Value};
use support::{
    http_harness::{Harness, TOKEN_DESKTOP},
    TestRuntime,
};
use workbench_protocol::{Workbench, PROTOCOL_VERSION};

async fn spawn() -> (TestRuntime, Harness) {
    let rt = TestRuntime::new();
    let harness = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    (rt, harness)
}

async fn handshake(harness: &Harness, token: Option<&str>, body: Value) -> reqwest::Response {
    let mut builder = harness
        .client()
        .post(format!("{}/v1/system/handshake", harness.base()))
        .json(&body);
    if let Some(token) = token {
        builder = builder.bearer_auth(token);
    }
    builder.send().await.unwrap()
}

fn protocol_header(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(workbench_server::PROTOCOL_HEADER)
        .map(|value| value.to_str().unwrap().to_owned())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handshake_selects_the_common_version() {
    let (_rt, harness) = spawn().await;
    let response = handshake(
        &harness,
        Some(TOKEN_DESKTOP),
        json!({ "supportedProtocolVersions": [PROTOCOL_VERSION, 99], "client": { "name": "t", "version": "0" } }),
    )
    .await;
    assert_eq!(response.status(), 200);
    assert_eq!(protocol_header(&response).as_deref(), Some("1"));
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["selectedProtocolVersion"], json!(PROTOCOL_VERSION));
    assert_eq!(body["apiMajor"], json!(1));
    assert_eq!(body["contractHash"].as_str().unwrap().len(), 64);
    assert!(body["instanceId"].is_string());
    assert!(body["serverEpoch"].is_string());
    assert_eq!(body["storageSchemaVersion"], json!(2));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn handshake_without_a_common_version_is_a_stable_conflict() {
    let (_rt, harness) = spawn().await;
    let response = handshake(
        &harness,
        Some(TOKEN_DESKTOP),
        json!({ "supportedProtocolVersions": [99] }),
    )
    .await;
    assert_eq!(response.status(), 409);
    assert_eq!(protocol_header(&response).as_deref(), Some("1"));
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["code"], json!("conflict"));
    assert_eq!(
        body["message"],
        json!(workbench_server::handshake::MESSAGE_PROTOCOL_UNSUPPORTED)
    );
    assert_eq!(
        body["details"]["supportedProtocolVersions"],
        json!([PROTOCOL_VERSION])
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unauthenticated_requests_get_401_with_the_protocol_header() {
    let (_rt, harness) = spawn().await;
    let response = handshake(&harness, None, json!({ "supportedProtocolVersions": [1] })).await;
    assert_eq!(response.status(), 401);
    assert_eq!(protocol_header(&response).as_deref(), Some("1"));
    let response = handshake(
        &harness,
        Some("wrong"),
        json!({ "supportedProtocolVersions": [1] }),
    )
    .await;
    assert_eq!(response.status(), 401);
    for path in ["/v1/calls", "/v1/event-tickets"] {
        let response = harness
            .client()
            .post(format!("{}{path}", harness.base()))
            .json(&json!({}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 401, "{path}");
        assert_eq!(protocol_header(&response).as_deref(), Some("1"), "{path}");
    }
    let live = harness
        .client()
        .get(format!("{}/health/live", harness.base()))
        .send()
        .await
        .unwrap();
    assert_eq!(live.status(), 200, "live needs no credential");
    assert_eq!(protocol_header(&live).as_deref(), Some("1"));
}
