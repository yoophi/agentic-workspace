//! 소유자 신원(044 research R5·R6): 서버 인스턴스 식별자와 소유자 자격 증명. 자격 증명은 안내 파일(0600)에만 있고,
//! 서버는 원문 bearer를 안내 파일에 쓴 뒤 버리고 digest만 보관한다. 신원 증명은
//! `HMAC-SHA256(SHA256(ownerToken), nonce ‖ "\n" ‖ instanceId)`(hex) — 자격 증명을 아는 서버만 만들 수 있고,
//! 증명 자체는 자격 증명을 드러내지 않는다.

use sha2::{Digest, Sha256};
use workbench_protocol::AuthenticatedPrincipal;
use workbench_server::auth::CredentialResolver;

#[derive(Clone)]
pub struct OwnerIdentity {
    instance_id: String,
    token_digest: [u8; 32],
}

/// 시작 중 안내 파일에 한 번 쓸 원문 bearer와, 런타임에 남길 digest 신원.
pub struct GeneratedOwnerIdentity {
    identity: OwnerIdentity,
    token: String,
}

impl std::fmt::Debug for OwnerIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnerIdentity")
            .field("instance_id", &self.instance_id)
            .field("token_digest", &"<redacted>")
            .finish()
    }
}

impl GeneratedOwnerIdentity {
    pub fn identity(&self) -> &OwnerIdentity {
        &self.identity
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn into_parts(self) -> (OwnerIdentity, String) {
        (self.identity, self.token)
    }
}

impl std::ops::Deref for GeneratedOwnerIdentity {
    type Target = OwnerIdentity;

    fn deref(&self) -> &Self::Target {
        &self.identity
    }
}

impl OwnerIdentity {
    /// 새 인스턴스 식별자와 32바이트 무작위 자격 증명(uuid v4 두 개의 바이트, hex).
    pub fn generate() -> GeneratedOwnerIdentity {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
        bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
        let token = hex(&bytes);
        GeneratedOwnerIdentity {
            identity: Self {
                instance_id: uuid::Uuid::new_v4().to_string(),
                token_digest: Sha256::digest(token.as_bytes()).into(),
            },
            token,
        }
    }

    /// 안내 파일에서 읽은 값으로 만든다(클라이언트가 증명을 검증할 때).
    pub fn from_parts(instance_id: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
            token_digest: Sha256::digest(token.into().as_bytes()).into(),
        }
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn proof(&self, nonce: &str, instance_id: &str) -> String {
        hex(&hmac_sha256(
            &self.token_digest,
            format!("{nonce}\n{instance_id}").as_bytes(),
        ))
    }

    fn token_digest(&self) -> [u8; 32] {
        self.token_digest
    }
}

/// 소유자 자격 증명 → 소유자 주체. 소유자 클라이언트(데스크톱 Rust·CLI)는 브라우저가 아니므로 Origin이 있으면 받지
/// 않는다(WebView가 자격 증명을 얻어도 쓰지 못하게).
pub struct OwnerResolver {
    digest: [u8; 32],
}

impl OwnerResolver {
    pub fn new(identity: &OwnerIdentity) -> Self {
        Self {
            digest: identity.token_digest(),
        }
    }
}

impl CredentialResolver for OwnerResolver {
    fn resolve(&self, bearer: &str, origin: Option<&str>) -> Option<AuthenticatedPrincipal> {
        if origin.is_some() {
            return None;
        }
        let presented: [u8; 32] = Sha256::digest(bearer.as_bytes()).into();
        let mut diff = 0u8;
        for (a, b) in presented.iter().zip(self.digest.iter()) {
            diff |= a ^ b;
        }
        (diff == 0).then(AuthenticatedPrincipal::owner)
    }
}

/// RFC 2104 HMAC-SHA256.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(block.map(|byte| byte ^ 0x36));
    inner.update(message);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(block.map(|byte| byte ^ 0x5c));
    outer.update(inner);
    outer.finalize().into()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 계약 고정 벡터(contracts/server-lifecycle.md §3): 키는 `SHA256(ownerToken UTF-8)`이고 메시지는
    /// `nonce + "\n" + instanceId`, 결과는 소문자 hex. 독립 클라이언트가 이 값으로 구현을 확인한다.
    #[test]
    fn identify_proof_matches_the_contract_vector() {
        let identity = OwnerIdentity::from_parts(
            "6f1a2b3c-4d5e-4f60-8a7b-9c0d1e2f3a4b",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        );
        assert_eq!(
            identity.proof(
                "3f2c9a1e7b6d4c5a8e9f0a1b2c3d4e5f",
                "6f1a2b3c-4d5e-4f60-8a7b-9c0d1e2f3a4b"
            ),
            "ec5b0b7d634e793e9ee94819c6219a3bd9df033afd49cb30d302f6076f6ab079"
        );
    }

    /// RFC 4231 시험 사례 2.
    #[test]
    fn hmac_matches_the_rfc_4231_vector() {
        assert_eq!(
            hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn only_the_exact_owner_token_without_an_origin_resolves_to_the_owner() {
        let generated = OwnerIdentity::generate();
        let resolver = OwnerResolver::new(generated.identity());
        assert_eq!(
            resolver.resolve(generated.token(), None),
            Some(AuthenticatedPrincipal::owner())
        );
        assert_eq!(
            resolver.resolve(generated.token(), Some("tauri://localhost")),
            None
        );
        assert_eq!(resolver.resolve("forged", None), None);
    }

    #[test]
    fn the_proof_depends_on_the_token_nonce_and_instance() {
        let generated = OwnerIdentity::generate();
        let a = generated.identity();
        let b = OwnerIdentity::from_parts(a.instance_id(), "other");
        assert_eq!(a.proof("n", a.instance_id()), a.proof("n", a.instance_id()));
        assert_ne!(a.proof("n", a.instance_id()), b.proof("n", a.instance_id()));
        assert_ne!(a.proof("n", a.instance_id()), a.proof("m", a.instance_id()));
    }
}
