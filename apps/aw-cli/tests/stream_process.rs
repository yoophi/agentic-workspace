mod support;
use serde_json::{json, Value};
use std::time::Duration;
use support::peer::{Action, Peer};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Child,
};
use tokio_tungstenite::tungstenite::Message;
fn hello() -> Message {
    Message::Text(json!({"type":"hello","protocolVersion":1,"epoch":"e"}).to_string())
}
fn event(sequence: u64, reason: &str) -> Message {
    Message::Text(json!({"type":"event","event":{"eventId":format!("event-{sequence}"),"streamId":"orchestration:binding","epoch":"e","sequence":sequence,"schema":"orchestration.workspaceUpdated.v1","occurredAt":"2026-09-29T00:00:00Z","body":{"workspaceId":"workspace","revision":2,"reason":reason}}}).to_string())
}
fn ticket() -> Action {
    Action::Reply(
        200,
        json!({"ticket":"private-sentinel","expiresAt":"2026-09-29T00:00:30Z"}),
    )
}
async fn watch(peer: &Peer, timeout: &str) -> Child {
    let mut child = support::spawn(&[
        "events",
        "watch",
        "--input",
        "-",
        "--descriptor",
        peer.descriptor.to_str().unwrap(),
        "--timeout-ms",
        timeout,
    ]);
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            json!({"streamId":"orchestration:binding","epoch":"e","afterSequence":0})
                .to_string()
                .as_bytes(),
        )
        .await
        .unwrap();
    drop(stdin);
    child
}
async fn record<R: tokio::io::AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Value {
    let mut line = String::new();
    let count = tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    assert!(count > 0, "missing JSONL record");
    assert!(line.ends_with('\n'));
    assert!(!line.contains("private-sentinel") && !line.contains("fixture-private-token"));
    serde_json::from_str(&line).unwrap()
}
fn interrupt(child: &Child) {
    assert_eq!(
        unsafe { libc::kill(child.id().unwrap() as i32, libc::SIGINT) },
        0
    );
}
#[tokio::test]
async fn actual_watch_outputs_same_revision_events_separately_and_sigint_end_with_exact_ack() {
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![
            hello(),
            event(1, "runtimeReconciled"),
            event(2, "notificationRecovery"),
        ]),
    ])
    .await;
    let mut child = watch(&peer, "500").await;
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    assert_eq!(record(&mut reader).await["type"], "stream.open");
    for (sequence, reason) in [(1, "runtimeReconciled"), (2, "notificationRecovery")] {
        let value = record(&mut reader).await;
        assert_eq!(value["event"]["sequence"], sequence);
        assert_eq!(value["event"]["body"]["reason"], reason);
        assert_eq!(value["event"]["body"]["revision"], 2);
    }
    interrupt(&child);
    let end = record(&mut reader).await;
    assert_eq!(end["type"], "stream.end");
    assert_eq!(end["cursor"]["afterSequence"], 2);
    assert_eq!(end["error"]["code"], "cancelled");
    let output = support::finish(child, b"").await;
    assert_eq!(output.status.code(), Some(130));
    assert!(output.stderr.is_empty());
    assert_eq!(reader.read_line(&mut String::new()).await.unwrap(), 0);
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 5);
}
#[tokio::test]
async fn idle_watch_outlives_request_deadline_and_still_applies_later_event() {
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello()],
            gate: release.clone(),
            after: vec![event(1, "runtimeReconciled")],
        },
    ])
    .await;
    let mut child = watch(&peer, "50").await;
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    assert_eq!(record(&mut reader).await["type"], "stream.open");
    assert!(
        tokio::time::timeout(Duration::from_millis(150), child.wait())
            .await
            .is_err()
    );
    release.notify_one();
    assert_eq!(record(&mut reader).await["event"]["sequence"], 1);
    interrupt(&child);
    assert_eq!(record(&mut reader).await["cursor"]["afterSequence"], 1);
    assert_eq!(support::finish(child, b"").await.status.code(), Some(130));
    peer.settled().await;
}
#[tokio::test]
async fn invalid_frame_after_open_has_one_safe_end_and_no_finite_stdout_record() {
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello()],
            gate: release.clone(),
            after: vec![Message::Text(
                json!({"type":"private-sentinel"}).to_string(),
            )],
        },
    ])
    .await;
    let mut child = watch(&peer, "500").await;
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    assert_eq!(record(&mut reader).await["type"], "stream.open");
    release.notify_one();
    let end = record(&mut reader).await;
    assert_eq!(end["type"], "stream.end");
    assert_eq!(end["error"]["code"], "protocolViolation");
    assert_eq!(end["cursor"]["afterSequence"], 0);
    let output = support::finish(child, b"").await;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    peer.settled().await;
}

#[tokio::test]
async fn actual_watch_redirected_to_dev_null_stays_live_and_sigint_reaps_with_no_output_error() {
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![hello(), event(1, "runtimeReconciled")]),
    ])
    .await;
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_aw"))
        .args([
            "events",
            "watch",
            "--input",
            "-",
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
            "--timeout-ms",
            "500",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            json!({"streamId":"orchestration:binding","epoch":"e","afterSequence":0})
                .to_string()
                .as_bytes(),
        )
        .await
        .unwrap();
    drop(stdin);
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let notified = peer.received.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if peer.requests.lock().unwrap().len() == 5 {
                break;
            }
            notified.await;
        }
    })
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(100), child.wait())
            .await
            .is_err()
    );
    interrupt(&child);
    let output = support::finish(child, b"").await;
    assert_eq!(output.status.code(), Some(130));
    assert!(output.stderr.is_empty());
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 5);
}

#[tokio::test]
async fn actual_child_shared_stdout_flags_restore_after_sigint_and_protocol_exit() {
    use std::{
        io::Write,
        os::fd::{AsRawFd, OwnedFd},
    };
    for originally_nonblocking in [false, true] {
        for sigint in [false, true] {
            let release = std::sync::Arc::new(tokio::sync::Notify::new());
            let action = if sigint {
                Action::WebSocket(vec![hello()])
            } else {
                Action::WebSocketGate {
                    before: vec![hello()],
                    gate: release.clone(),
                    after: vec![Message::Text(json!({"type":"unknown"}).to_string())],
                }
            };
            let mut peer = Peer::spawn_multi(vec![ticket(), action]).await;
            let (read, mut shared) = std::os::unix::net::UnixStream::pair().unwrap();
            // Establish Darwin's kernel-only FWASWRITTEN before the flag baseline.
            assert_eq!(
                unsafe { libc::write(shared.as_raw_fd(), b"before\n".as_ptr().cast(), 7) },
                7
            );
            shared.set_nonblocking(originally_nonblocking).unwrap();
            read.set_nonblocking(true).unwrap();
            let mut reader = BufReader::new(tokio::net::UnixStream::from_std(read).unwrap());
            let mut before_line = String::new();
            reader.read_line(&mut before_line).await.unwrap();
            assert_eq!(before_line, "before\n");
            let before = unsafe { libc::fcntl(shared.as_raw_fd(), libc::F_GETFL) };
            assert!(before >= 0);
            let output: OwnedFd = shared.try_clone().unwrap().into();
            let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_aw"))
                .args([
                    "events",
                    "watch",
                    "--input",
                    "-",
                    "--descriptor",
                    peer.descriptor.to_str().unwrap(),
                    "--timeout-ms",
                    "500",
                ])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::from(output))
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            let mut stdin = child.stdin.take().unwrap();
            stdin
                .write_all(
                    json!({"streamId":"orchestration:binding","epoch":"e","afterSequence":0})
                        .to_string()
                        .as_bytes(),
                )
                .await
                .unwrap();
            drop(stdin);
            assert_eq!(record(&mut reader).await["type"], "stream.open");
            assert_ne!(
                unsafe { libc::fcntl(shared.as_raw_fd(), libc::F_GETFL) } & libc::O_NONBLOCK,
                0
            );
            // The active watch owns a temporary O_NONBLOCK lease on the shared description.
            // A concurrent parent writer must handle WouldBlock and avoid mixing JSONL records.
            shared.write_all(b"parent-active\n").unwrap();
            let mut parent_line = String::new();
            tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut parent_line))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(parent_line, "parent-active\n");
            let chunk = [b'p'; 8192];
            let mut written = 0;
            loop {
                match std::io::Write::write(&mut shared, &chunk) {
                    Ok(count) => {
                        assert!(count > 0);
                        written += count;
                        assert!(written <= 4 * 1024 * 1024);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => panic!("shared writer failed: {error}"),
                }
            }
            assert!(written > 0);
            let mut parent_bytes = vec![0; written];
            tokio::time::timeout(Duration::from_secs(1), reader.read_exact(&mut parent_bytes))
                .await
                .unwrap()
                .unwrap();
            assert!(parent_bytes.iter().all(|byte| *byte == b'p'));
            if sigint {
                interrupt(&child);
            } else {
                release.notify_one();
            }
            assert_eq!(record(&mut reader).await["type"], "stream.end");
            let result = support::finish(child, b"").await;
            assert_eq!(result.status.code(), Some(if sigint { 130 } else { 1 }));
            assert!(result.stderr.is_empty());
            assert_eq!(
                unsafe { libc::fcntl(shared.as_raw_fd(), libc::F_GETFL) },
                before
            );
            shared.write_all(b"after\n").unwrap();
            let mut after_line = String::new();
            tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut after_line))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(after_line, "after\n");
            peer.settled().await;
            assert_eq!(peer.requests.lock().unwrap().len(), 5);
        }
    }
}

#[tokio::test]
async fn actual_watch_gap_connects_live_then_loads_http_snapshot_and_outputs_reset_before_event() {
    let snapshot = json!({"schemaVersion":1,"id":"workspace","worktreePath":"/private/fixture","eventStreamId":"orchestration:binding","mainNodeId":"main","activeCoordinatorGenerationId":null,"nodes":[],"generations":[],"tasks":[],"reports":[],"commands":[],"coordinatorNotifications":[],"dispatches":[],"idempotencyRecords":[],"revision":2,"createdAt":"2026-09-29T00:00:00Z","updatedAt":"2026-09-29T00:00:00Z"});
    let mut peer = Peer::spawn_multi(vec![ticket(),Action::WebSocket(vec![hello(),Message::Text(json!({"type":"gap","streamId":"orchestration:binding","epoch":"e","reason":"retentionExceeded","firstSequence":5,"lastSequence":5}).to_string())]),
        ticket(),Action::WebSocketConcurrent(vec![hello(),event(6,"notificationRecovery")]),
        Action::Reply(200,json!({"kind":"complete","output":[{"benchId":"actual-bench","workingDirectory":"/private/fixture","owner":"owner","runs":[]}],"replayed":false})),
        Action::Reply(200,json!({"kind":"complete","output":snapshot,"replayed":false})),
    ]).await;
    let mut child = watch(&peer, "1000").await;
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    assert_eq!(record(&mut reader).await["type"], "stream.open");
    let reset = record(&mut reader).await;
    assert_eq!(reset["type"], "stream.reset");
    assert_eq!(reset["applied"]["afterSequence"], 0);
    assert_eq!(reset["cursor"]["afterSequence"], 5);
    assert_eq!(reset["snapshot"]["revision"], 2);
    assert_eq!(record(&mut reader).await["event"]["sequence"], 6);
    interrupt(&child);
    assert_eq!(record(&mut reader).await["cursor"]["afterSequence"], 6);
    assert_eq!(support::finish(child, b"").await.status.code(), Some(130));
    peer.settled().await;
    let requests = peer.requests.lock().unwrap();
    assert_eq!(requests.len(), 14);
    assert_eq!(
        requests
            .iter()
            .filter(|r| r.0 == "/v1/event-tickets")
            .map(|r| r.2["cursors"][0]["afterSequence"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![0, 5]
    );
    let calls = requests
        .iter()
        .filter(|r| r.0 == "/v1/calls")
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].2["operation"], "bench.list");
    assert_eq!(calls[1].2["input"]["benchId"], "actual-bench");
}

#[tokio::test]
async fn invalid_and_gated_watch_preflights_emit_only_one_finite_error_and_send_nothing() {
    let mut peer = Peer::spawn_multi(vec![]).await;
    for (input, exit, code) in [
        (
            json!({"streamId":"unknown:binding","epoch":"e","afterSequence":0}),
            2,
            "invalidArgument",
        ),
        (
            json!({"streamId":"orchestration:binding","epoch":"e","afterSequence":-1}),
            2,
            "invalidArgument",
        ),
        (
            json!({"streamId":"orchestration:binding","epoch":"e","afterSequence":0,"token":"private-sentinel"}),
            2,
            "invalidArgument",
        ),
        (
            json!({"streamId":"orchestration:binding","epoch":"wrong","afterSequence":0}),
            9,
            "unsupportedProtocol",
        ),
        (
            json!({"streamId":"run:existing","epoch":"e","afterSequence":0}),
            8,
            "prerequisiteUnavailable",
        ),
    ] {
        let result = support::run(
            &[
                "events",
                "watch",
                "--input",
                "-",
                "--descriptor",
                peer.descriptor.to_str().unwrap(),
            ],
            input.to_string().as_bytes(),
        )
        .await;
        assert_eq!(result.status.code(), Some(exit));
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr.iter().filter(|b| **b == b'\n').count(), 1);
        let value: Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(value["error"]["code"], code);
        assert!(!String::from_utf8_lossy(&result.stderr).contains("private-sentinel"));
    }
    let result = support::run(
        &[
            "run",
            "watch",
            "existing",
            "--after",
            "0",
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
        ],
        b"",
    )
    .await;
    assert_eq!(result.status.code(), Some(8));
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stderr).unwrap()["error"]["code"],
        "prerequisiteUnavailable"
    );
    for flags in [
        vec!["--after", "1"],
        vec!["--idempotency-key", "key"],
        vec!["--retry-state", "private-sentinel"],
        vec!["--expected-revision", "1"],
    ] {
        let mut args = vec![
            "events",
            "watch",
            "--input",
            "-",
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
        ];
        args.extend(flags);
        assert_eq!(support::run(&args, b"{}").await.status.code(), Some(2));
    }
    assert!(peer.requests.lock().unwrap().is_empty());
    peer.stop().await;
}
#[tokio::test]
async fn failed_proof_or_hello_before_open_is_finite_and_never_prints_private_wire() {
    let mut bad_proof = Peer::spawn(vec![], false).await;
    let result = support::finish(watch(&bad_proof, "500").await, b"").await;
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stderr).unwrap()["error"]["code"],
        "unauthenticated"
    );
    bad_proof.settled().await;
    assert_eq!(bad_proof.requests.lock().unwrap().len(), 1);
    assert!(!bad_proof.requests.lock().unwrap()[0].1);
    let mut bad_hello = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocket(vec![Message::Text(
            json!({"type":"hello","protocolVersion":999,"epoch":"e"}).to_string(),
        )]),
    ])
    .await;
    let result = support::finish(watch(&bad_hello, "500").await, b"").await;
    assert_eq!(result.status.code(), Some(9));
    assert!(result.stdout.is_empty());
    assert_eq!(result.stderr.iter().filter(|b| **b == b'\n').count(), 1);
    bad_hello.settled().await;
}
#[tokio::test]
async fn sigint_during_pending_owned_watch_proof_is_finite_and_settles_socket_without_credentials()
{
    let mut peer = Peer::spawn_at(vec![], true, "/v1/system/identify").await;
    let child = watch(&peer, "1000").await;
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let notified = peer.received.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if peer.requests.lock().unwrap().len() == 1 {
                break;
            }
            notified.await;
        }
    })
    .await
    .unwrap();
    interrupt(&child);
    let result = support::finish(child, b"").await;
    assert_eq!(result.status.code(), Some(130));
    assert!(result.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stderr).unwrap()["error"]["code"],
        "cancelled"
    );
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 1);
    assert!(!peer.requests.lock().unwrap()[0].1);
}
#[tokio::test]
async fn broken_stdout_after_open_exits_without_resnapshot_or_implicit_command() {
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello()],
            gate: release.clone(),
            after: vec![event(1, "runtimeReconciled")],
        },
    ])
    .await;
    let mut child = watch(&peer, "500").await;
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    assert_eq!(record(&mut reader).await["type"], "stream.open");
    drop(reader);
    release.notify_one();
    let result = support::finish(child, b"").await;
    assert_eq!(result.status.code(), Some(8));
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stderr).unwrap()["error"]["code"],
        "outputUnavailable"
    );
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 5);
}

#[tokio::test]
async fn sigint_while_shared_stdout_is_physically_backpressured_reaps_and_restores_flags() {
    use std::os::fd::{AsRawFd, OwnedFd};
    use tokio::io::AsyncReadExt;
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let frame = event(1, &"x".repeat(700 * 1024));
    let message_size = frame.len();
    let mut peer = Peer::spawn_multi(vec![
        ticket(),
        Action::WebSocketGate {
            before: vec![hello()],
            gate: release.clone(),
            after: vec![frame],
        },
    ])
    .await;
    let (read, shared) = std::os::unix::net::UnixStream::pair().unwrap();
    let send_buffer: libc::c_int = 4096;
    assert_eq!(
        unsafe {
            libc::setsockopt(
                shared.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                (&send_buffer as *const libc::c_int).cast(),
                std::mem::size_of_val(&send_buffer) as libc::socklen_t,
            )
        },
        0
    );
    assert_eq!(
        unsafe { libc::write(shared.as_raw_fd(), b"before\n".as_ptr().cast(), 7) },
        7
    );
    let before = unsafe { libc::fcntl(shared.as_raw_fd(), libc::F_GETFL) };
    read.set_nonblocking(true).unwrap();
    let mut reader = BufReader::new(tokio::net::UnixStream::from_std(read).unwrap());
    reader.read_line(&mut String::new()).await.unwrap();
    let output: OwnedFd = shared.try_clone().unwrap().into();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_aw"))
        .args([
            "events",
            "watch",
            "--input",
            "-",
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
            "--timeout-ms",
            "1000",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::from(output))
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            json!({"streamId":"orchestration:binding","epoch":"e","afterSequence":0})
                .to_string()
                .as_bytes(),
        )
        .await
        .unwrap();
    drop(stdin);
    assert_eq!(record(&mut reader).await["type"], "stream.open");
    release.notify_one();
    // POLLOUT false plus resident bytes smaller than the record proves physical backpressure.
    // No read drains the event while we wait for or send SIGINT.
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let mut poll = libc::pollfd {
                fd: shared.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            assert!(unsafe { libc::poll(&mut poll, 1, 0) } >= 0);
            let mut bytes: libc::c_int = 0;
            assert_eq!(
                unsafe { libc::ioctl(reader.get_ref().as_raw_fd(), libc::FIONREAD, &mut bytes) },
                0
            );
            if poll.revents & libc::POLLOUT == 0
                && bytes > 0
                && (bytes as usize) + reader.buffer().len() < message_size
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    interrupt(&child);
    let result = support::finish(child, b"").await;
    assert_eq!(result.status.code(), Some(130));
    assert!(result.stderr.is_empty());
    assert_eq!(
        unsafe { libc::fcntl(shared.as_raw_fd(), libc::F_GETFL) },
        before
    );
    drop(shared);
    let mut partial = Vec::new();
    tokio::time::timeout(Duration::from_secs(1), reader.read_to_end(&mut partial))
        .await
        .unwrap()
        .unwrap();
    assert!(!partial.is_empty());
    assert!(!partial.contains(&b'\n'));
    assert!(!String::from_utf8_lossy(&partial).contains("stream.end"));
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 5);
}

#[path = "../../../crates/workbench-client/tests/support/harness.rs"]
mod harness;
#[tokio::test]
async fn actual_aw_bootstrap_and_recover_jsonl_matches_independent_consumer_with_both_reply_orders()
{
    use futures_util::FutureExt;
    use harness::{HarnessConnection, OwnedTask};
    use std::{panic::AssertUnwindSafe, sync::Arc};
    use tokio::sync::Notify;
    use workbench_protocol::{CallRequest, OperationId};
    for events_first in [false, true] {
        let release_events = Arc::new(Notify::new());
        let release_reply = Arc::new(Notify::new());
        let frame = |sequence, revision, reason| {
            let mut value: Value =
                serde_json::from_str(event(sequence, reason).to_text().unwrap()).unwrap();
            value["event"]["body"]["revision"] = json!(revision);
            Message::Text(value.to_string())
        };
        let expected = [
            frame(1, 1, "bootstrap"),
            frame(2, 2, "runtimeReconciled"),
            frame(3, 2, "notificationRecovery"),
        ];
        let snapshot = json!({"workspaceId":"workspace","revision":2,"currentRunId":null});
        let mut peer = Peer::spawn_multi(vec![
            ticket(),
            Action::WebSocketGateConcurrent {
                before: vec![hello(), expected[0].clone()],
                gate: release_events.clone(),
                after: expected[1..].to_vec(),
            },
            Action::ReplyGate {
                status: 200,
                body: json!({"kind":"complete","output":snapshot,"revision":2}),
                gate: release_reply.clone(),
            },
            Action::Reply(
                200,
                json!({"kind":"complete","output":snapshot,"revision":2}),
            ),
        ])
        .await;
        let mut child = watch(&peer, "1500").await;
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        let mut call = OwnedTask::default();
        let outcome = AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(3), async {
            assert_eq!(record(&mut reader).await["type"], "stream.open");
            let bootstrap = record(&mut reader).await;
            assert_eq!(
                bootstrap["event"],
                serde_json::from_str::<Value>(expected[0].to_text().unwrap()).unwrap()["event"]
            );
            let mut http = HarnessConnection::connect(peer.endpoint.clone()).await;
            call.start(async move {
                let result = http
                    .post(
                        "/v1/calls",
                        serde_json::to_value(CallRequest::command(
                            OperationId::OrchestrationRecover,
                            json!({"benchId":"bench"}),
                        ))
                        .unwrap(),
                        true,
                    )
                    .await;
                (http, result)
            });
            loop {
                let ready = peer.received.notified();
                tokio::pin!(ready);
                ready.as_mut().enable();
                if peer.requests.lock().unwrap().iter().any(|(path, _, body)| {
                    path == "/v1/calls" && body["operation"] == "orchestration.recover"
                }) {
                    break;
                }
                ready.await;
            }
            assert!(!call.is_finished());
            let mut records = vec![bootstrap["event"].clone()];
            if events_first {
                release_events.notify_one();
                for _ in 0..2 {
                    records.push(record(&mut reader).await["event"].clone());
                }
                assert!(!call.is_finished());
                release_reply.notify_one();
            } else {
                release_reply.notify_one();
            }
            let (mut http, reply) = call.wait().await;
            assert_eq!(reply["output"], snapshot);
            if !events_first {
                release_events.notify_one();
                for _ in 0..2 {
                    records.push(record(&mut reader).await["event"].clone());
                }
            }
            let final_snapshot = http
                .post(
                    "/v1/calls",
                    serde_json::to_value(CallRequest::query(
                        OperationId::OrchestrationGet,
                        json!({"benchId":"bench"}),
                    ))
                    .unwrap(),
                    true,
                )
                .await;
            assert_eq!(final_snapshot["output"], snapshot);
            http.close().await;
            // Compare actual JSONL with an independent port consumer and its applied cursor.
            let independent = independent_projection(&expected).await;
            assert_eq!(records, independent);
            for (index, value) in records.iter().enumerate() {
                assert_eq!(
                    *value,
                    serde_json::from_str::<Value>(expected[index].to_text().unwrap()).unwrap()
                        ["event"]
                );
            }
            interrupt(&child);
            let end = record(&mut reader).await;
            assert_eq!(end["type"], "stream.end");
            assert_eq!(end["cursor"]["afterSequence"], 3);
            assert_eq!(end["error"]["code"], "cancelled");
        }))
        .catch_unwind()
        .await;
        let call_cleanup = call.cancel_join().await;
        if !matches!(outcome, Ok(Ok(()))) {
            child.start_kill().unwrap();
            tokio::time::timeout(Duration::from_secs(1), child.wait())
                .await
                .expect("CLI failed fixture cleanup deadline")
                .unwrap();
            drop(reader);
            peer.stop().await;
        } else {
            assert_eq!(support::finish(child, b"").await.status.code(), Some(130));
            assert_eq!(reader.read_line(&mut String::new()).await.unwrap(), 0);
            peer.settled().await;
        }
        assert!(call_cleanup.is_ok(), "{call_cleanup:?}");
        match outcome {
            Ok(result) => result.expect("ordering subprocess deadline after cleanup"),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
}

struct IndependentProjection(Vec<workbench_protocol::workbench::EventEnvelope>);
#[async_trait::async_trait]
impl workbench_client::ports::EventConsumer for IndependentProjection {
    async fn consume(
        &mut self,
        event: &workbench_protocol::workbench::EventEnvelope,
    ) -> Result<(), workbench_client::ports::ClientError> {
        self.0.push(event.clone());
        Ok(())
    }
    async fn reset(
        &mut self,
        _: &workbench_client::ports::Snapshot,
    ) -> Result<(), workbench_client::ports::ClientError> {
        Err(workbench_client::ports::ClientError::Protocol)
    }
}
async fn independent_projection(frames: &[Message]) -> Vec<Value> {
    use workbench_client::{
        application::events::EventRecovery, domain::limits::Limits, ports::EventConsumer,
    };
    use workbench_protocol::{events::EventFrame, workbench::StreamCursor};
    let mut model = EventRecovery::new(
        StreamCursor {
            stream_id: "orchestration:binding".into(),
            epoch: "e".into(),
            after_sequence: 0,
        },
        Limits::default(),
    )
    .unwrap();
    let id = model.register(0).unwrap();
    model.hello(1, "e").unwrap();
    let mut consumer = IndependentProjection(Vec::new());
    for frame in frames {
        let EventFrame::Event { event } = serde_json::from_str(frame.to_text().unwrap()).unwrap()
        else {
            panic!("expected event fixture")
        };
        model.receive(event).unwrap();
        let delivery = model.next(id).unwrap().unwrap();
        consumer.consume(&delivery.event).await.unwrap();
        model.ack(delivery).unwrap();
    }
    assert_eq!(model.cursor().after_sequence, 3);
    consumer
        .0
        .into_iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect()
}

#[tokio::test]
async fn actual_watch_terminal_snapshot_auth_and_protocol_errors_keep_cause_without_retry() {
    use workbench_protocol::{FaultCode, Outcome};
    for (action, code, exit) in [
        (
            Action::Fault(FaultCode::Unauthenticated, Outcome::NotApplied),
            "unauthenticated",
            3,
        ),
        (
            Action::Raw(200, b"{invalid-json".to_vec()),
            "protocolViolation",
            1,
        ),
    ] {
        let release = std::sync::Arc::new(tokio::sync::Notify::new());
        let mut peer=Peer::spawn_multi(vec![ticket(),Action::WebSocketGate {before:vec![hello()],gate:release.clone(),after:vec![Message::Text(json!({"type":"gap","streamId":"orchestration:binding","epoch":"e","reason":"retentionExceeded","firstSequence":5,"lastSequence":5}).to_string())]}, ticket(),Action::WebSocketConcurrent(vec![hello(),event(6,"notificationRecovery")]),action]).await;
        let mut child = watch(&peer, "500").await;
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        use futures_util::FutureExt;
        let outcome = tokio::time::timeout(
            Duration::from_secs(3),
            std::panic::AssertUnwindSafe(async {
                assert_eq!(record(&mut reader).await["type"], "stream.open");
                release.notify_one();
                let end = record(&mut reader).await;
                assert_eq!(end["type"], "stream.end");
                assert_eq!(end["error"]["code"], code);
                assert_eq!(end["cursor"]["afterSequence"], 0);
            })
            .catch_unwind(),
        )
        .await;
        if !matches!(outcome, Ok(Ok(()))) {
            child.start_kill().ok();
        }
        // Preserve child ownership through bounded wait/reap even when assertions/timeouts fail.
        let output = support::finish(child, b"").await;
        peer.settled().await;
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(panic)) => std::panic::resume_unwind(panic),
            Err(_) => panic!("snapshot CLI deadline after owned kill/reap and peer EOF"),
        }
        assert_eq!(output.status.code(), Some(exit));
        assert!(output.stderr.is_empty());
        let mut eof = String::new();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), reader.read_line(&mut eof))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        let requests = peer.requests.lock().unwrap();
        assert_eq!(requests.iter().filter(|r| r.0 == "/v1/calls").count(), 1);
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.0 == "/v1/event-tickets")
                .count(),
            2
        );
        assert_eq!(requests.len(), 13);
    }
}

#[tokio::test]
async fn actual_watch_near_http_body_limit_snapshot_writes_full_reset_then_acks_live_event() {
    let mut snapshot = json!({"schemaVersion":1,"id":"workspace","worktreePath":"","eventStreamId":"orchestration:binding","mainNodeId":"main","activeCoordinatorGenerationId":null,"nodes":[],"generations":[],"tasks":[],"reports":[],"commands":[],"coordinatorNotifications":[],"dispatches":[],"idempotencyRecords":[],"revision":2,"createdAt":"2026-09-29T00:00:00Z","updatedAt":"2026-09-29T00:00:00Z"});
    let mut reply = json!({"kind":"complete","output":snapshot,"replayed":false});
    let overhead = serde_json::to_vec(&reply).unwrap().len();
    let payload_bytes = 8 * 1024 * 1024 - overhead;
    reply["output"]["worktreePath"] = json!("x".repeat(payload_bytes));
    snapshot = reply["output"].clone();
    assert_eq!(serde_json::to_vec(&reply).unwrap().len(), 8 * 1024 * 1024);
    assert!(serde_json::to_vec(&json!({"type":"stream.reset","cursor":{"streamId":"orchestration:binding","epoch":"e","afterSequence":5},"applied":{"streamId":"orchestration:binding","epoch":"e","afterSequence":0},"snapshot":snapshot})).unwrap().len()+1>8*1024*1024);
    let release = std::sync::Arc::new(tokio::sync::Notify::new());
    let mut peer=Peer::spawn_multi(vec![ticket(),Action::WebSocketGate {before:vec![hello()],gate:release.clone(),after:vec![Message::Text(json!({"type":"gap","streamId":"orchestration:binding","epoch":"e","reason":"retentionExceeded","firstSequence":5,"lastSequence":5}).to_string())]},ticket(),Action::WebSocketConcurrent(vec![hello(),event(6,"notificationRecovery")]),Action::Reply(200,json!({"kind":"complete","output":[{"benchId":"actual-bench","workingDirectory":"/private/fixture","owner":"owner","runs":[]}],"replayed":false})),Action::Reply(200,reply)]).await;
    let mut child = watch(&peer, "2000").await;
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    use futures_util::FutureExt;
    let outcome = tokio::time::timeout(
        Duration::from_secs(5),
        std::panic::AssertUnwindSafe(async {
            assert_eq!(record(&mut reader).await["type"], "stream.open");
            release.notify_one();
            let reset = record(&mut reader).await;
            assert_eq!(reset["type"], "stream.reset");
            assert_eq!(reset["snapshot"], snapshot);
            assert_eq!(reset["cursor"]["afterSequence"], 5);
            assert_eq!(reset["applied"]["afterSequence"], 0);
            assert_eq!(record(&mut reader).await["event"]["sequence"], 6);
            interrupt(&child);
            let end = record(&mut reader).await;
            assert_eq!(end["type"], "stream.end");
            assert_eq!(end["cursor"]["afterSequence"], 6);
        })
        .catch_unwind(),
    )
    .await;
    if !matches!(outcome, Ok(Ok(()))) {
        child.start_kill().ok();
    }
    let output = support::finish(child, b"").await;
    peer.settled().await;
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(_) => panic!("near-limit snapshot deadline after owned kill/reap and peer EOF"),
    }
    assert_eq!(output.status.code(), Some(130));
    assert!(output.stderr.is_empty());
    assert_eq!(peer.requests.lock().unwrap().len(), 14);
}
