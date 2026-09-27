//! 클라이언트 임대(044 research R9, data-model Lease). 붙은 클라이언트가 만료 시간과 함께 잡는 서버 유지 요청이다.
//! 유휴 정지 판정(임대 0 + 활동 작업 0이 유휴 시간 동안)은 T041에서 이 표를 읽는다.

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

use workbench_protocol::operations::lease::LeaseClientKindDto;

/// 임대 TTL. 데스크톱은 10초마다 갱신한다(R9).
pub const LEASE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct Lease {
    client_kind: LeaseClientKindDto,
    #[allow(dead_code)]
    client_id: String,
    expires_at: Instant,
}

#[derive(Debug)]
pub struct LeaseTable {
    ttl: Duration,
    leases: Mutex<HashMap<String, Lease>>,
}

impl Default for LeaseTable {
    fn default() -> Self {
        Self::with_ttl(LEASE_TTL)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl LeaseTable {
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            ttl,
            leases: Mutex::new(HashMap::new()),
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    fn prune(leases: &mut HashMap<String, Lease>, now: Instant) {
        leases.retain(|_, lease| lease.expires_at > now);
    }

    pub fn acquire(&self, client_kind: LeaseClientKindDto, client_id: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Instant::now();
        let mut leases = lock(&self.leases);
        Self::prune(&mut leases, now);
        leases.insert(
            id.clone(),
            Lease {
                client_kind,
                client_id,
                expires_at: now + self.ttl,
            },
        );
        id
    }

    /// 만료 전이면 연장하고 true. 없거나 만료됐으면 false.
    pub fn renew(&self, lease_id: &str) -> bool {
        let now = Instant::now();
        let mut leases = lock(&self.leases);
        Self::prune(&mut leases, now);
        match leases.get_mut(lease_id) {
            Some(lease) => {
                lease.expires_at = now + self.ttl;
                true
            }
            None => false,
        }
    }

    /// 이 호출이 임대를 풀었으면 true.
    pub fn release(&self, lease_id: &str) -> bool {
        lock(&self.leases).remove(lease_id).is_some()
    }

    /// 이 종류의 유효 임대 수. 미소비 교환은 데스크톱 임대가 있을 때만 활동으로 센다(R7).
    pub fn count_kind(&self, kind: LeaseClientKindDto) -> usize {
        let mut leases = lock(&self.leases);
        Self::prune(&mut leases, Instant::now());
        leases
            .values()
            .filter(|lease| lease.client_kind == kind)
            .count()
    }

    /// 유효 임대 수(만료된 것은 거둔다).
    pub fn count(&self) -> usize {
        let mut leases = lock(&self.leases);
        Self::prune(&mut leases, Instant::now());
        leases.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_leases_are_reaped_and_cannot_be_renewed() {
        let table = LeaseTable::with_ttl(Duration::from_millis(0));
        let id = table.acquire(LeaseClientKindDto::Test, "c".into());
        assert_eq!(table.count(), 0, "a zero TTL lease is already expired");
        assert!(!table.renew(&id));
        assert!(!table.release(&id));
    }
}
