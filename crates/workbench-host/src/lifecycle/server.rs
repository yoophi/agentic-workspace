//! 독립 서버 실행 `serve`(044 research R5, contracts/server-lifecycle.md §1). 순서:
//! `owner.lock` → 저장 형식 검사(읽기 전용) → 런타임 조립(시작 복구 포함) → 루프백 끝점 → 준비 → 안내 파일(원자적).
//! 정지는 운영체제 신호다. **이 증분의 중간 동작**: 신호를 받으면 열린 작업대를 닫고(run 취소) 받아들인 호출을
//! drain한 뒤 끝낸다. 상태 기계·비우기 분류·정지 세 방식은 T041에서 이 자리를 바꾼다.

use std::path::{Path, PathBuf};

use workbench_core::{
    application::workbench_runtime::RuntimeAdapters,
    infrastructure::{data_paths::DataPaths, sqlite_ledger},
};

use super::{
    descriptor::{Descriptor, read_descriptor, remove_descriptor_if, write_descriptor},
    ensure::{EXIT_ALREADY_RUNNING, EXIT_UNSUPPORTED_SCHEMA},
    identity::OwnerIdentity,
    lock::{ensure_server_dir, try_owner_lock},
};
use crate::assembly::{HostOptions, assemble};

pub struct ServeOptions {
    pub data_dir: PathBuf,
    pub server_version: String,
}

/// 서버를 실행하고 종료 코드를 돌려준다(0 정상 정지, 3 이미 있음, 4 저장 형식 거절, 1 그 밖).
pub fn serve(options: ServeOptions) -> i32 {
    match run(options) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("[workbench-server] failed: {error:#}");
            1
        }
    }
}

fn run(options: ServeOptions) -> anyhow::Result<i32> {
    std::fs::create_dir_all(&options.data_dir)?;
    let data_dir = std::fs::canonicalize(&options.data_dir)?;
    let server_dir = ensure_server_dir(&data_dir)?;
    let Some(owner_lock) = try_owner_lock(&data_dir)? else {
        let existing = read_descriptor(&server_dir)?
            .map(|descriptor| descriptor.public_json())
            .unwrap_or(serde_json::Value::Null);
        eprintln!(
            "[workbench-server] another server owns this data directory: {}",
            serde_json::json!({ "dataDir": data_dir, "server": existing })
        );
        return Ok(EXIT_ALREADY_RUNNING);
    };
    if let Some(code) = unsupported_schema(&data_dir)? {
        return Ok(code);
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let identity = OwnerIdentity::generate();
    let mut host_options = HostOptions::new(
        data_dir.clone(),
        RuntimeAdapters::production(),
        options.server_version.clone(),
        runtime.handle().clone(),
    );
    host_options.owner = Some(identity.clone());
    let host = match assemble(host_options) {
        Ok(host) => host,
        Err(error) => {
            let text = format!("{error:#}");
            eprintln!("[workbench-server] failed to assemble: {text}");
            return Ok(
                if text.contains("storage schema") || text.contains("schema version") {
                    EXIT_UNSUPPORTED_SCHEMA
                } else {
                    1
                },
            );
        }
    };
    let Some(http) = host.http.clone() else {
        eprintln!(
            "[workbench-server] failed to open the endpoint: {}",
            host.http_start_error.clone().unwrap_or_default()
        );
        return Ok(1);
    };
    let descriptor = Descriptor::for_endpoint(
        "server",
        &identity,
        host.runtime.epoch(),
        http.base_url(),
        &options.server_version,
    );
    write_descriptor(&server_dir, &descriptor)?;
    eprintln!(
        "[workbench-server] ready: {}",
        serde_json::json!({ "baseUrl": descriptor.base_url, "instanceId": descriptor.instance_id })
    );

    runtime.block_on(async {
        wait_for_stop_signal().await;
        eprintln!("[workbench-server] stopping (interim: close benches, drain accepted calls)");
        host.shutdown().await;
    });
    remove_descriptor_if(&server_dir, identity.instance_id())?;
    drop(owner_lock);
    Ok(0)
}

/// 모르는(더 새) 저장 형식이면 데이터를 건드리지 않고 거절한다.
fn unsupported_schema(data_dir: &Path) -> anyhow::Result<Option<i32>> {
    let ledger = DataPaths::new(data_dir).ledger_file();
    match sqlite_ledger::read_schema_version(&ledger) {
        Ok(Some(found)) if found > sqlite_ledger::SCHEMA_VERSION => {
            eprintln!(
                "[workbench-server] unsupported storage schema {found} (supported {}): {}",
                sqlite_ledger::SCHEMA_VERSION,
                ledger.display()
            );
            Ok(Some(EXIT_UNSUPPORTED_SCHEMA))
        }
        Ok(_) => Ok(None),
        Err(error) => Err(anyhow::anyhow!(
            "failed to read the storage schema: {error}"
        )),
    }
}

async fn wait_for_stop_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler");
    let mut interrupt = signal(SignalKind::interrupt()).expect("SIGINT handler");
    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }
}
