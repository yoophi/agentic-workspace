use anyhow::Result;

use crate::domain::provider_session::{ProviderSession, SessionScope};
use crate::ports::provider_session_repository::ProviderSessionRepository;

/// provider 네이티브 세션 목록을 조회하는 use case. 최신순으로 정렬하고
/// 선택적으로 개수를 제한한다.
pub struct ListProviderSessionsUseCase<R> {
    repository: R,
}

impl<R> ListProviderSessionsUseCase<R>
where
    R: ProviderSessionRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn execute(
        &self,
        agent_id: &str,
        scope: &SessionScope,
        limit: Option<usize>,
    ) -> Result<Vec<ProviderSession>> {
        let mut sessions = self.repository.list(agent_id, scope)?;
        // updated_at(RFC3339 문자열)은 사전식 비교로도 시간순이 보장된다.
        sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        if let Some(limit) = limit {
            sessions.truncate(limit);
        }
        Ok(sessions)
    }
}
