//! 테스트 전용 loopback HTTP 경로: `POST /v1/calls`. 운영 코드에 포함되지 않는다.
//! 계약: `specs/037-workbench-seam/contracts/workbench-call.md` §4.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use tokio::sync::oneshot;
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, CallRequest, Workbench, WorkbenchFault,
};

pub const TOKEN_DESKTOP: &str = "test-desktop";
pub const TOKEN_READONLY: &str = "test-readonly";

fn principal_from_headers(headers: &HeaderMap) -> Option<AuthenticatedPrincipal> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    match token {
        TOKEN_DESKTOP => Some(AuthenticatedPrincipal::desktop()),
        TOKEN_READONLY => Some(AuthenticatedPrincipal::test_readonly()),
        _ => None,
    }
}

fn problem_response(fault: &WorkbenchFault) -> Response {
    let status = fault.code.http_status();
    let mut body = serde_json::to_value(fault).expect("fault json");
    body["type"] = serde_json::Value::String(format!("urn:aw:fault:{}", fault.code.as_str()));
    body["title"] = serde_json::Value::String(fault.code.as_str().to_owned());
    body["status"] = serde_json::Value::from(status);
    Response::builder()
        .status(StatusCode::from_u16(status).expect("valid status"))
        .header(header::CONTENT_TYPE, "application/problem+json")
        .body(Body::from(serde_json::to_vec(&body).expect("body")))
        .expect("response")
}

async fn call_handler(
    State(workbench): State<Arc<dyn Workbench>>,
    headers: HeaderMap,
    Json(request): Json<CallRequest>,
) -> Response {
    let Some(principal) = principal_from_headers(&headers) else {
        return problem_response(&WorkbenchFault::unauthenticated(request.request_id));
    };
    match workbench.call(principal, request).await {
        Ok(reply) => (StatusCode::OK, Json(reply)).into_response(),
        Err(fault) => problem_response(&fault),
    }
}

pub fn router(workbench: Arc<dyn Workbench>) -> Router {
    Router::new()
        .route("/v1/calls", post(call_handler))
        .with_state(workbench)
}

pub struct Harness {
    pub addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    client: reqwest::Client,
}

impl Harness {
    pub async fn spawn(workbench: Arc<dyn Workbench>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = oneshot::channel::<()>();
        let app = router(workbench);
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .expect("serve");
        });
        Self {
            addr,
            shutdown: Some(tx),
            client: reqwest::Client::new(),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}/v1/calls", self.addr)
    }

    /// 실제 loopback 왕복. 200이면 `CallReply`, 아니면 problem body를 `WorkbenchFault`로 읽는다.
    pub async fn call(
        &self,
        token: Option<&str>,
        request: &CallRequest,
    ) -> Result<CallReply, WorkbenchFault> {
        let mut builder = self.client.post(self.url()).json(request);
        if let Some(token) = token {
            builder = builder.bearer_auth(token);
        }
        let response = builder.send().await.expect("http send");
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        let body: serde_json::Value = response.json().await.expect("json body");
        if status.is_success() {
            assert!(
                content_type.starts_with("application/json"),
                "{content_type}"
            );
            Ok(serde_json::from_value(body).expect("CallReply"))
        } else {
            assert!(
                content_type.starts_with("application/problem+json"),
                "{content_type}"
            );
            let fault: WorkbenchFault =
                serde_json::from_value(body.clone()).expect("WorkbenchFault");
            assert_eq!(
                u16::from(status),
                fault.code.http_status(),
                "status/code mismatch: {body}"
            );
            Err(fault)
        }
    }

    pub fn token_for(principal: &AuthenticatedPrincipal) -> &'static str {
        if *principal == AuthenticatedPrincipal::desktop() {
            TOKEN_DESKTOP
        } else {
            TOKEN_READONLY
        }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}
