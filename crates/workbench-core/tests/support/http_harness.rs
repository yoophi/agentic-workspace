//! 운영 router(`workbench-server`, 042)를 임의 루프백 포트에 띄우는 테스트 래퍼. 테스트 전용인 것은 자격 증명
//! (고정 토큰 resolver)·서버 정보·기록 수집뿐이고, 경로·인증 판정·표·problem 형식은 운영 코드 그대로다.
//! 계약: `specs/042-workbench-http/contracts/workbench-http.md`.

use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::http::header;
use futures_util::StreamExt;
use tokio::{sync::oneshot, task::JoinHandle};
use workbench_protocol::{
    events::EventFrame, AuthenticatedPrincipal, CallReply, CallRequest, PrincipalKind,
    StreamCursor, Workbench, WorkbenchFault,
};
use workbench_server::{
    access_log::CollectingAccessLog,
    auth::StaticResolver,
    handshake::ServerInfo,
    origin::OriginPolicy,
    tickets::{EventTicketStore, TICKET_CAPACITY, TICKET_TTL},
    ExposurePolicy, ServerConfig, DEFAULT_BODY_LIMIT,
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

fn test_resolver() -> StaticResolver {
    StaticResolver::new([
        (TOKEN_DESKTOP.to_owned(), AuthenticatedPrincipal::desktop()),
        (
            TOKEN_READONLY.to_owned(),
            AuthenticatedPrincipal::test_readonly(),
        ),
        (TOKEN_NOSCOPE.to_owned(), noscope_principal()),
        (
            TOKEN_DESKTOP2.to_owned(),
            AuthenticatedPrincipal::test_as("desktop2"),
        ),
    ])
    .with_agent_prefix(TOKEN_AGENT_PREFIX)
}

struct TestServerInfo;

impl ServerInfo for TestServerInfo {
    fn server_version(&self) -> String {
        "test".to_owned()
    }
    fn server_epoch(&self) -> String {
        "test-epoch".to_owned()
    }
    fn storage_schema_version(&self) -> i64 {
        2
    }
}

/// 운영 조립과 다른 테스트 설정.
pub struct HarnessOptions {
    pub origins: Vec<String>,
    pub exposure: ExposurePolicy,
    pub ticket_ttl: Duration,
    pub drain_warn_after: Duration,
}

impl Default for HarnessOptions {
    fn default() -> Self {
        Self {
            origins: Vec::new(),
            exposure: ExposurePolicy::All,
            ticket_ttl: TICKET_TTL,
            drain_warn_after: Duration::from_secs(30),
        }
    }
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
    pub async fn next_frame(&mut self, wait: Duration) -> Option<EventFrame> {
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
    served: Option<JoinHandle<std::io::Result<()>>>,
    client: reqwest::Client,
    pub access_log: Arc<CollectingAccessLog>,
    /// 받아들인 분리 호출 추적기(종료 수명 시험용).
    pub calls: Arc<workbench_server::drain::DetachedCalls>,
}

impl Harness {
    pub async fn spawn(workbench: Arc<dyn Workbench>) -> Self {
        Self::spawn_with(workbench, HarnessOptions::default()).await
    }

    pub async fn spawn_with(workbench: Arc<dyn Workbench>, options: HarnessOptions) -> Self {
        let listener = workbench_server::bind_loopback()
            .await
            .expect("bind loopback");
        let addr = listener.local_addr().expect("addr");
        let access_log = Arc::new(CollectingAccessLog::default());
        let config = ServerConfig {
            resolver: Arc::new(test_resolver()),
            server_info: Arc::new(TestServerInfo),
            origins: OriginPolicy::new(options.origins),
            access_log: access_log.clone(),
            exposure: options.exposure,
            tickets: Arc::new(EventTicketStore::new(options.ticket_ttl, TICKET_CAPACITY)),
            body_limit: DEFAULT_BODY_LIMIT,
            drain_warn_after: options.drain_warn_after,
        };
        let server = workbench_server::build_router(workbench, config, addr.port());
        let calls = Arc::clone(&server.calls);
        let (tx, rx) = oneshot::channel::<()>();
        let served = tokio::spawn(workbench_server::serve(listener, server, async {
            let _ = rx.await;
        }));
        Self {
            addr,
            shutdown: Some(tx),
            served: Some(served),
            client: reqwest::Client::new(),
            access_log,
            calls,
        }
    }

    pub fn base(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn url(&self) -> String {
        format!("{}/v1/calls", self.base())
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// 종료 신호를 보내고 `serve` 반환(받아들인 분리 호출 drain 포함)까지 기다린다.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(served) = self.served.take() {
            served.await.expect("serve task").expect("serve");
        }
    }

    /// 종료 신호만 보낸다. `serve` 완료를 기다리는 handle을 돌려준다.
    pub fn begin_shutdown(&mut self) -> JoinHandle<std::io::Result<()>> {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        self.served.take().expect("serve task")
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
        read_reply(response).await
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

    /// `POST /v1/event-tickets`. 성공이면 표, 아니면 problem.
    pub async fn issue_ticket(
        &self,
        token: &str,
        origin: Option<&str>,
        cursors: &[StreamCursor],
    ) -> Result<String, WorkbenchFault> {
        let mut builder = self
            .client
            .post(format!("{}/v1/event-tickets", self.base()))
            .bearer_auth(token)
            .json(&serde_json::json!({ "cursors": cursors }));
        if let Some(origin) = origin {
            builder = builder.header(header::ORIGIN, origin);
        }
        let response = builder.send().await.expect("http send");
        let status = response.status();
        let body: serde_json::Value = response.json().await.expect("json body");
        if status.is_success() {
            Ok(body["ticket"].as_str().expect("ticket").to_owned())
        } else {
            Err(serde_json::from_value(body).expect("WorkbenchFault"))
        }
    }

    /// 표로 `GET /v1/events` 연결. upgrade 거절이면 HTTP 상태를 돌려준다.
    pub async fn connect_ticket(
        &self,
        ticket: &str,
        origin: Option<&str>,
    ) -> Result<WsSubscription, u16> {
        use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Error};
        let mut request = format!("ws://{}/v1/events?ticket={ticket}", self.addr)
            .into_client_request()
            .expect("ws request");
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert(header::ORIGIN, origin.parse().expect("origin header"));
        }
        let mut socket = match tokio_tungstenite::connect_async(request).await {
            Ok((socket, _)) => socket,
            Err(Error::Http(response)) => return Err(response.status().as_u16()),
            Err(other) => panic!("ws connect: {other}"),
        };
        let hello = match socket.next().await.expect("hello").expect("hello frame") {
            tokio_tungstenite::tungstenite::Message::Text(text) => {
                serde_json::from_str(&text).expect("hello json")
            }
            other => panic!("unexpected first frame {other:?}"),
        };
        Ok(WsSubscription { socket, hello })
    }

    /// 표 발급 → 연결 → hello. 구독 판정 결과(fault 또는 event)는 이어지는 프레임으로 온다.
    pub async fn subscribe(&self, token: &str, cursors: Vec<StreamCursor>) -> WsSubscription {
        let ticket = self
            .issue_ticket(token, None, &cursors)
            .await
            .unwrap_or_else(|fault| panic!("ticket issuance failed: {fault:?}"));
        self.connect_ticket(&ticket, None)
            .await
            .unwrap_or_else(|status| panic!("ws upgrade rejected: {status}"))
    }
}

impl Harness {
    /// 요청을 보내고, `entered`(그 요청의 효과가 난 뒤·결과 기록 전 구간 진입 확인)가 끝난 뒤 응답을 읽지 않고
    /// 연결을 끊는다(클라이언트 단절). 시간 대기에 의존하지 않는다.
    pub async fn send_then_disconnect<F: std::future::Future>(
        &self,
        token: &str,
        request: &CallRequest,
        entered: F,
    ) -> F::Output {
        use tokio::io::AsyncWriteExt;
        let body = serde_json::to_vec(request).expect("json");
        let head = format!(
            "POST /v1/calls HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            self.addr,
            body.len()
        );
        let mut stream = tokio::net::TcpStream::connect(self.addr)
            .await
            .expect("connect");
        stream.write_all(head.as_bytes()).await.expect("head");
        stream.write_all(&body).await.expect("body");
        stream.flush().await.expect("flush");
        let output = entered.await;
        drop(stream);
        output
    }
}

/// problem 형식 검사까지 포함한 응답 해석.
pub async fn read_reply(response: reqwest::Response) -> Result<CallReply, WorkbenchFault> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        response
            .headers()
            .contains_key(workbench_server::PROTOCOL_HEADER),
        "missing protocol header"
    );
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
        let fault: WorkbenchFault = serde_json::from_value(body.clone()).expect("WorkbenchFault");
        assert_eq!(
            u16::from(status),
            fault.code.http_status(),
            "status/code mismatch: {body}"
        );
        Err(fault)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}
