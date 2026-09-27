//! 043 T016: TS 통합 시험(`packages/workbench-client` `test:integration`)용 시험 host. 운영 router(`workbench-server`)와
//! 실제 `WorkbenchRuntime`·event hub를 루프백에 띄운다. 다른 점은 셋이다: 가짜 run 엔진(`ScriptedRunEngine`, prompt마다
//! run 이벤트 하나), 낮춘 journal 보관 한도(보관 초과 복구 시험), 창 주체 두 개에 묶인 고정 토큰. 시험 전용 operation은
//! 없다 — 시나리오는 운영 operation(교환 전송·orchestration 변경·run prompt)으로 만든다.
//!
//! 입력(환경 변수): `HOST_DATA_DIR`(같은 데이터로 재기동 = 새 세대), `HOST_JOURNAL_CAPACITY`(기본 4),
//! `HOST_PROMPT_SETTLE_MS`(prompt 효과 뒤 응답 전 지연 — 응답 유실 시험).
//! 출력: 준비되면 stdout에 JSON 한 줄 `{"baseUrl","epoch","dataDir","tokens":{"windowA","windowB"},"workDir"}`.
//! 종료: stdin이 닫히면(EOF) 우아하게 끝낸다.

use std::{sync::Arc, time::Duration};

use tokio::io::AsyncReadExt;
use workbench_core::{
    application::workbench_runtime::{RuntimeAdapters, WorkbenchRuntime},
    infrastructure::{data_paths::DataPaths, event_hub::EventHubLimits},
    testing::scripted_run_engine::{RunScript, ScriptedRunEngine},
};
use workbench_protocol::{AuthenticatedPrincipal, Workbench};
use workbench_server::{
    access_log::StderrAccessLog, auth::StaticResolver, handshake::ServerInfo, origin::OriginPolicy,
    tickets::EventTicketStore, ExposurePolicy, ServerConfig,
};

const TOKEN_WINDOW_A: &str = "host-window-a";
const TOKEN_WINDOW_B: &str = "host-window-b";

struct HostInfo {
    epoch: String,
}

impl ServerInfo for HostInfo {
    fn server_version(&self) -> String {
        "http-test-host".into()
    }
    fn server_epoch(&self) -> String {
        self.epoch.clone()
    }
    fn storage_schema_version(&self) -> i64 {
        2
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let data_dir = match std::env::var("HOST_DATA_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => std::env::temp_dir().join(format!("wb-host-{}", uuid::Uuid::new_v4())),
    };
    std::fs::create_dir_all(&data_dir).expect("data dir");
    let work_dir = data_dir.join("work");
    std::fs::create_dir_all(&work_dir).expect("work dir");
    let work_dir = std::fs::canonicalize(work_dir).expect("canonical work dir");

    let capacity = env_usize("HOST_JOURNAL_CAPACITY", 4);
    let engine = Arc::new(ScriptedRunEngine::new(RunScript {
        prompt_settle_ms: env_usize("HOST_PROMPT_SETTLE_MS", 0) as u64,
        ..RunScript::default()
    }));
    let mut adapters = RuntimeAdapters::production();
    adapters.run_engine = Some(engine);
    adapters.event_limits = EventHubLimits {
        run_journal_capacity: capacity,
        exchange_journal_capacity: capacity,
        orchestration_journal_capacity: capacity,
        ..EventHubLimits::default()
    };
    let runtime = WorkbenchRuntime::bootstrap_with(DataPaths::new(&data_dir), adapters)
        .expect("bootstrap runtime");
    let epoch = runtime.epoch().to_owned();

    let listener = workbench_server::bind_loopback().await.expect("bind");
    let address = listener.local_addr().expect("address");
    let config = ServerConfig {
        resolver: Arc::new(StaticResolver::new([
            (
                TOKEN_WINDOW_A.to_owned(),
                AuthenticatedPrincipal::desktop_window("session-a", "inc-1"),
            ),
            (
                TOKEN_WINDOW_B.to_owned(),
                AuthenticatedPrincipal::desktop_window("session-b", "inc-1"),
            ),
        ])),
        server_info: Arc::new(HostInfo {
            epoch: epoch.clone(),
        }),
        origins: OriginPolicy::new(Vec::<String>::new()),
        access_log: Arc::new(StderrAccessLog),
        exposure: ExposurePolicy::network_default(),
        tickets: Arc::new(EventTicketStore::default()),
        body_limit: workbench_server::DEFAULT_BODY_LIMIT,
        drain_warn_after: Duration::from_secs(30),
        body_read_timeout: workbench_server::DEFAULT_BODY_READ_TIMEOUT,
        connection_grace: workbench_server::DEFAULT_CONNECTION_GRACE,
    };
    let server = workbench_server::build_router(
        runtime.clone() as Arc<dyn Workbench>,
        config,
        address.port(),
    );

    println!(
        "{}",
        serde_json::json!({
            "baseUrl": format!("http://{address}"),
            "epoch": epoch,
            "dataDir": data_dir.to_string_lossy(),
            "workDir": work_dir.to_string_lossy(),
            "tokens": { "windowA": TOKEN_WINDOW_A, "windowB": TOKEN_WINDOW_B },
        })
    );

    let stdin_closed = async {
        let mut sink = Vec::new();
        let _ = tokio::io::stdin().read_to_end(&mut sink).await;
    };
    if let Err(error) = workbench_server::serve(listener, server, stdin_closed).await {
        eprintln!("[http-test-host] server stopped: {error}");
    }
    runtime.close_all_benches().await;
}
