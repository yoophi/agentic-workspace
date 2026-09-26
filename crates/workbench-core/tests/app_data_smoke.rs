//! 038 T064 보조: 실제 앱 데이터의 **복사본**으로 기동해 본다. 원본을 건드리지 않도록 경로를 환경 변수로 받는다.
//!
//! ```sh
//! WORKBENCH_SMOKE_DIR=<앱 데이터 복사본> cargo test -p workbench-core --test app_data_smoke -- --ignored --nocapture
//! ```
//!
//! 확인하는 것: 037이 만든 v1 ledger가 schema 2로 승격되고, 저장 파일 4개가 기동·조회만으로 바뀌지 않으며,
//! 저장 단위 조회가 기존 파일을 그대로 읽는다. 개인 데이터는 출력하지 않고 개수만 남긴다.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::json;
use workbench_core::{
    application::workbench_runtime::WorkbenchRuntime, infrastructure::data_paths::DataPaths,
};
use workbench_protocol::{AuthenticatedPrincipal, CallRequest, OperationId, Workbench};

const STORES: [&str; 4] = [
    "projects.json",
    "saved-prompts.json",
    "goals.json",
    "agent-run-settings.json",
];

fn snapshot(dir: &Path) -> Vec<Option<Vec<u8>>> {
    STORES
        .iter()
        .map(|name| fs::read(dir.join(name)).ok())
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "needs WORKBENCH_SMOKE_DIR pointing at a copy of real app data"]
async fn app_data_copy_upgrades_ledger_and_reads_stores_without_writing() {
    let Some(dir) = std::env::var_os("WORKBENCH_SMOKE_DIR").map(PathBuf::from) else {
        eprintln!("WORKBENCH_SMOKE_DIR not set; skipping");
        return;
    };
    let before = snapshot(&dir);
    let runtime = WorkbenchRuntime::bootstrap(DataPaths::new(&dir)).expect("bootstrap on copy");

    let version: i64 = rusqlite::Connection::open(dir.join("workbench").join("ledger.sqlite"))
        .unwrap()
        .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 2, "v1 ledger must be upgraded to schema 2");

    let principal = AuthenticatedPrincipal::desktop();
    for (operation, input) in [
        (OperationId::ProjectList, json!({})),
        (OperationId::SavedPromptList, json!({})),
    ] {
        let reply = runtime
            .call(principal.clone(), CallRequest::query(operation, input))
            .await
            .unwrap_or_else(|fault| panic!("{operation}: {fault}"));
        let count = reply
            .output()
            .and_then(|value| value.as_array())
            .map(Vec::len);
        println!("{operation}: {count:?} items");
    }
    let describe = runtime
        .call(
            principal,
            CallRequest::query(OperationId::SystemDescribe, json!({})),
        )
        .await
        .unwrap();
    println!(
        "system.describe: {} operations",
        describe.output().unwrap()["operations"]
            .as_array()
            .unwrap()
            .len()
    );

    assert_eq!(
        snapshot(&dir),
        before,
        "startup and queries must not rewrite store files"
    );
}
