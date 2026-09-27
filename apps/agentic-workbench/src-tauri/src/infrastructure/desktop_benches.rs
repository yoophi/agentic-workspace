//! 데스크톱 창 ↔ 작업대(Bench) 대응(040, research R10). 창 label은 이 표 밖으로 나가지 않는다 — 서버 계약·도메인은
//! 작업대 id만 안다(ADR core 0004). 창마다 작업대를 **처음 쓸 때** 열고, 창이 `Destroyed`되면 닫는다(ADR 0005).
//!
//! 041 전까지는 AW에 남은 orchestration·교환 표시 경로가 창 label과 작업대를 서로 찾아야 하므로
//! `window_manager`처럼 프로세스 전역 표로 둔다.
//!
//! 열기와 닫기는 창 label별 lock으로 직렬화한다: 열기가 `bench.open`을 기다리는 동안 창이 닫히면, 닫기는 열기가
//! 끝나(대응 등록) 기다린 뒤 그 작업대를 닫는다. 닫힌 창에서는 새로 열지 않는다(창 label은 재사용되지 않는다).

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};

use serde_json::json;
use workbench_core::application::workbench_runtime::WorkbenchRuntime;
use workbench_protocol::{
    AuthenticatedPrincipal, CallReply, OperationId, Workbench, operations::bench::BenchOpenOutput,
};

use crate::inbound::workbench_compat;

pub const MESSAGE_WINDOW_UNAVAILABLE: &str = "Owner Worktree Session window is unavailable.";

#[derive(Default)]
struct Table {
    by_label: HashMap<String, String>,
    by_bench: HashMap<String, String>,
    closed_labels: HashSet<String>,
    locks: HashMap<String, Arc<tokio::sync::Mutex<()>>>,
}

fn table() -> MutexGuard<'static, Table> {
    static TABLE: OnceLock<Mutex<Table>> = OnceLock::new();
    TABLE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn label_lock(label: &str) -> Arc<tokio::sync::Mutex<()>> {
    Arc::clone(table().locks.entry(label.to_owned()).or_default())
}

/// 창의 작업대. 없으면 `None`(그 창이 소유한 run·교환도 없다).
pub fn lookup(label: &str) -> Option<String> {
    table().by_label.get(label).cloned()
}

/// 작업대를 연 창.
pub fn label_for(bench_id: &str) -> Option<String> {
    table().by_bench.get(bench_id).cloned()
}

/// 창의 작업대를 돌려주고, 없으면 연다. 경로는 창의 Worktree(없으면 `hint`: run 요청 `cwd`·교환 `worktreePath`).
pub async fn ensure(
    caller: &workbench_compat::Caller,
    label: &str,
    hint: Option<&str>,
) -> Result<String, String> {
    let lock = label_lock(label);
    let _guard = lock.lock().await;
    {
        let table = table();
        if table.closed_labels.contains(label) {
            return Err(MESSAGE_WINDOW_UNAVAILABLE.to_owned());
        }
        if let Some(bench) = table.by_label.get(label) {
            return Ok(bench.clone());
        }
    }
    let path = crate::infrastructure::window_manager::session_worktree_path(label)
        .or_else(|| hint.map(str::to_owned))
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| "A working directory is required to start agent work.".to_owned())?;
    let output: BenchOpenOutput = workbench_compat::call_command(
        caller,
        OperationId::BenchOpen,
        json!({ "workingDirectory": path }),
    )
    .await?;
    let mut table = table();
    table
        .by_label
        .insert(label.to_owned(), output.bench_id.clone());
    table
        .by_bench
        .insert(output.bench_id.clone(), label.to_owned());
    Ok(output.bench_id)
}

/// 창이 `Destroyed`될 때: 닫힌 창으로 표시 → 작업대 닫기(소유 run 취소·교환 삭제) → 대응 제거. `principal`은 작업대를
/// 연 창 주체다(043: 창 incarnation을 거둬들이기 전에 받아 둔 값 — 작업대는 연 주체만 닫을 수 있다).
pub async fn close(
    runtime: &Arc<WorkbenchRuntime>,
    label: &str,
    principal: AuthenticatedPrincipal,
) {
    let lock = label_lock(label);
    let _guard = lock.lock().await;
    let bench = {
        let mut table = table();
        table.closed_labels.insert(label.to_owned());
        table.by_label.get(label).cloned()
    };
    if let Some(bench) = &bench {
        let request =
            workbench_compat::command_request(OperationId::BenchClose, json!({ "benchId": bench }));
        let result: Result<CallReply, _> = runtime.call(principal, request).await;
        if let Err(fault) = result {
            eprintln!(
                "[workbench] failed to close bench for window {label}: {}",
                fault.message
            );
        }
    }
    let mut table = table();
    if let Some(bench) = bench {
        table.by_bench.remove(&bench);
    }
    table.by_label.remove(label);
    table.locks.remove(label);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use workbench_core::{
        application::workbench_runtime::{RuntimeAdapters, WorkbenchRuntime},
        infrastructure::data_paths::DataPaths,
    };

    use super::*;

    fn runtime() -> (tempfile::TempDir, Arc<WorkbenchRuntime>, String) {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let runtime = WorkbenchRuntime::bootstrap_with(
            DataPaths::new(dir.path().join("data")),
            RuntimeAdapters::production(),
        )
        .unwrap();
        (dir, runtime, work.to_string_lossy().into_owned())
    }

    fn window(runtime: &Arc<WorkbenchRuntime>, label: &str) -> workbench_compat::Caller {
        workbench_compat::Caller {
            runtime: Arc::clone(runtime),
            principal: AuthenticatedPrincipal::desktop_window(label, "inc-1"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ensure_opens_once_and_close_releases_the_window() {
        let (_dir, runtime, work) = runtime();
        let label = "session-test-ensure";
        let first = ensure(&window(&runtime, label), label, Some(&work))
            .await
            .unwrap();
        let second = ensure(&window(&runtime, label), label, Some(&work))
            .await
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(lookup(label).as_deref(), Some(first.as_str()));
        assert_eq!(label_for(&first).as_deref(), Some(label));

        close(&runtime, label, window(&runtime, label).principal).await;
        assert!(lookup(label).is_none());
        assert!(label_for(&first).is_none());
        assert!(
            runtime.benches().registry.is_empty(),
            "bench closed on the server by the window principal that opened it"
        );
        // 닫힌 창에서는 다시 열지 않는다.
        assert_eq!(
            ensure(&window(&runtime, label), label, Some(&work))
                .await
                .unwrap_err(),
            MESSAGE_WINDOW_UNAVAILABLE
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn close_racing_with_ensure_never_leaves_a_bench_open() {
        let (_dir, runtime, work) = runtime();
        for round in 0..50 {
            let label = format!("session-test-race-{round}");
            let opening = {
                let runtime = Arc::clone(&runtime);
                let label = label.clone();
                let work = work.clone();
                tokio::spawn(
                    async move { ensure(&window(&runtime, &label), &label, Some(&work)).await },
                )
            };
            close(&runtime, &label, window(&runtime, &label).principal).await;
            let _ = opening.await.unwrap();
            assert!(lookup(&label).is_none(), "round {round}");
        }
        assert!(runtime.benches().registry.is_empty());
    }

    #[tokio::test]
    async fn ensure_without_any_path_is_rejected() {
        let (_dir, runtime, _work) = runtime();
        assert!(
            ensure(
                &window(&runtime, "session-test-nopath"),
                "session-test-nopath",
                None
            )
            .await
            .is_err()
        );
    }
}
