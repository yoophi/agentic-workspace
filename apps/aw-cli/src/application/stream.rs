//! Generic watch composition. Process signals cancel local ownership, never a server operation.
use crate::{
    inbound::{read_input, Command, Options, WatchInput},
    infrastructure::{
        event_source::HttpEventSource,
        output::{finite, stream::JsonlOutput, CliError},
        stdout::StdoutWriter,
    },
};
use futures_util::FutureExt;
use std::{
    panic::AssertUnwindSafe,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::signal::unix::Signal;
use workbench_client::{
    application::{
        admission::{admit_stream, CallerProfile},
        events::session::EventSession,
    },
    infrastructure::locator::read_descriptor,
    ports::ClientError,
};
use workbench_protocol::{workbench::StreamCursor, Outcome};
async fn prepare(options: &Options) -> Result<(StreamCursor, Arc<HttpEventSource>), CliError> {
    let (input, run, after) = match &options.command {
        Command::Watch { input, run, after } => (input, run, after),
        _ => return Err(CliError::usage()),
    };
    // Explicit and generic run watches enter the same closed stream admission.
    let cursor = if let Some(run) = run {
        let stream = format!("run:{run}");
        admit_stream(&stream, CallerProfile::Owner).map_err(CliError::from_client)?;
        StreamCursor {
            stream_id: stream,
            epoch: String::new(),
            after_sequence: *after,
        }
    } else {
        let input = input.as_deref().ok_or_else(CliError::usage)?;
        let watch: WatchInput = serde_json::from_value(read_input(input, &options.limits).await?)
            .map_err(|_| CliError::usage())?;
        if watch.epoch.is_empty() {
            return Err(CliError::usage());
        }
        admit_stream(&watch.stream_id, CallerProfile::Owner).map_err(CliError::from_client)?;
        StreamCursor {
            stream_id: watch.stream_id,
            epoch: watch.epoch,
            after_sequence: watch.after_sequence,
        }
    };
    let descriptor = options
        .descriptor
        .as_ref()
        .ok_or_else(|| CliError::from_client(ClientError::Unavailable))?;
    let endpoint =
        Arc::new(read_descriptor(descriptor, CallerProfile::Owner).map_err(CliError::from_client)?);
    let cursor = if run.is_some() {
        StreamCursor {
            epoch: endpoint.identity().epoch().into(),
            ..cursor
        }
    } else {
        cursor
    };
    if cursor.epoch != endpoint.identity().epoch() {
        return Err(CliError::from_client(ClientError::Incompatible));
    }
    Ok((
        cursor,
        Arc::new(HttpEventSource::new(endpoint, options.limits.clone())),
    ))
}
async fn preflight_error(error: CliError, deadline: Duration) -> u8 {
    let code = error.exit;
    if matches!(
        tokio::time::timeout(deadline, finite(error.value(), true)).await,
        Ok(Ok(()))
    ) {
        code
    } else {
        8
    }
}
pub async fn run(options: Options, interrupt: &mut Signal) -> u8 {
    let deadline = options.limits.config().request_timeout;
    let prepared = tokio::select! { biased;
        _=interrupt.recv()=>Err(CliError::from_client(ClientError::Cancelled)),
        result=tokio::time::timeout(deadline, AssertUnwindSafe(prepare(&options)).catch_unwind())=>match result { Ok(Ok(result))=>result, Ok(Err(_))=>Err(CliError::new("internal",1,Outcome::NotApplied,false)), Err(_)=>Err(CliError::from_client(ClientError::Deadline)) },
    };
    let (cursor, source) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return preflight_error(error, deadline).await,
    };
    let writer = match StdoutWriter::stdout() {
        Ok(writer) => writer,
        Err(_) => {
            return preflight_error(
                CliError::new("outputUnavailable", 8, Outcome::NotApplied, false),
                deadline,
            )
            .await
        }
    };
    let output = JsonlOutput::new(writer, options.limits.clone());
    let mut session = match EventSession::new(cursor.clone(), source, options.limits.clone()) {
        Ok(session) => session,
        Err(error) => return preflight_error(CliError::from_client(error), deadline).await,
    };
    if let Err(error) = session.subscribe(cursor.after_sequence, Box::new(output.consumer())) {
        return preflight_error(CliError::from_client(error), deadline).await;
    }
    let progress = session.progress();
    let stopped = Arc::new(AtomicU8::new(0));
    let cause = stopped.clone();
    let stop = async {
        tokio::select! { biased;
            _=interrupt.recv()=>cause.store(130,Ordering::SeqCst),
            _=output.failure()=>cause.store(8,Ordering::SeqCst),
        }
    };
    let (final_cursor, result) = match AssertUnwindSafe(session.run(stop)).catch_unwind().await {
        Ok(result) => (
            result.cursor,
            match result.result {
                Ok(()) => result.cleanup_error.map_or(Ok(()), Err),
                Err(error) => Err(error),
            },
        ),
        Err(_) => (
            progress.cursor().unwrap_or(cursor),
            Err(ClientError::Protocol),
        ),
    };
    let signal = stopped.load(Ordering::SeqCst) == 130;
    let error = if signal {
        Some(CliError::from_client(ClientError::Cancelled))
    } else if output.failed() {
        Some(CliError::new(
            "outputUnavailable",
            8,
            Outcome::Unknown,
            false,
        ))
    } else {
        result.err().map(CliError::from_client)
    };
    let code = error.as_ref().map_or(0, |error| error.exit);
    if !output.opened() {
        return preflight_error(
            error.unwrap_or_else(|| CliError::new("internal", 1, Outcome::NotApplied, false)),
            deadline,
        )
        .await;
    }
    // A full pipe can prevent an end record. Retire the writer after a bounded attempt.
    let finished = tokio::time::timeout(
        deadline.min(Duration::from_millis(250)),
        output.finish(&final_cursor, error.as_ref()),
    )
    .await;
    if matches!(finished, Ok(Ok(()))) {
        return code;
    }
    if !signal {
        let _ = tokio::time::timeout(
            deadline.min(Duration::from_millis(250)),
            finite(
                CliError::new("outputUnavailable", 8, Outcome::Unknown, false).value(),
                true,
            ),
        )
        .await;
    }
    if signal {
        130
    } else {
        8
    }
}
