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
    external: ExternalTable,
}

/// 외부 서버 모드의 대응(044, OCR H2·M2). 작업대는 창 주체(label + incarnation) 소유이고 서버 메모리에만 있으므로, 대응을
/// **서버 인스턴스와 창 incarnation**에 묶는다: 서버가 다시 떴거나 같은 label로 창을 다시 열었으면 다시 연다. 닫힌 창도
/// incarnation별로 적는다 — 옛 창의 늦은 정리가 같은 label로 다시 연 창(`settings`·`main`)을 지우거나 막지 않는다.
#[derive(Default)]
struct ExternalTable {
    entries: HashMap<String, ExternalEntry>,
    closed: HashSet<(String, String)>,
}

struct ExternalEntry {
    incarnation: String,
    instance: String,
    bench: String,
}

impl ExternalTable {
    fn hit(&self, label: &str, incarnation: &str, instance: &str) -> Option<String> {
        self.entries
            .get(label)
            .filter(|entry| entry.incarnation == incarnation && entry.instance == instance)
            .map(|entry| entry.bench.clone())
    }

    fn record(&mut self, label: &str, incarnation: &str, instance: &str, bench: &str) {
        // OCR 2차 M4: 같은 label의 새 창이 작업대를 열면 그 label의 옛 닫힘 기록을 거둔다(표가 끝없이 자라지 않게). 옛
        // incarnation의 늦은 호출은 서버가 막는다 — 폐기한 창 주체는 서버 tombstone으로 거절된다(Codex 코드 리뷰 수정).
        self.closed
            .retain(|(closed_label, closed)| closed_label != label || closed == incarnation);
        self.entries.insert(
            label.to_owned(),
            ExternalEntry {
                incarnation: incarnation.to_owned(),
                instance: instance.to_owned(),
                bench: bench.to_owned(),
            },
        );
    }

    fn label_for(&self, bench: &str) -> Option<String> {
        self.entries
            .iter()
            .find(|(_, entry)| entry.bench == bench)
            .map(|(label, _)| label.clone())
    }

    fn lookup(&self, label: &str, incarnation: &str) -> Option<String> {
        self.entries
            .get(label)
            .filter(|entry| entry.incarnation == incarnation)
            .map(|entry| entry.bench.clone())
    }

    fn is_closed(&self, label: &str, incarnation: &str) -> bool {
        self.closed
            .contains(&(label.to_owned(), incarnation.to_owned()))
    }

    /// 그 incarnation의 창이 닫혔다. 대응은 같은 incarnation일 때만 지운다.
    fn forget(&mut self, label: &str, incarnation: &str) {
        self.closed
            .insert((label.to_owned(), incarnation.to_owned()));
        if self
            .entries
            .get(label)
            .is_some_and(|entry| entry.incarnation == incarnation)
        {
            self.entries.remove(label);
        }
    }
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
/// 외부 서버 모드의 대응도 본다(호환 command가 작업대를 찾으면 "외부 서버 모드에서는 쓸 수 없음"으로 끝난다 — 조용히
/// 성공하지 않는다).
pub fn lookup(label: &str) -> Option<String> {
    let table = table();
    table.by_label.get(label).cloned().or_else(|| {
        table
            .external
            .entries
            .get(label)
            .map(|entry| entry.bench.clone())
    })
}

/// 외부 서버 모드: 그 창 incarnation이 연 작업대(열지 않는다).
pub fn lookup_external(label: &str, incarnation: &str) -> Option<String> {
    table().external.lookup(label, incarnation)
}

/// 작업대를 연 창.
pub fn label_for(bench_id: &str) -> Option<String> {
    let table = table();
    table
        .by_bench
        .get(bench_id)
        .cloned()
        .or_else(|| table.external.label_for(bench_id))
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

/// 외부 서버 모드(044 T029): `ensure`와 같되 작업대를 서버에 창 토큰으로 연다(작업대는 창 주체 소유).
pub async fn ensure_external(
    server: &std::sync::Arc<crate::infrastructure::server_client::ExternalServer>,
    label: &str,
    incarnation: &str,
    origin: &str,
    hint: Option<&str>,
) -> Result<String, String> {
    let lock = label_lock(label);
    let _guard = lock.lock().await;
    if table().external.is_closed(label, incarnation) {
        return Err(MESSAGE_WINDOW_UNAVAILABLE.to_owned());
    }
    // 지금 붙은 서버 인스턴스(끊겼으면 다시 붙는다). 다른 인스턴스가 연 대응은 쓰지 않는다 — 서버가 다시 뜨면 작업대는
    // 사라졌다.
    let instance = server.connect().await?.instance_id;
    if let Some(bench) = table().external.hit(label, incarnation, &instance) {
        return Ok(bench);
    }
    let path = crate::infrastructure::window_manager::session_worktree_path(label)
        .or_else(|| hint.map(str::to_owned))
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| "A working directory is required to start agent work.".to_owned())?;
    let (bench, instance) = server
        .open_bench_on(label, incarnation, origin, &path)
        .await?;
    // 열기와 닫힘 정리는 같은 label 잠금으로 직렬화된다 — 닫힘은 이 기록 뒤에 와서 지운다.
    table()
        .external
        .record(label, incarnation, &instance, &bench);
    Ok(bench)
}

/// 외부 서버 모드의 창 `Destroyed`(044 T031): 그 incarnation의 창을 닫힌 것으로 표시하고 대응만 지운다. 작업대 닫기 여부는
/// 서버의 `desktop.retireWindow{closeBench}`가 정한다(창 닫기 의도, R8). 같은 label로 다시 연 창은 건드리지 않는다.
pub async fn forget_window(label: &str, incarnation: &str) {
    let lock = label_lock(label);
    let _guard = lock.lock().await;
    let mut table = table();
    table.external.forget(label, incarnation);
    // OCR 2차 M4: 그 label의 대응이 남지 않으면 잠금도 거둔다. 늦게 온 같은 incarnation의 `ensure`는 닫힘 기록으로 거절된다.
    if !table.external.entries.contains_key(label) && !table.by_label.contains_key(label) {
        table.locks.remove(label);
    }
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

    /// OCR H2: 외부 모드의 대응은 서버 인스턴스와 창 incarnation에 묶인다. 서버가 바뀌면(재기동) 다시 연다.
    #[test]
    fn an_external_bench_is_reused_only_for_the_same_incarnation_and_server() {
        let mut table = ExternalTable::default();
        table.record("session-x", "i1", "server-a", "bench-1");
        assert_eq!(
            table.hit("session-x", "i1", "server-a"),
            Some("bench-1".into())
        );
        assert_eq!(
            table.hit("session-x", "i1", "server-b"),
            None,
            "a new server instance"
        );
        assert_eq!(
            table.hit("session-x", "i2", "server-a"),
            None,
            "a new window incarnation"
        );
        table.record("session-x", "i1", "server-b", "bench-2");
        assert_eq!(
            table.hit("session-x", "i1", "server-b"),
            Some("bench-2".into())
        );
        assert_eq!(table.label_for("bench-2").as_deref(), Some("session-x"));
        assert_eq!(
            table.label_for("bench-1"),
            None,
            "the replaced bench is unmapped"
        );
    }

    /// OCR M2: 옛 incarnation의 늦은 정리는 같은 label로 다시 연 창을 지우거나 막지 않는다.
    #[test]
    fn forgetting_an_old_incarnation_keeps_the_reopened_window() {
        let mut table = ExternalTable::default();
        table.record("settings", "i2", "server-a", "bench-new");
        table.forget("settings", "i1");
        assert_eq!(
            table.hit("settings", "i2", "server-a"),
            Some("bench-new".into())
        );
        assert!(!table.is_closed("settings", "i2"));
        assert!(table.is_closed("settings", "i1"));
        table.forget("settings", "i2");
        assert_eq!(table.hit("settings", "i2", "server-a"), None);
        assert!(
            table.is_closed("settings", "i2"),
            "a closed window stays closed"
        );
    }

    /// OCR 2차 M4: 닫힌 창 표가 끝없이 자라지 않는다 — 같은 label에 새 incarnation이 기록되면 그 label의 옛 닫힘 기록을
    /// 거둔다(옛 incarnation의 늦은 호출은 서버의 폐기 tombstone이 막는다).
    #[test]
    fn a_newer_incarnation_prunes_the_closed_records_of_its_label() {
        let mut table = ExternalTable::default();
        table.record("settings", "i1", "server-a", "bench-1");
        table.forget("settings", "i1");
        assert!(table.is_closed("settings", "i1"));
        table.record("settings", "i2", "server-a", "bench-2");
        assert!(!table.is_closed("settings", "i1"));
        assert!(
            table.closed.is_empty(),
            "the old closed record of the label is pruned"
        );
        assert_eq!(
            table.hit("settings", "i2", "server-a"),
            Some("bench-2".into())
        );
    }

    /// OCR 2차 M4: 창을 잊고 대응이 남지 않으면 그 label의 잠금도 거둔다.
    #[tokio::test]
    async fn forgetting_the_last_entry_of_a_label_drops_its_lock() {
        let label = format!("session-lock-{}", uuid::Uuid::new_v4());
        table().external.record(&label, "i1", "server-a", "bench-1");
        let _ = label_lock(&label);
        assert!(table().locks.contains_key(&label));
        forget_window(&label, "i1").await;
        assert!(
            !table().locks.contains_key(&label),
            "the label lock is dropped"
        );
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
