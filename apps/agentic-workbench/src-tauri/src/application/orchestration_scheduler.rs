//! Capacity-aware orchestration worker scheduler.

use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use crate::domain::agent_orchestration::{OrchestrationError, OrchestrationErrorCode};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LeaseOutcome {
    Acquired,
    Queued { position: usize },
}

#[derive(Clone)]
pub struct OrchestrationScheduler {
    capacity: usize,
    state: Arc<Mutex<SchedulerState>>,
}

#[derive(Default)]
struct SchedulerState {
    active: HashSet<String>,
    queued: VecDeque<String>,
}

impl OrchestrationScheduler {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Arc::new(Mutex::new(SchedulerState::default())),
        }
    }

    pub fn acquire(&self, task_id: &str) -> Result<LeaseOutcome, OrchestrationError> {
        let mut state = self.lock()?;
        if state.active.contains(task_id) {
            return Ok(LeaseOutcome::Acquired);
        }
        if let Some(position) = state.queued.iter().position(|queued| queued == task_id) {
            return Ok(LeaseOutcome::Queued {
                position: position + 1,
            });
        }
        if state.active.len() < self.capacity {
            state.active.insert(task_id.into());
            return Ok(LeaseOutcome::Acquired);
        }
        state.queued.push_back(task_id.into());
        Ok(LeaseOutcome::Queued {
            position: state.queued.len(),
        })
    }

    pub fn release(&self, task_id: &str) -> Result<Option<String>, OrchestrationError> {
        let mut state = self.lock()?;
        state.active.remove(task_id);
        state.queued.retain(|queued| queued != task_id);
        let next = if state.active.len() < self.capacity {
            state.queued.pop_front()
        } else {
            None
        };
        if let Some(next_task_id) = &next {
            state.active.insert(next_task_id.clone());
        }
        Ok(next)
    }

    pub fn active_count(&self) -> Result<usize, OrchestrationError> {
        Ok(self.lock()?.active.len())
    }

    pub fn queued_count(&self) -> Result<usize, OrchestrationError> {
        Ok(self.lock()?.queued.len())
    }

    pub fn reconcile(
        &self,
        active_task_ids: &[String],
        ready_task_ids: &[String],
    ) -> Result<(), OrchestrationError> {
        let mut state = self.lock()?;
        state.active.clear();
        state.queued.clear();
        for task_id in active_task_ids {
            state.active.insert(task_id.clone());
        }
        for task_id in ready_task_ids {
            if !state.active.contains(task_id) && !state.queued.contains(task_id) {
                state.queued.push_back(task_id.clone());
            }
        }
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, SchedulerState>, OrchestrationError> {
        self.state.lock().map_err(|_| {
            OrchestrationError::new(
                OrchestrationErrorCode::WorkerUnavailable,
                "Orchestration scheduler is unavailable.",
            )
            .retryable()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_fifo_leases_without_exceeding_capacity() {
        let scheduler = OrchestrationScheduler::new(2);
        assert_eq!(scheduler.acquire("task-a").unwrap(), LeaseOutcome::Acquired);
        assert_eq!(scheduler.acquire("task-b").unwrap(), LeaseOutcome::Acquired);
        assert_eq!(
            scheduler.acquire("task-c").unwrap(),
            LeaseOutcome::Queued { position: 1 }
        );
        assert_eq!(scheduler.active_count().unwrap(), 2);

        assert_eq!(scheduler.release("task-a").unwrap(), Some("task-c".into()));
        assert_eq!(scheduler.active_count().unwrap(), 2);
        assert_eq!(scheduler.queued_count().unwrap(), 0);
    }

    #[test]
    fn deduplicates_active_and_queued_task_ids() {
        let scheduler = OrchestrationScheduler::new(1);
        assert_eq!(scheduler.acquire("task-a").unwrap(), LeaseOutcome::Acquired);
        assert_eq!(scheduler.acquire("task-a").unwrap(), LeaseOutcome::Acquired);
        assert_eq!(
            scheduler.acquire("task-b").unwrap(),
            LeaseOutcome::Queued { position: 1 }
        );
        assert_eq!(
            scheduler.acquire("task-b").unwrap(),
            LeaseOutcome::Queued { position: 1 }
        );
    }

    #[test]
    fn rebuilds_transfer_leases_from_durable_runtime_state() {
        let scheduler = OrchestrationScheduler::new(2);
        scheduler.acquire("stale-task").unwrap();
        scheduler
            .reconcile(
                &["running-a".into(), "running-b".into()],
                &["ready-c".into(), "ready-c".into()],
            )
            .unwrap();
        assert_eq!(scheduler.active_count().unwrap(), 2);
        assert_eq!(scheduler.queued_count().unwrap(), 1);
        assert_eq!(
            scheduler.release("running-a").unwrap(),
            Some("ready-c".into())
        );
    }
}
