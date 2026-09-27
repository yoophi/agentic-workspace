//! Workbench HTTP/WebSocket 어댑터(042, 3단계). 계약(`workbench_protocol::Workbench`)만 부르는 inbound 어댑터이며,
//! 인증·서버 정보는 포트로 주입받는다. 데스크톱 앱이 자기 런타임과 같은 인스턴스에 붙이고(3단계), 5단계 독립 서버가
//! 같은 조립을 쓴다. 규칙: `specs/042-workbench-http/contracts/workbench-http.md`.
//!
//! 보안 경계(contracts §1–§6): 127.0.0.1 bind, Host·Origin 정확 일치, 모든 경로 bearer 인증(live 제외), 30초 1회용
//! 구독 표, 본문 1 MiB, 비밀 비기록. 받아들인 호출은 연결과 무관하게 서버 소유 task에서 끝까지 실행된다(R17).

pub mod access_log;
pub mod auth;
pub mod drain;
pub mod handshake;
pub mod origin;
mod routes;
pub mod tickets;

use std::{collections::HashSet, future::Future, sync::Arc};

use axum::{
    extract::{Request, State},
    http::{header, HeaderName, HeaderValue, Method},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use workbench_protocol::{
    operations::OPERATIONS, OperationId, OperationKind, Workbench, PROTOCOL_VERSION,
};

use crate::{
    access_log::AccessLog,
    auth::CredentialResolver,
    drain::DetachedCalls,
    handshake::ServerInfo,
    origin::{HostPolicy, OriginCheck, OriginPolicy},
    tickets::EventTicketStore,
};

pub const PROTOCOL_HEADER: &str = "aw-protocol-version";
pub const DEFAULT_BODY_LIMIT: usize = 1024 * 1024;
/// 본문 읽기 제한 시간(느린 본문으로 연결을 붙잡지 못하게).
pub const DEFAULT_BODY_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// 종료 신호 뒤 받아들이지 않은 연결(헤더·본문을 보내다 멈춘 연결 등)을 기다리는 시간. 받아들인 호출은 이와
/// 무관하게 끝까지 drain한다.
pub const DEFAULT_CONNECTION_GRACE: std::time::Duration = std::time::Duration::from_secs(2);
/// 종료 때 구독에 close 프레임을 보내는 제한 시간(읽지 않는 클라이언트가 정리를 막지 못하게).
pub const SUBSCRIPTION_CLOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
pub const WS_MAX_MESSAGE: usize = 64 * 1024;
pub const MESSAGE_HOST_NOT_ALLOWED: &str = "host is not allowed.";
pub const MESSAGE_ORIGIN_NOT_ALLOWED: &str = "origin is not allowed.";
pub const MESSAGE_NOT_EXPOSED: &str = "operation is not exposed over the network.";

/// 네트워크에 여는 operation 집합(042 research R13·R17). 변경 operation은 중단·재시작·연결 단절 증거가 있을 때만 연다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExposurePolicy {
    All,
    Only(HashSet<OperationId>),
}

impl ExposurePolicy {
    /// 조회만(증거 게이트 전 시작 값).
    pub fn queries_only() -> Self {
        Self::Only(
            OPERATIONS
                .iter()
                .filter(|spec| spec.kind == OperationKind::Query)
                .map(|spec| spec.id)
                .collect(),
        )
    }

    /// 운영 기본값: 전체 공개. 변경 operation을 여는 근거는 공개 게이트 시험이다 — 연결 단절·종료 drain
    /// (`workbench-core/tests/http_disconnect_retry.rs`), 영속 변경 중단 판정(`crash_points_updates.rs`와 038·040
    /// crash 시험), 재시작 뒤 재시도(`restart_retry.rs`). 이 시험이 깨지면 공개를 다시 좁힌다.
    pub fn network_default() -> Self {
        Self::All
    }

    pub fn allows(&self, id: OperationId) -> bool {
        match self {
            Self::All => true,
            Self::Only(ids) => ids.contains(&id),
        }
    }
}

/// 조립이 주는 설정.
#[derive(Clone)]
pub struct ServerConfig {
    pub resolver: Arc<dyn CredentialResolver>,
    pub server_info: Arc<dyn ServerInfo>,
    pub origins: OriginPolicy,
    pub access_log: Arc<dyn AccessLog>,
    pub exposure: ExposurePolicy,
    pub tickets: Arc<EventTicketStore>,
    pub body_limit: usize,
    /// 종료 drain 경고 간격(`drain::DEFAULT_DRAIN_WARN_AFTER`). 상한이 아니다.
    pub drain_warn_after: std::time::Duration,
    /// 본문 읽기 제한 시간(`DEFAULT_BODY_READ_TIMEOUT`).
    pub body_read_timeout: std::time::Duration,
    /// 종료 신호 뒤 받아들이지 않은 연결을 기다리는 시간(`DEFAULT_CONNECTION_GRACE`).
    pub connection_grace: std::time::Duration,
}

pub(crate) struct AppState {
    pub workbench: Arc<dyn Workbench>,
    pub config: ServerConfig,
    pub host: HostPolicy,
    pub instance_id: String,
    pub calls: Arc<DetachedCalls>,
    pub subscriptions: Arc<DetachedCalls>,
}

/// 조립된 서버: router와 받아들인 호출·열린 구독 추적기. [`serve`]가 모두 쓴다.
pub struct WorkbenchServer {
    pub router: Router,
    pub calls: Arc<DetachedCalls>,
    /// 열린 WebSocket 구독(upgrade task). 종료 때 닫고 `serve` 반환 전에 0이 될 때까지 기다린다.
    pub subscriptions: Arc<DetachedCalls>,
    pub drain_warn_after: std::time::Duration,
    pub connection_grace: std::time::Duration,
}

/// 루프백 임의 포트에 bind한다(다른 주소는 받지 않는다).
pub async fn bind_loopback() -> std::io::Result<tokio::net::TcpListener> {
    tokio::net::TcpListener::bind(("127.0.0.1", 0)).await
}

/// `port`는 bind한 루프백 포트(Host 검사에 쓴다).
pub fn build_router(
    workbench: Arc<dyn Workbench>,
    config: ServerConfig,
    port: u16,
) -> WorkbenchServer {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(
            config
                .origins
                .allowed()
                .iter()
                .filter_map(|origin| HeaderValue::from_str(origin).ok()),
        ))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .expose_headers([HeaderName::from_static(PROTOCOL_HEADER)])
        .max_age(std::time::Duration::from_secs(600));
    let body_limit = config.body_limit;
    let drain_warn_after = config.drain_warn_after;
    let connection_grace = config.connection_grace;
    let calls = Arc::new(DetachedCalls::default());
    let subscriptions = Arc::new(DetachedCalls::default());
    let state = Arc::new(AppState {
        workbench,
        config,
        host: HostPolicy::new(port),
        instance_id: uuid::Uuid::new_v4().to_string(),
        calls: Arc::clone(&calls),
        subscriptions: Arc::clone(&subscriptions),
    });
    let router = Router::new()
        .route("/health/live", get(routes::health::live))
        .route("/health/ready", get(routes::health::ready))
        .route("/openapi.json", get(routes::openapi::document))
        .route("/v1/system/handshake", post(routes::handshake::handshake))
        .route("/v1/calls", post(routes::calls::call))
        .route("/v1/event-tickets", post(routes::events::issue_ticket))
        .route("/v1/events", get(routes::events::connect))
        .layer(axum::extract::DefaultBodyLimit::max(body_limit))
        .layer(cors)
        .layer(middleware::from_fn_with_state(Arc::clone(&state), guard))
        .layer(middleware::map_response(protocol_header))
        .with_state(state);
    WorkbenchServer {
        router,
        calls,
        subscriptions,
        drain_warn_after,
        connection_grace,
    }
}

/// Host·Origin 검사(모든 요청, preflight 포함). Origin 없음은 통과하고 자격 증명이 판정한다.
async fn guard(State(state): State<Arc<AppState>>, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok());
    if !state.host.allows(host) {
        return routes::forbidden(MESSAGE_HOST_NOT_ALLOWED);
    }
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .map(|value| value.to_str().unwrap_or("null"));
    if state.config.origins.check(origin) == OriginCheck::Rejected {
        return routes::forbidden(MESSAGE_ORIGIN_NOT_ALLOWED);
    }
    next.run(request).await
}

async fn protocol_header(mut response: Response) -> Response {
    response.headers_mut().insert(
        HeaderName::from_static(PROTOCOL_HEADER),
        HeaderValue::from(PROTOCOL_VERSION),
    );
    response
}

/// 종료 순서(R17, 구현 리뷰 반영):
/// 1. 신호 → 새 연결 받기 중단, 호출·구독 추적기를 닫는다: 새 호출 `503`, 본문을 읽던 받아들이지 않은 요청은 곧바로
///    `503`, 새 구독 거절, 열린 구독은 close를 보내고 스트림을 놓는다.
/// 2. 연결마다 graceful shutdown을 걸고 최대 `connection_grace` 기다린 뒤 남은 연결 task를 abort하고 모두 join한다 —
///    헤더를 보내다 멈춘 연결도 소켓·router·`Workbench` 참조까지 해제된다. (`axum::serve`는 연결 task를 detach해
///    future를 버려도 남으므로 같은 구성 — hyper-util `auto::Builder` + upgrade — 으로 연결을 직접 소유한다.)
/// 3. 열린 구독 task가 모두 끝날 때까지(소켓·EventStream 해제).
/// 4. **받아들인 분리 호출 drain** — 연결과 무관하게 추적기로, 상한 없음, 경고 간격마다 남은 수를 기록.
///
/// 소유 런타임(AW 종료 수명, 5단계 서버)은 이 future가 끝난 뒤에만 런타임을 내려야 한다.
pub async fn serve(
    listener: tokio::net::TcpListener,
    server: WorkbenchServer,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    use hyper_util::{
        rt::{TokioExecutor, TokioIo},
        server::conn::auto::Builder,
        service::TowerToHyperService,
    };
    use tower::ServiceExt as _;

    let WorkbenchServer {
        router,
        calls,
        subscriptions,
        drain_warn_after,
        connection_grace,
    } = server;
    let (graceful_tx, graceful_rx) = tokio::sync::watch::channel(false);
    let mut connections = tokio::task::JoinSet::new();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => break,
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
            accepted = listener.accept() => {
                let tcp = match accepted {
                    Ok((tcp, _)) => tcp,
                    Err(error) => {
                        eprintln!("[workbench-http] accept failed: {error}");
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        continue;
                    }
                };
                let service = TowerToHyperService::new(
                    router
                        .clone()
                        .map_request(|request: Request<hyper::body::Incoming>| {
                            request.map(axum::body::Body::new)
                        }),
                );
                let mut graceful = graceful_rx.clone();
                connections.spawn(async move {
                    let builder = Builder::new(TokioExecutor::new());
                    let connection =
                        builder.serve_connection_with_upgrades(TokioIo::new(tcp), service);
                    tokio::pin!(connection);
                    tokio::select! {
                        _ = connection.as_mut() => return,
                        _ = graceful.wait_for(|stop| *stop) => {}
                    }
                    connection.as_mut().graceful_shutdown();
                    let _ = connection.await;
                });
            }
        }
    }
    drop(listener);
    calls.close();
    subscriptions.close();
    graceful_tx.send_replace(true);
    let finished = tokio::time::timeout(connection_grace, async {
        while connections.join_next().await.is_some() {}
    })
    .await;
    if finished.is_err() {
        eprintln!(
            "[workbench-http] shutdown: closing {} connection(s) still open after the grace period",
            connections.len()
        );
        connections.abort_all();
        while connections.join_next().await.is_some() {}
    }
    drop(router);
    subscriptions
        .drain_until_idle(drain_warn_after, |left| {
            eprintln!("[workbench-http] shutdown waiting for {left} subscription(s) to close");
        })
        .await;
    calls
        .drain_until_idle(drain_warn_after, |left| {
            eprintln!("[workbench-http] shutdown waiting for {left} accepted call(s) to finish");
        })
        .await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_only_opens_exactly_the_read_operations() {
        let ExposurePolicy::Only(ids) = ExposurePolicy::queries_only() else {
            panic!("queries_only is a set");
        };
        assert_eq!(ids.len(), 32);
        assert!(ids.contains(&OperationId::ProjectList));
        assert!(!ids.contains(&OperationId::ProjectCreate));
        assert!(ExposurePolicy::network_default().allows(OperationId::ProjectCreate));
    }
}
