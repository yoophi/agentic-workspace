//! `agentic-workbench-server` — 독립 composition root(044). 명령줄 해석과 `main`만 둔다. 조립·생명주기는
//! `workbench-host`에 있다(contracts/server-lifecycle.md §1).
//!
//! - `serve --data-dir <dir>`: 0 정상 정지, 3 이미 서버 있음, 4 저장 형식 거절, 1 그 밖.
//! - `ensure --data-dir <dir>`: 준비된 서버의 안내 JSON(자격 증명 제외)을 표준 출력에. 0 준비됨, 1 실패.
//! - `status --data-dir <dir>`: 안내 파일의 서버를 확인해 JSON으로. 0, 2 서버 없음, 1 확인 실패.
//! - `stop --data-dir <dir> [--wait|--force]`: **아직 구현하지 않음**(T041·T044). 종료 코드 1.

use std::path::PathBuf;

use workbench_host::lifecycle::{
    client::verify,
    descriptor::read_descriptor,
    ensure::{EnsureOptions, ensure, server_executable},
    lock::server_dir,
    server::{ServeOptions, serve},
};

const USAGE: &str = "usage: agentic-workbench-server <serve|ensure|status|stop> --data-dir <dir>";

fn data_dir(args: &[String]) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == "--data-dir")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(command), Some(data_dir)) = (args.first(), data_dir(&args)) else {
        eprintln!("{USAGE}");
        std::process::exit(64);
    };
    let code = match command.as_str() {
        "serve" => serve(ServeOptions {
            data_dir,
            server_version: env!("CARGO_PKG_VERSION").to_owned(),
        }),
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
            Ok(Some(descriptor)) => match verify(&descriptor) {
                Ok(verified) => {
                    println!(
                        "{}",
                        serde_json::json!({
                            "instanceId": verified.instance_id,
                            "serverEpoch": verified.server_epoch,
                            "baseUrl": verified.base_url,
                            "pid": descriptor.pid,
                        })
                    );
                    0
                }
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
            eprintln!("[workbench-server] stop is not implemented yet (044 T041/T044)");
            1
        }
        _ => {
            eprintln!("{USAGE}");
            64
        }
    };
    std::process::exit(code);
}
