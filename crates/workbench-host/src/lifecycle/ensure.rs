//! 시작 절차 `ensure`(044 research R5, contracts/server-lifecycle.md §3):
//! 1. `startup.lock`(상한 대기).
//! 2. 안내 파일이 있으면 신원 증명 → 자격 증명 확인(`client::verify`) → 서빙 중인지(`server.status`). 통과하면 그 서버에
//!    붙는다. 비우는 중·정지 중인 서버는 붙을 대상이 아니다.
//! 3. 확인이 실패하면 `owner.lock`을 잠깐 시도한다. 잡히면 서버가 없다 → 남은 안내 파일을 지우고 잠금을 푼 뒤 서버를
//!    띄운다. 잡히지 않으면 서버는 살아 있지만 준비 전이거나 비우는 중 → 기다리며 **매번 잠금을 다시 시도**한다. 그
//!    서버가 끝나(또는 준비 전 죽어) 잠금이 풀리면 그때 띄운다(044 OCR 구현 리뷰).
//! 4. 새 안내 파일이 확인을 통과하면 끝낸다.
//!
//! 서버는 호출자의 프로세스 그룹과 분리해 띄운다(`process_group(0)`, 표준 입출력 null, 오류 출력은 `server.log`).
//! 호출자가 끝나거나 `tauri dev`가 Ctrl+C로 그룹에 신호를 보내도 서버는 산다.

use std::{
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use super::{
    client::{VerifyError, require_serving, verify},
    descriptor::{Descriptor, read_descriptor, remove_stale_descriptor},
    lock::{ensure_server_dir, open_owner_only, startup_lock, try_owner_lock},
};

pub const SERVER_LOG: &str = "server.log";
/// `serve` 종료 코드: 이미 서버 있음 / 모르는 저장 형식.
pub const EXIT_ALREADY_RUNNING: i32 = 3;
pub const EXIT_UNSUPPORTED_SCHEMA: i32 = 4;

#[derive(Debug, Clone)]
pub struct EnsureOptions {
    pub startup_lock_timeout: Duration,
    pub ready_timeout: Duration,
}

impl Default for EnsureOptions {
    fn default() -> Self {
        Self {
            startup_lock_timeout: Duration::from_secs(20),
            ready_timeout: Duration::from_secs(20),
        }
    }
}

#[derive(Debug)]
pub enum EnsureError {
    Io(std::io::Error),
    /// 띄운 서버가 저장 형식을 거절했다.
    UnsupportedSchema,
    /// 띄운 서버가 다른 이유로 끝났다.
    ServerExited(i32),
    /// 상한 안에 확인을 통과한 서버가 없다.
    Timeout(Option<VerifyError>),
}

impl std::fmt::Display for EnsureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::UnsupportedSchema => {
                f.write_str("the data directory uses an unsupported storage schema")
            }
            Self::ServerExited(code) => write!(f, "the Workbench server exited with code {code}"),
            Self::Timeout(Some(last)) => {
                write!(f, "timed out waiting for the Workbench server ({last})")
            }
            Self::Timeout(None) => f.write_str("timed out waiting for the Workbench server"),
        }
    }
}

impl std::error::Error for EnsureError {}

impl From<std::io::Error> for EnsureError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

fn verified_descriptor(server_dir: &Path) -> Result<Descriptor, Option<VerifyError>> {
    match read_descriptor(server_dir) {
        Ok(Some(descriptor)) => verify(&descriptor)
            .and_then(|_| require_serving(&descriptor))
            .map(|_| descriptor)
            .map_err(Some),
        _ => Err(None),
    }
}

/// 데이터 디렉터리의 서버를 찾거나 띄워, 확인을 통과한 안내를 돌려준다.
pub fn ensure(
    data_dir: &Path,
    server_exe: &Path,
    options: &EnsureOptions,
) -> Result<Descriptor, EnsureError> {
    let server_dir = ensure_server_dir(data_dir)?;
    let _startup = startup_lock(data_dir, options.startup_lock_timeout)?;
    if let Ok(descriptor) = verified_descriptor(&server_dir) {
        return Ok(descriptor);
    }
    let mut spawned = spawn_if_free(data_dir, &server_dir, server_exe)?;
    let deadline = Instant::now() + options.ready_timeout;
    loop {
        let last = match verified_descriptor(&server_dir) {
            Ok(descriptor) => return Ok(descriptor),
            Err(error) => error,
        };
        if let Some(exit) = spawned.as_ref().and_then(|status| *status.lock().unwrap()) {
            match exit {
                EXIT_UNSUPPORTED_SCHEMA => return Err(EnsureError::UnsupportedSchema),
                // 다른 시작이 먼저 소유 잠금을 잡았다 — 그 서버를 기다리며 잠금을 다시 시도한다.
                EXIT_ALREADY_RUNNING => spawned = None,
                code => return Err(EnsureError::ServerExited(code)),
            }
        }
        // 띄운 서버가 없으면(잠금이 잡혀 있었거나 띄운 서버가 양보했다) 잠금을 다시 시도한다: 비우던 서버가 끝났거나
        // 준비 전에 죽었으면 이제 잡힌다.
        if spawned.is_none() {
            spawned = spawn_if_free(data_dir, &server_dir, server_exe)?;
        }
        if Instant::now() >= deadline {
            return Err(EnsureError::Timeout(last));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// `owner.lock`이 비어 있으면(서버 없음) 남은 안내 파일을 지우고 서버를 띄운다. 잡혀 있으면 `None`.
fn spawn_if_free(
    data_dir: &Path,
    server_dir: &Path,
    server_exe: &Path,
) -> Result<Option<Arc<Mutex<Option<i32>>>>, EnsureError> {
    match try_owner_lock(data_dir)? {
        Some(owner) => {
            remove_stale_descriptor(server_dir)?;
            drop(owner);
            Ok(Some(spawn_server(data_dir, server_dir, server_exe)?))
        }
        None => Ok(None),
    }
}

/// 서버를 분리 프로세스 그룹으로 띄우고, 별도 스레드가 종료를 회수(reap)해 코드를 남긴다.
fn spawn_server(
    data_dir: &Path,
    server_dir: &Path,
    server_exe: &Path,
) -> Result<Arc<Mutex<Option<i32>>>, EnsureError> {
    let log = open_owner_only(&server_dir.join(SERVER_LOG))?;
    let mut child = Command::new(server_exe)
        .arg("serve")
        .arg("--data-dir")
        .arg(data_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log))
        .process_group(0)
        .spawn()?;
    let status = Arc::new(Mutex::new(None));
    let recorded = status.clone();
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|exit| exit.code()).unwrap_or(-1);
        *recorded.lock().unwrap() = Some(code);
    });
    Ok(status)
}

/// 띄울 서버 실행 파일(contracts/desktop-client.md §1): `AW_WORKBENCH_SERVER_PATH` → `fallback`.
pub fn server_executable(fallback: PathBuf) -> PathBuf {
    std::env::var_os("AW_WORKBENCH_SERVER_PATH")
        .map(PathBuf::from)
        .unwrap_or(fallback)
}
