#![cfg(windows)]

use process_supervisor::platform::windows::feasibility::{
    run_job_object_spike, run_server_crash_spike, WindowsCapabilityReport,
};

#[test]
fn windows_report_requires_every_job_object_invariant() {
    let incomplete = WindowsCapabilityReport {
        suspended_create: true,
        assign_before_resume: true,
        kill_on_close: true,
        breakaway_denied: false,
    };
    assert!(!incomplete.supports_required_containment());
}

#[test]
fn hard_killing_the_job_owner_terminates_direct_and_descendant_processes() {
    let result_path = std::env::temp_dir().join(format!(
        "aw-045-windows-owner-crash-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let evidence = run_server_crash_spike(
        std::path::Path::new(env!("CARGO_BIN_EXE_process-supervisor-tree-fixture")),
        &result_path,
    )
    .expect("execute real Job owner crash spike");
    println!("{evidence:#?}");
    assert_eq!(evidence.owner_terminate_error, None);
    assert!(evidence.owner_wait_completed);
    assert!(evidence.direct_wait_completed);
    assert!(evidence.descendant_wait_completed);
}

#[test]
fn actual_job_object_assigns_before_resume_denies_breakaway_and_kills_on_close() {
    let result_path = std::env::temp_dir().join(format!(
        "aw-045-windows-job-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let evidence = run_job_object_spike(
        std::path::Path::new(env!("CARGO_BIN_EXE_process-supervisor-tree-fixture")),
        &result_path,
    )
    .expect("execute real Job Object spike");
    println!("{evidence:#?}");
    assert!(evidence.suspended_create);
    assert!(evidence.assign_before_resume);
    assert!(evidence.resumed_once);
    assert!(evidence.active_processes_before_close >= 2);
    assert!(evidence.breakaway_denied);
    assert!(evidence.direct_wait_completed);
    assert!(evidence.descendant_wait_completed);
}
