//! 테스트 전용 loopback HTTP 경로: `POST /v1/calls`와 `GET /v1/events`(WebSocket, 039). 운영 코드에 포함되지 않는다.
//! 계약: `specs/037-workbench-seam/contracts/workbench-call.md` §4, `specs/039-workbench-events/contracts/workbench-events.md` §6.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::oneshot;
use workbench_protocol::{
    events::EventFrame, AuthenticatedPrincipal, CallReply, CallRequest, EventItem, OperationId,
    PrincipalKind, Subscription, Workbench, WorkbenchFault, PROTOCOL_VERSION,
};

pub const TOKEN_DESKTOP: &str = "test-desktop";
pub const TOKEN_READONLY: &str = "test-readonly";
/// scope가 하나도 없는 호출자(039 이벤트 권한 거절 fixture).
pub const TOKEN_NOSCOPE: &str = "test-noscope";
/// 040: 데스크톱과 같은 scope의 다른 주체.
pub const TOKEN_DESKTOP2: &str = "test-desktop2";
/// 040: `test-agent:<runId>` → run에 묶인 agent principal.
pub const TOKEN_AGENT_PREFIX: &str = "test-agent:";

pub fn noscope_principal() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal::new(PrincipalKind::Desktop, [])
}

fn principal_from_headers(headers: &HeaderMap) -> Option<AuthenticatedPrincipal> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    match token {
        TOKEN_DESKTOP => Some(AuthenticatedPrincipal::desktop()),
        TOKEN_READONLY => Some(AuthenticatedPrincipal::test_readonly()),
        TOKEN_NOSCOPE => Some(noscope_principal()),
        TOKEN_DESKTOP2 => Some(AuthenticatedPrincipal::test_as("desktop2")),
        other => other
            .strip_prefix(TOKEN_AGENT_PREFIX)
            .map(AuthenticatedPrincipal::agent),
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

async fn events_handler(
    State(workbench): State<Arc<dyn Workbench>>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    let Some(principal) = principal_from_headers(&headers) else {
        return problem_response(&WorkbenchFault::unauthenticated(
            workbench_protocol::RequestId::random(),
        ));
    };
    upgrade.on_upgrade(move |socket| serve_events(workbench, principal, socket))
}

async fn send_frame(socket: &mut WebSocket, frame: &EventFrame) -> bool {
    let text = serde_json::to_string(frame).expect("frame json");
    socket.send(Message::Text(text)).await.is_ok()
}

/// hello → subscribe 한 번 → event/gap 프레임. 연결 종료 = 구독 해제(스트림 drop).
async fn serve_events(
    workbench: Arc<dyn Workbench>,
    principal: AuthenticatedPrincipal,
    mut socket: WebSocket,
) {
    // 세대는 Workbench trait만으로 얻는다(system.describe).
    let describe = workbench
        .call(
            principal.clone(),
            CallRequest::query(OperationId::SystemDescribe, serde_json::json!({})),
        )
        .await;
    let epoch = describe
        .ok()
        .and_then(|reply| {
            reply
                .output()
                .and_then(|out| out["epoch"].as_str().map(str::to_owned))
        })
        .unwrap_or_default();
    if !send_frame(
        &mut socket,
        &EventFrame::Hello {
            protocol_version: PROTOCOL_VERSION,
            epoch,
        },
    )
    .await
    {
        return;
    }
    let cursors = match socket.recv().await {
        Some(Ok(Message::Text(text))) => match serde_json::from_str::<EventFrame>(&text) {
            Ok(EventFrame::Subscribe { cursors }) => cursors,
            _ => {
                let _ = socket.send(Message::Close(None)).await;
                return;
            }
        },
        _ => return,
    };
    let mut stream = match workbench.events(principal, Subscription { cursors }) {
        Ok(stream) => stream,
        Err(fault) => {
            let _ = send_frame(&mut socket, &EventFrame::Fault { fault }).await;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    loop {
        tokio::select! {
            item = stream.next() => {
                let Some(item) = item else { break };
                let frame = match item {
                    EventItem::Event { event } => EventFrame::Event { event },
                    EventItem::Gap { gap } => EventFrame::Gap { gap },
                };
                if !send_frame(&mut socket, &frame).await {
                    break;
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}

pub fn router(workbench: Arc<dyn Workbench>) -> Router {
    Router::new()
        .route("/v1/calls", post(call_handler))
        .route("/v1/events", get(events_handler))
        .with_state(workbench)
}

/// 테스트 WebSocket 구독 클라이언트.
pub struct WsSubscription {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    pub hello: EventFrame,
}

impl WsSubscription {
    /// 다음 프레임. 연결이 닫혔거나 시간 안에 오지 않으면 `None`.
    pub async fn next_frame(&mut self, wait: std::time::Duration) -> Option<EventFrame> {
        loop {
            let message = tokio::time::timeout(wait, self.socket.next())
                .await
                .ok()??
                .ok()?;
            match message {
                tokio_tungstenite::tungstenite::Message::Text(text) => {
                    return Some(serde_json::from_str(&text).expect("frame json"));
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => return None,
                _ => continue,
            }
        }
    }
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

    /// 040: 주체까지 구별하는 토큰(`desktop2`, `agent:<runId>` 포함).
    pub fn token_string(principal: &AuthenticatedPrincipal) -> String {
        if let Some(run_id) = principal.agent_run_id() {
            format!("{TOKEN_AGENT_PREFIX}{run_id}")
        } else if *principal == AuthenticatedPrincipal::test_as("desktop2") {
            TOKEN_DESKTOP2.to_owned()
        } else {
            Self::token_for(principal).to_owned()
        }
    }

    pub fn token_for(principal: &AuthenticatedPrincipal) -> &'static str {
        if *principal == AuthenticatedPrincipal::desktop() {
            TOKEN_DESKTOP
        } else if *principal == noscope_principal() {
            TOKEN_NOSCOPE
        } else {
            TOKEN_READONLY
        }
    }

    /// `GET /v1/events` WebSocket 구독. hello를 받고 subscribe 프레임을 보낸 뒤 돌려준다.
    pub async fn subscribe(
        &self,
        token: &str,
        cursors: Vec<workbench_protocol::StreamCursor>,
    ) -> WsSubscription {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut request = format!("ws://{}/v1/events", self.addr)
            .into_client_request()
            .expect("ws request");
        request.headers_mut().insert(
            header::AUTHORIZATION,
            format!("Bearer {token}").parse().expect("header"),
        );
        let (mut socket, _) = tokio_tungstenite::connect_async(request)
            .await
            .expect("ws connect");
        let hello = match socket.next().await.expect("hello").expect("hello frame") {
            tokio_tungstenite::tungstenite::Message::Text(text) => {
                serde_json::from_str(&text).expect("hello json")
            }
            other => panic!("unexpected first frame {other:?}"),
        };
        let subscribe = serde_json::to_string(&EventFrame::Subscribe { cursors }).expect("json");
        socket
            .send(tokio_tungstenite::tungstenite::Message::Text(subscribe))
            .await
            .expect("send subscribe");
        WsSubscription { socket, hello }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}
