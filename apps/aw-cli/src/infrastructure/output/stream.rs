//! Full JSONL writes are consumer completion; cancelled partial writes retire the sink.
use super::CliError;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    sync::Mutex,
};
use workbench_client::{
    domain::limits::{Limits, Resource},
    ports::{ClientError, EventConsumer, Snapshot},
};
use workbench_protocol::workbench::{EventEnvelope, StreamCursor};
struct Flags {
    opened: AtomicBool,
    failed: AtomicBool,
    ended: AtomicBool,
}
pub struct JsonlOutput<W> {
    writer: Arc<Mutex<W>>,
    flags: Arc<Flags>,
    limits: Limits,
}
impl<W> Clone for JsonlOutput<W> {
    fn clone(&self) -> Self {
        Self {
            writer: self.writer.clone(),
            flags: self.flags.clone(),
            limits: self.limits.clone(),
        }
    }
}
struct Flight {
    flags: Arc<Flags>,
    complete: bool,
}
impl Drop for Flight {
    fn drop(&mut self) {
        if !self.complete {
            self.flags.failed.store(true, Ordering::SeqCst);
        }
    }
}
impl<W: AsyncWrite + Unpin + Send> JsonlOutput<W> {
    pub fn new(writer: W, limits: Limits) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            flags: Arc::new(Flags {
                opened: AtomicBool::new(false),
                failed: AtomicBool::new(false),
                ended: AtomicBool::new(false),
            }),
            limits,
        }
    }
    pub fn opened(&self) -> bool {
        self.flags.opened.load(Ordering::SeqCst)
    }
    pub fn failed(&self) -> bool {
        self.flags.failed.load(Ordering::SeqCst)
    }
    pub fn consumer(&self) -> JsonlConsumer<W> {
        JsonlConsumer(self.clone())
    }
    async fn write(&self, value: Value, end: bool) -> Result<(), ClientError> {
        let mut bytes = serde_json::to_vec(&value).map_err(|_| ClientError::Protocol)?;
        bytes.push(b'\n');
        self.limits.check_add(Resource::Body, 0, bytes.len())?;
        let mut writer = self.writer.lock().await;
        if self.failed() || self.flags.ended.load(Ordering::SeqCst) {
            return Err(ClientError::Unavailable);
        }
        let mut flight = Flight {
            flags: self.flags.clone(),
            complete: false,
        };
        writer
            .write_all(&bytes)
            .await
            .map_err(|_| ClientError::Unavailable)?;
        writer.flush().await.map_err(|_| ClientError::Unavailable)?;
        if end {
            self.flags.ended.store(true, Ordering::SeqCst);
        }
        flight.complete = true;
        Ok(())
    }
    pub async fn finish(
        &self,
        cursor: &StreamCursor,
        error: Option<&CliError>,
    ) -> Result<(), ClientError> {
        if !self.opened() {
            return Err(ClientError::Protocol);
        }
        let mut value = json!({"type":"stream.end","cursor":cursor,"ok":error.is_none()});
        if let Some(error) = error {
            value["error"] = error.value()["error"].clone();
        }
        self.write(value, true).await
    }
}
pub struct JsonlConsumer<W>(JsonlOutput<W>);
#[async_trait]
impl<W: AsyncWrite + Unpin + Send + 'static> EventConsumer for JsonlConsumer<W> {
    async fn opened(&mut self, cursor: &StreamCursor) -> Result<(), ClientError> {
        if self.0.opened() {
            return Err(ClientError::Protocol);
        }
        self.0
            .write(json!({"type":"stream.open","cursor":cursor}), false)
            .await?;
        self.0.flags.opened.store(true, Ordering::SeqCst);
        Ok(())
    }
    async fn consume(&mut self, event: &EventEnvelope) -> Result<(), ClientError> {
        if !self.0.opened() {
            return Err(ClientError::Protocol);
        }
        self.0
            .write(json!({"type":"event","event":event}), false)
            .await
    }
    async fn reset(&mut self, snapshot: &Snapshot) -> Result<(), ClientError> {
        self.reset_from(snapshot, &snapshot.cursor).await
    }
    async fn reset_from(
        &mut self,
        snapshot: &Snapshot,
        applied: &StreamCursor,
    ) -> Result<(), ClientError> {
        if !self.0.opened() {
            return Err(ClientError::Protocol);
        }
        self.0.write(json!({"type":"stream.reset","cursor":snapshot.cursor,"applied":applied,"snapshot":snapshot.value}), false).await
    }
}
