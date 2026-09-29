//! CLI composition root; parsing, bounded lifetime, cancellation and safe output.
mod application;
mod inbound;
mod infrastructure;
use infrastructure::output::{finite, CliError};
use std::{
    process::ExitCode,
    sync::{Arc, Mutex},
};
use workbench_protocol::Outcome;
fn main() -> ExitCode {
    // Dependencies may panic with private protocol values: render a closed error in the parent.
    std::panic::set_hook(Box::new(|_| {}));
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
     result=&mut job=>result.unwrap_or_else(|_|Err(CliError::new("internal",1,Outcome::Unknown,false))),
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
