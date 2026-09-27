//! 외부 서버 모드의 데스크톱 쪽 서버 연결(044 T028, research R5·R6·R9, contracts/desktop-client.md §1–§3).
//!
//! - `connect`: 안내 파일로 서버를 찾거나 띄운다(`workbench-host` `ensure`: 신원 증명 뒤 자격 증명). 처음 붙을 때 임대를
//!   잡고 10초마다 갱신한다. 서버가 바뀌면(재기동) 다음 호출이 다시 `ensure`한다.
//! - 창 토큰: 소유자 자격 증명으로 `desktop.issueWindowToken`(창 주체·WebView 출처 묶음).
//! - 작업대: 창 토큰(+ 그 출처)으로 `bench.open` — 작업대는 창 주체 소유다.
//! - 창 폐기: `desktop.retireWindow{closeBench}`. 진행 중 폐기 수를 세어 종료 경로가 짧은 상한 안에 흘려보낸다.
//!
//! HTTP 호출은 루프백 blocking 클라이언트라 blocking pool에서 돈다.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use serde_json::{Value, json};
use tokio::sync::{Mutex, Notify};
use workbench_host::lifecycle::{
    calls::{CallError, call},
    descriptor::Descriptor,
    ensure::{EnsureOptions, ensure},
};

pub const SERVER_EXECUTABLE: &str = "agentic-workbench-server";
pub const LEASE_RENEW_EVERY: Duration = Duration::from_secs(10);
/// 종료 경로가 폐기·임대 해제를 기다리는 상한(contracts/desktop-client.md §3).
pub const EXIT_FLUSH_LIMIT: Duration = Duration::from_secs(2);

/// 서버 실행 파일 탐색(contracts/desktop-client.md §1): `AW_WORKBENCH_SERVER_PATH` → 앱 실행 파일 옆 → 개발 빌드 산출물.
pub fn discover_server_executable() -> PathBuf {
    let sibling = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(SERVER_EXECUTABLE)));
    let dev = [
        dev_output(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }),
        dev_output(if cfg!(debug_assertions) {
            "release"
        } else {
            "debug"
        }),
    ];
    pick_executable(
        std::env::var_os("AW_WORKBENCH_SERVER_PATH").map(PathBuf::from),
        sibling,
        &dev,
    )
}

fn dev_output(profile: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target")
        .join(profile)
        .join(SERVER_EXECUTABLE)
}

/// 순수 선택: 명시 경로(있으면 존재와 상관없이 — 틀리면 기동 오류로 드러난다) → 존재하는 옆 파일 → 존재하는 개발 산출물 →
/// 옆 경로(없음, 기동 오류가 이유를 보여 준다).
fn pick_executable(
    explicit: Option<PathBuf>,
    sibling: Option<PathBuf>,
    dev: &[PathBuf],
) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    if let Some(path) = sibling.as_ref().filter(|path| path.is_file()) {
        return path.clone();
    }
    if let Some(path) = dev.iter().find(|path| path.is_file()) {
        return path.clone();
    }
    sibling.unwrap_or_else(|| PathBuf::from(SERVER_EXECUTABLE))
}

struct Connected {
    descriptor: Descriptor,
    lease_id: Option<String>,
}

pub struct ExternalServer {
    data_dir: PathBuf,
    executable: PathBuf,
    client_id: String,
    connected: Mutex<Option<Connected>>,
    pending: AtomicUsize,
    settled: Notify,
}

impl ExternalServer {
    pub fn new(data_dir: PathBuf, executable: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            data_dir,
            executable,
            client_id: uuid::Uuid::new_v4().to_string(),
            connected: Mutex::new(None),
            pending: AtomicUsize::new(0),
            settled: Notify::new(),
        })
    }

    /// 확인된 서버의 안내. 없으면 `ensure`(서버 기동 포함)하고 임대를 잡는다. 동시에 부르면 한 번만 `ensure`한다.
    pub async fn connect(self: &Arc<Self>) -> Result<Descriptor, String> {
        let mut connected = self.connected.lock().await;
        if let Some(existing) = connected.as_ref() {
            return Ok(existing.descriptor.clone());
        }
        let data_dir = self.data_dir.clone();
        let executable = self.executable.clone();
        let descriptor = tokio::task::spawn_blocking(move || {
            ensure(&data_dir, &executable, &EnsureOptions::default())
        })
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
        let lease = self
            .owner_call(
                &descriptor,
                "lease.acquire",
                json!({ "clientKind": "desktop", "clientId": self.client_id }),
                true,
            )
            .await
            .map_err(|error| error.to_string())?;
        let lease_id = lease["leaseId"].as_str().map(str::to_owned);
        *connected = Some(Connected {
            descriptor: descriptor.clone(),
            lease_id: lease_id.clone(),
        });
        drop(connected);
        if let Some(lease_id) = lease_id {
            self.spawn_renewal(descriptor.instance_id.clone(), lease_id);
        }
        Ok(descriptor)
    }

    fn spawn_renewal(self: &Arc<Self>, instance_id: String, lease_id: String) {
        let server = Arc::downgrade(self);
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(LEASE_RENEW_EVERY).await;
                let Some(server) = server.upgrade() else {
                    return;
                };
                let Some(descriptor) = server.current(&instance_id).await else {
                    return; // 다른 서버로 바뀌었거나 연결을 잊었다
                };
                match server
                    .owner_call(
                        &descriptor,
                        "lease.renew",
                        json!({ "leaseId": lease_id }),
                        true,
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("[workbench-server] lease renewal failed: {error}");
                        server.forget(&instance_id).await;
                        return;
                    }
                }
            }
        });
    }

    async fn current(&self, instance_id: &str) -> Option<Descriptor> {
        let connected = self.connected.lock().await;
        connected
            .as_ref()
            .filter(|state| state.descriptor.instance_id == instance_id)
            .map(|state| state.descriptor.clone())
    }

    /// 그 인스턴스와의 연결을 잊는다(다음 호출이 다시 `ensure`한다).
    async fn forget(&self, instance_id: &str) {
        let mut connected = self.connected.lock().await;
        if connected
            .as_ref()
            .is_some_and(|state| state.descriptor.instance_id == instance_id)
        {
            *connected = None;
        }
    }

    async fn owner_call(
        &self,
        descriptor: &Descriptor,
        operation: &'static str,
        input: Value,
        command: bool,
    ) -> Result<Value, CallError> {
        let base = descriptor.base_url.clone();
        let token = descriptor.owner_token.clone();
        tokio::task::spawn_blocking(move || call(&base, &token, None, operation, input, command))
            .await
            .map_err(|error| CallError::Transport(error.to_string()))?
    }

    /// 소유자 호출. 전송 오류면 연결을 잊고 한 번 다시 `ensure`해 부른다(서버 재기동).
    async fn owner_call_reconnecting(
        self: &Arc<Self>,
        operation: &'static str,
        input: Value,
        command: bool,
    ) -> Result<(Descriptor, Value), String> {
        let descriptor = self.connect().await?;
        match self
            .owner_call(&descriptor, operation, input.clone(), command)
            .await
        {
            Ok(output) => Ok((descriptor, output)),
            Err(CallError::Transport(_)) => {
                self.forget(&descriptor.instance_id).await;
                let descriptor = self.connect().await?;
                self.owner_call(&descriptor, operation, input, command)
                    .await
                    .map(|output| (descriptor, output))
                    .map_err(|error| error.to_string())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// 창 토큰(`desktop.issueWindowToken`). `(base_url, token, expires_at)`.
    pub async fn issue_window_token(
        self: &Arc<Self>,
        label: &str,
        incarnation: &str,
        origin: &str,
    ) -> Result<(String, String, String), String> {
        let (descriptor, output) = self
            .owner_call_reconnecting(
                "desktop.issueWindowToken",
                json!({ "label": label, "incarnation": incarnation, "origin": origin }),
                true,
            )
            .await?;
        let token = output["token"]
            .as_str()
            .ok_or("the server returned no window token")?
            .to_owned();
        let expires_at = output["expiresAt"].as_str().unwrap_or_default().to_owned();
        Ok((descriptor.base_url, token, expires_at))
    }

    /// 창 주체로 작업대를 연다(창 토큰 + 출처).
    pub async fn open_bench(
        self: &Arc<Self>,
        label: &str,
        incarnation: &str,
        origin: &str,
        working_directory: &str,
    ) -> Result<String, String> {
        let (base, token, _) = self.issue_window_token(label, incarnation, origin).await?;
        let origin = origin.to_owned();
        let input = json!({ "workingDirectory": working_directory });
        let output = tokio::task::spawn_blocking(move || {
            call(&base, &token, Some(&origin), "bench.open", input, true)
        })
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
        output["benchId"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "the server returned no bench id".to_owned())
    }

    /// 창 폐기(`desktop.retireWindow`). 진행 중 수를 세어 `flush`가 기다릴 수 있게 한다. 서버에 붙은 적이 없으면
    /// 새로 띄우지 않는다(닫는 창 때문에 서버를 기동하지 않음). 앱은 분리 실행판(`retire_window_detached`)을 쓴다.
    #[cfg(test)]
    pub async fn retire_window(
        self: &Arc<Self>,
        label: &str,
        incarnation: &str,
        close_bench: bool,
    ) -> Result<Value, String> {
        self.pending.fetch_add(1, Ordering::SeqCst);
        let result = self.retire_inner(label, incarnation, close_bench).await;
        self.settle_one();
        result
    }

    async fn retire_inner(
        &self,
        label: &str,
        incarnation: &str,
        close_bench: bool,
    ) -> Result<Value, String> {
        let descriptor = {
            let connected = self.connected.lock().await;
            connected.as_ref().map(|state| state.descriptor.clone())
        };
        let Some(descriptor) = descriptor else {
            return Ok(Value::Null);
        };
        self.owner_call(
            &descriptor,
            "desktop.retireWindow",
            json!({ "label": label, "incarnation": incarnation, "closeBench": close_bench }),
            true,
        )
        .await
        .map_err(|error| error.to_string())
    }

    /// 종료 경로: 진행 중 폐기를 상한 안에서 기다리고 임대를 푼다. `close_all_benches`는 부르지 않는다(앱 종료 ≠ 창 닫기).
    pub async fn release_for_exit(self: &Arc<Self>, limit: Duration) {
        let deadline = tokio::time::Instant::now() + limit;
        let _ = tokio::time::timeout_at(deadline, async {
            loop {
                let settled = self.settled.notified();
                tokio::pin!(settled);
                settled.as_mut().enable();
                if self.pending.load(Ordering::SeqCst) == 0 {
                    return;
                }
                settled.await;
            }
        })
        .await;
        let state = {
            let mut connected = self.connected.lock().await;
            connected.take()
        };
        if let Some(Connected {
            descriptor,
            lease_id: Some(lease_id),
        }) = state
        {
            let _ = tokio::time::timeout_at(
                deadline.max(tokio::time::Instant::now() + Duration::from_millis(200)),
                self.owner_call(
                    &descriptor,
                    "lease.release",
                    json!({ "leaseId": lease_id }),
                    true,
                ),
            )
            .await;
        }
    }

    /// 창 `Destroyed`에서 부른다: 진행 중 수를 **곧바로** 올린 뒤 폐기를 띄운다. 종료 경로의 `release_for_exit`가 task의
    /// 첫 poll보다 먼저 돌아도 이 폐기를 기다린다.
    pub fn retire_window_detached(
        self: &Arc<Self>,
        label: String,
        incarnation: String,
        close_bench: bool,
    ) {
        self.pending.fetch_add(1, Ordering::SeqCst);
        let server = self.clone();
        tauri::async_runtime::spawn(async move {
            let result = server.retire_inner(&label, &incarnation, close_bench).await;
            if let Err(error) = result {
                eprintln!("[workbench-server] failed to retire window {label}: {error}");
            }
            server.settle_one();
        });
    }

    fn settle_one(&self) {
        if self.pending.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.settled.notify_waiters();
        }
    }

    pub fn pending_retirements(&self) -> usize {
        self.pending.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use workbench_core::application::workbench_runtime::RuntimeAdapters;
    use workbench_host::{
        assembly::{HostAssembly, HostOptions, assemble},
        lifecycle::{
            descriptor::write_descriptor, identity::OwnerIdentity, lock::ensure_server_dir,
        },
    };

    use super::*;

    const ORIGIN: &str = "tauri://localhost";

    struct Running {
        runtime: tokio::runtime::Runtime,
        host: HostAssembly,
        dir: tempfile::TempDir,
        work: String,
    }

    /// 프로세스 안의 host 조립 + 안내 파일: `ensure`는 확인을 통과한 안내로 붙고 서버를 띄우지 않는다.
    fn running_server() -> Running {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let work = dir.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let identity = OwnerIdentity::generate();
        let mut options = HostOptions::new(
            data.clone(),
            RuntimeAdapters::production(),
            "test",
            runtime.handle().clone(),
        );
        options.owner = Some(identity.clone());
        let host = assemble(options).unwrap();
        let descriptor = Descriptor::for_endpoint(
            "server",
            &identity,
            host.runtime.epoch(),
            host.http.as_ref().unwrap().base_url(),
            "test",
        );
        write_descriptor(&ensure_server_dir(&data).unwrap(), &descriptor).unwrap();
        Running {
            runtime,
            host,
            dir,
            work: std::fs::canonicalize(work)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        }
    }

    #[test]
    fn the_desktop_attaches_leases_opens_a_window_bench_and_retires_it() {
        let server = running_server();
        let client = ExternalServer::new(
            server.dir.path().join("data"),
            PathBuf::from("/nonexistent/agentic-workbench-server"),
        );
        server.runtime.block_on(async {
            let descriptor = client
                .connect()
                .await
                .expect("attach to the running server");
            let status = call(
                &descriptor.base_url,
                &descriptor.owner_token,
                None,
                "server.status",
                json!({}),
                false,
            )
            .unwrap();
            assert_eq!(
                status["leases"],
                json!(1),
                "the desktop holds one lease: {status}"
            );

            let (_, token, _) = client
                .issue_window_token("session-a", "i1", ORIGIN)
                .await
                .unwrap();
            assert!(!token.is_empty());
            let bench = client
                .open_bench("session-a", "i1", ORIGIN, &server.work)
                .await
                .unwrap();
            let listed = call(
                &descriptor.base_url,
                &descriptor.owner_token,
                None,
                "bench.list",
                json!({}),
                false,
            )
            .unwrap();
            assert!(listed.to_string().contains(&bench), "{listed}");

            let retired = client.retire_window("session-a", "i1", true).await.unwrap();
            assert_eq!(retired["closedBenches"], json!([bench]), "{retired}");
            assert_eq!(client.pending_retirements(), 0);
            // 폐기한 창에는 더 발급하지 않는다(tombstone).
            assert!(
                client
                    .issue_window_token("session-a", "i1", ORIGIN)
                    .await
                    .is_err()
            );

            client.release_for_exit(EXIT_FLUSH_LIMIT).await;
            let status = call(
                &descriptor.base_url,
                &descriptor.owner_token,
                None,
                "server.status",
                json!({}),
                false,
            )
            .unwrap();
            assert_eq!(
                status["leases"],
                json!(0),
                "exit released the lease: {status}"
            );
        });
        server.runtime.block_on(server.host.shutdown());
    }

    /// 창 `Destroyed` 직후 앱이 끝나도(종료 경로가 폐기 task의 첫 poll보다 먼저 돌아도) 폐기는 흘려보내진다.
    #[test]
    fn an_exit_right_after_a_window_destroy_still_retires_that_window() {
        let server = running_server();
        let client = ExternalServer::new(
            server.dir.path().join("data"),
            PathBuf::from("/nonexistent/agentic-workbench-server"),
        );
        server.runtime.block_on(async {
            let descriptor = client.connect().await.unwrap();
            let bench = client
                .open_bench("session-b", "i1", ORIGIN, &server.work)
                .await
                .unwrap();
            client.retire_window_detached("session-b".into(), "i1".into(), true);
            assert_eq!(
                client.pending_retirements(),
                1,
                "counted before the task runs"
            );
            client.release_for_exit(EXIT_FLUSH_LIMIT).await;
            assert_eq!(client.pending_retirements(), 0);
            let listed = call(
                &descriptor.base_url,
                &descriptor.owner_token,
                None,
                "bench.list",
                json!({}),
                false,
            )
            .unwrap();
            assert!(
                !listed.to_string().contains(&bench),
                "the retired window's bench is closed: {listed}"
            );
        });
        server.runtime.block_on(server.host.shutdown());
    }

    #[test]
    fn retiring_without_a_server_connection_never_starts_a_server() {
        let dir = tempfile::tempdir().unwrap();
        let client = ExternalServer::new(
            dir.path().to_path_buf(),
            PathBuf::from("/nonexistent/server"),
        );
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(client.retire_window("session-x", "i1", true));
        assert_eq!(result, Ok(Value::Null));
        assert!(
            !dir.path().join("workbench").exists(),
            "no server directory was created"
        );
    }

    #[test]
    fn a_missing_server_executable_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let client = ExternalServer::new(
            dir.path().to_path_buf(),
            PathBuf::from("/nonexistent/agentic-workbench-server"),
        );
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let error = runtime.block_on(client.connect()).unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn executable_discovery_prefers_explicit_then_sibling_then_dev_output() {
        let dir = tempfile::tempdir().unwrap();
        let sibling = dir.path().join("sibling");
        let dev = dir.path().join("dev");
        std::fs::write(&dev, b"").unwrap();
        let explicit = PathBuf::from("/explicit/server");
        assert_eq!(
            pick_executable(
                Some(explicit.clone()),
                Some(sibling.clone()),
                std::slice::from_ref(&dev)
            ),
            explicit
        );
        assert_eq!(
            pick_executable(None, Some(sibling.clone()), std::slice::from_ref(&dev)),
            dev,
            "missing sibling falls back to dev output"
        );
        std::fs::write(&sibling, b"").unwrap();
        assert_eq!(
            pick_executable(None, Some(sibling.clone()), std::slice::from_ref(&dev)),
            sibling
        );
        let none = dir.path().join("none");
        assert_eq!(
            pick_executable(None, Some(none.clone()), &[]),
            none,
            "nothing found: report the sibling path"
        );
    }
}
