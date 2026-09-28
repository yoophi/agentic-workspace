//! Windows feasibility entry points are compiled and exercised on the Windows
//! quality job before production consumers may migrate.

use std::{ffi::OsStr, io, mem, os::windows::ffi::OsStrExt, path::Path, ptr};

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

/// Runs the real suspended-create/Job assignment/kill-on-close sequence.
/// The fixture records whether an explicit breakaway creation was rejected.
pub fn run_job_object_spike(fixture: &Path, result_path: &Path) -> io::Result<WindowsJobEvidence> {
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

    let command = format!(
        "\"{}\" windows-job-probe \"{}\"",
        fixture.display(),
        result_path.display()
    );
    let mut command_wide: Vec<u16> = OsStr::new(&command).encode_wide().chain(Some(0)).collect();
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
    let process = SuspendedProcessGuard::new(process_info)?;

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
