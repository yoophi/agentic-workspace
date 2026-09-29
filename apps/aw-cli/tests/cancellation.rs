mod support;
use support::{
    assert_error, finish,
    peer::{Action, Peer},
    spawn, state_directory,
};
use tokio::io::AsyncWriteExt;
use workbench_client::{
    domain::limits::Limits, infrastructure::retry_store::PrivateRetryStore, ports::RetryStore,
};
use workbench_protocol::Outcome;
async fn submitted(peer: &Peer) {
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if peer.requests.lock().unwrap().len() == 3 {
                break;
            }
            peer.received.notified().await;
        }
    })
    .await
    .expect("CLI never submitted");
}
#[tokio::test]
async fn sigint_after_submission_exits_130_retains_unknown_and_never_cancels_server() {
    let mut peer = Peer::spawn(vec![Action::Pause], true).await;
    let state = state_directory();
    let root = state.path().canonicalize().unwrap();
    let mut child = spawn(&[
        "call",
        "project.create",
        "--input",
        "-",
        "--descriptor",
        peer.descriptor.to_str().unwrap(),
        "--state-dir",
        root.to_str().unwrap(),
    ]);
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"name":"private-sentinel","workingDirectory":"/private/tmp"}"#)
        .await
        .unwrap();
    drop(child.stdin.take());
    submitted(&peer).await;
    assert_eq!(
        unsafe { libc::kill(child.id().unwrap() as i32, libc::SIGINT) },
        0
    );
    let output = finish(child, b"").await;
    let error = assert_error(&output, 130);
    assert_eq!(error["error"]["outcome"], "unknown");
    let path = error["attempt"]["retryState"].as_str().unwrap();
    let store = PrivateRetryStore::open(std::path::Path::new(path), Limits::default()).unwrap();
    assert_eq!(store.load().unwrap().outcome, Outcome::Unknown);
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
    assert_eq!(peer.effects.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn local_deadline_retains_unknown_and_does_not_issue_cancel() {
    let mut peer = Peer::spawn(vec![Action::Pause], true).await;
    let state = state_directory();
    let root = state.path().canonicalize().unwrap();
    let child = spawn(&[
        "call",
        "project.create",
        "--input",
        "-",
        "--descriptor",
        peer.descriptor.to_str().unwrap(),
        "--state-dir",
        root.to_str().unwrap(),
        "--timeout-ms",
        "500",
    ]);
    let output = finish(
        child,
        br#"{"name":"private-sentinel","workingDirectory":"/private/tmp"}"#,
    )
    .await;
    let error = assert_error(&output, 7);
    assert_eq!(error["error"]["outcome"], "unknown");
    assert!(error["attempt"]["retryState"].is_string());
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
}
#[tokio::test]
async fn sigint_during_unfinished_stdin_has_bounded_exit() {
    use std::{
        io::Write,
        os::fd::{AsRawFd, FromRawFd},
        process::Stdio,
    };
    let mut fds = [-1; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let read = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let mut writer = unsafe { std::fs::File::from_raw_fd(fds[1]) };
    // Keep a read descriptor solely to observe kernel unread-byte count, never read it.
    writer.write_all(b"{").unwrap();
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_aw"))
        .args(["call", "project.list", "--input", "-"])
        .stdin(Stdio::from(read.try_clone().unwrap()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let mut unread: libc::c_int = -1;
            assert_eq!(
                unsafe { libc::ioctl(read.as_raw_fd(), libc::FIONREAD, &mut unread) },
                0
            );
            if unread == 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("child did not consume the unfinished input");
    // The application is now awaiting more input. Signal registration precedes job spawn.
    assert_eq!(
        unsafe { libc::kill(child.id().unwrap() as i32, libc::SIGINT) },
        0
    );
    let output = support::finish_with_open_input(child, writer).await;
    let error = assert_error(&output, 130);
    assert_eq!(error["error"]["code"], "cancelled");
    drop(read);
}

#[tokio::test]
async fn broken_stdout_pipe_has_safe_error_and_bounded_exit() {
    let mut child = spawn(&["operations"]);
    drop(child.stdout.take());
    let output = finish(child, b"").await;
    let error = assert_error(&output, 8);
    assert_eq!(error["error"]["code"], "outputUnavailable");
    assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
}
