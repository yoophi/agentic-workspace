//! 인증(042 research R3, contracts §2). router는 포트 `CredentialResolver`만 안다. 발급원은 조립이 합친다:
//! 데스크톱 토큰(`DesktopTokenIssuer`, WebView 출처에 묶임), MCP 실행 토큰(AW `CapabilityRegistry` → agent),
//! 테스트 고정 토큰(`StaticResolver`). 토큰 원문은 저장하지 않고 SHA-256 해시로만 찾는다.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use workbench_protocol::AuthenticatedPrincipal;

/// bearer 토큰 → principal. 요청 Origin(있으면)을 함께 받는다 — Origin에 묶인 자격 증명을 판정한다.
pub trait CredentialResolver: Send + Sync {
    fn resolve(&self, bearer: &str, origin: Option<&str>) -> Option<AuthenticatedPrincipal>;
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 256비트 무작위 URL-safe 토큰.
pub(crate) fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn digest(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// 데스크톱 토큰이 묶인 출처. `None`은 비브라우저 진단 토큰(요청에 Origin이 있으면 거절).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenOrigin {
    WebView(String),
    NoOrigin,
}

#[derive(Debug, Clone)]
struct DesktopEntry {
    principal: AuthenticatedPrincipal,
    origin: TokenOrigin,
    expires_at: Instant,
}

/// 발급 결과. `token`은 호출자에게 한 번만 건넨다.
#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub client_instance_id: String,
}

pub const DESKTOP_TOKEN_TTL: Duration = Duration::from_secs(15 * 60);
pub const DIAGNOSTIC_TOKEN_TTL: Duration = Duration::from_secs(10 * 60);
pub const DESKTOP_TOKEN_CAPACITY: usize = 256;

/// 메모리 데스크톱 토큰 발급기. 상한을 넘으면 가장 먼저 만료될 토큰부터 버린다.
pub struct DesktopTokenIssuer {
    entries: Mutex<HashMap<[u8; 32], DesktopEntry>>,
    capacity: usize,
}

impl Default for DesktopTokenIssuer {
    fn default() -> Self {
        Self::with_capacity(DESKTOP_TOKEN_CAPACITY)
    }
}

impl DesktopTokenIssuer {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            capacity,
        }
    }

    pub fn issue(&self, origin: TokenOrigin, ttl: Duration) -> IssuedToken {
        let token = random_token();
        let now = Instant::now();
        let mut entries = lock(&self.entries);
        entries.retain(|_, entry| entry.expires_at > now);
        while entries.len() >= self.capacity {
            let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, entry)| entry.expires_at)
                .map(|(key, _)| *key)
            else {
                break;
            };
            entries.remove(&oldest);
        }
        entries.insert(
            digest(&token),
            DesktopEntry {
                principal: AuthenticatedPrincipal::desktop(),
                origin,
                expires_at: now + ttl,
            },
        );
        IssuedToken {
            token,
            expires_at: chrono::Utc::now()
                + chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::zero()),
            client_instance_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn len(&self) -> usize {
        lock(&self.entries).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl CredentialResolver for DesktopTokenIssuer {
    fn resolve(&self, bearer: &str, origin: Option<&str>) -> Option<AuthenticatedPrincipal> {
        let entries = lock(&self.entries);
        let entry = entries.get(&digest(bearer))?;
        if entry.expires_at <= Instant::now() {
            return None;
        }
        let origin_ok = match (&entry.origin, origin) {
            (TokenOrigin::WebView(bound), Some(request)) => bound == request,
            (TokenOrigin::NoOrigin, None) => true,
            _ => false,
        };
        origin_ok.then(|| entry.principal.clone())
    }
}

/// 여러 발급원을 차례로 본다(첫 성공).
#[derive(Clone, Default)]
pub struct ChainResolver {
    resolvers: Vec<Arc<dyn CredentialResolver>>,
}

impl ChainResolver {
    pub fn new(resolvers: Vec<Arc<dyn CredentialResolver>>) -> Self {
        Self { resolvers }
    }
}

impl CredentialResolver for ChainResolver {
    fn resolve(&self, bearer: &str, origin: Option<&str>) -> Option<AuthenticatedPrincipal> {
        self.resolvers
            .iter()
            .find_map(|resolver| resolver.resolve(bearer, origin))
    }
}

/// 고정 토큰(계약 테스트 조립 전용). Origin은 보지 않는다.
#[derive(Clone, Default)]
pub struct StaticResolver {
    tokens: HashMap<String, AuthenticatedPrincipal>,
    /// 접두사 + run id → agent principal(테스트 `test-agent:<run>`).
    agent_prefix: Option<String>,
}

impl StaticResolver {
    pub fn new(tokens: impl IntoIterator<Item = (String, AuthenticatedPrincipal)>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
            agent_prefix: None,
        }
    }

    pub fn with_agent_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.agent_prefix = Some(prefix.into());
        self
    }
}

impl CredentialResolver for StaticResolver {
    fn resolve(&self, bearer: &str, _origin: Option<&str>) -> Option<AuthenticatedPrincipal> {
        if let Some(principal) = self.tokens.get(bearer) {
            return Some(principal.clone());
        }
        let prefix = self.agent_prefix.as_deref()?;
        bearer
            .strip_prefix(prefix)
            .filter(|run| !run.is_empty())
            .map(AuthenticatedPrincipal::agent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use workbench_protocol::PrincipalKind;

    #[test]
    fn webview_tokens_are_bound_to_their_origin() {
        let issuer = DesktopTokenIssuer::default();
        let issued = issuer.issue(
            TokenOrigin::WebView("http://localhost:1420".into()),
            DESKTOP_TOKEN_TTL,
        );
        assert_eq!(issued.token.len(), 43, "256-bit url-safe base64");
        let ok = issuer
            .resolve(&issued.token, Some("http://localhost:1420"))
            .unwrap();
        assert_eq!(ok.kind, PrincipalKind::Desktop);
        assert!(issuer
            .resolve(&issued.token, Some("tauri://localhost"))
            .is_none());
        assert!(
            issuer.resolve(&issued.token, None).is_none(),
            "webview token needs an origin"
        );
        assert!(issuer
            .resolve("unknown", Some("http://localhost:1420"))
            .is_none());
    }

    #[test]
    fn diagnostic_tokens_refuse_browser_origins() {
        let issuer = DesktopTokenIssuer::default();
        let issued = issuer.issue(TokenOrigin::NoOrigin, DIAGNOSTIC_TOKEN_TTL);
        assert!(issuer.resolve(&issued.token, None).is_some());
        assert!(issuer
            .resolve(&issued.token, Some("http://localhost:1420"))
            .is_none());
    }

    #[test]
    fn expired_tokens_do_not_resolve_and_raw_tokens_are_not_kept() {
        let issuer = DesktopTokenIssuer::default();
        let issued = issuer.issue(TokenOrigin::NoOrigin, Duration::from_millis(0));
        assert!(issuer.resolve(&issued.token, None).is_none());
        let kept = format!("{:?}", lock(&issuer.entries).keys().collect::<Vec<_>>());
        assert!(!kept.contains(&issued.token));
    }

    #[test]
    fn capacity_evicts_the_earliest_expiring_token() {
        let issuer = DesktopTokenIssuer::with_capacity(2);
        let first = issuer.issue(TokenOrigin::NoOrigin, Duration::from_secs(10));
        let _second = issuer.issue(TokenOrigin::NoOrigin, Duration::from_secs(20));
        let third = issuer.issue(TokenOrigin::NoOrigin, Duration::from_secs(30));
        assert_eq!(issuer.len(), 2);
        assert!(issuer.resolve(&first.token, None).is_none());
        assert!(issuer.resolve(&third.token, None).is_some());
    }

    #[test]
    fn chain_and_static_resolvers() {
        let fixed = StaticResolver::new([("t1".to_owned(), AuthenticatedPrincipal::desktop())])
            .with_agent_prefix("agent-");
        let chain = ChainResolver::new(vec![Arc::new(fixed)]);
        assert!(chain.resolve("t1", None).is_some());
        assert_eq!(
            chain.resolve("agent-run-7", None).unwrap().agent_run_id(),
            Some("run-7")
        );
        assert!(chain.resolve("agent-", None).is_none());
        assert!(chain.resolve("nope", None).is_none());
    }
}
