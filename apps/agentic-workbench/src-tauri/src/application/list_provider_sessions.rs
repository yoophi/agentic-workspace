use anyhow::Result;
use chrono::DateTime;

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
        // RFC3339 문자열의 사전식 비교는 타임존 오프셋이 섞이면 어긋날 수 있으므로
        // 실제 시각으로 파싱해 내림차순(최신 우선) 정렬한다. 시각이 없거나 파싱
        // 실패한 세션은 뒤로 보낸다. 키 파싱은 세션당 1회만 수행한다.
        sessions.sort_by_cached_key(|session| std::cmp::Reverse(updated_at_key(session)));
        if let Some(limit) = limit {
            sessions.truncate(limit);
        }
        Ok(sessions)
    }
}

fn updated_at_key(session: &ProviderSession) -> Option<DateTime<chrono::FixedOffset>> {
    session
        .updated_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubRepository(Vec<ProviderSession>);

    impl ProviderSessionRepository for StubRepository {
        fn list(&self, _agent_id: &str, _scope: &SessionScope) -> Result<Vec<ProviderSession>> {
            Ok(self.0.clone())
        }
    }

    fn session(id: &str, updated_at: Option<&str>) -> ProviderSession {
        ProviderSession {
            agent_id: "codex".to_string(),
            id: id.to_string(),
            cwd: None,
            title: None,
            file: format!("/tmp/{id}.jsonl"),
            message_count: 0,
            created_at: None,
            updated_at: updated_at.map(str::to_string),
            model: None,
            branch: None,
            source: None,
        }
    }

    #[test]
    fn sorts_newest_first_across_timezones() {
        // 2026-06-01T00:30:00+09:00 == 2026-05-31T15:30:00Z 이므로
        // 사전식이라면 "2026-06-01..."이 앞서지만, 실제로는 "Z"가 더 최신이다.
        let repo = StubRepository(vec![
            session("kst", Some("2026-06-01T00:30:00+09:00")),
            session("utc", Some("2026-05-31T16:00:00Z")),
        ]);
        let result = ListProviderSessionsUseCase::new(repo)
            .execute("codex", &SessionScope::All, None)
            .unwrap();
        assert_eq!(result[0].id, "utc");
        assert_eq!(result[1].id, "kst");
    }

    #[test]
    fn sessions_without_timestamp_sort_last() {
        let repo = StubRepository(vec![
            session("none", None),
            session("dated", Some("2026-06-01T00:00:00Z")),
        ]);
        let result = ListProviderSessionsUseCase::new(repo)
            .execute("codex", &SessionScope::All, None)
            .unwrap();
        assert_eq!(result[0].id, "dated");
        assert_eq!(result[1].id, "none");
    }

    #[test]
    fn respects_limit() {
        let repo = StubRepository(vec![
            session("a", Some("2026-06-03T00:00:00Z")),
            session("b", Some("2026-06-02T00:00:00Z")),
            session("c", Some("2026-06-01T00:00:00Z")),
        ]);
        let result = ListProviderSessionsUseCase::new(repo)
            .execute("codex", &SessionScope::All, Some(2))
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "a");
        assert_eq!(result[1].id, "b");
    }
}
