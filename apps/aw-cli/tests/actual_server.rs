//! Explicit opt-in actual merged044 wire validation; never starts an installed daemon.
#[path = "support/process_guard.rs"]
mod guard;
#[allow(dead_code)]
#[path = "../../../crates/workbench-client/tests/support/harness.rs"]
mod harness;
mod support;
use futures_util::FutureExt;
use guard::{OwnedProcess, PrivateRoots};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    panic::AssertUnwindSafe,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use workbench_client::{
    application::admission::CallerProfile,
    domain::limits::Limits,
    infrastructure::{http::HttpConnection, locator::read_descriptor},
    ports::CallTransport,
};
use workbench_protocol::{CallRequest, IdempotencyKey, OperationId};
fn sha256(path: &Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let mut hasher = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let read = file.read(&mut bytes).unwrap();
        if read == 0 {
            break;
        }
        hasher.update(&bytes[..read]);
    }
    format!("{:x}", hasher.finalize())
}
async fn cli(roots: &PrivateRoots, operation: &str, input: Value, key: Option<&str>) -> Value {
    use std::os::unix::fs::PermissionsExt;
    let descriptor = roots.descriptor();
    // Separate invocations use separate private caller stores. Reusing a completed
    // attempt file requires --retry-state; this path tests an actual server replay.
    let state = roots.control.join(uuid::Uuid::new_v4().to_string());
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    let mut args = vec![
        "call",
        operation,
        "--input",
        "-",
        "--descriptor",
        descriptor.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
    ];
    if let Some(key) = key {
        args.extend(["--idempotency-key", key]);
    }
    let result = support::finish(
        spawn_cli(roots, &args),
        &serde_json::to_vec(&input).unwrap(),
    )
    .await;
    assert_eq!(
        result.status.code(),
        Some(0),
        "actual finite CLI {operation} failed (safe stderr code={})",
        serde_json::from_slice::<Value>(&result.stderr).unwrap_or(Value::Null)["error"]["code"]
    );
    assert!(result.stderr.is_empty());
    assert_eq!(result.stdout.iter().filter(|b| **b == b'\n').count(), 1);
    serde_json::from_slice(&result.stdout).unwrap()
}
#[tokio::test]
#[ignore = "run scripts/test-workbench-client-wire.sh with the exact merged044 provenance"]
async fn actual_merged044_private_server_client_and_aw_wire() {
    let binary =
        std::env::var_os("AW_047_SERVER_BINARY").expect("explicit actual server binary required");
    let provenance =
        std::env::var_os("AW_047_SERVER_PROVENANCE").expect("explicit provenance required");
    let provenance: Value = serde_json::from_slice(&fs::read(provenance).unwrap()).unwrap();
    assert_eq!(
        provenance["serverCommit"],
        "20fcd5fdcf633ae06792d51a9b963e3857909440"
    );
    assert_eq!(provenance["buildExit"], 0);
    assert_eq!(provenance["binarySha256"], sha256(Path::new(&binary)));
    let roots = PrivateRoots::new();
    let working = roots.root.join("worktree");
    fs::create_dir(&working).unwrap();
    // Test-only enforcement: no reads/writes under the user's home, no descendants.
    // This private fixture is not 045 containment or signed-package readiness proof.
    let user_home = std::env::var("HOME").unwrap();
    assert!(user_home.starts_with("/Users/") && !roots.root.starts_with(&user_home));
    let profile = format!(
        "(version 1)(allow default)(deny file-read* file-write* (subpath {}))(deny process-fork)",
        serde_json::to_string(&user_home).unwrap()
    );
    // Positive/negative controls share the exact profile and bounded process owner.
    let sentinel = roots.root.join("sandbox-sentinel");
    fs::write(&sentinel, b"private fixture allowed\n").unwrap();
    let home_file = Path::new(&user_home).join("project/agentic-workspace/AGENTS.md");
    assert!(home_file.is_file());
    let positive = probe(
        &roots,
        &profile,
        "positive",
        Path::new("/bin/cat"),
        &[sentinel.as_os_str()],
    )
    .await;
    assert!(positive.0.success());
    assert_eq!(positive.1, b"private fixture allowed\n");
    assert!(positive.2.is_empty());
    let negative = probe(
        &roots,
        &profile,
        "home-denied",
        Path::new("/bin/cat"),
        &[home_file.as_os_str()],
    )
    .await;
    assert!(!negative.0.success() && negative.1.is_empty());
    assert!(permission_denied(&negative.2));
    let fork = probe(
        &roots,
        &profile,
        "fork-denied",
        Path::new("/bin/sh"),
        &[
            std::ffi::OsStr::new("-c"),
            std::ffi::OsStr::new("/usr/bin/true & wait \"$!\""),
        ],
    )
    .await;
    assert!(!fork.0.success() && fork.1.is_empty());
    assert!(permission_denied(&fork.2));
    use std::os::unix::fs::PermissionsExt;
    fs::write(roots.control.join("seatbelt.sb"), &profile).unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_aw"),
        roots.control.join("aw-under-test"),
    )
    .unwrap();
    fs::set_permissions(
        roots.control.join("aw-under-test"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert_eq!(
        sha256(Path::new(env!("CARGO_BIN_EXE_aw"))),
        sha256(&roots.control.join("aw-under-test"))
    );
    let ledger = Arc::new(Mutex::new(Vec::new()));
    let process = OwnedProcess::spawn(
        Path::new("/usr/bin/sandbox-exec"),
        &[
            std::ffi::OsStr::new("-p"),
            std::ffi::OsStr::new(&profile),
            &binary,
            std::ffi::OsStr::new("serve"),
            std::ffi::OsStr::new("--data-dir"),
            roots.data.as_os_str(),
            std::ffi::OsStr::new("--idle-timeout"),
            std::ffi::OsStr::new("600"),
        ],
        &roots,
        ledger.clone(),
    );
    let pid = process.pid();
    let mut server = process
        .ready(&roots.descriptor(), Duration::from_secs(10))
        .await
        .expect("actual private server did not become ready; private logs retained by root owner");
    let mut watcher = None;
    let mut library_job = harness::OwnedTask::default();
    let outcome = AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(30),async {
        let descriptor = roots.descriptor();
        let descriptor_json: Value = serde_json::from_slice(&fs::read(&descriptor).unwrap()).unwrap();
        assert_eq!(descriptor_json["pid"],pid);
        let endpoint = Arc::new(read_descriptor(&descriptor,CallerProfile::Owner).unwrap());
        let mut client = HttpConnection::connect(endpoint.clone(),Limits::default()).await.unwrap();
        for (operation,name) in [(OperationId::SystemDescribe,"system.describe"),(OperationId::ProjectList,"project.list")] {
            let reply = client.call(&CallRequest::query(operation,json!({}))).await.unwrap();
            assert_eq!(cli(&roots,name,json!({}),None).await["data"],reply.output().unwrap().clone());
        }
        let create = json!({"name":"047 fixture","workingDirectory":working,"description":"private wire only"});
        let created = cli(&roots,"project.create",create.clone(),Some("047-project-create")).await;
        let mut replay = CallRequest::command(OperationId::ProjectCreate,create.clone());
        replay.idempotency_key = Some(IdempotencyKey::new("047-project-create").unwrap());
        let reply = client.call(&replay).await.unwrap();
        assert_eq!(reply.output(),Some(&created["data"]));
        assert_eq!(serde_json::to_value(&reply).unwrap()["replayed"],true);
        let replay_cli = cli(&roots,"project.create",create,Some("047-project-create")).await;
        assert_eq!(replay_cli["replayed"],true);
        assert_eq!(replay_cli["data"],created["data"]);
        assert_eq!(client.call(&CallRequest::query(OperationId::ProjectList,json!({}))).await.unwrap().output().unwrap().as_array().unwrap().len(),1);
        let id = created["data"]["id"].as_str().unwrap();
        let updated = cli(&roots,"project.update",json!({"id":id,"name":"047 updated","workingDirectory":working,"description":null}),Some("047-project-update")).await;
        assert_eq!(updated["data"]["name"],"047 updated");
        let deleted = client.call(&CallRequest::command(OperationId::ProjectDelete,json!({"id":id}))).await.unwrap();
        assert_eq!(deleted.output(),Some(&Value::Null));
        assert_eq!(cli(&roots,"project.list",json!({}),None).await["data"],json!([]));
        let status = cli(&roots,"server.status",json!({}),None).await;
        assert_eq!(status["data"]["instanceId"],endpoint.identity().instance());
        assert_eq!(status["data"]["serverEpoch"],endpoint.identity().epoch());
        let bench = client.call(&CallRequest::command(OperationId::BenchOpen,json!({"workingDirectory":working}))).await.unwrap();
        let bench_id = bench.output().unwrap()["benchId"].as_str().unwrap().to_owned();
        let bootstrap = client.call(&CallRequest::command(OperationId::OrchestrationBootstrap,json!({"benchId":bench_id,"worktreePath":working}))).await.unwrap();
        let initial = bootstrap.output().unwrap().clone();
        assert_empty(&initial);
        let stream = initial["eventStreamId"].as_str().unwrap().to_owned();
        assert!(stream.starts_with("orchestration:") && stream != format!("orchestration:{bench_id}") && stream != format!("orchestration:{}",initial["id"].as_str().unwrap()));
        let revision = initial["revision"].as_u64().unwrap();
        let epoch = endpoint.identity().epoch().to_owned();
        let cursor = workbench_protocol::workbench::StreamCursor { stream_id:stream.clone(),epoch:epoch.clone(),after_sequence:0 };
        let benches = client.call(&CallRequest::query(OperationId::BenchList,json!({}))).await.unwrap();
        assert_eq!(benches.output().unwrap().as_array().unwrap().len(),1);
        assert_eq!(benches.output().unwrap()[0]["runs"],json!([]));
        let before_status = client.call(&CallRequest::query(OperationId::ServerStatus,json!({}))).await.unwrap();
        assert_idle(before_status.output().unwrap());
        let records = Arc::new(Mutex::new(Vec::new()));
        let changed = Arc::new(tokio::sync::Notify::new());
        let stop = Arc::new(tokio::sync::Notify::new());
        let mut session = workbench_client::application::events::session::EventSession::new(cursor.clone(),Arc::new(ActualSource(endpoint.clone())),Limits::default()).unwrap();
        session.subscribe(0,Box::new(ActualConsumer {records:records.clone(),changed:changed.clone()})).unwrap();
        let progress = session.progress();
        let stopped = stop.clone();
        library_job.start(async move {session.run(stopped.notified()).await});
        watcher = Some(spawn_cli(&roots,&["events","watch","--input","-","--descriptor",descriptor.to_str().unwrap()]));
        use tokio::io::AsyncWriteExt;
        let mut stdin = watcher.as_mut().unwrap().stdin.take().unwrap();
        stdin.write_all(&serde_json::to_vec(&cursor).unwrap()).await.unwrap();
        drop(stdin);
        let mut reader = tokio::io::BufReader::new(watcher.as_mut().unwrap().stdout.take().unwrap());
        assert_eq!(jsonl(&mut reader).await["type"],"stream.open");
        let first = jsonl(&mut reader).await;
        let event: workbench_protocol::workbench::EventEnvelope = serde_json::from_value(first["event"].clone()).unwrap();
        assert_event(&event,&stream,&epoch,&initial["id"],event.sequence,revision,"bootstrap");
        let sequence = event.sequence;
        wait_records(&records,&changed,1).await;
        assert_eq!(records.lock().unwrap()[0],event);
        while progress.cursor().unwrap().after_sequence != sequence {tokio::task::yield_now().await;}
        // Check the still-empty private fixture immediately before its only recover.
        let before = client.call(&CallRequest::query(OperationId::OrchestrationGet,json!({"benchId":bench_id}))).await.unwrap();
        assert_empty(before.output().unwrap());
        assert_eq!(before.output().unwrap()["revision"],revision);
        let gated = support::finish(spawn_cli(&roots,&["call","orchestration.recover","--input","-","--descriptor",descriptor.to_str().unwrap(),"--state-dir",roots.control.to_str().unwrap()]),&serde_json::to_vec(&json!({"benchId":bench_id})).unwrap()).await;
        assert_eq!(support::assert_error(&gated,8)["error"]["code"],"prerequisiteUnavailable");
        let mut authority = harness::HarnessConnection::connect(endpoint.clone()).await;
        let order = std::sync::atomic::AtomicUsize::new(0);
        let ((reply,reply_order),(targets,event_order)) = tokio::join!(
            async {
                let reply = authority.post("/v1/calls",serde_json::to_value(CallRequest::command(OperationId::OrchestrationRecover,json!({"benchId":bench_id}))).unwrap(),true).await;
                (reply,order.fetch_add(1,std::sync::atomic::Ordering::SeqCst))
            },
            async {
                let mut targets = Vec::new();
                let mut observed = Vec::new();
                for (offset,reason) in [(1,"runtimeReconciled"),(2,"notificationRecovery")] {
                    let value = jsonl(&mut reader).await;
                    let event: workbench_protocol::workbench::EventEnvelope = serde_json::from_value(value["event"].clone()).unwrap();
                    assert_event(&event,&stream,&epoch,&initial["id"],sequence+offset,revision+1,reason);
                    observed.push(order.fetch_add(1,std::sync::atomic::Ordering::SeqCst));
                    targets.push(event);
                }
                (targets,observed)
            }
        );
        assert_eq!(reply["kind"],"complete");
        assert_empty(&reply["output"]);
        assert_eq!(reply["output"]["revision"],revision+1);
        wait_records(&records,&changed,3).await;
        while progress.cursor().unwrap().after_sequence != sequence+2 {tokio::task::yield_now().await;}
        let expected = vec![event.clone(),targets[0].clone(),targets[1].clone()];
        assert_eq!(*records.lock().unwrap(),expected);
        let final_snapshot = client.call(&CallRequest::query(OperationId::OrchestrationGet,json!({"benchId":bench_id}))).await.unwrap();
        assert_eq!(final_snapshot.output(),Some(&reply["output"]));
        assert_empty(final_snapshot.output().unwrap());
        assert_eq!(final_snapshot.output().unwrap()["revision"],revision+1);
        authority.close().await;
        stop.notify_one();
        let finished = library_job.wait().await;
        assert_eq!(finished.cursor.after_sequence,sequence+2);
        assert!(finished.cleanup_error.is_none());
        assert_eq!(unsafe {libc::kill(watcher.as_ref().unwrap().id().unwrap() as i32,libc::SIGINT)},0);
        let end = jsonl(&mut reader).await;
        assert_eq!(end["type"],"stream.end");
        assert_eq!(end["cursor"]["streamId"],stream);
        assert_eq!(end["cursor"]["epoch"],epoch);
        assert_eq!(end["cursor"]["afterSequence"],sequence+2);
        let output = support::finish(watcher.take().unwrap(),b"").await;
        assert_eq!(output.status.code(),Some(130));
        assert!(output.stderr.is_empty());
        use tokio::io::AsyncBufReadExt;
        assert_eq!(reader.read_line(&mut String::new()).await.unwrap(),0);
        let status = client.call(&CallRequest::query(OperationId::ServerStatus,json!({}))).await.unwrap();
        assert_idle(status.output().unwrap());
        let benches = client.call(&CallRequest::query(OperationId::BenchList,json!({}))).await.unwrap();
        assert_eq!(benches.output().unwrap()[0]["runs"],json!([]));
        // Host-side observer: macOS denies KERN_PROC_ALL inside the seatbelt even
        // with allow-default. Keep the server/CLI profile unchanged and own/reap ps.
        let observer_ledger = Arc::new(Mutex::new(Vec::new()));
        let mut observer = OwnedProcess::spawn_named(Path::new("/bin/ps"),&[std::ffi::OsStr::new("-axo"),std::ffi::OsStr::new("pid=,ppid=")],&roots,observer_ledger.clone(),"server-children");
        assert!(observer.wait_exit(Duration::from_secs(1)).await.unwrap().success());
        drop(observer);
        assert!(observer_ledger.lock().unwrap().iter().all(|record|record.reaped && record.error.is_none()));
        let child_output = fs::read(roots.control.join("server-children-stdout.log")).unwrap();
        assert!(child_output.len()<=65536);
        let children = String::from_utf8(child_output).unwrap().lines().filter(|line|line.split_whitespace().nth(1).and_then(|s|s.parse::<u32>().ok())==Some(pid)).count();
        assert_eq!(children,0);
        println!("047 actual events bootstrap={sequence}/{revision} runtime={}/{} notification={}/{} replyOrdinal={reply_order} eventOrdinals={event_order:?} ACK={} Main1 run0 tasks0 commands0 notifications0 dispatch0 businessWork0 observerAccepted1 children0",sequence+1,revision+1,sequence+2,revision+1,sequence+2);
        if let Some(path) = std::env::var_os("AW_047_WIRE_EVIDENCE") {
            fs::write(path,serde_json::to_vec_pretty(&json!({"serverCommit":provenance["serverCommit"],"binarySha256":provenance["binarySha256"],"cliSha256":sha256(&roots.control.join("aw-under-test")),"events":expected,"replyOrdinal":reply_order,"eventOrdinals":event_order,"finalRevision":revision+1,"appliedSequence":sequence+2,"children":children,"privateSentinelRead":true,"homePermissionDenied":true,"forkProbePermissionDenied":true,"productionRecoverGate":"prerequisiteUnavailable","fixtureOnly":true})).unwrap()).unwrap();
        }
        client.close().await.unwrap();
    })).catch_unwind().await;
    let library_cleanup = library_job.cancel_join().await;
    if let Some(mut child) = watcher.take() {
        child.start_kill().unwrap();
        tokio::time::timeout(Duration::from_secs(1), child.wait())
            .await
            .expect("actual watch cleanup deadline")
            .unwrap();
    }
    let cleanup = server.terminate().await;
    drop(server);
    assert!(cleanup.is_ok(), "{cleanup:?}");
    assert!(library_cleanup.is_ok(), "{library_cleanup:?}");
    let records = ledger.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert!(
        records[0].killed
            && records[0].reaped
            && records[0].status.is_some()
            && records[0].error.is_none()
    );
    assert_eq!(records[0].pid, pid);
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    println!("047 actual server commit={} binarySHA={} pid={} reaped=true userHomeDenied=true forkProbeDenied=true positiveProbe=true (fixture only)",provenance["serverCommit"],provenance["binarySha256"],pid);
    match outcome {
        Ok(result) => result.expect("actual wire deadline after server kill/reap"),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn permission_denied(stderr: &[u8]) -> bool {
    let text = String::from_utf8_lossy(stderr);
    text.contains("Operation not permitted") || text.contains("Permission denied")
}
async fn probe(
    roots: &Arc<PrivateRoots>,
    profile: &str,
    name: &str,
    program: &Path,
    args: &[&std::ffi::OsStr],
) -> (std::process::ExitStatus, Vec<u8>, Vec<u8>) {
    let ledger = Arc::new(Mutex::new(Vec::new()));
    let mut all = vec![
        std::ffi::OsStr::new("-p"),
        std::ffi::OsStr::new(profile),
        program.as_os_str(),
    ];
    all.extend_from_slice(args);
    let mut process = OwnedProcess::spawn_named(
        Path::new("/usr/bin/sandbox-exec"),
        &all,
        roots,
        ledger.clone(),
        name,
    );
    let status = process
        .wait_exit(Duration::from_secs(1))
        .await
        .expect("probe deadline/cleanup failure");
    drop(process);
    let records = ledger.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].reaped && !records[0].killed && records[0].error.is_none());
    let stdout = fs::read(roots.control.join(format!("{name}-stdout.log"))).unwrap();
    let stderr = fs::read(roots.control.join(format!("{name}-stderr.log"))).unwrap();
    assert!(stdout.len() <= 65536 && stderr.len() <= 65536);
    (status, stdout, stderr)
}
fn spawn_cli(roots: &PrivateRoots, args: &[&str]) -> tokio::process::Child {
    tokio::process::Command::new("/usr/bin/sandbox-exec")
        .arg("-f")
        .arg(roots.control.join("seatbelt.sb"))
        .arg(roots.control.join("aw-under-test"))
        .args(args)
        .current_dir(&roots.root)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

fn assert_empty(value: &Value) {
    let session: workbench_protocol::operations::orchestration_dto::OrchestrationSessionDto =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(session.nodes.len(), 1);
    assert_eq!(
        session.nodes[0].kind,
        workbench_protocol::operations::orchestration_dto::AgentNodeKindDto::Main
    );
    assert!(
        session.nodes[0].current_run_id.is_none() && session.nodes[0].assigned_task_id.is_none()
    );
    assert!(session.active_coordinator_generation_id.is_none());
    assert!(
        session.generations.is_empty()
            && session.tasks.is_empty()
            && session.reports.is_empty()
            && session.commands.is_empty()
            && session.coordinator_notifications.is_empty()
            && session.dispatches.is_empty()
    );
}
fn assert_idle(value: &Value) {
    let status: workbench_protocol::operations::server::ServerStatusOutput =
        serde_json::from_value(value.clone()).unwrap();
    assert_eq!(status.idle_runs, 0);
    assert_eq!(status.active_work.busy_runs, 0);
    // merged044 server_status explicitly includes this observer HTTP query.
    // Exactly one means no other accepted call; it is not an agent/launch reservation.
    assert_eq!(status.active_work.accepted_calls, 1);
    assert_eq!(status.active_work.reservations, 0);
    assert_eq!(status.active_work.pending_operations, Some(0));
    assert_eq!(status.active_work.orchestration_tasks, Some(0));
    assert_eq!(status.active_work.queued_tasks, Some(0));
    assert_eq!(status.active_work.pending_notifications, Some(0));
}
fn assert_event(
    event: &workbench_protocol::workbench::EventEnvelope,
    stream: &str,
    epoch: &str,
    workspace: &Value,
    sequence: u64,
    revision: u64,
    reason: &str,
) {
    assert_eq!(event.stream_id, stream);
    assert_eq!(event.epoch, epoch);
    assert_eq!(event.schema, "orchestration.workspaceUpdated.v1");
    assert_eq!(event.sequence, sequence);
    assert_eq!(event.body["revision"], revision);
    assert_eq!(&event.body["workspaceId"], workspace);
    assert_eq!(event.body["reason"], reason);
}
async fn jsonl(reader: &mut tokio::io::BufReader<tokio::process::ChildStdout>) -> Value {
    use tokio::io::AsyncBufReadExt;
    let mut line = String::new();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), reader.read_line(&mut line))
            .await
            .expect("actual JSONL deadline")
            .unwrap()
            > 0
    );
    assert!(line.ends_with('\n'));
    serde_json::from_str(&line).unwrap()
}
struct ActualSource(Arc<workbench_client::infrastructure::locator::LocatedEndpoint>);
struct NoSnapshot;
#[async_trait::async_trait]
impl workbench_client::ports::SnapshotPort for NoSnapshot {
    async fn snapshot(
        &mut self,
        _: &workbench_protocol::workbench::StreamCursor,
    ) -> Result<workbench_client::ports::Snapshot, workbench_client::ports::ClientError> {
        Err(workbench_client::ports::ClientError::Protocol)
    }
}
#[async_trait::async_trait]
impl workbench_client::ports::EventSource for ActualSource {
    async fn connect(
        &self,
        cursor: &workbench_protocol::workbench::StreamCursor,
    ) -> Result<Box<dyn workbench_client::ports::EventSocket>, workbench_client::ports::ClientError>
    {
        Ok(Box::new(
            workbench_client::infrastructure::websocket::WebSocketConnection::connect(
                self.0.clone(),
                Limits::default(),
                vec![cursor.clone()],
            )
            .await?,
        ))
    }
    fn snapshot_port(&self) -> Box<dyn workbench_client::ports::SnapshotPort> {
        Box::new(NoSnapshot)
    }
}
struct ActualConsumer {
    records: Arc<Mutex<Vec<workbench_protocol::workbench::EventEnvelope>>>,
    changed: Arc<tokio::sync::Notify>,
}
#[async_trait::async_trait]
impl workbench_client::ports::EventConsumer for ActualConsumer {
    async fn consume(
        &mut self,
        event: &workbench_protocol::workbench::EventEnvelope,
    ) -> Result<(), workbench_client::ports::ClientError> {
        self.records.lock().unwrap().push(event.clone());
        self.changed.notify_one();
        Ok(())
    }
    async fn reset(
        &mut self,
        _: &workbench_client::ports::Snapshot,
    ) -> Result<(), workbench_client::ports::ClientError> {
        Err(workbench_client::ports::ClientError::Protocol)
    }
}
async fn wait_records(
    records: &Arc<Mutex<Vec<workbench_protocol::workbench::EventEnvelope>>>,
    changed: &tokio::sync::Notify,
    count: usize,
) {
    loop {
        let notified = changed.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if records.lock().unwrap().len() >= count {
            break;
        }
        notified.await;
    }
}
