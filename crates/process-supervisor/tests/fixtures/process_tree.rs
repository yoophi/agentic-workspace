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
