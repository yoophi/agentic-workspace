//! 042: Workbench HTTP/WS 어댑터(`workbench-server`) 조립. AW는 자기 `WorkbenchRuntime`을 `127.0.0.1:<임의 포트>`로
//! 연다(데스크톱 화면은 아직 Tauri 호환 command를 쓴다 — 4단계에서 전환). 허용 출처는 AW 창을 띄우는 WebView 출처뿐
//! 이다 — 개발(`devUrl`), macOS·Linux 배포(`tauri://localhost`), Windows 배포(`http://tauri.localhost`).
//!
//! 자격 증명: 데스크톱 WebView는 짧은 토큰(`get_workbench_connection`, 호출 창 출처에 묶임), agent는 MCP 실행 토큰
//! (run 하나 → `agent:<run>` principal, 폐기 즉시 무효). 종료: 새 호출 거절 → 받아들인 HTTP·MCP 호출 drain → 반환.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::sync::oneshot;
use workbench_protocol::{AuthenticatedPrincipal, Workbench};
use workbench_server::{
    ExposurePolicy, ServerConfig,
    access_log::StderrAccessLog,
    auth::{
        ChainResolver, CredentialResolver, DESKTOP_TOKEN_TTL, DesktopTokenIssuer, IssuedToken,
        TokenOrigin,
    },
    drain::{DEFAULT_DRAIN_WARN_AFTER, DetachedCalls},
    handshake::ServerInfo,
    origin::OriginPolicy,
    tickets::EventTicketStore,
};

use crate::infrastructure::mcp::capability_registry::CapabilityRegistry;

pub const WEBVIEW_ORIGINS: [&str; 3] = [
    "http://localhost:1420",
    "tauri://localhost",
    "http://tauri.localhost",
];

pub const MESSAGE_ORIGIN_NOT_ALLOWED: &str = "this window cannot connect to the Workbench server.";

pub fn origin_policy() -> OriginPolicy {
    OriginPolicy::new(WEBVIEW_ORIGINS)
}

/// MCP 실행 토큰 → run에 묶인 agent principal. agent는 비브라우저라 Origin이 있으면 받지 않는다. 폐기된 토큰은
/// 레지스트리에서 사라지므로 곧바로 거절된다.
pub struct McpCapabilityResolver {
    registry: CapabilityRegistry,
}

impl McpCapabilityResolver {
    pub fn new(registry: CapabilityRegistry) -> Self {
        Self { registry }
    }
}

impl CredentialResolver for McpCapabilityResolver {
    fn resolve(&self, bearer: &str, origin: Option<&str>) -> Option<AuthenticatedPrincipal> {
        if origin.is_some() {
            return None;
        }
        self.registry
            .resolve(bearer)
            .map(|principal| AuthenticatedPrincipal::agent(&principal.run_id))
    }
}

pub struct AwServerInfo {
    pub version: String,
    pub epoch: String,
}

impl ServerInfo for AwServerInfo {
    fn server_version(&self) -> String {
        self.version.clone()
    }
    fn server_epoch(&self) -> String {
        self.epoch.clone()
    }
    fn storage_schema_version(&self) -> i64 {
        workbench_core::infrastructure::sqlite_ledger::SCHEMA_VERSION
    }
}

/// 데스크톱 창에 돌려주는 연결 정보.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchConnection {
    pub base_url: String,
    pub token: String,
    pub expires_at: String,
}

/// 기동한 어댑터. 종료 신호와 `serve` 완료 신호를 쥔다.
pub struct WorkbenchHttpState {
    base_url: String,
    issuer: Arc<DesktopTokenIssuer>,
    shutdown: std::sync::Mutex<Option<oneshot::Sender<()>>>,
    /// `serve`가 끝나면 `true`. 여러 종료 경로가 함께 기다릴 수 있다.
    served: tokio::sync::watch::Receiver<bool>,
    /// 받아들인 HTTP 호출 추적기 — 종료 시작 때 곧바로 닫는다.
    http_calls: Arc<DetachedCalls>,
    drain_warn_after: Duration,
}

/// 조립 입력. 운영은 [`WorkbenchHttpState::start`]가 채운다. 시험은 짧은 경고 간격 등을 넣는다.
pub struct HttpAssembly {
    pub workbench: Arc<dyn Workbench>,
    pub mcp_registry: CapabilityRegistry,
    pub server_info: AwServerInfo,
    pub drain_warn_after: Duration,
}

impl WorkbenchHttpState {
    /// 루프백 임의 포트에 bind하고 현재 tokio 런타임에서 `serve`를 띄운다.
    pub fn start(assembly: HttpAssembly) -> Result<Self> {
        let std_listener = std::net::TcpListener::bind(("127.0.0.1", 0))
            .context("failed to bind the Workbench HTTP server to localhost")?;
        std_listener
            .set_nonblocking(true)
            .context("failed to configure the Workbench HTTP listener")?;
        let address = std_listener
            .local_addr()
            .context("failed to read the Workbench HTTP address")?;
        let issuer = Arc::new(DesktopTokenIssuer::default());
        let resolver = ChainResolver::new(vec![
            issuer.clone() as Arc<dyn CredentialResolver>,
            Arc::new(McpCapabilityResolver::new(assembly.mcp_registry)),
        ]);
        let config = ServerConfig {
            resolver: Arc::new(resolver),
            server_info: Arc::new(assembly.server_info),
            origins: origin_policy(),
            access_log: Arc::new(StderrAccessLog),
            exposure: ExposurePolicy::network_default(),
            tickets: Arc::new(EventTicketStore::default()),
            body_limit: workbench_server::DEFAULT_BODY_LIMIT,
            drain_warn_after: assembly.drain_warn_after,
            body_read_timeout: workbench_server::DEFAULT_BODY_READ_TIMEOUT,
            connection_grace: workbench_server::DEFAULT_CONNECTION_GRACE,
        };
        let server = workbench_server::build_router(assembly.workbench, config, address.port());
        let http_calls = Arc::clone(&server.calls);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (served_tx, served_rx) = tokio::sync::watch::channel(false);
        tauri::async_runtime::spawn(async move {
            match tokio::net::TcpListener::from_std(std_listener) {
                Ok(listener) => {
                    if let Err(error) = workbench_server::serve(listener, server, async {
                        let _ = shutdown_rx.await;
                    })
                    .await
                    {
                        eprintln!("[workbench-http] server stopped: {error}");
                    }
                }
                Err(error) => eprintln!("[workbench-http] failed to create listener: {error}"),
            }
            let _ = served_tx.send(true);
        });
        Ok(Self {
            base_url: format!("http://{address}"),
            issuer,
            shutdown: std::sync::Mutex::new(Some(shutdown_tx)),
            served: served_rx,
            http_calls,
            drain_warn_after: assembly.drain_warn_after,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 호출한 창의 WebView 출처에 묶인 짧은 토큰. 허용 출처가 아니면 거절한다.
    pub fn connection_for(&self, origin: &str) -> Result<WorkbenchConnection, String> {
        if !WEBVIEW_ORIGINS.contains(&origin) {
            return Err(MESSAGE_ORIGIN_NOT_ALLOWED.to_owned());
        }
        Ok(self.connection(
            self.issuer
                .issue(TokenOrigin::WebView(origin.to_owned()), DESKTOP_TOKEN_TTL),
        ))
    }

    /// debug 스모크 전용(R12): 운영 발급기의 "Origin 없음" 토큰 — 브라우저 밖 진단 클라이언트용.
    #[cfg(debug_assertions)]
    pub fn diagnostic_connection(&self) -> WorkbenchConnection {
        self.connection(self.issuer.issue(
            TokenOrigin::NoOrigin,
            workbench_server::auth::DIAGNOSTIC_TOKEN_TTL,
        ))
    }

    fn connection(&self, issued: IssuedToken) -> WorkbenchConnection {
        WorkbenchConnection {
            base_url: self.base_url.clone(),
            token: issued.token,
            expires_at: issued.expires_at.to_rfc3339(),
        }
    }

    /// 종료(research R17, T033): **먼저 HTTP·MCP 양쪽의 새 호출 수락을 닫고**(종료 중 들어온 변경은 `503`), 그다음
    /// 종료 신호 → `serve` 완료(받아들인 HTTP 호출 drain) → 받아들인 MCP 도구 호출 drain. 상한 없이 기다리고 경고
    /// 간격마다 남은 수를 기록한다. 소유자(앱 종료 경로)는 이 future가 끝난 뒤에 종료한다.
    pub async fn shutdown(&self, mcp_calls: &DetachedCalls) {
        self.http_calls.close();
        mcp_calls.close();
        if let Some(tx) = self
            .shutdown
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = tx.send(());
        }
        let mut served = self.served.clone();
        while !*served.borrow_and_update() {
            if served.changed().await.is_err() {
                break;
            }
        }
        drain_mcp(mcp_calls, self.drain_warn_after).await;
    }
}

/// HTTP 어댑터가 없을 때(기동 실패)도 MCP 도구 호출은 drain한다.
pub async fn drain_mcp(mcp_calls: &DetachedCalls, warn_after: Duration) {
    mcp_calls.close();
    mcp_calls
        .drain_until_idle(warn_after, |left| {
            eprintln!("[workbench-http] exit waiting for {left} MCP tool call(s) to finish");
        })
        .await;
}

pub fn default_drain_warn_after() -> Duration {
    DEFAULT_DRAIN_WARN_AFTER
}

/// WebView URL → 출처 문자열(`scheme://host[:port]`). 사용자 정의 scheme(`tauri:`)은 표준 origin 직렬화가 `null`이라
/// 직접 만든다.
pub fn origin_of(url: &tauri::Url) -> Option<String> {
    let host = url.host_str()?;
    Some(match url.port() {
        Some(port) => format!("{}://{host}:{port}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    })
}

/// 앱이 관리하는 어댑터 상태. 기동 실패면 `state`가 없고(FR-016: 앱은 계속 동작) 이유를 남긴다.
pub struct WorkbenchHttp {
    pub state: Option<Arc<WorkbenchHttpState>>,
    pub start_error: Option<String>,
    pub exit: ExitGate,
}

impl WorkbenchHttp {
    pub fn connection_for(&self, origin: &str) -> Result<WorkbenchConnection, String> {
        match &self.state {
            Some(state) => state.connection_for(origin),
            None => Err(format!(
                "Workbench HTTP server is not running: {}",
                self.start_error.as_deref().unwrap_or("not started")
            )),
        }
    }
}

/// 앱 종료 단계(T033). 첫 종료 요청은 종료를 미루고 drain을 시작한다. drain 중 요청은 계속 미룬다. drain이 끝나
/// 앱이 스스로 다시 종료를 요청하면 그때 통과시킨다.
#[derive(Debug, Default)]
pub struct ExitGate {
    phase: std::sync::atomic::AtomicU8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitDecision {
    /// 종료를 미루고 drain을 시작한다.
    StartDrain,
    /// drain 중이다 — 종료를 미룬다.
    KeepWaiting,
    /// drain이 끝났다 — 종료한다.
    Exit,
}

const EXIT_IDLE: u8 = 0;
const EXIT_DRAINING: u8 = 1;
const EXIT_DONE: u8 = 2;

impl ExitGate {
    pub fn on_exit_requested(&self) -> ExitDecision {
        use std::sync::atomic::Ordering;
        match self.phase.compare_exchange(
            EXIT_IDLE,
            EXIT_DRAINING,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => ExitDecision::StartDrain,
            Err(EXIT_DONE) => ExitDecision::Exit,
            Err(_) => ExitDecision::KeepWaiting,
        }
    }

    pub fn is_drained(&self) -> bool {
        self.phase.load(std::sync::atomic::Ordering::Acquire) == EXIT_DONE
    }

    pub fn drained(&self) {
        self.phase
            .store(EXIT_DONE, std::sync::atomic::Ordering::Release);
    }
}

/// 종료 drain: HTTP 어댑터가 있으면 그 종료(HTTP drain → MCP drain), 없으면 MCP drain만.
pub async fn drain_for_exit(http: Option<Arc<WorkbenchHttpState>>, mcp_calls: Arc<DetachedCalls>) {
    match http {
        Some(state) => state.shutdown(&mcp_calls).await,
        None => drain_mcp(&mcp_calls, DEFAULT_DRAIN_WARN_AFTER).await,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use async_trait::async_trait;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use workbench_protocol::{CallReply, CallRequest, EventStream, Subscription, WorkbenchFault};

    use super::*;

    #[test]
    fn mcp_tokens_resolve_to_their_run_and_stop_when_revoked() {
        let registry = CapabilityRegistry::default();
        let token = registry.issue("run-1");
        let resolver = McpCapabilityResolver::new(registry.clone());
        assert_eq!(
            resolver.resolve(&token, None),
            Some(AuthenticatedPrincipal::agent("run-1"))
        );
        assert_eq!(
            resolver.resolve(&token, Some("http://localhost:1420")),
            None,
            "agents are not browsers"
        );
        assert_eq!(resolver.resolve("awcap_forged", None), None);
        registry.revoke_run("run-1");
        assert_eq!(
            resolver.resolve(&token, None),
            None,
            "revocation is immediate"
        );
    }

    #[test]
    fn desktop_tokens_are_bound_to_the_issuing_window_origin() {
        let issuer = Arc::new(DesktopTokenIssuer::default());
        let chain = ChainResolver::new(vec![
            issuer.clone() as Arc<dyn CredentialResolver>,
            Arc::new(McpCapabilityResolver::new(CapabilityRegistry::default())),
        ]);
        let issued = issuer.issue(
            TokenOrigin::WebView("tauri://localhost".into()),
            DESKTOP_TOKEN_TTL,
        );
        assert_eq!(
            chain.resolve(&issued.token, Some("tauri://localhost")),
            Some(AuthenticatedPrincipal::desktop())
        );
        assert_eq!(
            chain.resolve(&issued.token, Some("http://localhost:1420")),
            None
        );
        assert_eq!(chain.resolve(&issued.token, None), None);
    }

    #[test]
    fn exit_gate_defers_until_drained() {
        let gate = ExitGate::default();
        assert_eq!(gate.on_exit_requested(), ExitDecision::StartDrain);
        assert_eq!(gate.on_exit_requested(), ExitDecision::KeepWaiting);
        assert_eq!(gate.on_exit_requested(), ExitDecision::KeepWaiting);
        gate.drained();
        assert_eq!(gate.on_exit_requested(), ExitDecision::Exit);
    }

    #[test]
    fn a_failed_start_still_answers_with_a_reason() {
        let http = WorkbenchHttp {
            state: None,
            start_error: Some("address in use".into()),
            exit: ExitGate::default(),
        };
        let error = http.connection_for("tauri://localhost").unwrap_err();
        assert!(error.contains("address in use"), "{error}");
    }

    #[test]
    fn window_origins_include_custom_schemes() {
        let url = tauri::Url::parse("tauri://localhost/index.html").unwrap();
        assert_eq!(origin_of(&url).as_deref(), Some("tauri://localhost"));
        let url = tauri::Url::parse("http://localhost:1420/#/session").unwrap();
        assert_eq!(origin_of(&url).as_deref(), Some("http://localhost:1420"));
        let url = tauri::Url::parse("http://tauri.localhost/").unwrap();
        assert_eq!(origin_of(&url).as_deref(), Some("http://tauri.localhost"));
    }

    /// 호출 하나를 받아들이면 `entered`를 알리고, `delay` 뒤 효과(`finished`)를 낸다.
    struct SlowWorkbench {
        entered: Arc<tokio::sync::Notify>,
        finished: Arc<AtomicBool>,
        delay: Duration,
    }

    #[async_trait]
    impl Workbench for SlowWorkbench {
        async fn call(
            &self,
            _principal: AuthenticatedPrincipal,
            request: CallRequest,
        ) -> Result<CallReply, WorkbenchFault> {
            self.entered.notify_one();
            tokio::time::sleep(self.delay).await;
            self.finished.store(true, Ordering::Release);
            let _ = request;
            Ok(CallReply::complete(serde_json::Value::Null, None))
        }

        fn events(
            &self,
            _principal: AuthenticatedPrincipal,
            _request: Subscription,
        ) -> Result<EventStream, WorkbenchFault> {
            Ok(EventStream::new(futures_util::stream::empty()))
        }
    }

    fn slow(
        delay: Duration,
    ) -> (
        Arc<SlowWorkbench>,
        Arc<tokio::sync::Notify>,
        Arc<AtomicBool>,
    ) {
        let entered = Arc::new(tokio::sync::Notify::new());
        let finished = Arc::new(AtomicBool::new(false));
        (
            Arc::new(SlowWorkbench {
                entered: entered.clone(),
                finished: finished.clone(),
                delay,
            }),
            entered,
            finished,
        )
    }

    /// T033: 앱 종료 경로(`shutdown`)는 연결이 이미 끊긴 받아들인 HTTP 호출(700ms)과 진행 중 MCP 호출(300ms)이
    /// 끝나기 전에 끝나지 않고, 종료를 시작하자마자 MCP 새 호출은 거절된다(HTTP drain이 더 오래 걸려도). 지연은
    /// 경고 간격(20ms)보다 길다.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn exit_waits_for_accepted_http_and_mcp_calls() {
        let (workbench, entered, finished) = slow(Duration::from_millis(700));
        let state = WorkbenchHttpState::start(HttpAssembly {
            workbench,
            mcp_registry: CapabilityRegistry::default(),
            server_info: AwServerInfo {
                version: "test".into(),
                epoch: "e".into(),
            },
            drain_warn_after: Duration::from_millis(20),
        })
        .unwrap();
        let origin = "http://localhost:1420";
        let connection = state.connection_for(origin).unwrap();
        assert!(state.connection_for("https://evil.example").is_err());
        let address = connection.base_url.trim_start_matches("http://").to_owned();

        // 받아들인 HTTP 호출: 요청을 보내고 효과 진입을 확인한 뒤 연결을 끊는다.
        let body = serde_json::to_vec(&CallRequest::query(
            workbench_protocol::OperationId::ProjectList,
            serde_json::json!({}),
        ))
        .unwrap();
        let mut stream = tokio::net::TcpStream::connect(&address).await.unwrap();
        let head = format!(
            "POST /v1/calls HTTP/1.1\r\nHost: {address}\r\nOrigin: {origin}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            connection.token,
            body.len()
        );
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.write_all(&body).await.unwrap();
        entered.notified().await;
        drop(stream);

        // 진행 중 MCP 도구 호출(운영 handler와 같은 추적기).
        let mcp_calls = Arc::new(DetachedCalls::default());
        let mcp_finished = Arc::new(AtomicBool::new(false));
        let guard = mcp_calls.accept().unwrap();
        {
            let mcp_finished = mcp_finished.clone();
            tokio::spawn(workbench_server::drain::spawn_accepted(guard, async move {
                tokio::time::sleep(Duration::from_millis(300)).await;
                mcp_finished.store(true, Ordering::Release);
            }));
        }

        let state = Arc::new(state);
        let exiting = {
            let (state, mcp_calls) = (state.clone(), mcp_calls.clone());
            tokio::spawn(async move { state.shutdown(&mcp_calls).await })
        };
        // 두 번째 종료 경로(macOS `RunEvent::Exit`)가 겹쳐도 같은 끝을 기다린다.
        let exiting_again = {
            let (state, mcp_calls) = (state.clone(), mcp_calls.clone());
            tokio::spawn(async move { state.shutdown(&mcp_calls).await })
        };
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            !exiting.is_finished(),
            "exit completed while calls were running"
        );
        // HTTP drain이 아직 진행 중인데도 MCP는 이미 새 호출을 받지 않는다.
        assert!(
            mcp_calls.accept().is_none(),
            "MCP accepted a new call while the HTTP drain was still running"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(
            mcp_finished.load(Ordering::Acquire),
            "the earlier MCP call ran to completion"
        );
        assert!(!exiting.is_finished(), "HTTP call (700ms) still draining");
        assert!(
            !exiting_again.is_finished(),
            "a second exit path returned early"
        );
        exiting.await.unwrap();
        exiting_again.await.unwrap();
        assert!(
            finished.load(Ordering::Acquire),
            "the disconnected HTTP call finished"
        );
        assert!(
            mcp_finished.load(Ordering::Acquire),
            "the MCP call finished"
        );
        assert!(mcp_calls.accept().is_none(), "no new MCP calls after exit");

        // 서버는 내려갔다.
        let mut probe = tokio::net::TcpStream::connect(&address).await;
        if let Ok(stream) = probe.as_mut() {
            let mut buffer = [0u8; 1];
            let read =
                tokio::time::timeout(Duration::from_millis(200), stream.read(&mut buffer)).await;
            assert!(!matches!(read, Ok(Ok(1))), "server still answering");
        }
    }
}
