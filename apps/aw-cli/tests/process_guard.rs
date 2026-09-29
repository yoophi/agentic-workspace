#[path = "support/process_guard.rs"]
mod guard;
#[allow(dead_code)]
#[path = "../../../crates/workbench-client/tests/support/harness.rs"]
mod harness;
mod support;
use guard::{CleanupLedger, OwnedProcess, PrivateRoots};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
fn sleeper(roots: &Arc<PrivateRoots>, ledger: CleanupLedger) -> OwnedProcess {
    OwnedProcess::spawn(
        Path::new("/bin/sleep"),
        &[std::ffi::OsStr::new("30")],
        roots,
        ledger,
    )
}
fn reaped(ledger: &CleanupLedger, pid: u32) {
    let records = ledger.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].pid, pid);
    assert!(records[0].killed && records[0].reaped);
    assert!(records[0].error.is_none());
    assert!(records[0].status.is_some());
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
#[tokio::test]
async fn startup_deadline_and_external_future_drop_kill_and_reap_the_owned_process() {
    for cancelled in [false, true] {
        let roots = PrivateRoots::new();
        let ledger = Arc::new(Mutex::new(Vec::new()));
        let process = sleeper(&roots, ledger.clone());
        let pid = process.pid();
        if cancelled {
            let descriptor = roots.descriptor();
            let mut future = Box::pin(process.ready(&descriptor, Duration::from_secs(30)));
            assert!(
                std::future::poll_fn(|cx| std::task::Poll::Ready(std::future::Future::poll(
                    future.as_mut(),
                    cx
                )))
                .await
                .is_pending()
            );
            drop(future);
        } else {
            assert!(matches!(
                process
                    .ready(&roots.descriptor(), Duration::from_millis(20))
                    .await,
                Err("fixture startup deadline")
            ));
        }
        reaped(&ledger, pid);
    }
}
#[tokio::test]
async fn panic_error_and_explicit_cleanup_preserve_single_reap_and_private_roots() {
    use std::os::unix::fs::PermissionsExt;
    for mode in [0, 1, 2] {
        let roots = PrivateRoots::new();
        for path in [&roots.root, &roots.data, &roots.control] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        let ledger = Arc::new(Mutex::new(Vec::new()));
        let mut process = sleeper(&roots, ledger.clone());
        let pid = process.pid();
        match mode {
            0 => {
                process.terminate().await.unwrap();
                process.terminate().await.unwrap();
                drop(process);
            }
            1 => drop(process),
            _ => assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                    let _owner = process;
                    panic!("guard fixture panic");
                }))
                .is_err()
            ),
        }
        reaped(&ledger, pid);
    }
}

#[tokio::test]
async fn process_ownership_keeps_private_roots_alive_until_the_child_is_reaped() {
    let roots = PrivateRoots::new();
    let path = roots.root.clone();
    let weak = Arc::downgrade(&roots);
    let ledger = Arc::new(Mutex::new(Vec::new()));
    let mut process = sleeper(&roots, ledger.clone());
    let pid = process.pid();
    drop(roots);
    assert!(weak.upgrade().is_some() && path.is_dir());
    process.terminate().await.unwrap();
    drop(process);
    reaped(&ledger, pid);
    assert!(weak.upgrade().is_none());
    assert!(!path.exists());
}

#[tokio::test]
async fn cancelling_a_timed_out_private_call_wait_joins_task_and_observes_peer_eof() {
    use harness::{HarnessConnection, OwnedTask};
    use serde_json::json;
    use support::peer::{Action, Peer};
    use workbench_protocol::{CallRequest, OperationId};
    let mut peer = Peer::spawn(vec![Action::Pause], true).await;
    let mut http = HarnessConnection::connect(peer.endpoint.clone()).await;
    let mut task = OwnedTask::default();
    task.start(async move {
        http.post(
            "/v1/calls",
            serde_json::to_value(CallRequest::command(
                OperationId::OrchestrationRecover,
                json!({"benchId":"private-empty"}),
            ))
            .unwrap(),
            true,
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let notified = peer.received.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if peer.requests.lock().unwrap().iter().any(|(path, _, body)| {
                path == "/v1/calls" && body["operation"] == "orchestration.recover"
            }) {
                break;
            }
            notified.await;
        }
    })
    .await
    .unwrap();
    assert!(!task.is_finished());
    assert!(tokio::time::timeout(Duration::from_millis(20), task.wait())
        .await
        .is_err());
    task.cancel_join().await.unwrap();
    peer.settled().await; // HTTP driver/socket EOF, not merely aborting the fixture.
}

#[tokio::test]
async fn finite_probe_wait_and_exit_deadline_both_reap_without_detaching() {
    let roots = PrivateRoots::new();
    let ledger = Arc::new(Mutex::new(Vec::new()));
    let mut process = OwnedProcess::spawn(Path::new("/usr/bin/true"), &[], &roots, ledger.clone());
    assert!(process
        .wait_exit(Duration::from_secs(1))
        .await
        .unwrap()
        .success());
    drop(process);
    {
        let records = ledger.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert!(
            records[0].reaped
                && !records[0].killed
                && records[0].status.unwrap().success()
                && records[0].error.is_none()
        );
    }
    let roots = PrivateRoots::new();
    let ledger = Arc::new(Mutex::new(Vec::new()));
    let mut process = sleeper(&roots, ledger.clone());
    let pid = process.pid();
    assert!(matches!(
        process.wait_exit(Duration::from_millis(20)).await,
        Err("fixture exit deadline")
    ));
    drop(process);
    reaped(&ledger, pid);
}
