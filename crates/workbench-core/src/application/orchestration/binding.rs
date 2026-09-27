//! 작업 영역 ↔ 작업대 묶임 표(041 research R3). 메모리 상태이며 서버가 재시작하면 비어 모든 작업 영역이 복구
//! 가능해진다. 묶일 때마다 새 묶임 id가 생기고, 그 id가 orchestration 스트림의 key다(FR-009).
//!
//! 표의 mutex가 R3의 **binding mutex**다 — 묶임 저장소(`BoundOrchestrationRepository`)가 저장소 경계보다 먼저 잡고
//! transaction이 끝날 때까지 쥔다(순서: binding mutex → 저장소 경계). 그래서 "찾기/만들기 + 이미 묶임 검사 + 표
//! 갱신"이 작업 영역 id를 아직 모르는 bootstrap·recover에서도 한 번에 일어난다.

use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub bench_id: String,
    pub binding_id: String,
}

/// 묶임 변화. 스트림 수명(생성·제거)과 발행 대상을 정한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingChange {
    Bound {
        workspace_id: String,
        binding: Binding,
    },
    Unbound {
        workspace_id: String,
        binding: Binding,
    },
}

#[derive(Debug, Default)]
pub struct BindingTable {
    by_workspace: HashMap<String, Binding>,
    by_bench: HashMap<String, String>,
}

impl BindingTable {
    pub fn binding_of(&self, workspace_id: &str) -> Option<&Binding> {
        self.by_workspace.get(workspace_id)
    }

    pub fn workspace_of_bench(&self, bench_id: &str) -> Option<&str> {
        self.by_bench.get(bench_id).map(String::as_str)
    }

    /// 작업 영역의 묶임을 `bench_id`로 맞추고 변화를 돌려준다. 같은 작업대면 변화 없음(묶임 id 유지).
    pub fn set(&mut self, workspace_id: &str, bench_id: Option<&str>) -> Vec<BindingChange> {
        let current = self.by_workspace.get(workspace_id).cloned();
        if current.as_ref().map(|binding| binding.bench_id.as_str()) == bench_id {
            return Vec::new();
        }
        let mut changes = Vec::new();
        if let Some(old) = current {
            self.by_workspace.remove(workspace_id);
            if self.by_bench.get(&old.bench_id).map(String::as_str) == Some(workspace_id) {
                self.by_bench.remove(&old.bench_id);
            }
            changes.push(BindingChange::Unbound {
                workspace_id: workspace_id.to_owned(),
                binding: old,
            });
        }
        if let Some(bench_id) = bench_id {
            let binding = Binding {
                bench_id: bench_id.to_owned(),
                binding_id: uuid::Uuid::new_v4().to_string(),
            };
            self.by_workspace
                .insert(workspace_id.to_owned(), binding.clone());
            self.by_bench
                .insert(bench_id.to_owned(), workspace_id.to_owned());
            changes.push(BindingChange::Bound {
                workspace_id: workspace_id.to_owned(),
                binding,
            });
        }
        changes
    }
}

#[derive(Debug, Default)]
pub struct OrchestrationBindings {
    table: Mutex<BindingTable>,
}

impl OrchestrationBindings {
    /// binding mutex를 잡는다. 저장소 경계보다 먼저 잡아야 한다.
    pub fn lock(&self) -> MutexGuard<'_, BindingTable> {
        self.table
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn binding_of(&self, workspace_id: &str) -> Option<Binding> {
        self.lock().binding_of(workspace_id).cloned()
    }

    pub fn workspace_of_bench(&self, bench_id: &str) -> Option<String> {
        self.lock().workspace_of_bench(bench_id).map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebinding_to_another_bench_issues_a_new_binding_id() {
        let mut table = BindingTable::default();
        let first = table.set("w1", Some("a"));
        let BindingChange::Bound {
            binding: bound_a, ..
        } = &first[0]
        else {
            panic!("expected bound");
        };
        assert!(
            table.set("w1", Some("a")).is_empty(),
            "same bench keeps the binding"
        );
        let moved = table.set("w1", Some("b"));
        assert!(matches!(&moved[0], BindingChange::Unbound { binding, .. } if binding == bound_a));
        let BindingChange::Bound {
            binding: bound_b, ..
        } = &moved[1]
        else {
            panic!("expected bound");
        };
        assert_ne!(bound_a.binding_id, bound_b.binding_id);
        assert_eq!(table.workspace_of_bench("a"), None);
        assert_eq!(table.workspace_of_bench("b"), Some("w1"));
        let released = table.set("w1", None);
        assert!(matches!(&released[0], BindingChange::Unbound { .. }));
        assert!(table.binding_of("w1").is_none());
    }
}
