//! CLI composition root; parsing, bounded lifetime, cancellation and safe output.
use aw_cli::{
    application, inbound,
    infrastructure::output::{finite, CliError},
};
use std::{
    process::ExitCode,
    sync::{Arc, Mutex},
};
use workbench_protocol::Outcome;
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|_| {}));
}
fn main() -> ExitCode {
    // Dependencies may panic with private protocol values: render a closed error in the parent.
    install_panic_hook();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => return ExitCode::from(1),
    };
    let code = runtime.block_on(async_main());
    runtime.shutdown_timeout(std::time::Duration::from_millis(100));
    ExitCode::from(code)
}
async fn async_main() -> u8 {
    let args = std::env::args_os()
        .skip(1)
        .map(|arg| arg.into_string().map_err(|_| CliError::usage()))
        .collect::<Result<Vec<_>, _>>();
    let options = match args.and_then(inbound::parse) {
        Ok(options) => options,
        Err(error) => {
            let code = error.exit;
            let _ = finite(error.value(), true).await;
            return code;
        }
    };
    let deadline = options.limits.config().request_timeout;
    let receipt: application::ReceiptSlot = Arc::new(Mutex::new(None));
    // Register before the application can consume input or submit a request.
    let mut interrupt =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()) {
            Ok(signal) => signal,
            Err(_) => {
                let _ = finite(
                    CliError::new("internal", 1, Outcome::NotApplied, false).value(),
                    true,
                )
                .await;
                return 1;
            }
        };
    let task_receipt = receipt.clone();
    let mut job = tokio::spawn(application::run(options, task_receipt));
    let result = tokio::select! {
     result=&mut job=>job_result(result),
     _=interrupt.recv()=>{job.abort();let _=(&mut job).await;Err(CliError::new("cancelled",130,Outcome::Unknown,false))},
     _=tokio::time::sleep(deadline)=>{job.abort();let _=(&mut job).await;Err(CliError::new("deadlineExceeded",7,Outcome::Unknown,true))},
    };
    let (value, stderr, code) = match result {
        Ok(value) => (value, false, 0),
        Err(mut error) => {
            if let Ok(receipt) = receipt.lock() {
                if let Some(receipt) = receipt.as_ref() {
                    error.state = Some(receipt.state.clone());
                    error.request_id = Some(receipt.request_id.clone());
                    error.outcome = receipt.outcome;
                }
            }
            let code = error.exit;
            (error.value(), true, code)
        }
    };
    match tokio::time::timeout(deadline, finite(value, stderr)).await {
        Ok(Ok(())) => code,
        _ => {
            let mut error = CliError::new("outputUnavailable", 8, Outcome::Unknown, false);
            if let Ok(receipt) = receipt.lock() {
                if let Some(receipt) = receipt.as_ref() {
                    error.state = Some(receipt.state.clone());
                    error.request_id = Some(receipt.request_id.clone());
                    error.outcome = receipt.outcome;
                }
            }
            if !stderr {
                let _ = tokio::time::timeout(deadline, finite(error.value(), true)).await;
            }
            8
        }
    }
}

fn job_result(
    result: Result<Result<serde_json::Value, CliError>, tokio::task::JoinError>,
) -> Result<serde_json::Value, CliError> {
    result.unwrap_or_else(|_| Err(CliError::new("internal", 1, Outcome::Unknown, false)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn dependency_panic_is_joined_and_projected_without_private_panic_payload() {
        let task = tokio::spawn(async {
            panic!("private-sentinel token");
            #[allow(unreachable_code)]
            Ok(serde_json::json!({}))
        });
        let settled = tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .unwrap();
        let error = job_result(settled).err().unwrap();
        assert_eq!(error.exit, 1);
        assert_eq!(error.outcome, Outcome::Unknown);
        assert!(!error.value().to_string().contains("private-sentinel"));
        assert_eq!(error.value()["error"]["code"], "internal");
    }
    #[tokio::test]
    async fn panic_subprocess_helper() {
        if std::env::var_os("AW_TEST_PANIC_HELPER").is_none() {
            return;
        }
        install_panic_hook();
        let task = tokio::spawn(async {
            panic!("private-sentinel token");
            #[allow(unreachable_code)]
            Ok(serde_json::json!({}))
        });
        let error = job_result(task.await).err().unwrap();
        assert!(finite(error.value(), true).await.is_ok());
        std::process::exit(error.exit as i32);
    }
    #[tokio::test]
    async fn production_hook_in_isolated_panic_process_emits_one_safe_json_and_exit_one() {
        use tokio::io::AsyncReadExt;
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::panic_subprocess_helper",
                "--nocapture",
                "--quiet",
            ])
            .env("AW_TEST_PANIC_HELPER", "1") // test binary only, never read by production
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut stdout = child.stdout.take().unwrap().take(16 * 1024 + 1);
        let mut stderr = child.stderr.take().unwrap().take(16 * 1024 + 1);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            tokio::join!(
                child.wait(),
                stdout.read_to_end(&mut out),
                stderr.read_to_end(&mut err)
            )
        })
        .await;
        let (status, out_read, err_read) = match result {
            Ok(result) => result,
            Err(_) => {
                child.start_kill().ok();
                tokio::time::timeout(std::time::Duration::from_secs(1), child.wait())
                    .await
                    .unwrap()
                    .unwrap();
                panic!("panic helper timeout after kill/reap");
            }
        };
        out_read.unwrap();
        err_read.unwrap();
        assert!(out.len() <= 16 * 1024 && err.len() <= 16 * 1024);
        assert_eq!(status.unwrap().code(), Some(1));
        assert!(!String::from_utf8_lossy(&out).contains("private-sentinel"));
        assert!(!String::from_utf8_lossy(&err).contains("private-sentinel"));
        assert_eq!(err.iter().filter(|b| **b == b'\n').count(), 1);
        let json: serde_json::Value = serde_json::from_slice(&err).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["error"]["code"], "internal");
        // libtest's banner is on stdout; application JSON/log output is absent there.
        assert!(!out.contains(&b'{'));
    }
}
