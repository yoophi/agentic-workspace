//! Windows feasibility entry points are compiled and exercised on the Windows
//! quality job before production consumers may migrate.

use std::{
    ffi::OsStr,
    io, mem,
    os::windows::ffi::OsStrExt,
    os::windows::io::AsRawHandle,
    path::Path,
    process::{Child, Command},
    ptr,
    time::Duration,
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ACCESS_DENIED, HANDLE, WAIT_OBJECT_0},
    System::{
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicAccountingInformation,
            JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        Threading::{
            CreateProcessW, OpenProcess, ResumeThread, TerminateProcess, WaitForSingleObject,
            CREATE_SUSPENDED, PROCESS_INFORMATION, STARTUPINFOW,
        },
    },
};

const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsCapabilityReport {
    pub suspended_create: bool,
    pub assign_before_resume: bool,
    pub kill_on_close: bool,
    pub breakaway_denied: bool,
}

impl WindowsCapabilityReport {
    #[must_use]
    pub fn supports_required_containment(&self) -> bool {
        self.suspended_create
            && self.assign_before_resume
            && self.kill_on_close
            && self.breakaway_denied
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsJobEvidence {
    pub suspended_create: bool,
    pub assign_before_resume: bool,
    pub resumed_once: bool,
    pub active_processes_before_close: u32,
    pub breakaway_denied: bool,
    pub direct_wait_completed: bool,
    pub descendant_wait_completed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowsServerCrashEvidence {
    pub owner_terminate_error: Option<i32>,
    pub owner_wait_completed: bool,
    pub direct_wait_completed: bool,
    pub descendant_wait_completed: bool,
}

struct Handle(HANDLE);

impl Handle {
    fn new(handle: HANDLE) -> io::Result<Self> {
        if handle.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: this wrapper uniquely owns every non-null handle it stores.
        unsafe { CloseHandle(self.0) };
    }
}

struct SuspendedProcessGuard {
    process: HANDLE,
    thread: HANDLE,
}

impl SuspendedProcessGuard {
    fn new(info: PROCESS_INFORMATION) -> io::Result<Self> {
        if info.hProcess.is_null() || info.hThread.is_null() {
            if !info.hProcess.is_null() {
                // SAFETY: CreateProcessW returned this process handle.
                unsafe {
                    TerminateProcess(info.hProcess, 125);
                    WaitForSingleObject(info.hProcess, 5_000);
                    CloseHandle(info.hProcess);
                }
            }
            if !info.hThread.is_null() {
                // SAFETY: CreateProcessW returned this thread handle.
                unsafe { CloseHandle(info.hThread) };
            }
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CreateProcessW returned an incomplete handle pair",
            ));
        }
        Ok(Self {
            process: info.hProcess,
            thread: info.hThread,
        })
    }
}

impl Drop for SuspendedProcessGuard {
    fn drop(&mut self) {
        // SAFETY: the guard uniquely owns both CreateProcessW handles. The
        // bounded wait also covers every early-return path after creation.
        unsafe {
            TerminateProcess(self.process, 125);
            WaitForSingleObject(self.process, 5_000);
            CloseHandle(self.thread);
            CloseHandle(self.process);
        }
    }
}

struct OwnerProcessGuard(Option<Child>);

struct OwnerTerminationEvidence {
    terminate_error: Option<i32>,
    wait_completed: bool,
}

impl OwnerProcessGuard {
    fn terminate_and_wait(&mut self) -> OwnerTerminationEvidence {
        let Some(child) = self.0.as_mut() else {
            return OwnerTerminationEvidence {
                terminate_error: None,
                wait_completed: true,
            };
        };
        let terminate_error = child.kill().err().and_then(|error| error.raw_os_error());
        // SAFETY: Child retains ownership of this process handle for the whole
        // bounded wait. It is not removed from the guard until try_wait reaps it.
        let wait_completed =
            unsafe { WaitForSingleObject(child.as_raw_handle().cast(), 5_000) == WAIT_OBJECT_0 };
        let reaped = wait_completed && matches!(child.try_wait(), Ok(Some(_)));
        if reaped {
            self.0.take();
        }
        OwnerTerminationEvidence {
            terminate_error,
            wait_completed: reaped,
        }
    }
}

impl Drop for OwnerProcessGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            // SAFETY: the guard still owns the process handle. Cleanup is
            // bounded; a timeout closes the handle on Child drop without an
            // unbounded wait, and the returned evidence has already failed.
            let completed = unsafe {
                WaitForSingleObject(child.as_raw_handle().cast(), 5_000) == WAIT_OBJECT_0
            };
            if completed {
                let _ = child.try_wait();
            }
        }
    }
}

fn create_kill_on_close_job() -> io::Result<Handle> {
    let job = Handle::new(unsafe { CreateJobObjectW(ptr::null(), ptr::null()) })?;
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: limits has the structure required by the selected information class.
    if unsafe {
        SetInformationJobObject(
            job.0,
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            mem::size_of_val(&limits) as u32,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(job)
}

fn create_suspended(command: &str) -> io::Result<SuspendedProcessGuard> {
    let mut command_wide: Vec<u16> = OsStr::new(command).encode_wide().chain(Some(0)).collect();
    let mut startup: STARTUPINFOW = unsafe { mem::zeroed() };
    startup.cb = mem::size_of::<STARTUPINFOW>() as u32;
    let mut process_info: PROCESS_INFORMATION = unsafe { mem::zeroed() };
    // SAFETY: command_wide is mutable and NUL terminated; output structures are writable.
    if unsafe {
        CreateProcessW(
            ptr::null(),
            command_wide.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            CREATE_SUSPENDED,
            ptr::null(),
            ptr::null(),
            &startup,
            &mut process_info,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    SuspendedProcessGuard::new(process_info)
}

/// Runs the real suspended-create/Job assignment/kill-on-close sequence.
/// The fixture records whether an explicit breakaway creation was rejected.
pub fn run_job_object_spike(fixture: &Path, result_path: &Path) -> io::Result<WindowsJobEvidence> {
    let job = create_kill_on_close_job()?;

    let command = format!(
        "\"{}\" windows-job-probe \"{}\"",
        fixture.display(),
        result_path.display()
    );
    let process = create_suspended(&command)?;

    // SAFETY: the process handle was returned by CreateProcessW and remains suspended.
    if unsafe { AssignProcessToJobObject(job.0, process.process) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the primary thread is still suspended and this is its first resume.
    let previous_suspend_count = unsafe { ResumeThread(process.thread) };
    if previous_suspend_count == u32::MAX {
        return Err(io::Error::last_os_error());
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let result = loop {
        if let Ok(value) = std::fs::read_to_string(result_path) {
            break value;
        }
        if std::time::Instant::now() >= deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "fixture result"));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let mut fields = result.trim().split(':');
    let outcome = fields.next().unwrap_or_default();
    let breakaway_error = fields
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "breakaway error"))?;
    let descendant_pid = fields
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "descendant PID"))?;
    let breakaway_denied = outcome == "denied" && breakaway_error == ERROR_ACCESS_DENIED;
    // SAFETY: the fixture reports a live contained child. The handle acquired
    // before Job close remains bound to that exact process even if its PID is reused.
    let descendant = Handle::new(unsafe { OpenProcess(SYNCHRONIZE_ACCESS, 0, descendant_pid) })?;

    let mut accounting: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { mem::zeroed() };
    // SAFETY: accounting matches JobObjectBasicAccountingInformation.
    if unsafe {
        QueryInformationJobObject(
            job.0,
            JobObjectBasicAccountingInformation,
            (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            mem::size_of_val(&accounting) as u32,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let active_processes = accounting.ActiveProcesses;

    drop(job);
    // SAFETY: both exact process handles remain valid after Job close. Timeouts
    // are returned as failed evidence rather than hanging the job.
    let direct_wait_completed =
        unsafe { WaitForSingleObject(process.process, 5_000) } == WAIT_OBJECT_0;
    let descendant_wait_completed =
        unsafe { WaitForSingleObject(descendant.0, 5_000) } == WAIT_OBJECT_0;
    let _ = std::fs::remove_file(result_path);
    Ok(WindowsJobEvidence {
        suspended_create: true,
        assign_before_resume: true,
        resumed_once: previous_suspend_count == 1,
        active_processes_before_close: active_processes,
        breakaway_denied,
        direct_wait_completed,
        descendant_wait_completed,
    })
}

/// Fixture entry point: owns a kill-on-close Job until this owner process is
/// terminated. The payload reports its own and its descendant's PID.
pub fn hold_job_until_owner_exit(fixture: &Path, result_path: &Path) -> io::Result<()> {
    let job = create_kill_on_close_job()?;
    let command = format!(
        "\"{}\" windows-job-owned-payload \"{}\"",
        fixture.display(),
        result_path.display()
    );
    let process = create_suspended(&command)?;
    // SAFETY: the payload remains suspended until Job assignment succeeds.
    if unsafe { AssignProcessToJobObject(job.0, process.process) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: this is the payload primary thread's first resume.
    if unsafe { ResumeThread(process.thread) } != 1 {
        return Err(io::Error::last_os_error());
    }

    let _owned_job = job;
    let _owned_process = process;
    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

/// Kills the separate Job owner process and waits on handles opened before the
/// kill, proving owner-handle loss terminates both payload generations.
pub fn run_server_crash_spike(
    fixture: &Path,
    result_path: &Path,
) -> io::Result<WindowsServerCrashEvidence> {
    let owner = Command::new(fixture)
        .arg("windows-job-owner")
        .arg(result_path)
        .spawn()?;
    let mut owner = OwnerProcessGuard(Some(owner));
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let (direct_pid, descendant_pid) = loop {
        if let Ok(value) = std::fs::read_to_string(result_path) {
            let mut fields = value.trim().split(':');
            let direct = fields.next().and_then(|value| value.parse::<u32>().ok());
            let descendant = fields.next().and_then(|value| value.parse::<u32>().ok());
            if let (Some(direct), Some(descendant)) = (direct, descendant) {
                break (direct, descendant);
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Job owner fixture result",
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    // SAFETY: the fixture reported live contained processes. These handles are
    // acquired before killing the owner and remain identity-safe across reuse.
    let direct = Handle::new(unsafe { OpenProcess(SYNCHRONIZE_ACCESS, 0, direct_pid) })?;
    let descendant = Handle::new(unsafe { OpenProcess(SYNCHRONIZE_ACCESS, 0, descendant_pid) })?;
    let owner_termination = owner.terminate_and_wait();
    let direct_wait_completed = unsafe { WaitForSingleObject(direct.0, 5_000) } == WAIT_OBJECT_0;
    let descendant_wait_completed =
        unsafe { WaitForSingleObject(descendant.0, 5_000) } == WAIT_OBJECT_0;
    let _ = std::fs::remove_file(result_path);
    Ok(WindowsServerCrashEvidence {
        owner_terminate_error: owner_termination.terminate_error,
        owner_wait_completed: owner_termination.wait_completed,
        direct_wait_completed,
        descendant_wait_completed,
    })
}
