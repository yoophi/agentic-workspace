#[path = "../examples/support/echo.rs"]
mod echo;

use local_mcp_server::{HostConfig, LocalMcpHost, start};
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

const TOKEN: &str = "fixture-credential";
type Grants = Arc<Mutex<HashMap<String, String>>>;

async fn host(name: &'static str, config: HostConfig) -> (LocalMcpHost, Grants) {
    let grants = Arc::new(Mutex::new(HashMap::from([
        (TOKEN.to_owned(), "scope-a".to_owned()),
        ("other-credential".to_owned(), "scope-b".to_owned()),
    ])));
    let resolver = grants.clone();
    let host = start(
        move || Ok(echo::Echo(name)),
        move |token| resolver.lock().unwrap().get(token).cloned(),
        config,
    )
    .await
    .unwrap();
    (host, grants)
}

fn client() -> Client {
    Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(4))
        .build()
        .unwrap()
}

async fn post(client: &Client, host: &LocalMcpHost, body: Value) -> Response {
    client
        .post(host.endpoint())
        .bearer_auth(TOKEN)
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", "2025-11-25")
        .json(&body)
        .send()
        .await
        .unwrap()
}

fn request(id: i64, method: &str, params: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
}

async fn initialize(client: &Client, host: &LocalMcpHost) {
    let response = post(
        client,
        host,
        request(
            1,
            "initialize",
            json!({
                "protocolVersion":"2025-11-25", "capabilities":{},
                "clientInfo":{"name":"http-integration","version":"1"}
            }),
        ),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key("mcp-session-id"));
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["result"]["protocolVersion"], "2025-11-25");
    assert!(body["result"]["capabilities"]["tools"].is_object());
    let response = post(
        client,
        host,
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(response.bytes().await.unwrap().is_empty());
}

async fn shutdown(host: LocalMcpHost) {
    tokio::time::timeout(Duration::from_secs(3), host.shutdown())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn real_http_initialization_discovery_structured_output_and_errors() {
    let (host, _) = host("echo", HostConfig::default()).await;
    let client = client();
    initialize(&client, &host).await;
    let response = post(&client, &host, request(2, "tools/list", json!({}))).await;
    assert_eq!(response.headers()["content-type"], "application/json");
    let tools: Value = response.json().await.unwrap();
    assert_eq!(tools["result"]["tools"][0]["name"], "echo");
    assert_eq!(
        tools["result"]["tools"][0]["inputSchema"]["additionalProperties"],
        false
    );
    assert_eq!(
        tools["result"]["tools"][0]["outputSchema"]["type"],
        "object"
    );
    let result: Value = post(
        &client,
        &host,
        request(
            3,
            "tools/call",
            json!({
                "name":"echo","arguments":{"value":"한글 🌱"}
            }),
        ),
    )
    .await
    .json()
    .await
    .unwrap();
    assert_eq!(result["result"]["isError"], false);
    assert_eq!(
        result["result"]["structuredContent"]["data"],
        json!({"value":"한글 🌱","principal":"scope-a"})
    );
    let text: Value =
        serde_json::from_str(result["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, result["result"]["structuredContent"]);
    for arguments in [
        json!({"value":"fail"}),
        json!({"value":1}),
        json!({"value":"x","extra":true}),
    ] {
        let response = post(
            &client,
            &host,
            request(
                4,
                "tools/call",
                json!({"name":"echo","arguments":arguments}),
            ),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["result"]["isError"], true, "{result}");
    }
    let result: Value = post(
        &client,
        &host,
        request(5, "tools/call", json!({"name":"missing","arguments":{}})),
    )
    .await
    .json()
    .await
    .unwrap();
    assert!(result["error"]["code"].is_number(), "{result}");
    let result: Value = post(&client, &host, request(6, "tools/list", json!({})))
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(result["result"]["tools"][0]["name"], "echo");
    shutdown(host).await;
}

#[tokio::test]
async fn every_method_requires_auth_and_revocation_is_immediate() {
    let (host, grants) = host("echo", HostConfig::default()).await;
    let client = client();
    for method in [Method::POST, Method::GET, Method::DELETE] {
        for credential in [
            None,
            Some("Bearer missing"),
            Some("Basic fixture-credential"),
            Some("Bearer fixture-credential extra"),
        ] {
            let mut request = client.request(method.clone(), host.endpoint());
            if let Some(value) = credential {
                request = request.header("authorization", value);
            }
            let response = request.send().await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert_eq!(response.headers()["www-authenticate"], "Bearer");
            assert_eq!(response.text().await.unwrap(), "Unauthorized");
        }
    }
    let mut headers = reqwest::header::HeaderMap::new();
    headers.append(
        "authorization",
        "Bearer fixture-credential".parse().unwrap(),
    );
    headers.append("authorization", "Bearer other-credential".parse().unwrap());
    assert_eq!(
        client
            .get(host.endpoint())
            .headers(headers)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    initialize(&client, &host).await;
    grants.lock().unwrap().remove(TOKEN);
    assert_eq!(
        post(&client, &host, request(2, "tools/list", json!({})))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .post(host.endpoint())
            .bearer_auth("other-credential")
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-11-25")
            .json(&request(
                3,
                "tools/call",
                json!({"name":"echo","arguments":{"value":"still alive"}})
            ))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()["result"]["structuredContent"]["data"]["principal"],
        "scope-b"
    );
    shutdown(host).await;
}

#[tokio::test]
async fn host_origin_and_protocol_failures_do_not_stop_valid_requests() {
    let (host, _) = host("echo", HostConfig::default()).await;
    let client = client();
    for origin in [
        "https://untrusted.example:443",
        "null",
        "http://localhost:1430",
    ] {
        let response = client
            .post(host.endpoint())
            .bearer_auth(TOKEN)
            .header("origin", origin)
            .header("accept", "application/json, text/event-stream")
            .json(&request(1, "tools/list", json!({})))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    for authority in ["untrusted.example", "127.0.0.1:1", "localhost:1"] {
        let response = client
            .post(host.endpoint())
            .bearer_auth(TOKEN)
            .header("host", authority)
            .header("accept", "application/json, text/event-stream")
            .json(&request(1, "tools/list", json!({})))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    for version in ["invalid", "2024-11-05", "2026-07-28"] {
        let response = client
            .post(host.endpoint())
            .bearer_auth(TOKEN)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", version)
            .json(&request(1, "tools/list", json!({})))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    initialize(&client, &host).await;
    assert_eq!(
        client
            .get(host.endpoint())
            .bearer_auth(TOKEN)
            .header("accept", "text/event-stream")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    shutdown(host).await;
}

#[tokio::test]
async fn explicit_origin_allowlist_matches_scheme_host_and_port() {
    let config = HostConfig::default()
        .with_allowed_origin("http://localhost:1430")
        .unwrap();
    let (host, _) = host("echo", config).await;
    let client = client();
    for (origin, status) in [
        ("http://localhost:1430", StatusCode::OK),
        ("http://localhost:1431", StatusCode::FORBIDDEN),
        ("https://localhost:1430", StatusCode::FORBIDDEN),
        ("http://127.0.0.1:1430", StatusCode::FORBIDDEN),
    ] {
        let response = client
            .post(host.endpoint())
            .bearer_auth(TOKEN)
            .header("origin", origin)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-11-25")
            .json(&request(1, "tools/list", json!({})))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let mut headers = reqwest::header::HeaderMap::new();
    headers.append("origin", "http://localhost:1430".parse().unwrap());
    headers.append("origin", "https://untrusted.example:443".parse().unwrap());
    assert_eq!(
        client
            .get(host.endpoint())
            .bearer_auth(TOKEN)
            .headers(headers)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    shutdown(host).await;
}

#[tokio::test]
async fn body_limit_and_invalid_json_leave_host_usable() {
    let (host, _) = host(
        "echo",
        HostConfig::default().with_max_request_bytes(512).unwrap(),
    )
    .await;
    let client = client();
    let response = client
        .post(host.endpoint())
        .bearer_auth(TOKEN)
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .body("x".repeat(513))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let response = client
        .post(host.endpoint())
        .bearer_auth(TOKEN)
        .header("accept", "application/json, text/event-stream")
        .header("content-type", "application/json")
        .body("{")
        .send()
        .await
        .unwrap();
    // rmcp 3.5.1 reports a JSON decoding failure as 415, not a JSON-RPC result.
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    initialize(&client, &host).await;
    shutdown(host).await;
}

#[tokio::test]
async fn two_hosts_and_principals_are_isolated_and_shutdown_is_owned() {
    let (first, _) = host("first", HostConfig::default()).await;
    let (second, _) = host("second", HostConfig::default()).await;
    let client = client();
    let endpoint = first.endpoint().to_owned();
    for (host, name) in [(&first, "first"), (&second, "second")] {
        let result: Value = post(&client, host, request(1, "tools/list", json!({})))
            .await
            .json()
            .await
            .unwrap();
        assert_eq!(result["result"]["tools"][0]["name"], name);
    }
    let call = |token: &'static str| {
        let client = &client;
        let first = &first;
        async move {
            client
                .post(first.endpoint())
                .bearer_auth(token)
                .header("accept", "application/json, text/event-stream")
                .header("mcp-protocol-version", "2025-11-25")
                .json(&request(
                    4,
                    "tools/call",
                    json!({"name":"first","arguments":{"value":token.len().to_string()}}),
                ))
                .send()
                .await
                .unwrap()
                .json::<Value>()
                .await
                .unwrap()
        }
    };
    let (a, b) = tokio::join!(call(TOKEN), call("other-credential"));
    assert_eq!(
        a["result"]["structuredContent"]["data"]["principal"],
        "scope-a"
    );
    assert_eq!(
        b["result"]["structuredContent"]["data"]["principal"],
        "scope-b"
    );
    first.stop_accepting();
    assert_eq!(
        post(&client, &first, request(2, "tools/list", json!({})))
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    shutdown(first).await;
    assert!(client.get(endpoint).send().await.is_err());
    initialize(&client, &second).await;
    shutdown(second).await;
}

#[tokio::test]
async fn factory_failure_is_sanitized_and_retry_can_recover() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let first = AtomicBool::new(true);
    let host = start(
        move || {
            if first.swap(false, Ordering::SeqCst) {
                Err(std::io::Error::other("fixture-sensitive-error"))
            } else {
                Ok(echo::Echo("echo"))
            }
        },
        |token| (token == TOKEN).then(|| "scope-a".to_owned()),
        HostConfig::default(),
    )
    .await
    .unwrap();
    let client = client();
    let response = post(&client, &host, request(1, "tools/list", json!({}))).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = response.text().await.unwrap();
    assert!(!body.contains("fixture-sensitive-error"));
    initialize(&client, &host).await;
    shutdown(host).await;
}

#[tokio::test]
async fn shutdown_cancels_an_incomplete_authenticated_body() {
    use tokio::io::AsyncWriteExt;
    let (host, _) = host("echo", HostConfig::default()).await;
    let address = host
        .endpoint()
        .strip_prefix("http://")
        .unwrap()
        .strip_suffix("/mcp")
        .unwrap();
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Type: application/json\r\nContent-Length: 400\r\nAccept: application/json, text/event-stream\r\n\r\n{{").as_bytes()).await.unwrap();
    // The socket remains open and never sends the remaining declared bytes.
    shutdown(host).await;
    drop(stream);
}

#[tokio::test]
async fn origin_and_host_validation_apply_to_get_delete_and_notifications() {
    let (host, _) = host("echo", HostConfig::default()).await;
    let client = client();
    for method in [Method::GET, Method::DELETE, Method::POST] {
        for (name, value) in [
            ("origin", "https://untrusted.example:443"),
            ("host", "untrusted.example"),
        ] {
            let response = client
                .request(method.clone(), host.endpoint())
                .bearer_auth(TOKEN)
                .header(name, value)
                .header("accept", "application/json, text/event-stream")
                .json(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
    initialize(&client, &host).await;
    shutdown(host).await;
}

#[test]
fn invalid_configuration_cannot_disable_security_or_body_bounds() {
    assert!(
        HostConfig::default()
            .with_protocol_versions(vec![
                local_mcp_server::rmcp::model::ProtocolVersion::V_2026_07_28
            ])
            .is_err()
    );
    assert!(
        HostConfig::default()
            .with_protocol_versions(vec![])
            .is_err()
    );
    for origin in [
        "http://localhost",
        "http://localhost:*",
        "https://*.example:443",
        "null",
        "ftp://localhost:21",
        "http://user@localhost:80",
        "http://localhost:80/path",
        "http://localhost:80?x=1",
    ] {
        assert!(
            HostConfig::default().with_allowed_origin(origin).is_err(),
            "{origin}"
        );
    }
    assert!(HostConfig::default().with_max_request_bytes(0).is_err());
}

#[tokio::test]
async fn missing_version_can_initialize_but_cannot_use_an_unsupported_implicit_version() {
    let (host, _) = host("echo", HostConfig::default()).await;
    let client = client();
    let send = |body: Value| {
        client
            .post(host.endpoint())
            .bearer_auth(TOKEN)
            .header("accept", "application/json, text/event-stream")
            .json(&body)
    };
    let response = send(request(1,"initialize",json!({"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"client","version":"1"}}))).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json::<Value>().await.unwrap()["result"]["protocolVersion"],
        "2025-11-25"
    );
    assert_eq!(
        send(request(2, "tools/list", json!({})))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    initialize(&client, &host).await;
    shutdown(host).await;
}

struct WaitingHandler {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

impl local_mcp_server::rmcp::ServerHandler for WaitingHandler {
    fn get_info(&self) -> local_mcp_server::rmcp::model::ServerConfig {
        echo::Echo("wait").get_info()
    }
    fn supported_protocol_versions(
        &self,
    ) -> std::borrow::Cow<'static, [local_mcp_server::rmcp::model::ProtocolVersion]> {
        std::borrow::Cow::Borrowed(&[local_mcp_server::rmcp::model::ProtocolVersion::V_2025_11_25])
    }
    async fn call_tool(
        &self,
        _: local_mcp_server::rmcp::model::CallToolRequestParams,
        _: local_mcp_server::rmcp::service::RequestContext<local_mcp_server::rmcp::RoleServer>,
    ) -> Result<local_mcp_server::rmcp::model::CallToolResponse, local_mcp_server::rmcp::ErrorData>
    {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(local_mcp_server::rmcp::model::CallToolResult::structured(json!({"done":true})).into())
    }
}

#[tokio::test]
async fn stop_admission_preserves_an_inflight_handler_until_application_drain() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (factory_entered, factory_release) = (entered.clone(), release.clone());
    let host = start(
        move || {
            Ok(WaitingHandler {
                entered: factory_entered.clone(),
                release: factory_release.clone(),
            })
        },
        |token| (token == TOKEN).then(|| "scope-a".to_owned()),
        HostConfig::default(),
    )
    .await
    .unwrap();
    let client = client();
    let endpoint = host.endpoint().to_owned();
    let pending = tokio::spawn(async move {
        client
            .post(endpoint)
            .bearer_auth(TOKEN)
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-11-25")
            .json(&request(
                1,
                "tools/call",
                json!({"name":"wait","arguments":{}}),
            ))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()
    });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    host.stop_accepting();
    assert_eq!(
        post(&self::client(), &host, request(2, "tools/list", json!({})))
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    release.notify_one();
    assert_eq!(
        pending.await.unwrap()["result"]["structuredContent"],
        json!({"done":true})
    );
    shutdown(host).await;
}

#[tokio::test]
async fn dropping_host_releases_its_owned_listener() {
    let (host, _) = host("echo", HostConfig::default()).await;
    let address = host
        .endpoint()
        .strip_prefix("http://")
        .unwrap()
        .strip_suffix("/mcp")
        .unwrap()
        .to_owned();
    drop(host);
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(listener) = tokio::net::TcpListener::bind(&address).await {
                drop(listener);
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
