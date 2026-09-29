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
pub struct PrivateRetryStore {
    directory: File,
    name: CString,
    lock: File,
    lock_name: CString,
    limits: Limits,
    transaction: std::sync::Mutex<()>,
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
            file.sync_all().map_err(|_| ClientError::PrivateState)?;
            self.check_lease()?;
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
impl RetryStore for PrivateRetryStore {
    fn publish(&self, record: &RetryRecord) -> Result<(), ClientError> {
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
        self.write(&State {
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
        })
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
