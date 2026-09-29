//! Caller interfaces and signature types, without filesystem, HTTP or server runtime.
use crate::domain::{attempt::EndpointIdentity, limits::LimitError};
use async_trait::async_trait;
use std::fmt;
use workbench_protocol::{
    workbench::{EventEnvelope, StreamCursor},
    CallReply, CallRequest, WorkbenchFault,
};

pub struct Credential(String);
impl Credential {
    pub fn new(value: String) -> Result<Self, ClientError> {
        if value.is_empty()
            || value.len() > 4096
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~+/=".contains(&b))
        {
            return Err(ClientError::Identity);
        }
        Ok(Self(value))
    }
    /// For proof/auth adapters only. This type intentionally has no Serialize/Display.
    pub fn expose(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Credential([redacted])")
    }
}

pub enum ClientError {
    Unavailable,
    Identity,
    Incompatible,
    Protocol,
    InvalidInput,
    Deadline,
    Cancelled,
    TransportUnknown,
    Limit(LimitError),
    PrerequisiteUnavailable,
    Fault(WorkbenchFault),
    PrivateState,
    StaleGeneration,
}
impl fmt::Debug for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "server unavailable",
            Self::Identity => "endpoint identity rejected",
            Self::Incompatible => "endpoint incompatible",
            Self::Protocol => "invalid server protocol",
            Self::InvalidInput => "invalid operation input",
            Self::Deadline => "local deadline exceeded",
            Self::Cancelled => "local wait cancelled",
            Self::TransportUnknown => "transport lost; operation outcome unknown",
            Self::Limit(_) => "resource budget exceeded",
            Self::PrerequisiteUnavailable => "operation admission rejected",
            Self::Fault(_) => "server fault (private fields redacted)",
            Self::PrivateState => "private retry state rejected",
            Self::StaleGeneration => "stale completion rejected",
        })
    }
}
impl std::error::Error for ClientError {}
impl From<LimitError> for ClientError {
    fn from(value: LimitError) -> Self {
        Self::Limit(value)
    }
}

pub trait CredentialProvider: Send + Sync {
    fn credential(&self) -> &Credential;
}

/// Identity belongs to the owned, freshly proven connection. Implementations must not reconnect implicitly.
#[async_trait]
pub trait CallTransport: Send {
    fn identity(&self) -> &EndpointIdentity;
    async fn call(&mut self, request: &CallRequest) -> Result<CallReply, ClientError>;
    /// Settle all driver/socket ownership before returning.
    async fn close(&mut self) -> Result<(), ClientError>;
}

pub struct RetryRecord {
    pub request: CallRequest,
    pub endpoint: EndpointIdentity,
    pub generation: u64,
    pub outcome: workbench_protocol::Outcome,
    pub result: Option<Result<CallReply, WorkbenchFault>>,
}
impl fmt::Debug for RetryRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RetryRecord([redacted])")
    }
}
pub trait RetryStore: Send + Sync {
    /// Durable pre-send publish. Existing identity may not be replaced.
    fn publish(&self, record: &RetryRecord) -> Result<(), ClientError>;
    fn load(&self) -> Result<RetryRecord, ClientError>;
    /// Compare-and-swap under exclusive ownership across processes.
    fn complete(
        &self,
        generation: u64,
        result: Result<CallReply, WorkbenchFault>,
    ) -> Result<(), ClientError>;
}

pub struct Snapshot {
    pub cursor: StreamCursor,
    pub value: serde_json::Value,
}
#[async_trait]
pub trait SnapshotPort: Send {
    /// Return full state/journal coverage through the returned cursor, not a delta
    /// starting at the requested cursor: an earlier listener may join during loading.
    async fn snapshot(&mut self, cursor: &StreamCursor) -> Result<Snapshot, ClientError>;
    /// Live boundary is recovery coverage; actual applied remains the replay/reset origin.
    async fn snapshot_after(
        &mut self,
        applied: &StreamCursor,
        live: &StreamCursor,
    ) -> Result<Snapshot, ClientError> {
        let _ = live;
        self.snapshot(applied).await
    }
}
/// A freshly proven, hello-verified owned socket. Dropping it releases the connection.
#[async_trait]
pub trait EventSocket: Send {
    fn identity(&self) -> &EndpointIdentity;
    async fn next(&mut self)
        -> Result<Option<workbench_protocol::events::EventFrame>, ClientError>;
    async fn close(&mut self) -> Result<(), ClientError>;
}
/// Connection policy and snapshot construction remain adapters; sessions own their jobs.
#[async_trait]
pub trait EventSource: Send + Sync {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError>;
    /// Each load owns a fresh port, allowing cancellation without reusing a retired HTTP sender.
    fn snapshot_port(&self) -> Box<dyn SnapshotPort>;
}
#[async_trait]
pub trait EventConsumer: Send {
    /// Called after verified subscription hello, before this consumer receives events.
    async fn opened(&mut self, _cursor: &StreamCursor) -> Result<(), ClientError> {
        Ok(())
    }
    /// Success means the full event was applied/output, not just received.
    async fn consume(&mut self, event: &EventEnvelope) -> Result<(), ClientError>;
    async fn reset(&mut self, snapshot: &Snapshot) -> Result<(), ClientError>;
    /// Replay consumers can use their actual applied cursor; a gap boundary is not an ACK.
    async fn reset_from(
        &mut self,
        snapshot: &Snapshot,
        applied: &StreamCursor,
    ) -> Result<(), ClientError> {
        let _ = applied;
        self.reset(snapshot).await
    }
}
