#![cfg(unix)]

use std::{
    fs,
    process::{Child, Command},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use process_supervisor::platform::unix::feasibility::{
    capability_report, probe_environment_marker, process_environment, process_start_identity,
    ProcessStartIdentity,
};
#[cfg(target_os = "macos")]
use process_supervisor::platform::unix::feasibility::{
    current_process_audit_token, probe_identity_safe_signal, signal_audit_token,
};
#[cfg(target_os = "linux")]
use process_supervisor::platform::unix::feasibility::{probe_cgroup_v2_delegation, LinuxPidFd};

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn spawn(command: &mut Command) -> Self {
        Self(Some(command.spawn().expect("spawn probe child")))
    }

    fn id(&self) -> u32 {
        self.0.as_ref().expect("live child").id()
    }

    fn cleanup(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn exited_within(&mut self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if self
                .0
                .as_mut()
                .expect("live child")
                .try_wait()
                .expect("query child")
                .is_some()
            {
                self.0.take();
                return true;
            }
            thread::sleep(Duration::from_millis(10));
        }
        false
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.cleanup();
    }
}

struct EscapedPidCleanup {
    path: std::path::PathBuf,
    expected_identity: Option<ProcessStartIdentity>,
}

impl EscapedPidCleanup {
    fn arm(&mut self, identity: ProcessStartIdentity) {
        self.expected_identity = Some(identity);
    }
}

impl Drop for EscapedPidCleanup {
    fn drop(&mut self) {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(pid) = text.trim().parse::<u32>() else {
            return;
        };
        if let Some(expected_identity) = self.expected_identity {
            identity_guarded_fixture_kill(pid, expected_identity);
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn identity_guarded_fixture_kill(pid: u32, expected_identity: ProcessStartIdentity) {
    let current_identity = process_start_identity(pid).ok();
    if current_identity == Some(expected_identity) {
        // SAFETY: this isolated fixture cleanup compares the full recorded
        // start identity immediately before signal. The comparison still
        // cannot close the check-to-signal PID-reuse window, so it is not
        // production containment evidence.
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGKILL);
        }
    }
}

fn wait_for_identity_to_disappear(pid: u32, expected_identity: ProcessStartIdentity) -> bool {
    for _ in 0..200 {
        if process_start_identity(pid).ok() != Some(expected_identity) {
            return true;
        }
        thread::sleep(Duration::from_millis(10));
    }
    false
}

fn wait_for_observation(pid: u32) {
    for _ in 0..50 {
        if process_start_identity(pid).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("child {pid} never became observable");
}

fn wait_for_fixture_result(path: &std::path::Path) -> bool {
    for _ in 0..100 {
        if let Ok(value) = fs::read(path) {
            return value == b"1";
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("fixture did not report its marker state");
}

#[cfg(target_os = "macos")]
fn wait_for_bytes(path: &std::path::Path, expected_len: usize) -> Vec<u8> {
    for _ in 0..100 {
        if let Ok(value) = fs::read(path) {
            if value.len() == expected_len {
                return value;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("fixture did not write {expected_len} bytes");
}

fn wait_for_pid(path: &std::path::Path) -> u32 {
    for _ in 0..100 {
        if let Ok(text) = fs::read_to_string(path) {
            if let Ok(pid) = text.trim().parse::<u32>() {
                return pid;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("fixture did not write a PID");
}

#[test]
fn fixture_child_confirms_marker_and_waits() {
    if std::env::var_os("AW_045_FIXTURE_CHILD").is_none() {
        return;
    }
    let result_path = std::env::var_os("AW_045_RESULT_PATH").expect("result path");
    let present = std::env::var_os("AW_045_NONCE").is_some();
    fs::write(result_path, if present { b"1" } else { b"0" }).expect("write marker result");
    thread::sleep(Duration::from_secs(30));
}

#[cfg(target_os = "macos")]
#[test]
fn fixture_child_exports_own_audit_token_and_waits() {
    if std::env::var_os("AW_045_AUDIT_CHILD").is_none() {
        return;
    }
    let result_path = std::env::var_os("AW_045_RESULT_PATH").expect("result path");
    let token = current_process_audit_token().expect("read own audit token");
    fs::write(result_path, token).expect("write audit token");
    thread::sleep(Duration::from_secs(30));
}

#[test]
fn start_identity_is_observable_and_environment_permission_is_recorded() {
    let marker = format!("aw-045-{}", std::process::id());
    let result_path = std::env::temp_dir().join(format!(
        "aw-045-marker-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args([
            "--exact",
            "fixture_child_confirms_marker_and_waits",
            "--nocapture",
        ])
        .env("AW_045_FIXTURE_CHILD", "1")
        .env("AW_045_RESULT_PATH", &result_path)
        .env("AW_045_NONCE", &marker);
    let mut child = ChildGuard::spawn(&mut command);
    let pid = child.id();
    wait_for_observation(pid);
    let fixture_saw_marker = wait_for_fixture_result(&result_path);

    let identity = process_start_identity(pid).expect("read start identity");
    let evidence = probe_environment_marker(pid, b"AW_045_NONCE", marker.as_bytes());
    child.cleanup();
    let _ = fs::remove_file(&result_path);

    assert_eq!(identity.pid, pid);
    assert!(
        fixture_saw_marker,
        "fixture must observe its injected marker"
    );
    println!(
        "procargs_size={} errno={:?} raw_marker_present={} parsed_key_present={} parsed_value_matches={}",
        evidence.returned_size,
        evidence.sysctl_errno,
        evidence.raw_marker_present,
        evidence.parsed_key_present,
        evidence.parsed_value_matches
    );
    assert!(
        !evidence.raw_marker_present || evidence.parsed_value_matches,
        "raw marker visibility with parser miss is a parser defect"
    );
    let report = capability_report(evidence.parsed_value_matches);
    assert_eq!(
        report.same_uid_environment_readable,
        evidence.parsed_value_matches
    );
    assert!(!report.supports_required_containment());
}

#[test]
fn env_clear_removes_the_nonce_used_for_descendant_discovery() {
    let mut command = Command::new("/bin/sleep");
    command.arg("30").env_clear();
    let mut child = ChildGuard::spawn(&mut command);
    wait_for_observation(child.id());
    let environment = process_environment(child.id()).expect("read cleared environment");
    child.cleanup();
    assert!(!environment.contains_key(b"AW_045_NONCE".as_slice()));
}

#[test]
fn env_clear_and_new_session_escape_removes_raw_and_parsed_nonce() {
    let marker = format!("aw-045-escape-{}", std::process::id());
    let result_path = std::env::temp_dir().join(format!(
        "aw-045-escape-pid-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let mut cleanup = EscapedPidCleanup {
        path: result_path.clone(),
        expected_identity: None,
    };
    let status = Command::new(env!("CARGO_BIN_EXE_process-supervisor-tree-fixture"))
        .arg("env-clear-exec")
        .env("AW_045_NONCE", &marker)
        .env("AW_045_RESULT_PATH", &result_path)
        .status()
        .expect("run escape fixture");
    assert!(status.success());
    let pid = fs::read_to_string(&result_path)
        .expect("escaped pid recorded")
        .trim()
        .parse::<u32>()
        .expect("valid escaped pid");
    wait_for_observation(pid);
    cleanup.arm(process_start_identity(pid).expect("record escaped start identity"));
    let evidence = probe_environment_marker(pid, b"AW_045_NONCE", marker.as_bytes());
    println!(
        "escape_procargs_size={} errno={:?} raw_marker_present={} parsed_key_present={}",
        evidence.returned_size,
        evidence.sysctl_errno,
        evidence.raw_marker_present,
        evidence.parsed_key_present
    );
    assert!(!evidence.raw_marker_present);
    assert!(!evidence.parsed_key_present);
}

#[test]
fn keeper_only_hard_kill_leaves_an_unattributed_env_clear_descendant() {
    let marker = format!("aw-045-keeper-{}", std::process::id());
    let result_path = std::env::temp_dir().join(format!(
        "aw-045-keeper-child-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let mut escaped_cleanup = EscapedPidCleanup {
        path: result_path.clone(),
        expected_identity: None,
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_process-supervisor-tree-fixture"));
    command
        .arg("keeper-env-clear")
        .env("AW_045_NONCE", &marker)
        .env("AW_045_RESULT_PATH", &result_path);
    let mut keeper = ChildGuard::spawn(&mut command);
    let child_pid = wait_for_pid(&result_path);
    wait_for_observation(child_pid);
    let identity = process_start_identity(child_pid).expect("escaped identity");
    escaped_cleanup.arm(identity);
    keeper.cleanup();

    let after_keeper_death = process_start_identity(child_pid).expect("escaped child remains live");
    let evidence = probe_environment_marker(child_pid, b"AW_045_NONCE", marker.as_bytes());
    assert_eq!(after_keeper_death, identity);
    assert!(!evidence.raw_marker_present);
    println!(
        "keeper_dead=true escaped_child_live=true nonce_visible={} cleanup_atomic=false",
        evidence.raw_marker_present
    );
}

#[test]
fn server_and_keeper_hard_kill_leave_the_env_clear_descendant_for_recovery() {
    let suffix = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    );
    let descendant_path = std::env::temp_dir().join(format!("aw-045-combined-child-{suffix}"));
    let keeper_path = std::env::temp_dir().join(format!("aw-045-combined-keeper-{suffix}"));
    let mut descendant_cleanup = EscapedPidCleanup {
        path: descendant_path.clone(),
        expected_identity: None,
    };
    let mut keeper_cleanup = EscapedPidCleanup {
        path: keeper_path.clone(),
        expected_identity: None,
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_process-supervisor-tree-fixture"));
    command
        .arg("server-keeper-env-clear")
        .env("AW_045_NONCE", format!("aw-045-combined-{suffix}"))
        .env("AW_045_RESULT_PATH", &descendant_path)
        .env("AW_045_KEEPER_PID_PATH", &keeper_path);
    let mut server = ChildGuard::spawn(&mut command);
    let keeper_pid = wait_for_pid(&keeper_path);
    let descendant_pid = wait_for_pid(&descendant_path);
    wait_for_observation(keeper_pid);
    wait_for_observation(descendant_pid);
    let keeper_identity = process_start_identity(keeper_pid).expect("keeper identity");
    let descendant_identity = process_start_identity(descendant_pid).expect("descendant identity");
    keeper_cleanup.arm(keeper_identity);
    descendant_cleanup.arm(descendant_identity);

    server.cleanup();
    identity_guarded_fixture_kill(keeper_pid, keeper_identity);
    assert!(
        wait_for_identity_to_disappear(keeper_pid, keeper_identity),
        "keeper hard kill must complete before recovery observation"
    );
    let after_combined_death = process_start_identity(descendant_pid)
        .expect("escaped descendant remains for startup recovery");
    assert_eq!(after_combined_death, descendant_identity);
    println!(
        "server_dead=true keeper_dead=true escaped_child_live=true recovery_anchor_required=true cleanup_atomic=false"
    );
}

#[test]
fn target_report_does_not_claim_unproven_containment() {
    let report = capability_report(true);
    println!("{report:#?}");
    assert!(!report.blockers.is_empty());
    assert!(
        !report.supports_required_containment(),
        "the spike must not claim readiness before env-clear tracking and atomic signaling are proven"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn pidfd_signal_terminates_the_exact_opened_process() {
    let mut command = Command::new("/bin/sleep");
    command.arg("30");
    let mut child = ChildGuard::spawn(&mut command);
    let identity = process_start_identity(child.id()).expect("record child identity");
    let pidfd = LinuxPidFd::open(child.id()).expect("open pidfd for exact child");
    pidfd.signal(libc::SIGTERM).expect("signal through pidfd");
    assert!(child.exited_within(Duration::from_secs(2)));
    assert_ne!(process_start_identity(identity.pid).ok(), Some(identity));
}

#[cfg(target_os = "linux")]
#[test]
fn cgroup_v2_delegation_is_measured_without_claiming_descendant_containment() {
    let evidence = probe_cgroup_v2_delegation();
    println!("{evidence:#?}");
    assert!(
        !evidence.child_cgroup_creatable || evidence.unified_hierarchy,
        "a writable probe directory must belong to the unified hierarchy"
    );
    let report = capability_report(true);
    assert!(
        !report.supports_required_containment(),
        "directory writability alone does not prove env-clear descendant containment"
    );
}

#[cfg(target_os = "linux")]
struct DelegatedCgroupCleanup {
    cgroup: std::path::PathBuf,
    result: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl Drop for DelegatedCgroupCleanup {
    fn drop(&mut self) {
        let _ = fs::write(self.cgroup.join("cgroup.kill"), "1");
        for _ in 0..100 {
            let empty = fs::read_to_string(self.cgroup.join("cgroup.events"))
                .map(|events| events.lines().any(|line| line == "populated 0"))
                .unwrap_or(false);
            if empty {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = fs::remove_dir(&self.cgroup);
        let _ = fs::remove_file(&self.result);
    }
}

#[cfg(target_os = "linux")]
fn wait_for_pid_pair(path: &std::path::Path) -> (u32, u32) {
    for _ in 0..200 {
        if let Ok(value) = fs::read_to_string(path) {
            let mut fields = value.trim().split(':');
            let direct = fields.next().and_then(|field| field.parse::<u32>().ok());
            let descendant = fields.next().and_then(|field| field.parse::<u32>().ok());
            if let (Some(direct), Some(descendant)) = (direct, descendant) {
                return (direct, descendant);
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("cgroup payload did not report direct and descendant PIDs");
}

#[cfg(target_os = "linux")]
fn process_cgroup(pid: u32) -> String {
    fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .expect("read fixture cgroup")
        .lines()
        .find_map(|line| line.strip_prefix("0::").map(str::to_owned))
        .expect("fixture belongs to unified cgroup hierarchy")
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an actual systemd Delegate=yes service boundary"]
fn delegated_cgroup_birth_placement_contains_env_clear_descendant() {
    assert_eq!(
        std::env::var("AW_045_DELEGATED_CGROUP").as_deref(),
        Ok("1"),
        "run only inside the dedicated delegated-cgroup CI service"
    );
    let evidence = probe_cgroup_v2_delegation();
    assert!(evidence.unified_hierarchy);
    assert!(evidence.cgroup_kill_available);
    assert!(evidence.child_cgroup_creatable);
    let relative = evidence.current_path.expect("current delegated cgroup");
    let current = std::path::Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    let attempt_name = format!("aw-045-attempt-{}", std::process::id());
    let attempt = current.join(&attempt_name);
    fs::create_dir(&attempt).expect("create attempt cgroup in delegated subtree");
    let result = std::env::temp_dir().join(format!(
        "aw-045-linux-cgroup-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let cleanup = DelegatedCgroupCleanup {
        cgroup: attempt.clone(),
        result: result.clone(),
    };

    let status = Command::new(env!("CARGO_BIN_EXE_process-supervisor-tree-fixture"))
        .arg("linux-clone-into-cgroup")
        .arg(&attempt)
        .arg(&result)
        .status()
        .expect("run clone3 birth-placement fixture");
    assert!(status.success());
    let (direct_pid, descendant_pid) = wait_for_pid_pair(&result);
    let direct_identity = process_start_identity(direct_pid).expect("direct identity");
    let descendant_identity = process_start_identity(descendant_pid).expect("descendant identity");
    let expected_suffix = format!("/{attempt_name}");
    assert!(process_cgroup(direct_pid).ends_with(&expected_suffix));
    assert!(process_cgroup(descendant_pid).ends_with(&expected_suffix));

    fs::write(attempt.join("cgroup.kill"), "1").expect("kill attempt cgroup");
    let mut empty = false;
    for _ in 0..250 {
        empty = fs::read_to_string(attempt.join("cgroup.events"))
            .map(|events| events.lines().any(|line| line == "populated 0"))
            .unwrap_or(false);
        if empty {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        empty,
        "attempt cgroup must become unpopulated after cgroup.kill"
    );
    assert!(wait_for_identity_to_disappear(direct_pid, direct_identity));
    assert!(wait_for_identity_to_disappear(
        descendant_pid,
        descendant_identity
    ));
    println!(
        "delegated=true clone_into_cgroup=true env_clear_descendant_contained=true populated_zero=true"
    );
    drop(cleanup);
}

#[cfg(target_os = "macos")]
#[test]
fn audit_token_signal_api_records_actual_ordinary_process_permission() {
    let mut command = Command::new("/bin/sleep");
    command.arg("30");
    let mut child = ChildGuard::spawn(&mut command);
    wait_for_observation(child.id());
    let evidence = probe_identity_safe_signal(child.id());
    println!(
        "task_for_pid_status={} task_info_status={:?} signal_result={:?} signal_errno={:?}",
        evidence.task_for_pid_status,
        evidence.task_info_status,
        evidence.signal_result,
        evidence.signal_errno
    );
    if evidence.signal_result == Some(0) {
        assert!(
            child.exited_within(Duration::from_secs(2)),
            "successful audit-token signal must terminate the exact fixture"
        );
    } else {
        assert!(
            evidence.task_for_pid_status != 0
                || evidence.task_info_status != Some(0)
                || evidence.signal_errno.is_some(),
            "failed signal probe must preserve its failure boundary"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn cooperative_audit_token_handshake_signals_the_exact_child() {
    let result_path = std::env::temp_dir().join(format!(
        "aw-045-audit-token-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let mut command = Command::new(std::env::current_exe().expect("test executable"));
    command
        .args([
            "--exact",
            "fixture_child_exports_own_audit_token_and_waits",
            "--nocapture",
        ])
        .env("AW_045_AUDIT_CHILD", "1")
        .env("AW_045_RESULT_PATH", &result_path);
    let mut child = ChildGuard::spawn(&mut command);
    wait_for_observation(child.id());
    let token: [u8; 32] = wait_for_bytes(&result_path, 32)
        .try_into()
        .expect("fixed audit token length");
    signal_audit_token(token, libc::SIGTERM).expect("signal exact audit-token process");
    assert!(child.exited_within(Duration::from_secs(2)));
    let _ = fs::remove_file(result_path);
}
