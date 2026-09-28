//! `agentic-workbench-server` — 독립 composition root(044). 명령줄 해석과 `main`만 둔다. 조립·생명주기는
//! `workbench-host`에 있다(contracts/server-lifecycle.md §1).
//!
//! - `serve --data-dir <dir> [--idle-timeout <sec>] [--log <file>]`: 0 정상 정지, 3 이미 서버 있음, 4 저장 형식 거절,
//!   1 그 밖.
//! - `ensure --data-dir <dir>`: 준비된 서버의 안내 JSON(자격 증명 제외)을 표준 출력에. 0 준비됨, 1 실패.
//! - `status --data-dir <dir>`: 안내 파일의 서버를 확인해 JSON으로. 0, 2 서버 없음, 1 확인 실패.
//! - `stop --data-dir <dir> [--wait|--force]`: `server.stop` 뒤 정지 완료(안내 파일 삭제)까지 기다린다. 0 정지함(또는 서버
//!   없음), 5 `default`가 활동 작업으로 거절됨(blocker JSON을 표준 출력에), 1 그 밖.

use std::{path::PathBuf, time::Duration};

use workbench_host::lifecycle::{
    client::{server_status, verify_instance},
    descriptor::read_descriptor,
    ensure::{EnsureOptions, ensure, server_executable},
    lock::server_dir,
    monitor::DEFAULT_IDLE_TIMEOUT,
    server::{ServeOptions, serve},
    stop::{StopMode, stop},
};

const USAGE: &str = "usage: agentic-workbench-server <serve|ensure|status|stop> --data-dir <dir> \
[serve: --idle-timeout <sec> --log <file>] [stop: --wait|--force]";

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a String> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
}

fn data_dir(args: &[String]) -> Option<PathBuf> {
    flag_value(args, "--data-dir").map(PathBuf::from)
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(64);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(command), Some(data_dir)) = (args.first(), data_dir(&args)) else {
        usage();
    };
    let code = match command.as_str() {
        "serve" => {
            let idle_timeout = match flag_value(&args, "--idle-timeout") {
                None => DEFAULT_IDLE_TIMEOUT,
                Some(value) => match value.parse::<u64>() {
                    Ok(seconds) => Duration::from_secs(seconds),
                    Err(_) => usage(),
                },
            };
            serve(ServeOptions {
                data_dir,
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
                idle_timeout,
                log: flag_value(&args, "--log").map(PathBuf::from),
            })
        }
        "ensure" => {
            let exe = server_executable(std::env::current_exe().expect("current executable"));
            match ensure(&data_dir, &exe, &EnsureOptions::default()) {
                Ok(descriptor) => {
                    println!("{}", descriptor.public_json());
                    0
                }
                Err(error) => {
                    eprintln!("[workbench-server] ensure failed: {error}");
                    1
                }
            }
        }
        "status" => match read_descriptor(&server_dir(&data_dir)) {
            Ok(Some(descriptor)) => match verify_instance(&descriptor) {
                Ok(verified) => match server_status(&descriptor) {
                    Ok(mut status) => {
                        if let Some(object) = status.as_object_mut() {
                            object.insert("instanceId".into(), verified.instance_id.into());
                            object.insert("serverEpoch".into(), verified.server_epoch.into());
                            object.insert("baseUrl".into(), verified.base_url.into());
                            object.insert("pid".into(), descriptor.pid.into());
                        }
                        println!("{status}");
                        0
                    }
                    Err(error) => {
                        eprintln!("[workbench-server] {error}");
                        1
                    }
                },
                Err(error) => {
                    eprintln!("[workbench-server] {error}");
                    1
                }
            },
            Ok(None) => {
                eprintln!("[workbench-server] no server for this data directory");
                2
            }
            Err(error) => {
                eprintln!("[workbench-server] {error}");
                1
            }
        },
        "stop" => {
            let mode = match (
                args.iter().any(|arg| arg == "--wait"),
                args.iter().any(|arg| arg == "--force"),
            ) {
                (false, false) => StopMode::Default,
                (true, false) => StopMode::Wait,
                (false, true) => StopMode::Force,
                (true, true) => usage(),
            };
            let result = stop(&data_dir, mode);
            if let Some(stdout) = result.stdout {
                println!("{stdout}");
            }
            if let Some(stderr) = result.stderr {
                eprintln!("[workbench-server] {stderr}");
            }
            result.code
        }
        _ => {
            eprintln!("{USAGE}");
            64
        }
    };
    std::process::exit(code);
}
