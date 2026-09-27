//! 구독 표(042 research R4·R5, contracts §3·§4). 30초 1회용, 발급 주체·cursor·Origin에 묶인다. 연결 때 원자적으로
//! 꺼내(take) 소모한다. 발급 때는 형식과 메모리 안전용 고정 상한만 본다 — cursor 0개·hub 상한·스트림 권한은 연결 때
//! `Workbench.events`가 판정한다(설계 리뷰 D1: hub 상한은 런타임 설정이라 여기서 알 수 없다).

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use workbench_protocol::{AuthenticatedPrincipal, PrincipalSubject, StreamCursor};

use crate::auth::{digest, lock, random_token};

pub const TICKET_TTL: Duration = Duration::from_secs(30);
pub const TICKET_CAPACITY: usize = 1_024;
/// 표 하나의 cursor 고정 상한(hub의 어떤 설정보다 크다).
pub const TICKET_MAX_CURSORS: usize = 1_024;
pub const MESSAGE_TOO_MANY_CURSORS: &str = "too many cursors for a ticket.";
pub const MESSAGE_TICKETS_EXHAUSTED: &str = "too many pending event tickets.";

#[derive(Debug, Clone)]
pub struct EventTicket {
    pub principal: AuthenticatedPrincipal,
    pub cursors: Vec<StreamCursor>,
    pub origin: Option<String>,
    expires_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueError {
    TooManyCursors,
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakeError {
    /// 없음·만료·이미 사용.
    Invalid,
    /// 발급 때와 다른 Origin(표는 소모된다).
    OriginMismatch,
}

pub struct EventTicketStore {
    entries: Mutex<HashMap<[u8; 32], EventTicket>>,
    ttl: Duration,
    capacity: usize,
}

impl Default for EventTicketStore {
    fn default() -> Self {
        Self::new(TICKET_TTL, TICKET_CAPACITY)
    }
}

impl EventTicketStore {
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
            capacity,
        }
    }

    pub fn issue(
        &self,
        principal: AuthenticatedPrincipal,
        cursors: Vec<StreamCursor>,
        origin: Option<String>,
    ) -> Result<(String, chrono::DateTime<chrono::Utc>), IssueError> {
        if cursors.len() > TICKET_MAX_CURSORS {
            return Err(IssueError::TooManyCursors);
        }
        let now = Instant::now();
        let mut entries = lock(&self.entries);
        entries.retain(|_, ticket| ticket.expires_at > now);
        if entries.len() >= self.capacity {
            return Err(IssueError::Exhausted);
        }
        let token = random_token();
        entries.insert(
            digest(&token),
            EventTicket {
                principal,
                cursors,
                origin,
                expires_at: now + self.ttl,
            },
        );
        let expires_at = chrono::Utc::now()
            + chrono::Duration::from_std(self.ttl).unwrap_or_else(|_| chrono::Duration::zero());
        Ok((token, expires_at))
    }

    /// 주체의 남은 표를 모두 지운다(043: 창 Destroyed — 폐기 전에 받은 표로 구독하지 못하게). 지운 개수.
    pub fn revoke_subject(&self, subject: &PrincipalSubject) -> usize {
        let mut entries = lock(&self.entries);
        let before = entries.len();
        entries.retain(|_, ticket| ticket.principal.subject != *subject);
        before - entries.len()
    }

    /// 원자적으로 꺼낸다 — 같은 표로 두 연결이 동시에 와도 하나만 성공한다.
    pub fn take(&self, token: &str, origin: Option<&str>) -> Result<EventTicket, TakeError> {
        let ticket = lock(&self.entries)
            .remove(&digest(token))
            .ok_or(TakeError::Invalid)?;
        if ticket.expires_at <= Instant::now() {
            return Err(TakeError::Invalid);
        }
        if ticket.origin.as_deref() != origin {
            return Err(TakeError::OriginMismatch);
        }
        Ok(ticket)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn cursor() -> StreamCursor {
        StreamCursor {
            stream_id: "run:r1".into(),
            epoch: "e".into(),
            after_sequence: 0,
        }
    }

    #[test]
    fn tickets_are_single_use() {
        let store = EventTicketStore::default();
        let (token, _) = store
            .issue(AuthenticatedPrincipal::desktop(), vec![cursor()], None)
            .unwrap();
        assert_eq!(store.take(&token, None).unwrap().cursors.len(), 1);
        assert_eq!(store.take(&token, None).unwrap_err(), TakeError::Invalid);
    }

    #[test]
    fn expired_and_origin_mismatched_tickets_fail() {
        let store = EventTicketStore::new(Duration::from_millis(0), 8);
        let (token, _) = store
            .issue(AuthenticatedPrincipal::desktop(), vec![cursor()], None)
            .unwrap();
        assert_eq!(store.take(&token, None).unwrap_err(), TakeError::Invalid);

        let store = EventTicketStore::default();
        let (token, _) = store
            .issue(
                AuthenticatedPrincipal::desktop(),
                vec![cursor()],
                Some("http://localhost:1420".into()),
            )
            .unwrap();
        assert_eq!(
            store.take(&token, Some("tauri://localhost")).unwrap_err(),
            TakeError::OriginMismatch
        );
        assert_eq!(
            store
                .take(&token, Some("http://localhost:1420"))
                .unwrap_err(),
            TakeError::Invalid,
            "a mismatched attempt consumes the ticket"
        );
    }

    #[test]
    fn shape_limits_only() {
        let store = EventTicketStore::new(TICKET_TTL, 1);
        assert!(
            store
                .issue(AuthenticatedPrincipal::desktop(), Vec::new(), None)
                .is_ok(),
            "zero cursors are judged at connect"
        );
        assert_eq!(
            store
                .issue(AuthenticatedPrincipal::desktop(), vec![cursor()], None)
                .unwrap_err(),
            IssueError::Exhausted
        );
        let store = EventTicketStore::default();
        assert_eq!(
            store
                .issue(
                    AuthenticatedPrincipal::desktop(),
                    vec![cursor(); TICKET_MAX_CURSORS + 1],
                    None
                )
                .unwrap_err(),
            IssueError::TooManyCursors
        );
    }

    #[test]
    fn concurrent_takes_succeed_once() {
        for _ in 0..200 {
            let store = Arc::new(EventTicketStore::default());
            let (token, _) = store
                .issue(AuthenticatedPrincipal::desktop(), vec![cursor()], None)
                .unwrap();
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let (store, token) = (Arc::clone(&store), token.clone());
                    std::thread::spawn(move || store.take(&token, None).is_ok())
                })
                .collect();
            let wins = handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .filter(|won| *won)
                .count();
            assert_eq!(wins, 1);
        }
    }
}
