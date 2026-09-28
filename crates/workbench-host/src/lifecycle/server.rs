//! 독립 서버 실행 `serve`(044 research R5, contracts/server-lifecycle.md §1). 순서:
//! `owner.lock` → 저장 형식 검사(읽기 전용) → 런타임 조립(시작 복구 포함) → 루프백 끝점 → 준비 → 안내 파일(원자적).
//! 서빙 중에는 감시 루프(`monitor`)가 유휴·비우기 정지를 판정한다(contracts/server-lifecycle.md §5). `server.stop`
//! (default·wait·force)도 같은 상태 기계로 `stopping`에 들어간다. `SIGTERM`·`SIGINT`는 `force`와 같다(R10, 상한
//! [`SIGNAL_STOP_CAP`]). `stopping` 뒤: 받아들인 호출 drain·쉬는 세션 취소(`WorkbenchHost::shutdown`) → 자기 인스턴스의
//! 안내 파일 삭제 → `owner.lock` 해제.

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use workbench_core::{
    application::workbench_runtime::RuntimeAdapters,
    infrastructure::{data_paths::DataPaths, sqlite_ledger},
};

use super::{
    descriptor::{Descriptor, read_descriptor, remove_descriptor_if, write_descriptor},
    ensure::{EXIT_ALREADY_RUNNING, EXIT_UNSUPPORTED_SCHEMA},
    identity::OwnerIdentity,
    lock::{ensure_server_dir, try_owner_lock},
    monitor::{MonitorOptions, run_until_stopped},
};
use crate::assembly::{HostOptions, assemble};

/// 신호 정지(force)의 상한. 넘으면 남은 호출을 버리고 끝낸다(경고 기록, R10).
pub const SIGNAL_STOP_CAP: Duration = Duration::from_secs(30);

pub struct ServeOptions {
    pub data_dir: PathBuf,
    pub server_version: String,
    /// 임대 0 + 활동 작업 0이 이만큼 이어지면 정지한다(`--idle-timeout`, 기본 10분).
    pub idle_timeout: Duration,
    /// 서버 기록을 이 파일에 덧붙인다(`--log`). 없으면 표준 오류.
    pub log: Option<PathBuf>,
}

/// 서버 자신의 기록(준비·정지·경고). `--log`가 있으면 그 파일에 덧붙이고, 열 수 없으면 표준 오류로 돌아간다.
#[derive(Clone)]
struct Log(Option<Arc<std::sync::Mutex<std::fs::File>>>);

impl Log {
    fn open(path: Option<&Path>) -> Self {
        let file = path.and_then(|path| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|error| {
                    eprintln!(
                        "[workbench-server] cannot open log {}: {error}",
                        path.display()
                    )
                })
                .ok()
        });
        Self(file.map(|file| Arc::new(std::sync::Mutex::new(file))))
    }

    fn line(&self, message: impl std::fmt::Display) {
        match &self.0 {
            Some(file) => {
                let mut file = file.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                let _ = writeln!(file, "[workbench-server] {message}");
            }
            None => eprintln!("[workbench-server] {message}"),
        }
    }
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
    let log = Log::open(options.log.as_deref());
    std::fs::create_dir_all(&options.data_dir)?;
    let data_dir = std::fs::canonicalize(&options.data_dir)?;
    let server_dir = ensure_server_dir(&data_dir)?;
    let Some(owner_lock) = try_owner_lock(&data_dir)? else {
        let existing = read_descriptor(&server_dir)?
            .map(|descriptor| descriptor.public_json())
            .unwrap_or(serde_json::Value::Null);
        // 계약(§1): 이미 있는 서버의 안내 JSON은 항상 표준 오류로.
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
            log.line(format!("failed to assemble: {text}"));
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
        log.line(format!(
            "failed to open the endpoint: {}",
            host.http_start_error.clone().unwrap_or_default()
        ));
        return Ok(1);
    };
    let descriptor = Descriptor::for_endpoint(
        "server",
        &identity,
        host.runtime.epoch(),
        http.base_url(),
        &options.server_version,
    );
    // 신호 처리기를 안내 파일보다 먼저 건다: 안내 파일을 쓴 뒤 기본 처리로 죽으면 남은 안내 파일이 생긴다(044 OCR 구현 리뷰).
    let mut signals = {
        let _enter = runtime.enter();
        StopSignals::install()?
    };
    write_descriptor(&server_dir, &descriptor)?;
    log.line(format!(
        "ready: {}",
        serde_json::json!({ "baseUrl": descriptor.base_url, "instanceId": descriptor.instance_id })
    ));

    let control = Arc::clone(host.runtime.server_control());
    let monitor = MonitorOptions {
        idle_timeout: options.idle_timeout,
        ..MonitorOptions::default()
    };
    let finish = runtime.block_on(async {
        tokio::select! {
            () = run_until_stopped(Arc::clone(&control), monitor) => {
                log.line(format!("stopping: {:?}", control.work_gate().state()));
                // 정상 정지 중에도 신호는 force다(R10): 받은 호출 drain이 멈춰도 상한 안에 끝난다.
                finish_shutdown(
                    host.shutdown(),
                    signals.recv(),
                    async {
                        control.force_stop().await;
                    },
                    SIGNAL_STOP_CAP,
                    |signal| log.line(format!("{signal} during shutdown: force stop (cap {}s)", SIGNAL_STOP_CAP.as_secs())),
                )
                .await
            }
            signal = signals.recv() => {
                // SIGTERM·SIGINT = force(R10): 새 작업 차단 → 작업대 닫기(run 취소) → stopping → 받은 호출 drain. 상한을 넘으면
                // 남은 호출을 버리고 끝낸다.
                log.line(format!("{signal}: force stop (cap {}s)", SIGNAL_STOP_CAP.as_secs()));
                let forced = async {
                    control.force_stop().await;
                    host.shutdown().await;
                };
                if tokio::time::timeout(SIGNAL_STOP_CAP, forced).await.is_err() {
                    Finish::Abandoned
                } else {
                    Finish::Forced
                }
            }
        }
    });
    if finish == Finish::Abandoned {
        control.work_gate().force_stop();
        log.line("warning: stop exceeded its cap; abandoning the remaining calls");
    }
    // 단일 writer(R4): 소유 잠금은 런타임·저장소를 모두 내린 **뒤**에만 푼다. 먼저 풀면 남은 작업이 쓰는 동안 다음 서버가 같은
    // 데이터 디렉터리를 연다(044 OCR 구현 리뷰).
    drop(host);
    let teardown = std::time::Instant::now();
    runtime.shutdown_timeout(RUNTIME_SHUTDOWN_CAP);
    let abandoned = finish == Finish::Abandoned || teardown.elapsed() >= RUNTIME_SHUTDOWN_CAP;
    remove_descriptor_if(&server_dir, identity.instance_id())?;
    if abandoned {
        // 남은 blocking 작업이 아직 돌 수 있다 — 잠금을 쥔 채 프로세스를 끝내 그 스레드와 함께 잠금을 OS가 풀게 한다.
        log.line(
            "warning: leftover work after the runtime cap; exiting while holding the owner lock",
        );
        std::process::exit(0);
    }
    drop(owner_lock);
    Ok(0)
}

/// 런타임 해체 상한. 넘으면 남은 작업을 버리고 잠금을 쥔 채 끝낸다.
const RUNTIME_SHUTDOWN_CAP: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Finish {
    Graceful,
    Forced,
    Abandoned,
}

/// 진행 중인 정상 정지(`shutdown`)를 신호와 경주시킨다. 신호가 오면 `force`를 한 뒤 **같은** 정지를 상한 안에서 마저
/// 기다린다(새로 시작하지 않음). 상한을 넘으면 `Abandoned`.
async fn finish_shutdown(
    shutdown: impl std::future::Future<Output = ()>,
    signal: impl std::future::Future<Output = &'static str>,
    force: impl std::future::Future<Output = ()>,
    cap: Duration,
    on_signal: impl FnOnce(&'static str),
) -> Finish {
    tokio::pin!(shutdown);
    tokio::select! {
        () = &mut shutdown => Finish::Graceful,
        name = signal => {
            on_signal(name);
            let forced = async {
                force.await;
                (&mut shutdown).await;
            };
            if tokio::time::timeout(cap, forced).await.is_err() {
                Finish::Abandoned
            } else {
                Finish::Forced
            }
        }
    }
}

/// `SIGTERM`·`SIGINT` 수신기. 런타임 문맥에서 한 번 건다.
struct StopSignals {
    terminate: tokio::signal::unix::Signal,
    interrupt: tokio::signal::unix::Signal,
}

impl StopSignals {
    fn install() -> std::io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};
        Ok(Self {
            terminate: signal(SignalKind::terminate())?,
            interrupt: signal(SignalKind::interrupt())?,
        })
    }

    async fn recv(&mut self) -> &'static str {
        tokio::select! {
            _ = self.terminate.recv() => "SIGTERM",
            _ = self.interrupt.recv() => "SIGINT",
        }
    }
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

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use super::*;

    /// 정상 정지 중 신호가 오면 force가 진행 중 정지를 풀어 상한 안에 끝난다(signal 뒤 무시되던 문제, 044 OCR 구현 리뷰).
    #[tokio::test(start_paused = true)]
    async fn a_signal_during_a_stuck_graceful_shutdown_forces_it_within_the_cap() {
        let released = Arc::new(tokio::sync::Notify::new());
        let forced = Arc::new(AtomicBool::new(false));
        let shutdown = {
            let released = Arc::clone(&released);
            async move { released.notified().await }
        };
        let force = {
            let (released, forced) = (Arc::clone(&released), Arc::clone(&forced));
            async move {
                forced.store(true, Ordering::SeqCst);
                released.notify_one();
            }
        };
        let mut seen = None;
        let finish = finish_shutdown(
            shutdown,
            async { "SIGTERM" },
            force,
            Duration::from_secs(30),
            |signal| seen = Some(signal),
        )
        .await;
        assert_eq!(finish, Finish::Forced);
        assert!(forced.load(Ordering::SeqCst));
        assert_eq!(seen, Some("SIGTERM"));
    }

    #[tokio::test(start_paused = true)]
    async fn a_shutdown_that_ignores_the_force_is_abandoned_at_the_cap() {
        let started = tokio::time::Instant::now();
        let finish = finish_shutdown(
            std::future::pending::<()>(),
            async { "SIGINT" },
            async {},
            Duration::from_secs(30),
            |_| {},
        )
        .await;
        assert_eq!(finish, Finish::Abandoned);
        assert_eq!(started.elapsed(), Duration::from_secs(30));
    }

    #[tokio::test(start_paused = true)]
    async fn without_a_signal_the_graceful_shutdown_completes() {
        let finish = finish_shutdown(
            async {},
            std::future::pending::<&'static str>(),
            async { panic!("no force without a signal") },
            Duration::from_secs(30),
            |_| panic!("no signal"),
        )
        .await;
        assert_eq!(finish, Finish::Graceful);
    }
}
