//! Application-independent authenticated loopback MCP hosting.
//!
//! The application owns principals, grants, tool contracts and accepted mutation
//! tasks. Stop admission, drain those tasks, then shut down the host. HTTP request
//! cancellation alone must not own a committed application mutation.

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use http::{HeaderMap, StatusCode, Uri, header};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use std::{
    error::Error,
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;

pub use rmcp;

/// Defaults reject every present Origin and bound requests to 2 MiB.
#[derive(Clone, Debug)]
pub struct HostConfig {
    allowed_origins: Vec<String>,
    max_request_bytes: usize,
    protocol_versions: Vec<rmcp::model::ProtocolVersion>,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            allowed_origins: Vec::new(),
            max_request_bytes: 2 * 1024 * 1024,
            protocol_versions: vec![rmcp::model::ProtocolVersion::V_2025_11_25],
        }
    }
}

impl HostConfig {
    /// Legacy versions only; must agree with ServerHandler's negotiation list.
    /// The SDK's legacy stateless path only checks whether a header is known to
    /// the SDK, so the host also checks the application's configured versions.
    pub fn with_protocol_versions(
        mut self,
        versions: Vec<rmcp::model::ProtocolVersion>,
    ) -> io::Result<Self> {
        if versions.is_empty()
            || versions.iter().any(|version| {
                !rmcp::model::ProtocolVersion::KNOWN_VERSIONS.contains(version)
                    || version.as_str() > rmcp::model::ProtocolVersion::V_2025_11_25.as_str()
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid protocol versions",
            ));
        }
        self.protocol_versions = versions;
        Ok(self)
    }

    /// Allow an exact HTTP(S) origin with an explicit port. Wildcards, userinfo,
    /// opaque origins, query strings and non-root paths are rejected.
    pub fn with_allowed_origin(mut self, origin: &str) -> io::Result<Self> {
        let uri: Uri = origin
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid origin"))?;
        let valid = matches!(uri.scheme_str(), Some("http" | "https"))
            && uri
                .authority()
                .is_some_and(|a| a.port_u16().is_some() && !a.as_str().contains(['*', '@']))
            && matches!(uri.path(), "" | "/")
            && uri.query().is_none();
        if !valid {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "origin must include an exact HTTP(S) host and port",
            ));
        }
        self.allowed_origins.push(origin.to_owned());
        Ok(self)
    }

    pub fn with_max_request_bytes(mut self, bytes: usize) -> io::Result<Self> {
        if bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "request limit must be nonzero",
            ));
        }
        self.max_request_bytes = bytes;
        Ok(self)
    }
}

/// Owned host lifetime. Dropping cancels this listener/transport only; explicit
/// shutdown awaits completion. This is not an application mutation task registry.
pub struct LocalMcpHost {
    endpoint: String,
    accepting: Arc<AtomicBool>,
    server_cancel: CancellationToken,
    transport_cancel: CancellationToken,
    server: Option<JoinHandle<io::Result<()>>>,
}

impl LocalMcpHost {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Reject new requests without cancelling accepted handler futures.
    pub fn stop_accepting(&self) {
        self.accepting.store(false, Ordering::SeqCst);
    }

    /// Call after the application's accepted mutation tasks have drained.
    pub async fn shutdown(mut self) -> io::Result<()> {
        self.stop_accepting();
        self.server_cancel.cancel();
        self.transport_cancel.cancel();
        if let Some(server) = self.server.take() {
            server.await.map_err(io::Error::other)?
        } else {
            Ok(())
        }
    }
}

impl Drop for LocalMcpHost {
    fn drop(&mut self) {
        self.stop_accepting();
        self.server_cancel.cancel();
        self.transport_cancel.cancel();
    }
}

/// Bind 127.0.0.1 on an ephemeral port in the caller's Tokio runtime.
///
/// `authorize` resolves a strictly parsed bearer credential on every request.
/// Return an opaque non-secret principal; unknown/revoked tokens return None.
/// Raw Authorization headers are removed before reaching the SDK/handler.
/// Each handler must advertise its actual supported protocol versions itself.
pub async fn start<S, P, F, A>(
    factory: F,
    authorize: A,
    config: HostConfig,
) -> io::Result<LocalMcpHost>
where
    S: rmcp::ServerHandler + Send + 'static,
    P: Clone + Send + Sync + 'static,
    F: Fn() -> io::Result<S> + Send + Sync + 'static,
    A: Fn(&str) -> Option<P> + Send + Sync + 'static,
{
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
    let address = listener.local_addr()?;
    let server_cancel = CancellationToken::new();
    let transport_cancel = CancellationToken::new();
    let origins = Arc::new(config.allowed_origins.clone());
    let sdk_config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_max_request_body_bytes(config.max_request_bytes)
        .with_allowed_hosts([address.to_string()])
        .with_allowed_origins(config.allowed_origins)
        .enforce_origin_validation()
        .with_cancellation_token(transport_cancel.clone());
    let service = StreamableHttpService::new(
        move || factory().map_err(|_| io::Error::other("MCP handler unavailable")),
        Arc::new(NeverSessionManager::default()),
        sdk_config,
    );
    let accepting = Arc::new(AtomicBool::new(true));
    let gate = accepting.clone();
    let authorize = Arc::new(authorize);
    let body_cancel = transport_cancel.clone();
    let versions = Arc::new(config.protocol_versions);
    let app = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn(move |request: Request, next: Next| {
            let authorize = authorize.clone();
            let gate = gate.clone();
            let body_cancel = body_cancel.clone();
            let versions = versions.clone();
            let origins = origins.clone();
            async move {
                if !gate.load(Ordering::SeqCst) {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                let Some(principal) = bearer(request.headers()).and_then(|token| authorize(token))
                else {
                    return (
                        StatusCode::UNAUTHORIZED,
                        [(header::WWW_AUTHENTICATE, "Bearer")],
                        "Unauthorized",
                    )
                        .into_response();
                };
                // Apply security before body/protocol errors. The SDK validates
                // the same boundaries again after the admission middleware.
                if !allowed_boundaries(&request, address.port(), &origins) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                if request.headers().get_all(header::ORIGIN).iter().count() > 1 {
                    return StatusCode::FORBIDDEN.into_response();
                }
                let mut supplied = request.headers().get_all("mcp-protocol-version").iter();
                if let Some(version) = supplied.next()
                    && (supplied.next().is_some()
                        || !version.to_str().ok().is_some_and(|value| {
                            versions.iter().any(|allowed| allowed.as_str() == value)
                        }))
                {
                    return StatusCode::BAD_REQUEST.into_response();
                }
                authenticated_request(
                    request,
                    next,
                    principal,
                    config.max_request_bytes,
                    body_cancel,
                    &versions,
                )
                .await
            }
        }));
    let shutdown = server_cancel.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown.cancelled_owned())
            .await
    });
    Ok(LocalMcpHost {
        endpoint: format!("http://{address}/mcp"),
        accepting,
        server_cancel,
        transport_cancel,
        server: Some(server),
    })
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer")
        || token.is_empty()
        || token.contains(|c: char| c.is_whitespace() || c == ',')
    {
        return None;
    }
    Some(token)
}

fn allowed_boundaries(request: &Request, port: u16, origins: &[String]) -> bool {
    let mut hosts = request.headers().get_all(header::HOST).iter();
    let authority = if let Some(host) = hosts.next() {
        if hosts.next().is_some() {
            return false;
        }
        host.to_str()
            .ok()
            .and_then(|host| host.parse::<http::uri::Authority>().ok())
    } else {
        request.uri().authority().cloned()
    };
    if !authority.is_some_and(|a| a.host() == "127.0.0.1" && a.port_u16() == Some(port)) {
        return false;
    }
    let mut supplied = request.headers().get_all(header::ORIGIN).iter();
    let Some(origin) = supplied.next() else {
        return true;
    };
    if supplied.next().is_some() {
        return false;
    }
    let Some(origin) = origin.to_str().ok().and_then(normalized_origin) else {
        return false;
    };
    origins
        .iter()
        .any(|allowed| normalized_origin(allowed).is_some_and(|allowed| allowed == origin))
}

fn normalized_origin(value: &str) -> Option<(String, String, u16)> {
    let uri: Uri = value.parse().ok()?;
    let scheme = uri.scheme_str()?.to_ascii_lowercase();
    let default_port = match scheme.as_str() {
        "http" => 80,
        "https" => 443,
        _ => return None,
    };
    let authority = uri.authority()?;
    if authority.as_str().contains(['*', '@'])
        || !matches!(uri.path(), "" | "/")
        || uri.query().is_some()
    {
        return None;
    }
    Some((
        scheme,
        authority.host().to_ascii_lowercase(),
        authority.port_u16().unwrap_or(default_port),
    ))
}

async fn authenticated_request<P: Clone + Send + Sync + 'static>(
    request: Request,
    next: Next,
    principal: P,
    limit: usize,
    cancel: CancellationToken,
    versions: &[rmcp::model::ProtocolVersion],
) -> Response {
    let (mut parts, body) = request.into_parts();
    parts.headers.remove(header::AUTHORIZATION);
    parts.extensions.insert(principal);
    let body_result = tokio::select! {
        result = to_bytes(body, limit) => result,
        _ = cancel.cancelled() => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let bytes = match body_result {
        Ok(bytes) => bytes,
        Err(error) => {
            let mut cause: &(dyn Error + 'static) = &error;
            loop {
                if cause.is::<http_body_util::LengthLimitError>() {
                    return StatusCode::PAYLOAD_TOO_LARGE.into_response();
                }
                match cause.source() {
                    Some(source) => cause = source,
                    None => return StatusCode::BAD_REQUEST.into_response(),
                }
            }
        }
    };
    // Legacy HTTP assumes 2025-03-26 without the version header. Initialize
    // can negotiate another version without it. Use SDK message types solely
    // to classify this boundary; the SDK owns parsing errors and dispatch.
    if parts.method == http::Method::POST
        && !parts.headers.contains_key("mcp-protocol-version")
        && !versions.contains(&rmcp::model::ProtocolVersion::V_2025_03_26)
        && let Ok(message) = serde_json::from_slice::<rmcp::model::ClientJsonRpcMessage>(&bytes)
        && !matches!(message, rmcp::model::ClientJsonRpcMessage::Request(request)
            if matches!(request.request, rmcp::model::ClientRequest::InitializeRequest(_)))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

/// Read the authenticated application principal in a tool handler.
pub fn principal<P: Send + Sync + 'static>(
    context: &rmcp::service::RequestContext<rmcp::RoleServer>,
) -> Option<&P> {
    context
        .extensions
        .get::<http::request::Parts>()?
        .extensions
        .get::<P>()
}
