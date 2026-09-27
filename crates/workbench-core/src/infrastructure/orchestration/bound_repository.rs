//! 묶임 저장소(041 research R3): 영속 저장소를 감싸 작업대 묶임(메모리)을 세션의 `bound_bench_id`로 채우고,
//! commit 때 바뀐 묶임을 묶임 표에 반영한다. binding mutex를 저장소 경계보다 먼저 잡아 transaction이 끝날 때까지
//! 쥔다. 표 갱신은 파일 저장이 성공한 뒤에만 한다(저장 실패 시 표와 파일이 어긋나지 않게).

use std::sync::{Arc, Mutex, MutexGuard};

use crate::{
    application::orchestration::{
        binding::{BindingChange, BindingTable, OrchestrationBindings},
        revision_watch::RevisionWatch,
    },
    domain::agent_orchestration::{OrchestrationError, OrchestrationSession},
    ports::orchestration_repository::{OrchestrationRepository, OrchestrationTransaction},
};

/// 묶임 변화를 받는 쪽(스트림 수명·발행). commit 안(binding mutex를 쥔 채)에서 불리므로 빠르게 끝나야 하고
/// 저장소·binding mutex를 다시 잡으면 안 된다.
pub type BindingObserver = Arc<dyn Fn(&[BindingChange]) + Send + Sync>;

pub struct BoundOrchestrationRepository<R> {
    inner: R,
    bindings: Arc<OrchestrationBindings>,
    observer: Arc<Mutex<Option<BindingObserver>>>,
    /// commit이 바꾼 작업 영역을 알린다(research R12, `waitChildTasks`).
    revisions: Arc<RevisionWatch>,
}

impl<R: Clone> Clone for BoundOrchestrationRepository<R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            bindings: Arc::clone(&self.bindings),
            observer: Arc::clone(&self.observer),
            revisions: Arc::clone(&self.revisions),
        }
    }
}

impl<R> BoundOrchestrationRepository<R> {
    pub fn new(inner: R, bindings: Arc<OrchestrationBindings>) -> Self {
        Self {
            inner,
            bindings,
            observer: Arc::new(Mutex::new(None)),
            revisions: Arc::new(RevisionWatch::default()),
        }
    }

    pub fn revisions(&self) -> &Arc<RevisionWatch> {
        &self.revisions
    }

    pub fn bindings(&self) -> &Arc<OrchestrationBindings> {
        &self.bindings
    }

    pub fn set_observer(&self, observer: BindingObserver) {
        *self
            .observer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(observer);
    }
}

fn fill(sessions: &mut [OrchestrationSession], table: &BindingTable) {
    for session in sessions {
        session.bound_bench_id = table
            .binding_of(&session.id)
            .map(|binding| binding.bench_id.clone());
    }
}

impl<R: OrchestrationRepository> OrchestrationRepository for BoundOrchestrationRepository<R> {
    type Tx<'a>
        = BoundTx<'a, R::Tx<'a>>
    where
        Self: 'a;

    fn begin(&self) -> Result<Self::Tx<'_>, OrchestrationError> {
        let table = self.bindings.lock();
        let mut inner = self.inner.begin()?;
        fill(inner.sessions(), &table);
        let loaded = inner
            .sessions()
            .iter()
            .map(|session| (session.id.clone(), session.revision))
            .collect();
        let observer = self
            .observer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        Ok(BoundTx {
            table,
            inner,
            observer,
            loaded,
            revisions: Arc::clone(&self.revisions),
        })
    }

    fn snapshot(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
        let table = self.bindings.lock();
        let mut sessions = self.inner.snapshot()?;
        fill(&mut sessions, &table);
        Ok(sessions)
    }
}

pub struct BoundTx<'a, T> {
    table: MutexGuard<'a, BindingTable>,
    inner: T,
    observer: Option<BindingObserver>,
    /// begin 때 읽은 작업 영역별 revision(commit 뒤 바뀐 것만 알린다).
    loaded: std::collections::HashMap<String, u64>,
    revisions: Arc<RevisionWatch>,
}

impl<T: OrchestrationTransaction> OrchestrationTransaction for BoundTx<'_, T> {
    fn sessions(&mut self) -> &mut Vec<OrchestrationSession> {
        self.inner.sessions()
    }

    fn commit(mut self) -> Result<(), OrchestrationError> {
        let desired: Vec<(String, Option<String>)> = self
            .inner
            .sessions()
            .iter()
            .map(|session| (session.id.clone(), session.bound_bench_id.clone()))
            .collect();
        let changed: Vec<(String, u64)> = self
            .inner
            .sessions()
            .iter()
            .filter(|session| self.loaded.get(&session.id) != Some(&session.revision))
            .map(|session| (session.id.clone(), session.revision))
            .collect();
        self.inner.commit()?;
        for (workspace_id, revision) in &changed {
            self.revisions.notify(workspace_id, *revision);
        }
        let mut changes = Vec::new();
        for (workspace_id, bench_id) in desired {
            changes.extend(self.table.set(&workspace_id, bench_id.as_deref()));
        }
        if let (Some(observer), false) = (&self.observer, changes.is_empty()) {
            observer(&changes);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::orchestration::memory_store::InMemoryOrchestrationRepository;

    fn session(id: &str, bench: &str) -> OrchestrationSession {
        OrchestrationSession::new(id, format!("/repo/{id}"), bench, "2026-09-27T00:00:00Z")
    }

    #[test]
    fn bindings_live_in_memory_and_survive_a_reload() {
        let bindings = Arc::new(OrchestrationBindings::default());
        let repo = BoundOrchestrationRepository::new(
            InMemoryOrchestrationRepository::default(),
            Arc::clone(&bindings),
        );
        let mut tx = repo.begin().unwrap();
        tx.sessions().push(session("w1", "bench-a"));
        tx.commit().unwrap();
        assert_eq!(
            bindings.binding_of("w1").map(|binding| binding.bench_id),
            Some("bench-a".to_owned())
        );
        assert_eq!(
            repo.snapshot().unwrap()[0].bound_bench_id.as_deref(),
            Some("bench-a")
        );

        // 새 묶임 표(= 서버 재시작): 같은 저장 내용이 복구 가능으로 보인다.
        let restarted = BoundOrchestrationRepository::new(
            InMemoryOrchestrationRepository::from_sessions(repo.snapshot().unwrap()),
            Arc::new(OrchestrationBindings::default()),
        );
        assert_eq!(restarted.snapshot().unwrap()[0].bound_bench_id, None);
    }

    #[test]
    fn a_dropped_transaction_changes_neither_store_nor_bindings() {
        let bindings = Arc::new(OrchestrationBindings::default());
        let repo = BoundOrchestrationRepository::new(
            InMemoryOrchestrationRepository::default(),
            Arc::clone(&bindings),
        );
        {
            let mut tx = repo.begin().unwrap();
            tx.sessions().push(session("w1", "bench-a"));
        }
        assert!(repo.snapshot().unwrap().is_empty());
        assert!(bindings.binding_of("w1").is_none());
    }
}
