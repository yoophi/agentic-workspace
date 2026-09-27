//! 호환성 협상(042 research R6, contracts §3). 서버는 클라이언트가 지원하는 프로토콜 버전과의 교집합에서 가장 높은
//! 것을 고르고, 없으면 안정된 비호환 오류를 낸다. `contractHash`는 계약 문서 JSON의 SHA-256(drift 진단용).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use workbench_protocol::PROTOCOL_VERSION;

pub const MESSAGE_PROTOCOL_UNSUPPORTED: &str = "protocol version is not supported.";
pub const API_MAJOR: u16 = 1;
pub const SUPPORTED_PROTOCOL_VERSIONS: &[u16] = &[PROTOCOL_VERSION];

/// 조립이 주는 서버 정보.
pub trait ServerInfo: Send + Sync {
    /// 빌드 버전(CALVER 등).
    fn server_version(&self) -> String;
    fn server_epoch(&self) -> String;
    fn storage_schema_version(&self) -> i64;
    /// 서버 인스턴스 식별자(044). 독립 서버는 안내 파일과 같은 값을 쓴다. 없으면 router가 새로 만든다.
    fn instance_id(&self) -> Option<String> {
        None
    }
    /// `/v1/system/identify` 증명(044 research R5): 소유자 자격 증명을 아는 서버만 만들 수 있는 값. 없으면 그 경로는
    /// `notFound`다(embedded·시험 조립).
    fn identity_proof(&self, _nonce: &str, _instance_id: &str) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeRequest {
    pub supported_protocol_versions: Vec<u16>,
    #[serde(default)]
    pub client: Option<ClientInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResponse {
    pub selected_protocol_version: u16,
    pub supported_protocol_versions: Vec<u16>,
    pub server_version: String,
    pub api_major: u16,
    pub contract_hash: String,
    pub instance_id: String,
    pub server_epoch: String,
    pub storage_schema_version: i64,
    pub features: Vec<String>,
}

pub fn contract_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let digest = Sha256::digest(workbench_protocol::openapi::render_openapi().as_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    })
}

/// 교집합의 가장 높은 버전.
pub fn negotiate(client: &[u16]) -> Option<u16> {
    SUPPORTED_PROTOCOL_VERSIONS
        .iter()
        .copied()
        .filter(|version| client.contains(version))
        .max()
}

pub fn respond(
    request: &HandshakeRequest,
    info: &dyn ServerInfo,
    instance_id: &str,
) -> Option<HandshakeResponse> {
    let selected = negotiate(&request.supported_protocol_versions)?;
    Some(HandshakeResponse {
        selected_protocol_version: selected,
        supported_protocol_versions: SUPPORTED_PROTOCOL_VERSIONS.to_vec(),
        server_version: info.server_version(),
        api_major: API_MAJOR,
        contract_hash: contract_hash().to_owned(),
        instance_id: instance_id.to_owned(),
        server_epoch: info.server_epoch(),
        storage_schema_version: info.storage_schema_version(),
        features: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiation_picks_the_highest_common_version() {
        assert_eq!(negotiate(&[1, 7]), Some(1));
        assert_eq!(negotiate(&[7]), None);
        assert_eq!(negotiate(&[]), None);
    }

    #[test]
    fn contract_hash_is_stable_hex() {
        assert_eq!(contract_hash().len(), 64);
        assert_eq!(contract_hash(), contract_hash());
    }
}
