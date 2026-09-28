//! 042: Workbench HTTP/WS 어댑터(`workbench-server`) 조립(044: AW `src-tauri`에서 host crate로 옮김 — Tauri와 무관).
//! 런타임을 `127.0.0.1:<임의 포트>`로 연다. 허용 출처는 AW 창을 띄우는 WebView 출처뿐이다 — 개발(`devUrl`),
//! macOS·Linux 배포(`tauri://localhost`), Windows 배포(`http://tauri.localhost`).
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

use workbench_core::application::work_gate::GateState;
use workbench_core::ports::server_host::{ServerHost, WindowToken, WindowTokenError};

use crate::{
    lifecycle::identity::{OwnerIdentity, OwnerResolver},
    mcp::capability_registry::CapabilityRegistry,
};

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
    /// 043: 토큰이 묶인 창 incarnation(창 토큰일 때). 전달 선언·재연결이 같은 창인지 확인하는 데 쓴다.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incarnation: Option<String>,
}

/// core `ServerHost` 구현(044 T026). 창 토큰은 WebView 허용 출처에만, 폐기 tombstone 확인과 함께 발급기 잠금 아래에서
/// 발급한다. 폐기는 토큰과 이벤트 표 양쪽에 tombstone을 세운다(Codex 설계 리뷰 C4).
struct HttpServerHost {
    issuer: Arc<DesktopTokenIssuer>,
    tickets: Arc<EventTicketStore>,
    instance_id: Option<String>,
    http_calls: Arc<DetachedCalls>,
    mcp_calls: Arc<DetachedCalls>,
}

impl ServerHost for HttpServerHost {
    fn instance_id(&self) -> Option<String> {
        self.instance_id.clone()
    }

    fn issue_window_token(
        &self,
        principal: AuthenticatedPrincipal,
        origin: &str,
    ) -> Result<WindowToken, WindowTokenError> {
        if !WEBVIEW_ORIGINS.contains(&origin) {
            return Err(WindowTokenError::OriginNotAllowed);
        }
        let issued = self
            .issuer
            .issue_window(
                principal,
                TokenOrigin::WebView(origin.to_owned()),
                DESKTOP_TOKEN_TTL,
            )
            .map_err(|_| WindowTokenError::Retired)?;
        Ok(WindowToken {
            token: issued.token,
            expires_at: issued.expires_at.to_rfc3339(),
        })
    }

    fn retire_window(&self, subject: &workbench_protocol::PrincipalSubject) -> u64 {
        let tokens = self.issuer.retire_subject(subject);
        let tickets = self.tickets.retire_subject(subject);
        (tokens + tickets) as u64
    }

    fn accepted_calls(&self) -> u64 {
        (self.http_calls.active() + self.mcp_calls.active()) as u64
    }
}

/// 기동한 어댑터. 종료 신호와 `serve` 완료 신호를 쥔다.
pub struct WorkbenchHttpState {
    base_url: String,
    issuer: Arc<DesktopTokenIssuer>,
    tickets: Arc<EventTicketStore>,
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
    /// 독립 서버의 소유자 신원(044 R5·R6): 소유자 자격 증명 resolver, 인스턴스 식별자, `/v1/system/identify` 증명.
    /// embedded·시험 조립은 `None`.
    pub owner: Option<OwnerIdentity>,
    pub control: Option<Arc<workbench_core::application::server_control::ServerControl>>,
}

/// 소유자 신원을 더한 서버 정보(인스턴스 식별자·신원 증명).
struct HostServerInfo {
    base: AwServerInfo,
    owner: Option<OwnerIdentity>,
    control: Option<Arc<workbench_core::application::server_control::ServerControl>>,
}

impl ServerInfo for HostServerInfo {
    fn server_version(&self) -> String {
        self.base.server_version()
    }
    fn server_epoch(&self) -> String {
        self.base.server_epoch()
    }
    fn storage_schema_version(&self) -> i64 {
        self.base.storage_schema_version()
    }
    fn state(&self) -> String {
        let Some(control) = &self.control else {
            return "serving".to_owned();
        };
        match control.work_gate().state() {
            GateState::Serving => "serving",
            GateState::Draining(_) => "draining",
            GateState::Stopping => "stopping",
        }
        .to_owned()
    }
    fn instance_id(&self) -> Option<String> {
        self.owner
            .as_ref()
            .map(|owner| owner.instance_id().to_owned())
    }
    fn identity_proof(&self, nonce: &str, instance_id: &str) -> Option<String> {
        self.owner
            .as_ref()
            .map(|owner| owner.proof(nonce, instance_id))
    }
}

impl WorkbenchHttpState {
    /// 루프백 임의 포트에 bind하고 `spawner` 런타임에서 `serve`를 띄운다.
    pub fn start(assembly: HttpAssembly, spawner: &tokio::runtime::Handle) -> Result<Self> {
        let std_listener = std::net::TcpListener::bind(("127.0.0.1", 0))
            .context("failed to bind the Workbench HTTP server to localhost")?;
        std_listener
            .set_nonblocking(true)
            .context("failed to configure the Workbench HTTP listener")?;
        let address = std_listener
            .local_addr()
            .context("failed to read the Workbench HTTP address")?;
        let issuer = Arc::new(DesktopTokenIssuer::default());
        let tickets = Arc::new(EventTicketStore::default());
        let mut resolvers: Vec<Arc<dyn CredentialResolver>> = vec![
            issuer.clone() as Arc<dyn CredentialResolver>,
            Arc::new(McpCapabilityResolver::new(assembly.mcp_registry)),
        ];
        if let Some(owner) = &assembly.owner {
            resolvers.push(Arc::new(OwnerResolver::new(owner)));
        }
        let resolver = ChainResolver::new(resolvers);
        let config = ServerConfig {
            resolver: Arc::new(resolver),
            server_info: Arc::new(HostServerInfo {
                base: assembly.server_info,
                owner: assembly.owner,
                control: assembly.control,
            }),
            origins: origin_policy(),
            access_log: Arc::new(StderrAccessLog),
            exposure: ExposurePolicy::network_default(),
            tickets: tickets.clone(),
            body_limit: workbench_server::DEFAULT_BODY_LIMIT,
            drain_warn_after: assembly.drain_warn_after,
            body_read_timeout: workbench_server::DEFAULT_BODY_READ_TIMEOUT,
            connection_grace: workbench_server::DEFAULT_CONNECTION_GRACE,
        };
        let server = workbench_server::build_router(assembly.workbench, config, address.port());
        let http_calls = Arc::clone(&server.calls);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (served_tx, served_rx) = tokio::sync::watch::channel(false);
        spawner.spawn(async move {
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
            tickets,
            shutdown: std::sync::Mutex::new(Some(shutdown_tx)),
            served: served_rx,
            http_calls,
            drain_warn_after: assembly.drain_warn_after,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 호출한 창의 WebView 출처와 창 주체에 묶인 짧은 토큰(043). 허용 출처가 아니면 거절한다.
    pub fn connection_for(
        &self,
        origin: &str,
        principal: AuthenticatedPrincipal,
    ) -> Result<WorkbenchConnection, String> {
        if !WEBVIEW_ORIGINS.contains(&origin) {
            return Err(MESSAGE_ORIGIN_NOT_ALLOWED.to_owned());
        }
        Ok(self.connection(self.issuer.issue_for(
            principal,
            TokenOrigin::WebView(origin.to_owned()),
            DESKTOP_TOKEN_TTL,
        )))
    }

    /// 044 `desktop.*`·`server.status`가 쓰는 core 서버 host port. 이 어댑터의 발급기·이벤트 표를 그대로 쓴다.
    pub fn server_host(
        &self,
        instance_id: Option<String>,
        mcp_calls: Arc<DetachedCalls>,
    ) -> Arc<dyn ServerHost> {
        Arc::new(HttpServerHost {
            issuer: Arc::clone(&self.issuer),
            tickets: Arc::clone(&self.tickets),
            instance_id,
            http_calls: Arc::clone(&self.http_calls),
            mcp_calls,
        })
    }

    /// 창 `Destroyed`(043): 그 창 주체의 토큰과 아직 쓰지 않은 이벤트 표를 모두 지운다.
    pub fn revoke_window(&self, subject: &workbench_protocol::PrincipalSubject) {
        self.issuer.revoke_subject(subject);
        self.tickets.revoke_subject(subject);
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
            incarnation: None,
        }
    }

    /// 종료(research R17, T033): 먼저 HTTP·MCP 양쪽의 새 호출 수락을 닫고(종료 중 들어온 변경은 `503`), 진행 중 작업을
    /// 푼다(`release_work` — AW는 열린 작업대를 닫아 소유 run을 취소하고 권한 대기를 지운다). 그다음 종료 신호 →
    /// `serve` 완료(받아들인 HTTP 호출 drain) → 받아들인 MCP 도구 호출 drain. 상한 없이 기다리고 경고 간격마다 남은
    /// 수를 기록한다. 소유자(앱 종료 경로)는 이 future가 끝난 뒤에 종료한다.
    pub async fn shutdown(
        &self,
        mcp_calls: &DetachedCalls,
        release_work: impl std::future::Future<Output = ()>,
    ) {
        self.http_calls.close();
        mcp_calls.close();
        // 수락을 닫은 뒤, 받아들인 호출을 기다리기 전에 진행 중 작업을 푼다(권한 대기 등) — 닫힌 수락 때문에 응답·취소
        // 요청이 더는 들어올 수 없으므로 종료가 스스로 풀어야 한다(Codex 재리뷰).
        release_work.await;
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

/// 종료 단계(042 T033, 044: host로 옮김). 첫 종료 요청은 종료를 미루고 drain을 시작한다. drain 중 요청은 계속 미룬다. drain이 끝나
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

/// 종료 drain: 수락 닫기 → `release_work`(진행 중 작업 풀기) → HTTP drain → MCP drain. HTTP 어댑터가 없으면 MCP만.
pub async fn drain_for_exit(
    http: Option<Arc<WorkbenchHttpState>>,
    mcp_calls: Arc<DetachedCalls>,
    release_work: impl std::future::Future<Output = ()>,
) {
    match http {
        Some(state) => state.shutdown(&mcp_calls, release_work).await,
        None => {
            mcp_calls.close();
            release_work.await;
            drain_mcp(&mcp_calls, DEFAULT_DRAIN_WARN_AFTER).await
        }
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
        let state = WorkbenchHttpState::start(
            HttpAssembly {
                workbench,
                mcp_registry: CapabilityRegistry::default(),
                server_info: AwServerInfo {
                    version: "test".into(),
                    epoch: "e".into(),
                },
                drain_warn_after: Duration::from_millis(20),
                owner: None,
                control: None,
            },
            &tokio::runtime::Handle::current(),
        )
        .unwrap();
        let origin = "http://localhost:1420";
        let principal = AuthenticatedPrincipal::desktop_window("session-test", "inc-1");
        let connection = state.connection_for(origin, principal.clone()).unwrap();
        assert!(
            state
                .connection_for("https://evil.example", principal)
                .is_err()
        );
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
            tokio::spawn(async move { state.shutdown(&mcp_calls, async {}).await })
        };
        // 두 번째 종료 경로(macOS `RunEvent::Exit`)가 겹쳐도 같은 끝을 기다린다.
        let exiting_again = {
            let (state, mcp_calls) = (state.clone(), mcp_calls.clone());
            tokio::spawn(async move { state.shutdown(&mcp_calls, async {}).await })
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

    /// 사용자 권한 응답을 기다리는 받아들인 호출(`run.cancelAndSend`가 권한 요청에 막힌 상태). 응답은 풀림 신호가
    /// 올 때만 끝난다 — 실제로는 run 취소가 권한 대기를 지운다(acp-agent-core `cancel_run_clears_owner_and_permission_state_for_that_run`).
    struct PermissionWaitingWorkbench {
        entered: Arc<tokio::sync::Notify>,
        released: Arc<tokio::sync::Notify>,
        finished: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Workbench for PermissionWaitingWorkbench {
        async fn call(
            &self,
            _principal: AuthenticatedPrincipal,
            _request: CallRequest,
        ) -> Result<CallReply, WorkbenchFault> {
            let released = self.released.notified();
            self.entered.notify_one();
            released.await;
            self.finished.store(true, Ordering::Release);
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

    /// Codex 재리뷰: 종료는 수락을 닫은 뒤 진행 중 작업을 풀고(`release_work`) 나서 drain한다 — 권한 응답을 기다리는
    /// 받아들인 호출이 있어도 종료가 끝나고, 그 호출도 끝난 뒤에 끝난다. 풀지 않으면 끝나지 않는다.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn exit_releases_calls_waiting_for_a_permission_answer() {
        for release in [true, false] {
            let entered = Arc::new(tokio::sync::Notify::new());
            let released = Arc::new(tokio::sync::Notify::new());
            let finished = Arc::new(AtomicBool::new(false));
            let state = WorkbenchHttpState::start(
                HttpAssembly {
                    workbench: Arc::new(PermissionWaitingWorkbench {
                        entered: entered.clone(),
                        released: released.clone(),
                        finished: finished.clone(),
                    }),
                    mcp_registry: CapabilityRegistry::default(),
                    server_info: AwServerInfo {
                        version: "test".into(),
                        epoch: "e".into(),
                    },
                    drain_warn_after: Duration::from_millis(20),
                    owner: None,
                    control: None,
                },
                &tokio::runtime::Handle::current(),
            )
            .unwrap();
            let origin = "http://localhost:1420";
            let connection = state
                .connection_for(
                    origin,
                    AuthenticatedPrincipal::desktop_window("session-test", "inc-1"),
                )
                .unwrap();
            let address = connection.base_url.trim_start_matches("http://").to_owned();
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

            let mcp_calls = Arc::new(DetachedCalls::default());
            let release_work = {
                let released = released.clone();
                async move {
                    if release {
                        released.notify_one();
                    }
                }
            };
            let exited = tokio::time::timeout(
                Duration::from_secs(5),
                state.shutdown(&mcp_calls, release_work),
            )
            .await;
            if release {
                assert!(
                    exited.is_ok(),
                    "exit finished once the permission wait was released"
                );
                assert!(
                    finished.load(Ordering::Acquire),
                    "the accepted call completed first"
                );
            } else {
                assert!(
                    exited.is_err(),
                    "without releasing work the drain keeps waiting"
                );
                released.notify_one();
            }
            drop(stream);
        }
    }
}
