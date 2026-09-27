//! 작업대 registry(040, research R1). 메모리 전용 — 재시작하면 사라진다(Q3).
//!
//! # 입장(admission)과 닫기
//!
//! 작업대 아래 새 자원을 등록하는 동작(`run.start`의 소유 기록, 교환 쓰기, 과도기 orchestration 기동)은
//! 입장권을 잡는다: registry lock 안에서 `Open`을 확인하고 같은 lock 안에서 `try_read_owned()`로 read guard를
//! 얻은 뒤, lock 밖에서 await한다(std lock을 쥔 채 기다리지 않는다). 닫기는 lock 안에서 `Open → Closing`으로
//! 바꾼 뒤 write guard를 기다린다 — 이미 입장한 동작이 끝날 때까지. `Closing` 이후의 입장은 `NotFound`다.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use chrono::Utc;
use tokio::sync::{watch, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use workbench_protocol::PrincipalSubject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchLimits {
    pub max_benches: usize,
}

impl Default for BenchLimits {
    fn default() -> Self {
        Self { max_benches: 256 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchView {
    pub id: String,
    pub working_directory: String,
    pub opened_by: PrincipalSubject,
    pub opened_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchError {
    /// 없음·닫히는 중·닫힘.
    NotFound,
    /// 연 주체가 아님.
    Forbidden,
    /// 작업대 수 상한.
    Limit,
}

enum BenchState {
    Open,
    /// 닫기가 진행 중. 두 번째 닫기는 이 채널이 `true`가 될 때까지 기다린다.
    Closing(watch::Receiver<bool>),
}

struct BenchRecord {
    view: BenchView,
    state: BenchState,
    admission: Arc<RwLock<()>>,
}

/// 입장권. drop하면 입장이 끝난다(닫기가 진행될 수 있다).
pub struct BenchAdmission {
    pub bench: BenchView,
    _guard: OwnedRwLockReadGuard<()>,
}

/// 닫기 첫 단계의 결과.
pub enum CloseStart {
    /// 이 호출이 닫기를 시작했다. `wait_admissions` → 정리 → `finish_close` 순서로 진행한다.
    Started(CloseTicket),
    /// 다른 닫기가 진행 중이다. 끝날 때까지 기다린 뒤 `closed: false`로 답한다.
    InProgress(watch::Receiver<bool>),
    /// 모르는(또는 이미 닫힌) 작업대.
    Unknown,
}

pub struct CloseTicket {
    pub bench: BenchView,
    admission: Arc<RwLock<()>>,
    done: watch::Sender<bool>,
}

impl CloseTicket {
    /// 이미 입장한 동작이 모두 끝날 때까지 기다린다(새 입장은 `Closing` 때문에 불가).
    pub async fn wait_admissions(&self) -> OwnedRwLockWriteGuard<()> {
        Arc::clone(&self.admission).write_owned().await
    }
}

#[derive(Default)]
pub struct InMemoryBenchRegistry {
    limits: BenchLimits,
    benches: Mutex<HashMap<String, BenchRecord>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl InMemoryBenchRegistry {
    pub fn new(limits: BenchLimits) -> Self {
        Self {
            limits,
            benches: Mutex::default(),
        }
    }

    /// `working_directory`는 호출자가 정규화한 실제 경로다.
    pub fn open(
        &self,
        working_directory: String,
        opened_by: PrincipalSubject,
    ) -> Result<BenchView, BenchError> {
        let mut benches = lock(&self.benches);
        if benches.len() >= self.limits.max_benches {
            return Err(BenchError::Limit);
        }
        let view = BenchView {
            id: uuid::Uuid::new_v4().to_string(),
            working_directory,
            opened_by,
            opened_at: Utc::now().to_rfc3339(),
        };
        benches.insert(
            view.id.clone(),
            BenchRecord {
                view: view.clone(),
                state: BenchState::Open,
                admission: Arc::new(RwLock::new(())),
            },
        );
        Ok(view)
    }

    fn check<'a>(
        record: Option<&'a BenchRecord>,
        subject: &PrincipalSubject,
    ) -> Result<&'a BenchRecord, BenchError> {
        let record = record.ok_or(BenchError::NotFound)?;
        if !matches!(record.state, BenchState::Open) {
            return Err(BenchError::NotFound);
        }
        if &record.view.opened_by != subject {
            return Err(BenchError::Forbidden);
        }
        Ok(record)
    }

    /// 입장 없이 작업대를 확인한다(기존 run 제어·조회).
    pub fn resolve(&self, id: &str, subject: &PrincipalSubject) -> Result<BenchView, BenchError> {
        let benches = lock(&self.benches);
        Self::check(benches.get(id), subject).map(|record| record.view.clone())
    }

    /// 주체 검사 없이 열린 작업대를 찾는다(agent principal이 run의 소유 작업대를 찾을 때).
    pub fn resolve_any(&self, id: &str) -> Result<BenchView, BenchError> {
        let benches = lock(&self.benches);
        match benches.get(id) {
            Some(record) if matches!(record.state, BenchState::Open) => Ok(record.view.clone()),
            _ => Err(BenchError::NotFound),
        }
    }

    /// 입장권을 얻는다. `subject`가 `None`이면 주체 검사를 하지 않는다(agent·과도기 orchestration).
    pub fn admit(
        &self,
        id: &str,
        subject: Option<&PrincipalSubject>,
    ) -> Result<BenchAdmission, BenchError> {
        let benches = lock(&self.benches);
        let record = match subject {
            Some(subject) => Self::check(benches.get(id), subject)?,
            None => match benches.get(id) {
                Some(record) if matches!(record.state, BenchState::Open) => record,
                _ => return Err(BenchError::NotFound),
            },
        };
        // `Open`인 동안에는 대기 중인 writer(닫기)가 없으므로 실패하지 않는다. 실패하면 닫히는 중으로 본다.
        let guard = Arc::clone(&record.admission)
            .try_read_owned()
            .map_err(|_| BenchError::NotFound)?;
        Ok(BenchAdmission {
            bench: record.view.clone(),
            _guard: guard,
        })
    }

    pub fn begin_close(
        &self,
        id: &str,
        subject: &PrincipalSubject,
    ) -> Result<CloseStart, BenchError> {
        let mut benches = lock(&self.benches);
        let Some(record) = benches.get_mut(id) else {
            return Ok(CloseStart::Unknown);
        };
        if &record.view.opened_by != subject {
            return Err(BenchError::Forbidden);
        }
        match &record.state {
            BenchState::Closing(done) => Ok(CloseStart::InProgress(done.clone())),
            BenchState::Open => {
                let (done, receiver) = watch::channel(false);
                record.state = BenchState::Closing(receiver);
                Ok(CloseStart::Started(CloseTicket {
                    bench: record.view.clone(),
                    admission: Arc::clone(&record.admission),
                    done,
                }))
            }
        }
    }

    /// 정리를 마치고 registry에서 뺀다. 기다리던 두 번째 닫기를 깨운다.
    pub fn finish_close(&self, ticket: CloseTicket) {
        lock(&self.benches).remove(&ticket.bench.id);
        let _ = ticket.done.send(true);
    }

    pub fn len(&self) -> usize {
        lock(&self.benches).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 진행 중인 닫기가 끝날 때까지 기다린다.
pub async fn wait_closed(mut done: watch::Receiver<bool>) {
    while !*done.borrow() {
        if done.changed().await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn subject(name: &str) -> PrincipalSubject {
        PrincipalSubject::new(name)
    }

    #[test]
    fn open_resolve_and_subject_checks() {
        let registry = InMemoryBenchRegistry::new(BenchLimits { max_benches: 1 });
        let bench = registry.open("/w".into(), subject("desktop")).unwrap();
        assert_eq!(
            registry
                .resolve(&bench.id, &subject("desktop"))
                .unwrap()
                .working_directory,
            "/w"
        );
        assert_eq!(
            registry.resolve(&bench.id, &subject("test:x")),
            Err(BenchError::Forbidden)
        );
        assert_eq!(
            registry.resolve("nope", &subject("desktop")),
            Err(BenchError::NotFound)
        );
        assert_eq!(
            registry.open("/w2".into(), subject("desktop")),
            Err(BenchError::Limit)
        );
        assert!(registry.admit(&bench.id, Some(&subject("test:x"))).is_err());
        assert!(registry.admit(&bench.id, None).is_ok());
    }

    #[tokio::test]
    async fn close_waits_for_admissions_and_rejects_new_ones() {
        let registry = Arc::new(InMemoryBenchRegistry::default());
        let desktop = subject("desktop");
        let bench = registry.open("/w".into(), desktop.clone()).unwrap();
        let admission = registry.admit(&bench.id, Some(&desktop)).unwrap();

        let CloseStart::Started(ticket) = registry.begin_close(&bench.id, &desktop).unwrap() else {
            panic!("close should start");
        };
        // 닫히는 중에는 새 입장·조회가 없다.
        assert!(matches!(
            registry.admit(&bench.id, Some(&desktop)),
            Err(BenchError::NotFound)
        ));
        assert_eq!(
            registry.resolve(&bench.id, &desktop),
            Err(BenchError::NotFound)
        );
        // 두 번째 닫기는 진행 중 신호를 받는다.
        let CloseStart::InProgress(done) = registry.begin_close(&bench.id, &desktop).unwrap()
        else {
            panic!("second close should wait");
        };

        let write = tokio::spawn(async move {
            let guard = ticket.wait_admissions().await;
            drop(guard);
            ticket
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            !write.is_finished(),
            "close must wait for the admitted operation"
        );
        drop(admission);
        let ticket = write.await.unwrap();
        registry.finish_close(ticket);
        wait_closed(done).await;
        assert!(registry.is_empty());
        assert!(matches!(
            registry.begin_close(&bench.id, &desktop).unwrap(),
            CloseStart::Unknown
        ));
    }
}
