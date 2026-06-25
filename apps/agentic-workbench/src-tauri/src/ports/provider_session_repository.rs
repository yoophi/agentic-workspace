use anyhow::Result;

use crate::domain::provider_session::{ProviderSession, SessionScope};

/// provider 네이티브 세션을 읽어오는 outbound 포트. 구현체는 각 provider의
/// 로컬 저장소(파일시스템 등)에서 세션 메타데이터를 수집한다.
pub trait ProviderSessionRepository: Send + Sync + 'static {
    /// `agent_id`에 해당하는 provider의 세션을 `scope`로 필터링해 돌려준다.
    /// 지원하지 않는 provider면 빈 목록을 돌려준다.
    fn list(&self, agent_id: &str, scope: &SessionScope) -> Result<Vec<ProviderSession>>;
}
