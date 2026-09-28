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
        let current_identity = process_start_identity(pid).ok();
        if self.expected_identity.is_some() && current_identity == self.expected_identity {
            // SAFETY: this isolated fixture cleanup compares the full recorded
            // start identity immediately before signal. The comparison still
            // cannot close the check-to-signal PID-reuse window, so it is not
            // production containment evidence.
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
        let _ = fs::remove_file(&self.path);
    }
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
fn target_report_does_not_claim_unproven_containment() {
    let report = capability_report(true);
    println!("{report:#?}");
    assert!(!report.blockers.is_empty());
    assert!(
        !report.supports_required_containment(),
        "the spike must not claim readiness before env-clear tracking and atomic signaling are proven"
    );
}
