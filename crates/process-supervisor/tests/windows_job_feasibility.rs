#![cfg(windows)]

use process_supervisor::platform::windows::feasibility::WindowsCapabilityReport;

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
