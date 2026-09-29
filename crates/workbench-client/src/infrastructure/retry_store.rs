//! Caller control state only: private directory FD, process lease and fsync CAS.
use crate::{
    domain::{
        attempt::EndpointIdentity,
        limits::{Limits, Resource},
    },
    infrastructure::locator::{open_private_parent, read_bounded, validate_private_file},
    ports::{ClientError, RetryRecord, RetryStore},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::CString,
    fs::File,
    io::Write,
    os::unix::io::{AsRawFd, FromRawFd},
    path::Path,
};
use workbench_protocol::{
    CallReply, CallRequest, Outcome, WorkbenchFault, CONTRACT_REVISION, PROTOCOL_VERSION,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct State {
    version: u16,
    protocol: u16,
    contract: u32,
    request: CallRequest,
    instance: String,
    epoch: String,
    generation: u64,
    outcome: Outcome,
    input_digest: String,
    result: Option<Result<CallReply, WorkbenchFault>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum WriteStage {
    FileSync,
    Rename,
    DirectorySync,
}
pub struct PrivateRetryStore {
    directory: File,
    name: CString,
    lock: File,
    lock_name: CString,
    limits: Limits,
    transaction: std::sync::Mutex<()>,
}
impl Drop for PrivateRetryStore {
    fn drop(&mut self) {
        // End this instance's lease explicitly; close alone can leave a shared open
        // file description alive temporarily across a concurrent process launch.
        unsafe {
            libc::flock(self.lock.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
impl PrivateRetryStore {
    /// Lifetime owns the exclusive cross-process active-use lease; no token in argv.
    pub fn open(path: &Path, limits: Limits) -> Result<Self, ClientError> {
        let (directory, name) = open_private_parent(path).map_err(|_| ClientError::PrivateState)?;
        // A symlink state is rejected even before load/publish; absent state is allowed.
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        let found = unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if found == 0 && unsafe { stat.assume_init() }.st_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(ClientError::PrivateState);
        }
        if found < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound {
            return Err(ClientError::PrivateState);
        }
        let lock_name = CString::new(format!(".{}.lock", name.to_string_lossy()))
            .map_err(|_| ClientError::PrivateState)?;
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                lock_name.as_ptr(),
                libc::O_RDWR
                    | libc::O_CREAT
                    | libc::O_NOFOLLOW
                    | libc::O_CLOEXEC
                    | libc::O_NONBLOCK,
                0o600,
            )
        };
        if fd < 0 {
            return Err(ClientError::PrivateState);
        }
        let lock = unsafe { File::from_raw_fd(fd) };
        validate_private_file(&lock)?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(ClientError::PrivateState);
        }
        let store = Self {
            directory,
            name,
            lock,
            lock_name,
            limits,
            transaction: std::sync::Mutex::new(()),
        };
        store.check_lease()?;
        Ok(store)
    }
    fn check_lease(&self) -> Result<(), ClientError> {
        use std::os::unix::fs::MetadataExt;
        validate_private_file(&self.lock)?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                self.lock_name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return Err(ClientError::PrivateState);
        }
        let stat = unsafe { stat.assume_init() };
        let meta = self
            .lock
            .metadata()
            .map_err(|_| ClientError::PrivateState)?;
        if stat.st_ino != meta.ino() || stat.st_dev as u64 != meta.dev() {
            return Err(ClientError::PrivateState);
        }
        Ok(())
    }
    fn write(&self, state: &State) -> Result<(), ClientError> {
        self.write_checked(state, |_| Ok(()))
    }
    fn write_checked(
        &self,
        state: &State,
        checkpoint: impl Fn(WriteStage) -> Result<(), ClientError>,
    ) -> Result<(), ClientError> {
        self.check_lease()?;
        let bytes = serde_json::to_vec(state).map_err(|_| ClientError::PrivateState)?;
        self.limits.check_add(Resource::Body, 0, bytes.len())?;
        let temp = CString::new(format!(
            ".{}.{}.tmp",
            self.name.to_string_lossy(),
            uuid::Uuid::new_v4().simple()
        ))
        .map_err(|_| ClientError::PrivateState)?;
        let result = (|| {
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    temp.as_ptr(),
                    libc::O_WRONLY
                        | libc::O_CREAT
                        | libc::O_EXCL
                        | libc::O_NOFOLLOW
                        | libc::O_CLOEXEC,
                    0o600,
                )
            };
            if fd < 0 {
                return Err(ClientError::PrivateState);
            }
            let mut file = unsafe { File::from_raw_fd(fd) };
            validate_private_file(&file)?;
            file.write_all(&bytes)
                .map_err(|_| ClientError::PrivateState)?;
            checkpoint(WriteStage::FileSync)?;
            file.sync_all().map_err(|_| ClientError::PrivateState)?;
            self.check_lease()?;
            checkpoint(WriteStage::Rename)?;
            if unsafe {
                libc::renameat(
                    self.directory.as_raw_fd(),
                    temp.as_ptr(),
                    self.directory.as_raw_fd(),
                    self.name.as_ptr(),
                )
            } != 0
            {
                return Err(ClientError::PrivateState);
            }
            checkpoint(WriteStage::DirectorySync)?;
            self.directory
                .sync_all()
                .map_err(|_| ClientError::PrivateState)
        })();
        if result.is_err() {
            unsafe { libc::unlinkat(self.directory.as_raw_fd(), temp.as_ptr(), 0) };
        }
        result
    }
    fn load_state(&self) -> Result<State, ClientError> {
        self.check_lease()?;
        let fd = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                self.name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if fd < 0 {
            return Err(ClientError::PrivateState);
        }
        let file = unsafe { File::from_raw_fd(fd) };
        validate_private_file(&file)?;
        let bytes = read_bounded(file, self.limits.maximum(Resource::Body))?;
        let state: State = serde_json::from_slice(&bytes).map_err(|_| ClientError::PrivateState)?;
        if state.version != 1
            || state.protocol != PROTOCOL_VERSION
            || state.contract != CONTRACT_REVISION
            || state.request.protocol_version != PROTOCOL_VERSION
            || state.generation == 0
            || state.input_digest != digest(&state.request.input)?
        {
            return Err(ClientError::PrivateState);
        }
        self.limits.check_add(
            Resource::Input,
            0,
            serde_json::to_vec(&state.request.input)
                .map_err(|_| ClientError::PrivateState)?
                .len(),
        )?;
        crate::application::call::validate_request(&state.request)
            .map_err(|_| ClientError::PrivateState)?;
        if let Some(result) = &state.result {
            if state.outcome != outcome(result)
                || result
                    .as_ref()
                    .err()
                    .is_some_and(|f| f.request_id != state.request.request_id)
            {
                return Err(ClientError::PrivateState);
            }
        } else if state.outcome != Outcome::Unknown {
            return Err(ClientError::PrivateState);
        }
        Ok(state)
    }
    pub fn begin_retry(
        &self,
        expected_generation: u64,
        endpoint: &EndpointIdentity,
    ) -> Result<RetryRecord, ClientError> {
        let _transaction = self
            .transaction
            .lock()
            .map_err(|_| ClientError::PrivateState)?;
        let mut state = self.load_state()?;
        if state.instance != endpoint.instance() || state.epoch != endpoint.epoch() {
            return Err(ClientError::Incompatible);
        }
        let retryable = match &state.result {
            None => true,
            Some(Err(fault)) => crate::domain::attempt::fault_allows_explicit_retry(fault),
            Some(Ok(_)) => false,
        };
        if state.generation != expected_generation || !retryable {
            return Err(ClientError::StaleGeneration);
        }
        state.generation = state
            .generation
            .checked_add(1)
            .ok_or(ClientError::PrivateState)?;
        state.outcome = Outcome::Unknown;
        state.result = None;
        self.write(&state)?;
        record(state)
    }
}
fn outcome(result: &Result<CallReply, WorkbenchFault>) -> Outcome {
    match result {
        Ok(CallReply::Complete { .. }) => Outcome::Applied,
        Ok(CallReply::Accepted { .. }) => Outcome::Unknown,
        Err(fault) => fault.outcome,
    }
}
fn digest(input: &serde_json::Value) -> Result<String, ClientError> {
    let bytes = serde_json::to_vec(input).map_err(|_| ClientError::PrivateState)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
fn record(state: State) -> Result<RetryRecord, ClientError> {
    Ok(RetryRecord {
        request: state.request,
        endpoint: EndpointIdentity::new(state.instance, state.epoch)
            .map_err(|_| ClientError::PrivateState)?,
        generation: state.generation,
        outcome: state.outcome,
        result: state.result,
    })
}
impl PrivateRetryStore {
    fn publish_checked(
        &self,
        record: &RetryRecord,
        checkpoint: impl Fn(WriteStage) -> Result<(), ClientError>,
    ) -> Result<(), ClientError> {
        let _transaction = self
            .transaction
            .lock()
            .map_err(|_| ClientError::PrivateState)?;
        self.check_lease()?;
        crate::application::call::validate_request(&record.request)
            .map_err(|_| ClientError::PrivateState)?;
        self.limits.check_add(
            Resource::Input,
            0,
            serde_json::to_vec(&record.request.input)
                .map_err(|_| ClientError::PrivateState)?
                .len(),
        )?;
        if record.generation != 1 || record.outcome != Outcome::Unknown || record.result.is_some() {
            return Err(ClientError::PrivateState);
        }
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                self.name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } == 0
            || std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound
        {
            return Err(ClientError::StaleGeneration);
        }
        self.write_checked(
            &State {
                version: 1,
                protocol: PROTOCOL_VERSION,
                contract: CONTRACT_REVISION,
                request: record.request.clone(),
                instance: record.endpoint.instance().into(),
                epoch: record.endpoint.epoch().into(),
                generation: 1,
                outcome: Outcome::Unknown,
                input_digest: digest(&record.request.input)?,
                result: None,
            },
            checkpoint,
        )
    }
}
impl RetryStore for PrivateRetryStore {
    fn publish(&self, record: &RetryRecord) -> Result<(), ClientError> {
        self.publish_checked(record, |_| Ok(()))
    }
    fn load(&self) -> Result<RetryRecord, ClientError> {
        let _transaction = self
            .transaction
            .lock()
            .map_err(|_| ClientError::PrivateState)?;
        record(self.load_state()?)
    }
    fn complete(
        &self,
        generation: u64,
        result: Result<CallReply, WorkbenchFault>,
    ) -> Result<(), ClientError> {
        let _transaction = self
            .transaction
            .lock()
            .map_err(|_| ClientError::PrivateState)?;
        let mut state = self.load_state()?;
        if state.generation != generation
            || state.result.is_some()
            || result
                .as_ref()
                .err()
                .is_some_and(|f| f.request_id != state.request.request_id)
        {
            return Err(ClientError::StaleGeneration);
        }
        state.outcome = outcome(&result);
        state.result = Some(result);
        self.write(&state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn sync_and_rename_failure_injection_never_reports_publish_success_or_loses_identity() {
        for stage in [
            WriteStage::FileSync,
            WriteStage::Rename,
            WriteStage::DirectorySync,
        ] {
            let root = tempfile::tempdir().unwrap();
            let parent = root.path().canonicalize().unwrap();
            std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
            let path = parent.join("attempt.json");
            let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
            let original = RetryRecord {
                request: CallRequest::command(
                    workbench_protocol::OperationId::ProjectCreate,
                    serde_json::json!({"name":"private","workingDirectory":"/private/tmp"}),
                ),
                endpoint: EndpointIdentity::new("i", "e").unwrap(),
                generation: 1,
                outcome: Outcome::Unknown,
                result: None,
            };
            let failed = store.publish_checked(&original, |at| {
                if at == stage {
                    Err(ClientError::PrivateState)
                } else {
                    Ok(())
                }
            });
            assert!(matches!(failed, Err(ClientError::PrivateState)));
            // This is syscall-boundary failure injection, not reboot durability proof.
            // Callers must not submit after this error, even if rename made state visible.
            if stage == WriteStage::DirectorySync {
                let restored = store.load().unwrap();
                assert_eq!(restored.request, original.request);
                assert_eq!(restored.endpoint, original.endpoint);
                assert_eq!(restored.generation, 1);
                assert_eq!(restored.outcome, Outcome::Unknown);
                assert!(restored.result.is_none());
            } else {
                assert!(!path.exists());
                assert!(store.load().is_err());
                store.publish(&original).unwrap();
            }

            let mut next = store.load_state().unwrap();
            next.generation = 2;
            assert!(matches!(
                store.write_checked(&next, |at| if at == stage {
                    Err(ClientError::PrivateState)
                } else {
                    Ok(())
                }),
                Err(ClientError::PrivateState)
            ));
            let restored = store.load().unwrap();
            assert_eq!(restored.request, original.request);
            assert_eq!(restored.endpoint, original.endpoint);
            assert_eq!(restored.outcome, Outcome::Unknown);
            assert_eq!(
                restored.generation,
                if stage == WriteStage::DirectorySync {
                    2
                } else {
                    1
                }
            );
            assert_eq!(std::fs::read_dir(parent).unwrap().count(), 2); // state+lease, no temp residue
        }
    }
    struct FailingPublish<'a> {
        store: &'a PrivateRetryStore,
        stage: WriteStage,
    }
    impl RetryStore for FailingPublish<'_> {
        fn publish(&self, record: &RetryRecord) -> Result<(), ClientError> {
            self.store.publish_checked(record, |at| {
                if at == self.stage {
                    Err(ClientError::PrivateState)
                } else {
                    Ok(())
                }
            })
        }
        fn load(&self) -> Result<RetryRecord, ClientError> {
            self.store.load()
        }
        fn complete(
            &self,
            g: u64,
            result: Result<CallReply, WorkbenchFault>,
        ) -> Result<(), ClientError> {
            self.store.complete(g, result)
        }
    }
    #[tokio::test]
    async fn initial_sync_rename_failures_never_reach_owned_http_command_transport() {
        use crate::{
            application::call::{execute, publish_attempt},
            infrastructure::http::HttpConnection,
            ports::CallTransport,
        };
        for stage in [
            WriteStage::FileSync,
            WriteStage::Rename,
            WriteStage::DirectorySync,
        ] {
            let mut peer = crate::fixture::Peer::spawn(vec![], true).await;
            let root = tempfile::tempdir().unwrap();
            let parent = root.path().canonicalize().unwrap();
            std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
            let path = parent.join("attempt.json");
            let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
            let mut transport = HttpConnection::connect(peer.endpoint.clone(), Limits::default())
                .await
                .unwrap();
            let record = RetryRecord {
                request: CallRequest::command(
                    workbench_protocol::OperationId::ProjectCreate,
                    serde_json::json!({"name":"private","workingDirectory":"/private/tmp"}),
                ),
                endpoint: transport.identity().clone(),
                generation: 1,
                outcome: Outcome::Unknown,
                result: None,
            };
            let prepared = publish_attempt(
                &FailingPublish {
                    store: &store,
                    stage,
                },
                &record,
            );
            let failed = prepared.is_err();
            if let Ok(mut attempt) = prepared {
                let _ = execute(&mut transport, &mut attempt).await;
            }
            transport.close().await.unwrap();
            peer.settled().await;
            assert!(failed);
            assert_eq!(peer.requests.lock().unwrap().len(), 2); // identity+handshake, calls0
            assert_eq!(peer.effects.lock().unwrap().len(), 0);
            if stage == WriteStage::DirectorySync {
                let restored = store.load().unwrap();
                assert_eq!(restored.request, record.request);
                assert_eq!(restored.endpoint, record.endpoint);
                assert_eq!(restored.outcome, Outcome::Unknown);
            } else {
                assert!(!path.exists());
            }
        }
    }
    #[test]
    fn ending_store_ownership_releases_lease_even_when_a_duplicate_fd_is_still_open() {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().canonicalize().unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = parent.join("attempt.json");
        let store = PrivateRetryStore::open(&path, Limits::default()).unwrap();
        let duplicate = store.lock.try_clone().unwrap();
        drop(store);
        let reopened = PrivateRetryStore::open(&path, Limits::default()).unwrap();
        drop(duplicate);
        assert!(PrivateRetryStore::open(&path, Limits::default()).is_err());
        drop(reopened);
    }
}
