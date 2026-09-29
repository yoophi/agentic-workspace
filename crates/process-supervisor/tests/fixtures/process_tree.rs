//! Standalone fixture source compiled by containment integration tests.

use std::{env, fs, process::Command, thread, time::Duration};

// The fixture intentionally creates unreaped/reparented children so the
// supervisor tests can prove that containment, not the fixture parent, cleans
// them. Every integration test installs its own cleanup guard.
#[allow(clippy::zombie_processes)]
fn main() {
    let mode = env::args().nth(1).unwrap_or_else(|| "sleep".into());
    match mode.as_str() {
        "sleep" => sleep_forever(),
        "grandchild" => {
            let executable = env::current_exe().expect("fixture executable");
            let _child = Command::new(executable)
                .arg("sleep")
                .spawn()
                .expect("spawn grandchild");
            sleep_forever();
        }
        "leader-exit" => {
            let executable = env::current_exe().expect("fixture executable");
            let child = Command::new(executable)
                .arg("sleep")
                .spawn()
                .expect("spawn surviving child");
            record_pid(child.id());
        }
        "env-clear-exec" => {
            let executable = env::current_exe().expect("fixture executable");
            let mut child = Command::new(executable);
            child.arg("sleep").env_clear();
            #[cfg(unix)]
            new_session(&mut child);
            let child = child.spawn().expect("spawn env-cleared descendant");
            record_pid(child.id());
        }
        "keeper-env-clear" => {
            let executable = env::current_exe().expect("fixture executable");
            let mut child = Command::new(executable);
            child.arg("sleep").env_clear();
            #[cfg(unix)]
            new_session(&mut child);
            let child = child.spawn().expect("spawn keeper-owned escaped child");
            record_pid(child.id());
            sleep_forever();
        }
        "server-keeper-env-clear" => {
            let executable = env::current_exe().expect("fixture executable");
            let keeper = Command::new(executable)
                .arg("keeper-env-clear")
                .spawn()
                .expect("spawn keeper fixture");
            if let Some(path) = env::var_os("AW_045_KEEPER_PID_PATH") {
                fs::write(path, keeper.id().to_string()).expect("record keeper pid");
            }
            sleep_forever();
        }
        "windows-job-probe" => windows_job_probe(),
        "windows-job-owner" => windows_job_owner(),
        "windows-job-owned-payload" => windows_job_owned_payload(),
        "linux-clone-into-cgroup" => linux_clone_into_cgroup(),
        "linux-cgroup-payload" => linux_cgroup_payload(),
        "new-process-group" => {
            #[cfg(unix)]
            // SAFETY: the fixture is single-threaded and changes only its own
            // process-group membership.
            unsafe {
                if libc::setpgid(0, 0) == -1 {
                    panic!("setpgid failed: {}", std::io::Error::last_os_error());
                }
            }
            record_pid(std::process::id());
            sleep_forever();
        }
        "new-session" => {
            #[cfg(unix)]
            // SAFETY: the fixture is single-threaded at this point and calls
            // setsid only to exercise containment escape behavior.
            unsafe {
                if libc::setsid() == -1 {
                    panic!("setsid failed: {}", std::io::Error::last_os_error());
                }
            }
            record_pid(std::process::id());
            sleep_forever();
        }
        "double-fork-reparent" => double_fork_reparent(),
        "control-fd-close" => {
            #[cfg(unix)]
            if let Some(fd) = env::var("AW_045_CONTROL_FD")
                .ok()
                .and_then(|value| value.parse::<libc::c_int>().ok())
            {
                // SAFETY: the fixture owns the inherited descriptor identified
                // by the test and intentionally closes it.
                unsafe {
                    libc::close(fd);
                }
            }
            record_pid(std::process::id());
            sleep_forever();
        }
        "signal-ignore" => {
            #[cfg(unix)]
            // SAFETY: SIG_IGN is a valid signal disposition for this fixture.
            unsafe {
                libc::signal(libc::SIGTERM, libc::SIG_IGN);
            }
            sleep_forever();
        }
        other => panic!("unknown fixture mode: {other}"),
    }
}

fn record_pid(pid: u32) {
    if let Some(path) = env::var_os("AW_045_RESULT_PATH") {
        fs::write(path, pid.to_string()).expect("record fixture pid");
    }
}

#[cfg(unix)]
fn double_fork_reparent() {
    // SAFETY: this dedicated single-threaded fixture uses fork only to create
    // the exact daemonization escape sequence under test. Child branches call
    // only libc primitives, a small file write, and the sleep loop.
    unsafe {
        let first = libc::fork();
        if first < 0 {
            panic!("first fork failed: {}", std::io::Error::last_os_error());
        }
        if first > 0 {
            let mut status = 0;
            libc::waitpid(first, &mut status, 0);
            return;
        }
        if libc::setsid() == -1 {
            libc::_exit(120);
        }
        let second = libc::fork();
        if second < 0 {
            libc::_exit(121);
        }
        if second > 0 {
            libc::_exit(0);
        }
        record_pid(std::process::id());
        sleep_forever();
    }
}

#[cfg(not(unix))]
fn double_fork_reparent() {
    panic!("double-fork fixture is Unix-only");
}

fn sleep_forever() -> ! {
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(windows)]
#[allow(clippy::zombie_processes)]
fn windows_job_probe() -> ! {
    use std::os::windows::process::CommandExt;
    use windows_sys::Win32::System::Threading::CREATE_BREAKAWAY_FROM_JOB;

    let result_path = env::args().nth(2).expect("result path argument");
    let executable = env::current_exe().expect("fixture executable");
    let contained_descendant = Command::new(&executable)
        .arg("sleep")
        .spawn()
        .expect("spawn contained descendant");
    let descendant_pid = contained_descendant.id();
    let breakaway = Command::new(executable)
        .arg("sleep")
        .creation_flags(CREATE_BREAKAWAY_FROM_JOB)
        .spawn();
    let outcome = match breakaway {
        Ok(mut child) => {
            let _ = child.kill();
            let _ = child.wait();
            format!("spawned:0:{descendant_pid}")
        }
        Err(error) => format!(
            "denied:{}:{descendant_pid}",
            error.raw_os_error().unwrap_or_default()
        ),
    };
    fs::write(result_path, outcome).expect("write breakaway result");
    sleep_forever();
}

#[cfg(windows)]
fn windows_job_owner() -> ! {
    let result_path = env::args().nth(2).expect("result path argument");
    let executable = env::current_exe().expect("fixture executable");
    process_supervisor::platform::windows::feasibility::hold_job_until_owner_exit(
        &executable,
        std::path::Path::new(&result_path),
    )
    .expect("hold Job until owner exit");
    unreachable!("Job owner fixture runs until it is terminated");
}

#[cfg(windows)]
#[allow(clippy::zombie_processes)]
fn windows_job_owned_payload() -> ! {
    let result_path = env::args().nth(2).expect("result path argument");
    let executable = env::current_exe().expect("fixture executable");
    let descendant = Command::new(executable)
        .arg("sleep")
        .spawn()
        .expect("spawn Job-owned descendant");
    fs::write(
        result_path,
        format!("{}:{}", std::process::id(), descendant.id()),
    )
    .expect("write Job-owned process ids");
    sleep_forever();
}

#[cfg(target_os = "linux")]
fn linux_clone_into_cgroup() {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::CommandExt;

    #[repr(C)]
    #[derive(Default)]
    struct CloneArgs {
        flags: u64,
        pidfd: u64,
        child_tid: u64,
        parent_tid: u64,
        exit_signal: u64,
        stack: u64,
        stack_size: u64,
        tls: u64,
        set_tid: u64,
        set_tid_size: u64,
        cgroup: u64,
    }

    const CLONE_INTO_CGROUP: u64 = 1 << 33;
    let cgroup_path = env::args().nth(2).expect("cgroup path argument");
    let result_path = env::args().nth(3).expect("result path argument");
    let cgroup = fs::File::open(cgroup_path).expect("open attempt cgroup");
    let args = CloneArgs {
        flags: CLONE_INTO_CGROUP,
        exit_signal: libc::SIGCHLD as u64,
        cgroup: cgroup.as_raw_fd() as u64,
        ..CloneArgs::default()
    };
    // SAFETY: clone3 receives a complete zero-initialized clone_args. The
    // returned child immediately execs the single-threaded fixture binary.
    let child = unsafe {
        libc::syscall(
            libc::SYS_clone3,
            &args as *const CloneArgs,
            std::mem::size_of::<CloneArgs>(),
        )
    };
    if child < 0 {
        panic!(
            "clone3(CLONE_INTO_CGROUP) failed: {}",
            std::io::Error::last_os_error()
        );
    }
    if child == 0 {
        let executable = env::current_exe().expect("fixture executable");
        let error = Command::new(executable)
            .arg("linux-cgroup-payload")
            .arg(result_path)
            .env_clear()
            .exec();
        eprintln!("exec linux cgroup payload failed: {error}");
        // SAFETY: exec failed in the clone child; no Rust destructors may run.
        unsafe { libc::_exit(126) };
    }
}

#[cfg(target_os = "linux")]
#[allow(clippy::zombie_processes)]
fn linux_cgroup_payload() -> ! {
    let result_path = env::args().nth(2).expect("result path argument");
    let executable = env::current_exe().expect("fixture executable");
    let mut descendant = Command::new(executable);
    descendant.arg("sleep").env_clear();
    new_session(&mut descendant);
    let descendant = descendant
        .spawn()
        .expect("spawn env-cleared cgroup descendant");
    fs::write(
        result_path,
        format!("{}:{}", std::process::id(), descendant.id()),
    )
    .expect("write cgroup process ids");
    sleep_forever();
}

#[cfg(not(target_os = "linux"))]
fn linux_clone_into_cgroup() {
    panic!("Linux cgroup fixture is Linux-only");
}

#[cfg(not(target_os = "linux"))]
fn linux_cgroup_payload() -> ! {
    panic!("Linux cgroup payload fixture is Linux-only");
}

#[cfg(not(windows))]
fn windows_job_probe() -> ! {
    panic!("Windows Job fixture is Windows-only");
}

#[cfg(not(windows))]
fn windows_job_owner() -> ! {
    panic!("Windows Job owner fixture is Windows-only");
}

#[cfg(not(windows))]
fn windows_job_owned_payload() -> ! {
    panic!("Windows Job payload fixture is Windows-only");
}

#[cfg(unix)]
fn new_session(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: this closure only calls async-signal-safe setsid before exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}
