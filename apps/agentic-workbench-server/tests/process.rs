//! 044 T018(research R4·R5, contracts/server-lifecycle.md §1–§3): 서버 바이너리 프로세스 시험. 한 데이터 디렉터리에
//! 데이터를 여는 서버는 하나이고, 강제 종료 뒤 다음 시작이 복구하며, 안내 파일·잠금은 소유 사용자만 읽는다. 다른
//! 데이터 디렉터리는 따로 뜨고, 모르는 저장 형식은 데이터를 건드리지 않고 거절한다.
//!
//! 동기화는 고정 sleep이 아니라 프로세스 종료 대기와 상한 있는 polling으로 한다. 이 시험이 띄운 PID는 실패해도
//! `Cleanup`이 정확히 그 PID만 끝낸다.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

use workbench_host::lifecycle::{client::verify, descriptor::read_descriptor};

const BIN: &str = env!("CARGO_BIN_EXE_agentic-workbench-server");

/// 이 시험이 시작한 PID만 끝낸다(실패해도 drop에서).
#[derive(Default)]
struct Cleanup(Mutex<Vec<u32>>);

impl Cleanup {
    fn track(&self, pid: u32) {
        self.0.lock().unwrap().push(pid);
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        for pid in self.0.lock().unwrap().drain(..) {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }
}

fn server_dir(data: &Path) -> PathBuf {
    data.join("workbench").join("server")
}

fn serve(data: &Path) -> Child {
    Command::new(BIN)
        .args(["serve", "--data-dir"])
        .arg(data)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn serve")
}

fn ensure(data: &Path) -> (i32, String) {
    let output = Command::new(BIN)
        .args(["ensure", "--data-dir"])
        .arg(data)
        .env("AW_WORKBENCH_SERVER_PATH", BIN)
        .stdin(Stdio::null())
        .output()
        .expect("run ensure");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

fn wait_until(deadline: Duration, what: &str, mut ready: impl FnMut() -> bool) {
    let until = Instant::now() + deadline;
    while Instant::now() < until {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {what}");
}

fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[test]
fn only_one_of_ten_concurrent_servers_opens_the_data_dir() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    let mut children: Vec<Child> = (0..10).map(|_| serve(&data)).collect();
    for child in &children {
        cleanup.track(child.id());
    }
    let mut exits: Vec<Option<i32>> = vec![None; 10];
    wait_until(Duration::from_secs(60), "nine servers to give up", || {
        for (index, child) in children.iter_mut().enumerate() {
            if exits[index].is_none()
                && let Ok(Some(status)) = child.try_wait()
            {
                exits[index] = Some(status.code().unwrap_or(-1));
            }
        }
        exits.iter().filter(|exit| exit.is_some()).count() == 9
    });
    assert!(
        exits.iter().flatten().all(|code| *code == 3),
        "losers exit with 3: {exits:?}"
    );
    let winner = exits.iter().position(|exit| exit.is_none()).unwrap();
    wait_until(Duration::from_secs(30), "the winner's descriptor", || {
        read_descriptor(&server_dir(&data))
            .ok()
            .flatten()
            .is_some_and(|descriptor| verify(&descriptor).is_ok())
    });
    let descriptor = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    assert_eq!(
        descriptor.pid,
        children[winner].id(),
        "the running server owns the descriptor"
    );

    unsafe {
        libc::kill(children[winner].id() as i32, libc::SIGTERM);
    }
    let status = children[winner].wait().unwrap();
    assert_eq!(status.code(), Some(0), "a signalled server stops cleanly");
    assert!(
        read_descriptor(&server_dir(&data)).unwrap().is_none(),
        "the stopped server removes its descriptor"
    );
}

#[test]
fn ensure_recovers_within_five_seconds_after_the_server_is_killed() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    let (code, _) = ensure(&data);
    assert_eq!(code, 0, "first ensure starts a server");
    let first = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track(first.pid);

    unsafe {
        libc::kill(first.pid as i32, libc::SIGKILL);
    }
    wait_until(
        Duration::from_secs(10),
        "the killed server to disappear",
        || !alive(first.pid),
    );
    assert!(
        read_descriptor(&server_dir(&data)).unwrap().is_some(),
        "a killed server leaves its descriptor behind"
    );

    let started = Instant::now();
    let (code, stdout) = ensure(&data);
    let elapsed = started.elapsed();
    assert_eq!(code, 0, "{stdout}");
    let second = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track(second.pid);
    assert_ne!(
        second.instance_id, first.instance_id,
        "a new server instance"
    );
    assert!(verify(&second).is_ok());
    assert!(elapsed < Duration::from_secs(5), "recovered in {elapsed:?}");
    assert!(
        !stdout.contains(&second.owner_token),
        "ensure never prints the owner token"
    );
}

#[test]
fn server_files_are_owner_only() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    assert_eq!(ensure(&data).0, 0);
    let descriptor = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track(descriptor.pid);
    let mode = |path: PathBuf| fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(server_dir(&data)), 0o700);
    for file in ["server.json", "owner.lock", "startup.lock"] {
        assert_eq!(mode(server_dir(&data).join(file)), 0o600, "{file}");
    }
}

#[test]
fn separate_data_dirs_get_separate_servers() {
    let cleanup = Cleanup::default();
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    let one = fs::canonicalize(one.path()).unwrap();
    let two = fs::canonicalize(two.path()).unwrap();
    assert_eq!(ensure(&one).0, 0);
    assert_eq!(ensure(&two).0, 0);
    let first = read_descriptor(&server_dir(&one)).unwrap().unwrap();
    let second = read_descriptor(&server_dir(&two)).unwrap().unwrap();
    cleanup.track(first.pid);
    cleanup.track(second.pid);
    assert_ne!(first.instance_id, second.instance_id);
    assert_ne!(first.base_url, second.base_url);
    assert!(verify(&first).is_ok() && verify(&second).is_ok());
}

#[test]
fn an_unknown_storage_schema_is_refused_without_touching_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    let ledger = data.join("workbench").join("ledger.sqlite");
    fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    {
        let conn = rusqlite::Connection::open(&ledger).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_version (version, applied_at) VALUES (99, '2026-09-28T00:00:00Z');",
        )
        .unwrap();
    }
    let before = fs::read(&ledger).unwrap();
    let output = Command::new(BIN)
        .args(["serve", "--data-dir"])
        .arg(&data)
        .stdin(Stdio::null())
        .output()
        .expect("run serve");
    assert_eq!(
        output.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&ledger).unwrap(),
        before,
        "the ledger is untouched"
    );
    assert!(read_descriptor(&server_dir(&data)).unwrap().is_none());
}
