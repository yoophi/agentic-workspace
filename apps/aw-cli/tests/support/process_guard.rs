//! Private fixture ownership, including cancellation of the startup future.
use std::{
    fs::{self, OpenOptions},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::process::{Child, Command};
pub struct PrivateRoots {
    _owner: tempfile::TempDir,
    pub root: PathBuf,
    pub data: PathBuf,
    pub control: PathBuf,
}
impl PrivateRoots {
    pub fn new() -> Arc<Self> {
        let owner = tempfile::tempdir().unwrap();
        let root = owner.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let data = root.join("data");
        let control = root.join("control");
        for dir in [&data, &control] {
            fs::create_dir(dir).unwrap();
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Arc::new(Self {
            _owner: owner,
            root,
            data,
            control,
        })
    }
    pub fn descriptor(&self) -> PathBuf {
        self.data.join("workbench/server/server.json")
    }
}
#[derive(Debug, Clone)]
pub struct CleanupRecord {
    pub pid: u32,
    pub killed: bool,
    pub reaped: bool,
    pub status: Option<ExitStatus>,
    pub error: Option<&'static str>,
}
pub type CleanupLedger = Arc<Mutex<Vec<CleanupRecord>>>;
pub struct OwnedProcess {
    child: Option<Child>,
    pid: u32,
    ledger: CleanupLedger,
    _roots: Arc<PrivateRoots>,
}
impl OwnedProcess {
    pub fn spawn(
        executable: &Path,
        args: &[&std::ffi::OsStr],
        roots: &Arc<PrivateRoots>,
        ledger: CleanupLedger,
    ) -> Self {
        Self::spawn_named(executable, args, roots, ledger, "process")
    }
    pub fn spawn_named(
        executable: &Path,
        args: &[&std::ffi::OsStr],
        roots: &Arc<PrivateRoots>,
        ledger: CleanupLedger,
        name: &str,
    ) -> Self {
        let log = |name: &str| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(roots.control.join(name))
                .unwrap()
        };
        let child = Command::new(executable)
            .args(args)
            .current_dir(&roots.root)
            .stdin(Stdio::null())
            .stdout(log(&format!("{name}-stdout.log")))
            .stderr(log(&format!("{name}-stderr.log")))
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let pid = child.id().unwrap();
        Self {
            child: Some(child),
            pid,
            ledger,
            _roots: roots.clone(),
        }
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub async fn wait_exit(&mut self, deadline: Duration) -> Result<ExitStatus, &'static str> {
        let child = self.child.as_mut().ok_or("fixture already reaped")?;
        let status = match tokio::time::timeout(deadline, child.wait()).await {
            Ok(Ok(status)) => status,
            Ok(Err(_)) => {
                self.ledger.lock().unwrap().push(CleanupRecord {
                    pid: self.pid,
                    killed: false,
                    reaped: false,
                    status: None,
                    error: Some("fixture wait failed"),
                });
                return Err("fixture wait failed");
            }
            Err(_) => {
                self.terminate().await?;
                return Err("fixture exit deadline");
            }
        };
        self.child.take();
        self.ledger.lock().unwrap().push(CleanupRecord {
            pid: self.pid,
            killed: false,
            reaped: true,
            status: Some(status),
            error: None,
        });
        Ok(status)
    }
    /// This consumes ownership so dropping a pending startup future kills/reaps.
    pub async fn ready(
        mut self,
        descriptor: &Path,
        deadline: Duration,
    ) -> Result<Self, &'static str> {
        let startup = tokio::time::timeout(deadline, async {
            loop {
                if self
                    .child
                    .as_mut()
                    .unwrap()
                    .try_wait()
                    .map_err(|_| "startup wait failed")?
                    .is_some()
                {
                    return Err("fixture process exited before ready");
                }
                if descriptor.is_file() {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap_or(Err("fixture startup deadline"));
        match startup {
            Ok(()) => Ok(self),
            Err(error) => {
                self.terminate().await?;
                Err(error)
            }
        }
    }
    pub async fn terminate(&mut self) -> Result<(), &'static str> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        let mut killed = false;
        let result = async {
            match child
                .try_wait()
                .map_err(|_| "fixture pre-cleanup wait failed")?
            {
                Some(status) => Ok(status),
                None => {
                    killed = true;
                    child.start_kill().map_err(|_| "fixture kill failed")?;
                    tokio::time::timeout(Duration::from_secs(1), child.wait())
                        .await
                        .map_err(|_| "fixture reap deadline")?
                        .map_err(|_| "fixture reap failed")
                }
            }
        }
        .await;
        let status = match result {
            Ok(status) => status,
            Err(error) => {
                self.ledger.lock().unwrap().push(CleanupRecord {
                    pid: self.pid,
                    killed,
                    reaped: false,
                    status: None,
                    error: Some(error),
                });
                return Err(error); // Retain ownership for a bounded Drop cleanup retry.
            }
        };
        self.child.take();
        self.ledger.lock().unwrap().push(CleanupRecord {
            pid: self.pid,
            killed,
            reaped: true,
            status: Some(status),
            error: None,
        });
        Ok(())
    }
}
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        // Async Drop is unavailable. Keep ownership through bounded nonblocking wait
        // even when panic or an external select drops the startup/fixture future.
        let killed = !matches!(child.try_wait(), Ok(Some(_)));
        let kill_error = if killed {
            child.start_kill().err().map(|_| "fixture Drop kill failed")
        } else {
            None
        };
        let deadline = Instant::now() + Duration::from_secs(1);
        let (status, error) = loop {
            match child.try_wait() {
                Ok(Some(status)) => break (Some(status), kill_error),
                Err(_) => break (None, Some("fixture Drop wait failed")),
                Ok(None) if Instant::now() >= deadline => {
                    break (None, Some("fixture Drop reap deadline"))
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(1)),
            }
        };
        self.ledger
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(CleanupRecord {
                pid: self.pid,
                killed,
                reaped: status.is_some(),
                status,
                error,
            });
    }
}
