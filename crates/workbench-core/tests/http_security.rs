//! 042 US3 보안 경계(contracts §1–§6, SC-002·SC-007): 허용 안 된 Host·Origin, 자격 증명 없음·잘못됨·만료, 다른
//! Origin의 데스크톱 토큰, 본문 상한, CORS preflight를 모두 거절하고, 거절 뒤 상태가 바뀌지 않으며, 기록에 토큰·표
//! 문자열이 남지 않는다.

use std::{sync::Arc, time::Duration};

use reqwest::{header, Method, StatusCode};
use serde_json::{json, Value};
use support::{
    create_request,
    http_harness::{Harness, HarnessOptions, TOKEN_DESKTOP},
    list_request, TestRuntime,
};
use workbench_protocol::{StreamCursor, Workbench};
use workbench_server::auth::{DesktopTokenIssuer, TokenOrigin};

mod support;

const APP: &str = "http://localhost:1420";
const RELEASE: &str = "tauri://localhost";

struct Env {
    rt: TestRuntime,
    harness: Harness,
    issuer: Arc<DesktopTokenIssuer>,
    dir: String,
}

async fn env() -> Env {
    let rt = TestRuntime::new();
    let dir = rt.dir.path().join("wt");
    std::fs::create_dir_all(&dir).unwrap();
    let issuer = Arc::new(DesktopTokenIssuer::default());
    let harness = Harness::spawn_with(
        rt.runtime.clone() as Arc<dyn Workbench>,
        HarnessOptions {
            origins: vec![APP.into(), RELEASE.into(), "http://tauri.localhost".into()],
            extra_resolver: Some(issuer.clone()),
            ..HarnessOptions::default()
        },
    )
    .await;
    Env {
        rt,
        harness,
        issuer,
        dir: dir.to_string_lossy().into_owned(),
    }
}

impl Env {
    fn create_body(&self, key: &str) -> Value {
        serde_json::to_value(create_request(key, key, &self.dir)).unwrap()
    }

    async fn post(
        &self,
        path: &str,
        token: Option<&str>,
        headers: &[(header::HeaderName, &str)],
        body: &Value,
    ) -> reqwest::Response {
        let mut builder = self
            .harness
            .client()
            .post(format!("{}{path}", self.harness.base()))
            .json(body);
        if let Some(token) = token {
            builder = builder.bearer_auth(token);
        }
        for (name, value) in headers {
            builder = builder.header(name.clone(), *value);
        }
        builder.send().await.unwrap()
    }

    async fn project_count(&self) -> usize {
        let reply = self
            .harness
            .call(Some(TOKEN_DESKTOP), &list_request())
            .await
            .unwrap();
        reply.output().unwrap().as_array().unwrap().len()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hosts_and_origins_must_match_exactly() {
    let env = env().await;
    let body = env.create_body("k");
    for host in ["evil.example", "127.0.0.1.evil.example:1", "localhost:1"] {
        let response = env
            .post(
                "/v1/calls",
                Some(TOKEN_DESKTOP),
                &[(header::HOST, host)],
                &body,
            )
            .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "host {host}");
    }
    for origin in [
        "https://evil.example",
        "http://localhost:14200",
        "http://localhost:1420.evil.example",
        "HTTP://LOCALHOST:1420",
        "null",
        "",
    ] {
        for path in ["/v1/calls", "/v1/event-tickets", "/v1/system/handshake"] {
            let response = env
                .post(
                    path,
                    Some(TOKEN_DESKTOP),
                    &[(header::ORIGIN, origin)],
                    &body,
                )
                .await;
            assert_eq!(
                response.status(),
                StatusCode::FORBIDDEN,
                "{path} origin {origin:?}"
            );
        }
        // WebSocket upgrade도 Origin 검사가 먼저다.
        let ticket = env
            .harness
            .issue_ticket(TOKEN_DESKTOP, None, &Vec::<StreamCursor>::new())
            .await
            .unwrap();
        if !origin.is_empty() {
            assert_eq!(
                env.harness
                    .connect_ticket(&ticket, Some(origin))
                    .await
                    .err(),
                Some(403),
                "ws origin {origin:?}"
            );
        }
    }
    assert_eq!(env.project_count().await, 0, "nothing was created");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn credentials_are_required_and_bound() {
    let env = env().await;
    let body = env.create_body("k");
    assert_eq!(env.post("/v1/calls", None, &[], &body).await.status(), 401);
    assert_eq!(
        env.post("/v1/calls", Some("forged"), &[], &body)
            .await
            .status(),
        401
    );

    // 데스크톱 토큰은 발급한 WebView 출처에만 묶인다.
    let desktop = env
        .issuer
        .issue(TokenOrigin::WebView(APP.into()), Duration::from_secs(60));
    let other = env
        .post(
            "/v1/calls",
            Some(&desktop.token),
            &[(header::ORIGIN, RELEASE)],
            &body,
        )
        .await;
    assert_eq!(other.status(), 401, "other allowed origin");
    let absent = env
        .post("/v1/calls", Some(&desktop.token), &[], &body)
        .await;
    assert_eq!(absent.status(), 401, "origin stripped");
    let expired = env
        .issuer
        .issue(TokenOrigin::WebView(APP.into()), Duration::from_millis(0));
    tokio::time::sleep(Duration::from_millis(5)).await;
    let late = env
        .post(
            "/v1/calls",
            Some(&expired.token),
            &[(header::ORIGIN, APP)],
            &body,
        )
        .await;
    assert_eq!(late.status(), 401, "expired");
    // 진단용 무출처 토큰은 Origin이 붙으면 거절.
    let diagnostic = env
        .issuer
        .issue(TokenOrigin::NoOrigin, Duration::from_secs(60));
    let with_origin = env
        .post(
            "/v1/calls",
            Some(&diagnostic.token),
            &[(header::ORIGIN, APP)],
            &body,
        )
        .await;
    assert_eq!(
        with_origin.status(),
        401,
        "no-origin token used from a page"
    );
    assert_eq!(env.project_count().await, 0);

    // 올바른 출처에서는 통한다.
    let ok = env
        .post(
            "/v1/calls",
            Some(&desktop.token),
            &[(header::ORIGIN, APP)],
            &body,
        )
        .await;
    assert_eq!(ok.status(), 200);
    assert_eq!(env.project_count().await, 1);

    // 기록에 토큰 문자열이 없다.
    let lines = env.harness.access_log.lines();
    assert!(!lines.is_empty());
    for secret in [&desktop.token, &expired.token, &diagnostic.token] {
        assert!(
            lines.iter().all(|line| !line.contains(secret.as_str())),
            "token leaked"
        );
    }
    assert!(lines.iter().all(|line| !line.contains(TOKEN_DESKTOP)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversized_bodies_are_rejected() {
    let env = env().await;
    let mut body = env.create_body("big");
    body["input"]["description"] = json!("x".repeat(workbench_server::DEFAULT_BODY_LIMIT + 1));
    let response = env.post("/v1/calls", Some(TOKEN_DESKTOP), &[], &body).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(env.project_count().await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn preflight_answers_only_allowed_origins_without_credentials() {
    let env = env().await;
    let preflight = |origin: &'static str| {
        env.harness
            .client()
            .request(Method::OPTIONS, format!("{}/v1/calls", env.harness.base()))
            .header(header::ORIGIN, origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "authorization, content-type",
            )
            .send()
    };
    let allowed = preflight(APP).await.unwrap();
    assert!(allowed.status().is_success(), "{}", allowed.status());
    assert_eq!(
        allowed
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        APP
    );
    assert!(allowed
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
        .is_none());
    let methods = allowed
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_METHODS)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert!(
        methods.contains("POST") && methods.contains("GET"),
        "{methods}"
    );

    let denied = preflight("https://evil.example").await.unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert!(denied
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .is_none());

    let response = env
        .post(
            "/v1/calls",
            Some(TOKEN_DESKTOP),
            &[(header::ORIGIN, APP)],
            &serde_json::to_value(list_request()).unwrap(),
        )
        .await;
    assert_eq!(response.status(), 200);
    let expose = response
        .headers()
        .get(header::ACCESS_CONTROL_EXPOSE_HEADERS)
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(expose.contains("aw-protocol-version"), "{expose}");
}
