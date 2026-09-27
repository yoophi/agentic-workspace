//! 단일 writer 잠금(044 research R4). 데이터 디렉터리 아래 `workbench/server/`(0700)에 둔다:
//! - `owner.lock`: 서버가 실행 내내 쥐는 배타 잠금. 못 잡으면 데이터를 열지 않는다.
//! - `startup.lock`: 시작 절차(`ensure`)를 직렬화하는 짧은 배타 잠금. 정확성 조건이 아니라 불필요한 기동을 줄이는
//!   최적화다(정확성은 `owner.lock`).
//!
//! 표준 라이브러리 파일 잠금을 쓴다. 프로세스가 죽으면 OS가 푼다(남는 것은 안내 파일뿐 — `ensure`가 판별한다).

use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub const OWNER_LOCK: &str = "owner.lock";
pub const STARTUP_LOCK: &str = "startup.lock";

/// 데이터 디렉터리의 서버 파일 디렉터리.
pub fn server_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("workbench").join("server")
}

/// 서버 파일 디렉터리를 만들고 소유 사용자만 쓰게(0700) 한다.
pub fn ensure_server_dir(data_dir: &Path) -> io::Result<PathBuf> {
    let dir = server_dir(data_dir);
    fs::create_dir_all(&dir)?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    Ok(dir)
}

/// 소유 사용자만 읽고 쓰는(0600) 파일을 연다(없으면 만든다).
pub fn open_owner_only(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

/// 쥔 동안 잠금이 유지되는 파일. drop하면 푼다.
#[derive(Debug)]
pub struct HeldLock {
    file: File,
}

impl Drop for HeldLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn try_lock(path: &Path) -> io::Result<Option<HeldLock>> {
    let file = open_owner_only(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(HeldLock { file })),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

/// `owner.lock`을 시도한다. 다른 프로세스가 쥐고 있으면 `None`(그 서버가 살아 있다).
pub fn try_owner_lock(data_dir: &Path) -> io::Result<Option<HeldLock>> {
    let dir = ensure_server_dir(data_dir)?;
    try_lock(&dir.join(OWNER_LOCK))
}

/// `startup.lock`을 `timeout` 안에 잡는다(상한 있는 polling).
pub fn startup_lock(data_dir: &Path, timeout: Duration) -> io::Result<HeldLock> {
    let dir = ensure_server_dir(data_dir)?;
    let path = dir.join(STARTUP_LOCK);
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(held) = try_lock(&path)? {
            return Ok(held);
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "timed out waiting for the Workbench server startup lock",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_owner_lock_is_refused_until_the_first_is_released() {
        let dir = tempfile::tempdir().unwrap();
        let first = try_owner_lock(dir.path()).unwrap().expect("first lock");
        assert!(try_owner_lock(dir.path()).unwrap().is_none());
        drop(first);
        assert!(try_owner_lock(dir.path()).unwrap().is_some());
        let mode = fs::metadata(server_dir(dir.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700);
    }
}
