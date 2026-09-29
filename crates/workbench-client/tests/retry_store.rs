use serde_json::json;
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
};
use workbench_client::{
    domain::{attempt::EndpointIdentity, limits::Limits},
    infrastructure::retry_store::PrivateRetryStore,
    ports::{RetryRecord, RetryStore},
};
use workbench_protocol::{CallReply, CallRequest, OperationId, Outcome};
fn root() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().canonicalize().unwrap();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
    (dir, parent.join("attempt.json"))
}
fn record() -> RetryRecord {
    RetryRecord {
        request: CallRequest::command(
            OperationId::ProjectCreate,
            json!({"name":"private-sentinel","workingDirectory":"/private/tmp"}),
        ),
        endpoint: EndpointIdentity::new("i", "e").unwrap(),
        generation: 1,
        outcome: Outcome::Unknown,
        result: None,
    }
}
#[test]
fn publish_reopen_immutable_input_and_unknown_survive_before_output_crash() {
    let (_dir, path) = root();
    let original = record();
    {
        let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
        store.publish(&original).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
    let restored = store.load().unwrap();
    assert_eq!(restored.request, original.request);
    assert_eq!(restored.endpoint, original.endpoint);
    assert_eq!(restored.outcome, Outcome::Unknown);
    assert!(!format!("{restored:?}").contains("private-sentinel"));
}
#[test]
fn active_use_and_stale_completion_cannot_overwrite_new_generation() {
    let (_dir, path) = root();
    let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
    store.publish(&record()).unwrap();
    assert!(PrivateRetryStore::open(&path, Limits::default()).is_err());
    let next = store
        .begin_retry(1, &EndpointIdentity::new("i", "e").unwrap())
        .unwrap();
    assert_eq!(next.generation, 2);
    assert!(store
        .complete(1, Ok(CallReply::complete(json!({"old":true}), None)))
        .is_err());
    store
        .complete(
            2,
            Ok(CallReply::complete(json!({"result":"preserved"}), Some(4))),
        )
        .unwrap();
    let final_state = store.load().unwrap();
    assert_eq!(final_state.outcome, Outcome::Applied);
    assert!(final_state.result.is_some());
}
#[test]
fn epoch_mismatch_and_input_replacement_preserve_original_state() {
    let (_dir, path) = root();
    let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
    let original = record();
    store.publish(&original).unwrap();
    assert!(store
        .begin_retry(1, &EndpointIdentity::new("i", "changed").unwrap())
        .is_err());
    let mut replacement = record();
    replacement.request.input = json!({"replaced":true});
    assert!(store.publish(&replacement).is_err());
    assert_eq!(store.load().unwrap().request, original.request);
}
#[test]
fn symlink_permissions_missing_parent_and_oversize_are_private_failures() {
    let (_dir, path) = root();
    let link = path.with_file_name("link");
    symlink(&path, &link).unwrap();
    assert!(PrivateRetryStore::open(&link, Limits::default()).is_err());
    {
        let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
        store.publish(&record()).unwrap();
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(PrivateRetryStore::open(&path, Limits::default())
        .unwrap()
        .load()
        .is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&path, vec![b' '; 8 * 1024 * 1024 + 1]).unwrap();
    assert!(PrivateRetryStore::open(&path, Limits::default())
        .unwrap()
        .load()
        .is_err());
    assert!(PrivateRetryStore::open(
        &path.parent().unwrap().join("missing/attempt.json"),
        Limits::default()
    )
    .is_err());
}

#[test]
fn concurrent_threads_claim_exactly_one_generation() {
    let (_dir, path) = root();
    let store = std::sync::Arc::new(PrivateRetryStore::open(&path, Limits::default()).unwrap());
    store.publish(&record()).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let store = store.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            store.begin_retry(1, &EndpointIdentity::new("i", "e").unwrap())
        }));
    }
    barrier.wait();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .filter_map(|r| r.as_ref().err())
        .all(|e| matches!(e, workbench_client::ports::ClientError::StaleGeneration)));
    assert_eq!(store.load().unwrap().generation, 2);
}
#[test]
fn process_lease_blocks_another_process_and_reopens_after_owner_exit() {
    let (_dir, path) = root();
    let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
    store.publish(&record()).unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lease_child", "--nocapture"])
        .env("AW_RETRY_TEST_PATH", &path)
        .env("AW_RETRY_TEST_BLOCKED", "1")
        .output()
        .unwrap();
    assert!(child.status.success());
    drop(store);
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lease_child", "--nocapture"])
        .env("AW_RETRY_TEST_PATH", &path)
        .env("AW_RETRY_TEST_BLOCKED", "0")
        .output()
        .unwrap();
    assert!(child.status.success());
}
#[test]
fn lease_child() {
    let Some(path) = std::env::var_os("AW_RETRY_TEST_PATH") else {
        return;
    };
    if std::env::var("AW_RETRY_TEST_CRASH").ok().as_deref() == Some("1") {
        let store =
            PrivateRetryStore::open(std::path::Path::new(&path), Limits::default()).unwrap();
        store.publish(&record()).unwrap();
        std::process::exit(17);
    }
    let store = PrivateRetryStore::open(std::path::Path::new(&path), Limits::default());
    if std::env::var("AW_RETRY_TEST_BLOCKED").unwrap() == "1" {
        assert!(store.is_err());
    } else {
        assert_eq!(store.unwrap().load().unwrap().outcome, Outcome::Unknown);
    }
}

#[test]
fn concurrent_publish_and_complete_have_one_winner_without_overwrite() {
    let (_dir, path) = root();
    let store = std::sync::Arc::new(PrivateRetryStore::open(&path, Limits::default()).unwrap());
    let original = std::sync::Arc::new(record());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut threads = Vec::new();
    for _ in 0..2 {
        let store = store.clone();
        let record = original.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            store.publish(&record)
        }));
    }
    barrier.wait();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .filter_map(|r| r.as_ref().err())
        .all(|e| matches!(e, workbench_client::ports::ClientError::StaleGeneration)));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut threads = Vec::new();
    for winner in 0..2 {
        let store = store.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            (
                winner,
                store.complete(
                    1,
                    Ok(CallReply::complete(json!({"winner":winner}), Some(4))),
                ),
            )
        }));
    }
    barrier.wait();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    let winners: Vec<_> = results.iter().filter(|(_, r)| r.is_ok()).collect();
    assert_eq!(winners.len(), 1);
    assert!(results
        .iter()
        .filter_map(|(_, r)| r.as_ref().err())
        .all(|e| matches!(e, workbench_client::ports::ClientError::StaleGeneration)));
    let final_result = store.load().unwrap().result.unwrap().unwrap();
    assert_eq!(final_result.output().unwrap()["winner"], winners[0].0);
}

#[test]
fn abrupt_process_exit_after_durable_publish_reopens_unknown_exact_payload() {
    let (_dir, path) = root();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "lease_child", "--nocapture"])
        .env("AW_RETRY_TEST_PATH", &path)
        .env("AW_RETRY_TEST_CRASH", "1")
        .output()
        .unwrap();
    assert_eq!(child.status.code(), Some(17));
    let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
    let restored = store.load().unwrap();
    assert_eq!(restored.outcome, Outcome::Unknown);
    assert_eq!(
        restored.request.input,
        json!({"name":"private-sentinel","workingDirectory":"/private/tmp"})
    );
}

#[test]
fn reopened_retryable_unknown_and_not_applied_faults_retry_exact_identity() {
    use workbench_protocol::{FaultCode, WorkbenchFault};
    for outcome in [Outcome::Unknown, Outcome::NotApplied] {
        let (_dir, path) = root();
        let original = record();
        {
            let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
            store.publish(&original).unwrap();
            store
                .complete(
                    1,
                    Err(WorkbenchFault::new(
                        FaultCode::Unavailable,
                        original.request.request_id.clone(),
                        "private-sentinel",
                    )
                    .with_outcome(outcome)
                    .with_retryable(true)),
                )
                .unwrap();
        }
        let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
        let retry = store.begin_retry(1, &original.endpoint).unwrap();
        assert_eq!(retry.generation, 2);
        assert_eq!(retry.request, original.request);
        assert_eq!(retry.endpoint, original.endpoint);
        assert_eq!(retry.outcome, Outcome::Unknown);
        assert!(retry.result.is_none());
    }
}
#[test]
fn reopened_applied_nonretryable_accepted_and_complete_results_are_terminal_cache() {
    use workbench_protocol::{FaultCode, WorkbenchFault};
    let original = record();
    for result in [
        Err(WorkbenchFault::new(
            FaultCode::Unavailable,
            original.request.request_id.clone(),
            "private-sentinel",
        )
        .with_outcome(Outcome::Applied)
        .with_retryable(true)),
        Err(WorkbenchFault::new(
            FaultCode::Internal,
            original.request.request_id.clone(),
            "private-sentinel",
        )
        .with_outcome(Outcome::Unknown)
        .with_retryable(false)),
        Ok(CallReply::Accepted {
            execution_id: "execution".into(),
            revision: Some(4),
        }),
        Ok(CallReply::complete(json!({"terminal":true}), Some(4))),
    ] {
        let (_dir, path) = root();
        {
            let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
            store.publish(&original).unwrap();
            store.complete(1, result.clone()).unwrap();
        }
        let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
        assert!(store.begin_retry(1, &original.endpoint).is_err());
        assert_eq!(store.load().unwrap().result, Some(result));
    }
}

#[test]
fn reopened_record_still_enforces_input_budget() {
    let (_dir, path) = root();
    {
        let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
        store.publish(&record()).unwrap();
    }
    let limits = Limits::new(workbench_client::domain::limits::LimitConfig {
        input_bytes: 1,
        ..Default::default()
    })
    .unwrap();
    let store = PrivateRetryStore::open(&path, limits).unwrap();
    assert!(store.load().is_err());
}

#[test]
fn near_body_limit_applied_reply_with_nonempty_input_reopens_as_terminal_cache() {
    let (_dir, path) = root();
    let limits = Limits::default();
    let original = record();
    let empty = CallReply::complete(json!({"description":""}), Some(17));
    let overhead = serde_json::to_vec(&empty).unwrap().len();
    let reply = CallReply::complete(
        json!({"description":"x".repeat(limits.maximum(workbench_client::domain::limits::Resource::Body)-overhead)}),
        Some(17),
    );
    assert_eq!(
        serde_json::to_vec(&reply).unwrap().len(),
        limits.maximum(workbench_client::domain::limits::Resource::Body)
    );
    {
        let store = PrivateRetryStore::open(&path, limits.clone()).unwrap();
        store.publish(&original).unwrap();
        store.complete(1, Ok(reply.clone())).unwrap();
    }
    assert!(
        fs::metadata(&path).unwrap().len()
            > limits.maximum(workbench_client::domain::limits::Resource::Body) as u64
    );
    let store = PrivateRetryStore::open(&path, limits).unwrap();
    let cached = store.load().unwrap();
    assert_eq!(cached.request, original.request);
    assert_eq!(cached.generation, 1);
    assert_eq!(cached.outcome, Outcome::Applied);
    assert_eq!(cached.result, Some(Ok(reply)));
    assert!(matches!(
        store.begin_retry(1, &original.endpoint),
        Err(workbench_client::ports::ClientError::StaleGeneration)
    ));
}

#[test]
fn normalized_exponent_reply_and_fault_fit_reserved_state_and_reopen() {
    use workbench_client::domain::limits::{LimitConfig, Resource};
    use workbench_protocol::WorkbenchFault;
    let body_bytes = 64 * 1024;
    let numbers = std::iter::repeat_n("1e10", (body_bytes - 512) / 5)
        .collect::<Vec<_>>()
        .join(",");
    let reply_wire =
        format!(r#"{{"kind":"complete","output":[{numbers}],"revision":17,"replayed":false}}"#);
    let original = record();
    let fault_wire = format!(
        r#"{{"code":"internal","message":"failure","retryable":false,"outcome":"applied","requestId":"{}","details":[{numbers}]}}"#,
        original.request.request_id.as_str()
    );
    let results = [
        Ok(serde_json::from_str::<CallReply>(&reply_wire).unwrap()),
        Err(serde_json::from_str::<WorkbenchFault>(&fault_wire).unwrap()),
    ];
    for (wire, result) in [reply_wire, fault_wire].into_iter().zip(results) {
        assert!(wire.len() <= body_bytes);
        assert!(serde_json::to_vec(&result).unwrap().len() > body_bytes + 64 * 1024);
        let (_dir, path) = root();
        let limits = Limits::new(LimitConfig {
            body_bytes,
            ..Default::default()
        })
        .unwrap();
        {
            let store = PrivateRetryStore::open(&path, limits.clone()).unwrap();
            store.publish(&original).unwrap();
            store.complete(1, result.clone()).unwrap();
        }
        assert!(fs::metadata(&path).unwrap().len() > (body_bytes + 64 * 1024) as u64);
        assert!(fs::metadata(&path).unwrap().len() < limits.maximum(Resource::RetryState) as u64);
        let store = PrivateRetryStore::open(&path, limits).unwrap();
        assert_eq!(store.load().unwrap().result, Some(result));
        assert_eq!(store.load().unwrap().outcome, Outcome::Applied);
        assert!(matches!(
            store.begin_retry(1, &original.endpoint),
            Err(workbench_client::ports::ClientError::StaleGeneration)
        ));
    }
}
