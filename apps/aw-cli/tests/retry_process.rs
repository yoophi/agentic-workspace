mod support;
use serde_json::json;
use support::{
    assert_error,
    peer::{Action, Peer},
    run, state_directory,
};
#[tokio::test]
async fn actual_invocations_replay_unknown_key_and_cache_complete_without_new_http() {
    let peer = Peer::spawn_multi(vec![
        Action::Close,
        Action::Reply(
            200,
            json!({"kind":"complete","output":{"effectCount":1},"revision":4,"replayed":true}),
        ),
    ])
    .await;
    let state = state_directory();
    let root = state.path().canonicalize().unwrap();
    let output = run(
        &[
            "call",
            "project.create",
            "--input",
            "-",
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
            "--state-dir",
            root.to_str().unwrap(),
        ],
        br#"{"name":"private-sentinel","workingDirectory":"/private/tmp"}"#,
    )
    .await;
    let error = assert_error(&output, 8);
    assert_eq!(error["error"]["outcome"], "unknown");
    let retry = error["attempt"]["retryState"].as_str().unwrap();
    let output = run(
        &[
            "call",
            "--retry-state",
            retry,
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
        ],
        b"",
    )
    .await;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["effectCount"], 1);
    assert_eq!(value["replayed"], true);
    assert_eq!(value["requestId"], error["requestId"]);
    assert_eq!(peer.effects.lock().unwrap().len(), 1);
    let cached = run(&["call", "--retry-state", retry], b"").await;
    assert!(cached.status.success());
    assert_eq!(cached.stdout, output.stdout);
    assert_eq!(peer.requests.lock().unwrap().len(), 6);
}
#[tokio::test]
async fn epoch_change_and_replaced_input_key_flags_send_zero_commands() {
    let peer = Peer::spawn_multi(vec![Action::Close]).await;
    let state = state_directory();
    let root = state.path().canonicalize().unwrap();
    let output = run(
        &[
            "call",
            "project.create",
            "--input",
            "-",
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
            "--state-dir",
            root.to_str().unwrap(),
        ],
        br#"{"name":"private-sentinel","workingDirectory":"/private/tmp"}"#,
    )
    .await;
    let error = assert_error(&output, 8);
    let retry = error["attempt"]["retryState"].as_str().unwrap();
    assert_error(
        &run(&["call", "--retry-state", retry, "--input", "-"], b"{}").await,
        2,
    );
    assert_error(
        &run(
            &[
                "call",
                "--retry-state",
                retry,
                "--idempotency-key",
                "replaced",
            ],
            b"",
        )
        .await,
        2,
    );
    let mut descriptor: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&peer.descriptor).unwrap()).unwrap();
    descriptor["serverEpoch"] = json!("changed");
    std::fs::write(&peer.descriptor, descriptor.to_string()).unwrap();
    let error = assert_error(
        &run(
            &[
                "call",
                "--retry-state",
                retry,
                "--descriptor",
                peer.descriptor.to_str().unwrap(),
            ],
            b"",
        )
        .await,
        9,
    );
    assert_eq!(error["error"]["outcome"], "unknown");
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
}
#[tokio::test]
async fn pre_send_persistence_failure_sends_zero_mutations() {
    let mut peer = Peer::spawn(vec![], true).await;
    let state = state_directory();
    let root = state.path().canonicalize().unwrap().join("missing");
    let output = run(
        &[
            "call",
            "project.create",
            "--input",
            "-",
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
            "--state-dir",
            root.to_str().unwrap(),
        ],
        br#"{"name":"private-sentinel","workingDirectory":"/private/tmp"}"#,
    )
    .await;
    let error = assert_error(&output, 5);
    assert_eq!(error["error"]["outcome"], "notApplied");
    peer.settled().await;
    assert_eq!(peer.requests.lock().unwrap().len(), 2);
    assert_eq!(peer.effects.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn unknown_preflight_failures_preserve_previous_outcome_request_and_receipt() {
    let peer = Peer::spawn_multi(vec![Action::Close]).await;
    let state = state_directory();
    let root = state.path().canonicalize().unwrap();
    let descriptor = peer.descriptor.to_str().unwrap();
    let output = run(
        &[
            "call",
            "project.create",
            "--input",
            "-",
            "--descriptor",
            descriptor,
            "--state-dir",
            root.to_str().unwrap(),
        ],
        br#"{"name":"private-sentinel","workingDirectory":"/private/tmp"}"#,
    )
    .await;
    let original = assert_error(&output, 8);
    let retry = original["attempt"]["retryState"].as_str().unwrap();
    let cases = [
        vec!["call", "--retry-state", retry],
        vec![
            "call",
            "--retry-state",
            retry,
            "--descriptor",
            "/private/missing-private-sentinel/server.json",
        ],
    ];
    for args in cases {
        let error = assert_error(&run(&args, b"").await, 8);
        assert_eq!(error["error"]["outcome"], "unknown");
        assert_eq!(error["attempt"], original["attempt"]);
        assert_eq!(error["requestId"], original["requestId"]);
    }
    let original_descriptor = std::fs::read(&peer.descriptor).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&original_descriptor).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    value["baseUrl"] = json!(format!("http://{address}"));
    std::fs::write(&peer.descriptor, value.to_string()).unwrap();
    let error = assert_error(
        &run(
            &["call", "--retry-state", retry, "--descriptor", descriptor],
            b"",
        )
        .await,
        8,
    );
    assert_eq!(error["error"]["outcome"], "unknown");
    assert_eq!(error["attempt"], original["attempt"]);
    let bad_identity = Peer::spawn(vec![], false).await;
    value["baseUrl"] = json!(format!("http://{}", bad_identity.endpoint.address()));
    std::fs::write(&peer.descriptor, value.to_string()).unwrap();
    let error = assert_error(
        &run(
            &["call", "--retry-state", retry, "--descriptor", descriptor],
            b"",
        )
        .await,
        3,
    );
    assert_eq!(error["error"]["outcome"], "unknown");
    assert_eq!(error["attempt"], original["attempt"]);
    assert_eq!(error["requestId"], original["requestId"]);
    assert_eq!(bad_identity.requests.lock().unwrap().len(), 1);
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn killed_cli_reopens_unknown_and_active_invocation_cannot_claim_its_lease() {
    use tokio::io::AsyncWriteExt;
    use workbench_client::{
        domain::limits::Limits, infrastructure::retry_store::PrivateRetryStore, ports::RetryStore,
    };
    let peer = Peer::spawn_multi(vec![
        Action::Pause,
        Action::Reply(
            200,
            json!({"kind":"complete","output":{"effectCount":1},"revision":4,"replayed":true}),
        ),
    ])
    .await;
    let state = state_directory();
    let root = state.path().canonicalize().unwrap();
    let mut child = support::spawn(&[
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
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let notified = peer.received.notified();
            if peer.requests.lock().unwrap().len() == 3 {
                break;
            }
            notified.await;
        }
    })
    .await
    .expect("first CLI did not submit");
    let paths: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    assert_eq!(paths.len(), 1);
    let retry = paths[0].to_str().unwrap();
    let loser = run(
        &[
            "call",
            "--retry-state",
            retry,
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
        ],
        b"",
    )
    .await;
    assert_error(&loser, 5);
    assert_eq!(peer.requests.lock().unwrap().len(), 3);
    child.start_kill().unwrap();
    let killed = support::finish(child, b"").await;
    assert!(!killed.status.success());
    assert!(killed.stdout.is_empty() && killed.stderr.is_empty());
    let original = {
        let store = PrivateRetryStore::open(&paths[0], Limits::default()).unwrap();
        let record = store.load().unwrap();
        assert_eq!(record.outcome, workbench_protocol::Outcome::Unknown);
        record.request
    };
    let retried = run(
        &[
            "call",
            "--retry-state",
            retry,
            "--descriptor",
            peer.descriptor.to_str().unwrap(),
        ],
        b"",
    )
    .await;
    assert!(retried.status.success());
    let value: serde_json::Value = serde_json::from_slice(&retried.stdout).unwrap();
    assert_eq!(value["requestId"], json!(original.request_id));
    assert_eq!(value["replayed"], true);
    let requests = peer.requests.lock().unwrap();
    assert_eq!(requests.len(), 6);
    assert_eq!(requests[2].2, requests[5].2);
    assert_eq!(peer.effects.lock().unwrap().len(), 1);
}
