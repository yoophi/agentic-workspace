//! Capacity-aware orchestration worker scheduler.
//!
//! 자리(slot)는 task별이다. Codex r8: 같은 task의 자리를 여러 배정 시도가 동시에 얻을 수 있으므로(되돌리는 중인 앞 기동 A와
//! 새 배정 B), 자리마다 **시도 보유(hold)** 를 따로 센다. 기동 시도는 [`OrchestrationScheduler::acquire_hold`]로 보유를 얻고,
//! 자기 기동이 성공하면 [`OrchestrationScheduler::transfer`]로 실행 중 자리로 넘기며, 그 밖의 모든 끝(실패·abort 정리·다른
//! 기동의 run 반환)은 [`OrchestrationScheduler::release_hold`]로 **자기 보유만** 놓는다. 보유가 모두 빠지고 실행 중도 아니면
//! 자리가 비고 다음 대기 task가 승격된다. task가 끝나는 경로(결과 보고·취소)는 [`OrchestrationScheduler::release`]로 자리를
//! 통째로 비운다.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use crate::domain::agent_orchestration::{OrchestrationError, OrchestrationErrorCode};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LeaseOutcome {
    Acquired,
    Queued { position: usize },
}

/// 한 기동 시도가 쥔 자리 보유. 복제하지 않는다(한 번만 놓거나 넘긴다).
#[derive(Debug, Eq, PartialEq)]
pub struct SlotHold {
    task_id: String,
    id: u64,
}

impl SlotHold {
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum HoldOutcome {
    Acquired(SlotHold),
    Queued { position: usize },
}

#[derive(Clone)]
pub struct OrchestrationScheduler {
    capacity: usize,
    state: Arc<Mutex<SchedulerState>>,
}

#[derive(Default)]
struct Slot {
    /// 진행 중 기동 시도의 보유.
    holds: HashSet<u64>,
    /// 기동이 성공해 실행 중인 task의 자리(보유가 없어도 남는다 — task 끝에서 `release`가 비운다).
    running: bool,
}

#[derive(Default)]
struct SchedulerState {
    active: HashMap<String, Slot>,
    queued: VecDeque<String>,
    next_hold: u64,
}

impl SchedulerState {
    fn hold(&mut self, task_id: &str) -> SlotHold {
        self.next_hold += 1;
        let id = self.next_hold;
        self.active
            .entry(task_id.to_owned())
            .or_default()
            .holds
            .insert(id);
        SlotHold {
            task_id: task_id.to_owned(),
            id,
        }
    }

    /// 자리 하나가 비었다: 한도 안이면 다음 대기 task를 승격한다(보유 없는 예약 자리 — 그 task의 배정이 보유를 얻는다).
    fn promote(&mut self, capacity: usize) -> Option<String> {
        let next = if self.active.len() < capacity {
            self.queued.pop_front()
        } else {
            None
        };
        if let Some(next_task_id) = &next {
            self.active.insert(next_task_id.clone(), Slot::default());
        }
        next
    }
}

impl OrchestrationScheduler {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Arc::new(Mutex::new(SchedulerState::default())),
        }
    }

    /// 기동 시도의 자리 보유를 얻는다. 같은 task의 자리가 이미 있으면(실행 중·다른 시도·승격 예약) 그 자리에 보유를 더한다.
    pub fn acquire_hold(&self, task_id: &str) -> Result<HoldOutcome, OrchestrationError> {
        let mut state = self.lock()?;
        if state.active.contains_key(task_id) {
            return Ok(HoldOutcome::Acquired(state.hold(task_id)));
        }
        if let Some(position) = state.queued.iter().position(|queued| queued == task_id) {
            // 복구(`reconcile`)는 Ready task를 자리와 무관하게 대기열에 넣는다. 자리가 비어 있으면 배정이 곧 시작이다
            // — 그러지 않으면 실행 중 task가 없어 `release`가 오지 않아 영원히 대기한다.
            if state.active.len() < self.capacity {
                state.queued.remove(position);
                return Ok(HoldOutcome::Acquired(state.hold(task_id)));
            }
            return Ok(HoldOutcome::Queued {
                position: position + 1,
            });
        }
        if state.active.len() < self.capacity {
            return Ok(HoldOutcome::Acquired(state.hold(task_id)));
        }
        state.queued.push_back(task_id.into());
        Ok(HoldOutcome::Queued {
            position: state.queued.len(),
        })
    }

    /// 보유를 따지지 않는 옛 획득(시험·호환): 자리를 실행 중으로 잡는다(task 끝의 `release`가 비운다).
    pub fn acquire(&self, task_id: &str) -> Result<LeaseOutcome, OrchestrationError> {
        Ok(match self.acquire_hold(task_id)? {
            HoldOutcome::Acquired(hold) => {
                self.transfer(hold);
                LeaseOutcome::Acquired
            }
            HoldOutcome::Queued { position } => LeaseOutcome::Queued { position },
        })
    }

    /// 자기 기동이 성공했다: 보유를 실행 중 자리로 넘긴다. 그 사이 task가 끝나 자리가 비워졌으면 아무것도 하지 않는다.
    pub fn transfer(&self, hold: SlotHold) {
        if let Ok(mut state) = self.lock() {
            if let Some(slot) = state.active.get_mut(&hold.task_id) {
                slot.holds.remove(&hold.id);
                slot.running = true;
            }
        }
    }

    /// 기동 시도가 끝났지만 자기 run을 실행하지 않았다: 자기 보유만 놓는다. 보유가 모두 빠지고 실행 중이 아니면 자리를 비우고
    /// 승격한 다음 task를 돌려준다. 이미 비워진 자리(task 끝)나 다른 시도의 보유에는 영향이 없다.
    pub fn release_hold(&self, hold: SlotHold) -> Option<String> {
        let mut state = self.lock().ok()?;
        let slot = state.active.get_mut(&hold.task_id)?;
        if !slot.holds.remove(&hold.id) || !slot.holds.is_empty() || slot.running {
            return None;
        }
        state.active.remove(&hold.task_id);
        state.promote(self.capacity)
    }

    pub fn release(&self, task_id: &str) -> Result<Option<String>, OrchestrationError> {
        let mut state = self.lock()?;
        state.active.remove(task_id);
        state.queued.retain(|queued| queued != task_id);
        Ok(state.promote(self.capacity))
    }

    pub fn active_count(&self) -> Result<usize, OrchestrationError> {
        Ok(self.lock()?.active.len())
    }

    /// 자리의 진행 중 기동 시도 보유 수(시험·관측).
    pub fn hold_count(&self, task_id: &str) -> usize {
        self.lock()
            .ok()
            .and_then(|state| state.active.get(task_id).map(|slot| slot.holds.len()))
            .unwrap_or(0)
    }

    pub fn queued_count(&self) -> Result<usize, OrchestrationError> {
        Ok(self.lock()?.queued.len())
    }

    pub fn reconcile(
        &self,
        active_task_ids: &[String],
        ready_task_ids: &[String],
    ) -> Result<(), OrchestrationError> {
        self.reconcile_preserving(active_task_ids, ready_task_ids, &[])
    }

    /// 복구의 재구성(Codex r10): 저장소가 본 실행 중 task(`active_task_ids`)와 준비 task(`ready_task_ids`)로 자리를 다시 짓되,
    /// **진행 중인 기동 시도의 보유는 보존**한다 — 보유를 지우면 그 시도의 정리(`release_hold`)가 자리를 찾지 못해 자리가
    /// 남고(누수), 성공 인계(`transfer`)가 자리를 찾지 못해 실행 중 run이 자리 없이 돈다(한도 초과). `launching_task_ids`
    /// (이 프로세스에서 기동 중이거나 되돌리는 중인 task)는 저장소에 `Running`으로 보여도 성공 인계 전이라 실행 중으로 확정하지
    /// 않는다(그 시도의 `transfer`가 확정한다). 보유를 쥔 자리는 대기열에 넣지 않는다.
    pub fn reconcile_preserving(
        &self,
        active_task_ids: &[String],
        ready_task_ids: &[String],
        launching_task_ids: &[String],
    ) -> Result<(), OrchestrationError> {
        let mut state = self.lock()?;
        let mut previous = std::mem::take(&mut state.active);
        state.queued.clear();
        for task_id in active_task_ids {
            let mut slot = previous.remove(task_id).unwrap_or_default();
            if !launching_task_ids.contains(task_id) {
                slot.running = true;
            }
            state.active.insert(task_id.clone(), slot);
        }
        // 저장소가 실행 중으로 보지 않는 task라도 진행 중 시도의 보유가 있으면 자리를 남긴다(실행 중 표시는 내린다 — 그 시도가
        // 성공하면 `transfer`가 다시 세운다).
        for (task_id, mut slot) in previous {
            if !slot.holds.is_empty() {
                slot.running = false;
                state.active.insert(task_id, slot);
            }
        }
        for task_id in ready_task_ids {
            if !state.active.contains_key(task_id) && !state.queued.contains(task_id) {
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

    /// 044: 복구(`reconcile`)가 대기열에 넣은 Ready task는 자리가 비어 있으면 배정(`acquire`)으로 바로 시작한다.
    /// 그러지 않으면 실행 중 task가 없어 `release`가 없으므로 영원히 대기열에 남는다(재시작 뒤 재배정 불가).
    #[test]
    fn a_reconciled_ready_task_is_acquired_when_capacity_is_free() {
        let scheduler = OrchestrationScheduler::new(1);
        scheduler
            .reconcile(&[], &["task-a".into(), "task-b".into()])
            .unwrap();
        assert_eq!(scheduler.acquire("task-b").unwrap(), LeaseOutcome::Acquired);
        assert_eq!(scheduler.active_count().unwrap(), 1);
        assert_eq!(scheduler.queued_count().unwrap(), 1);
        assert_eq!(
            scheduler.acquire("task-a").unwrap(),
            LeaseOutcome::Queued { position: 1 },
            "capacity is respected"
        );
        assert_eq!(scheduler.release("task-b").unwrap(), Some("task-a".into()));
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

    /// Codex r8: 되돌리는 중인 앞 시도 A의 보유 정리는 같은 자리에 보유를 더한 새 시도 B의 자리를 비우지 않는다.
    #[test]
    fn releasing_one_attempt_hold_keeps_the_slot_of_another_attempt() {
        let scheduler = OrchestrationScheduler::new(1);
        let HoldOutcome::Acquired(a) = scheduler.acquire_hold("task-a").unwrap() else {
            panic!("a acquires")
        };
        let HoldOutcome::Acquired(b) = scheduler.acquire_hold("task-a").unwrap() else {
            panic!("b joins the same slot")
        };
        assert_eq!(scheduler.hold_count("task-a"), 2);
        assert_eq!(scheduler.release_hold(a), None);
        assert_eq!(
            scheduler.active_count().unwrap(),
            1,
            "b still owns the slot"
        );
        assert_eq!(
            scheduler.acquire_hold("task-b").unwrap(),
            HoldOutcome::Queued { position: 1 },
            "capacity stays at one"
        );
        scheduler.transfer(b);
        assert_eq!(scheduler.hold_count("task-a"), 0);
        assert_eq!(scheduler.active_count().unwrap(), 1, "running slot remains");
        assert_eq!(scheduler.release("task-a").unwrap(), Some("task-b".into()));
    }

    /// Codex r10: 복구의 재구성은 진행 중 시도의 보유를 지우지 않는다. 성공 인계 전인(기동 중) task는 저장소가 실행 중으로
    /// 보여도 실행 중 자리로 확정하지 않으므로, 그 시도가 끝나 보유를 놓으면 자리가 빈다.
    #[test]
    fn a_recovery_keeps_an_in_flight_hold_and_does_not_confirm_it_running() {
        let scheduler = OrchestrationScheduler::new(1);
        let HoldOutcome::Acquired(hold) = scheduler.acquire_hold("task-a").unwrap() else {
            panic!("acquires")
        };
        scheduler
            .reconcile_preserving(&["task-a".into()], &[], &["task-a".into()])
            .unwrap();
        assert_eq!(scheduler.hold_count("task-a"), 1);
        assert_eq!(scheduler.release_hold(hold), None);
        assert_eq!(scheduler.active_count().unwrap(), 0, "no leaked slot");
    }

    /// 저장소가 준비(Ready)로 보는 task라도 진행 중 시도의 보유가 있으면 자리를 남기고 대기열에 넣지 않는다 — 그 시도가 성공하면
    /// `transfer`가 실행 중 자리로 확정하고 한도가 지켜진다.
    #[test]
    fn a_recovery_keeps_the_slot_of_a_ready_task_with_an_in_flight_hold() {
        let scheduler = OrchestrationScheduler::new(1);
        let HoldOutcome::Acquired(hold) = scheduler.acquire_hold("task-a").unwrap() else {
            panic!("acquires")
        };
        scheduler
            .reconcile_preserving(&[], &["task-a".into()], &["task-a".into()])
            .unwrap();
        assert_eq!(scheduler.queued_count().unwrap(), 0);
        assert_eq!(scheduler.active_count().unwrap(), 1);
        scheduler.transfer(hold);
        assert_eq!(
            scheduler.acquire_hold("task-b").unwrap(),
            HoldOutcome::Queued { position: 1 },
            "the limit holds after the transfer"
        );
    }

    /// 실행 중 자리(복구·성공한 기동)는 뒤늦은 시도가 보유를 얻었다 놓아도 비지 않는다.
    #[test]
    fn a_late_attempt_hold_does_not_free_a_running_slot() {
        let scheduler = OrchestrationScheduler::new(1);
        scheduler.reconcile(&["task-a".into()], &[]).unwrap();
        let HoldOutcome::Acquired(late) = scheduler.acquire_hold("task-a").unwrap() else {
            panic!("joins")
        };
        assert_eq!(scheduler.release_hold(late), None);
        assert_eq!(scheduler.active_count().unwrap(), 1);
    }

    /// 보유를 놓은 뒤 task 끝의 `release`가 다시 얻은 새 시도의 보유를 옛 보유가 지우지 못한다.
    #[test]
    fn a_stale_hold_after_a_full_release_does_not_touch_a_new_attempt() {
        let scheduler = OrchestrationScheduler::new(1);
        let HoldOutcome::Acquired(old) = scheduler.acquire_hold("task-a").unwrap() else {
            panic!()
        };
        scheduler.release("task-a").unwrap();
        let HoldOutcome::Acquired(new) = scheduler.acquire_hold("task-a").unwrap() else {
            panic!()
        };
        assert_eq!(scheduler.release_hold(old), None);
        assert_eq!(
            scheduler.hold_count("task-a"),
            1,
            "the new attempt keeps its hold"
        );
        assert_eq!(scheduler.release_hold(new), None);
        assert_eq!(scheduler.active_count().unwrap(), 0);
    }
}
