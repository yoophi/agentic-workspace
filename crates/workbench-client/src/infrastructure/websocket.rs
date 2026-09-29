//! Ticket authentication on an owned HTTP socket; fresh proof on the separate upgrade socket.
use crate::{
    application::admission::{admit_stream, CallerProfile},
    domain::limits::{LimitError, Limits, Resource},
    infrastructure::{http::HttpConnection, locator::LocatedEndpoint},
    ports::{CallTransport, ClientError, Credential},
};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::Request;
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::Arc};
use tokio::time::timeout;
use tokio_tungstenite::{
    tungstenite::{
        handshake::client::generate_key,
        protocol::{Role, WebSocketConfig},
        Error, Message,
    },
    WebSocketStream,
};
use workbench_protocol::{events::EventFrame, workbench::StreamCursor, PROTOCOL_VERSION};

mod frame_io;
use frame_io::FrameIo;
type Socket = WebSocketStream<FrameIo<TokioIo<hyper::upgrade::Upgraded>>>;
pub struct WebSocketConnection {
    socket: Option<Socket>,
    limits: Limits,
    cursors: BTreeMap<String, StreamCursor>,
}
impl std::fmt::Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebSocketConnection([redacted])")
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ticket {
    ticket: String,
    expires_at: String,
}
impl WebSocketConnection {
    pub async fn connect(
        endpoint: Arc<LocatedEndpoint>,
        limits: Limits,
        cursors: Vec<StreamCursor>,
    ) -> Result<Self, ClientError> {
        let mut requested = BTreeMap::new();
        if cursors.is_empty() {
            return Err(ClientError::InvalidInput);
        }
        limits.check_add(Resource::QueueItems, 0, cursors.len())?;
        limits.check_add(
            Resource::Input,
            0,
            serde_json::to_vec(&cursors)
                .map_err(|_| ClientError::InvalidInput)?
                .len(),
        )?;
        for cursor in &cursors {
            admit_stream(&cursor.stream_id, CallerProfile::Owner)?;
            if cursor.epoch != endpoint.identity().epoch()
                || requested
                    .insert(cursor.stream_id.clone(), cursor.clone())
                    .is_some()
            {
                return Err(ClientError::InvalidInput);
            }
        }
        timeout(
            limits.config().request_timeout,
            Self::connect_inner(endpoint, limits.clone(), cursors, requested),
        )
        .await
        .map_err(|_| ClientError::Deadline)?
    }
    async fn connect_inner(
        endpoint: Arc<LocatedEndpoint>,
        limits: Limits,
        cursors: Vec<StreamCursor>,
        requested: BTreeMap<String, StreamCursor>,
    ) -> Result<Self, ClientError> {
        let mut http = HttpConnection::connect(endpoint.clone(), limits.clone()).await?;
        let (status, value) = http
            .json_request("/v1/event-tickets", json!({"cursors":cursors}), true)
            .await?;
        let ticket: Ticket = if status == 200 {
            serde_json::from_value(value).map_err(|_| ClientError::Protocol)?
        } else {
            return Err(ClientError::Fault(
                crate::infrastructure::http::decode_fault(status, value)?,
            ));
        };
        if ticket.expires_at.is_empty() || ticket.expires_at.len() > 128 {
            return Err(ClientError::Protocol);
        }
        let ticket = Credential::new(ticket.ticket).map_err(|_| ClientError::Protocol)?;
        // No reconnect is hidden inside the sender. Retire ticket socket before the new proof.
        http.close().await?;
        let proven = HttpConnection::prove(endpoint.clone(), limits.clone()).await?;
        let key = generate_key();
        let request = Request::builder()
            .method("GET")
            .uri(format!(
                "/v1/events?ticket={}",
                encode_query(ticket.expose())
            ))
            .header("host", endpoint.address().to_string())
            .header("upgrade", "websocket")
            .header("connection", "Upgrade")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", key)
            .body(Full::new(Bytes::new()))
            .map_err(|_| ClientError::Protocol)?;
        let upgraded = proven.upgrade(request).await?;
        let config = WebSocketConfig {
            max_message_size: Some(limits.maximum(Resource::Message)),
            max_frame_size: Some(limits.maximum(Resource::Frame)),
            ..Default::default()
        };
        let socket = WebSocketStream::from_raw_socket(
            FrameIo::new(
                TokioIo::new(upgraded),
                limits.config().request_timeout,
                limits.maximum(Resource::Frame),
            ),
            Role::Client,
            Some(config),
        )
        .await;
        let mut caller = Self {
            socket: Some(socket),
            limits,
            cursors: requested,
        };
        // The hello follows server subscription registration, before event/gap/fault.
        let frame = caller.read().await?.ok_or(ClientError::Unavailable)?;
        match frame {
            EventFrame::Hello {
                protocol_version,
                epoch,
            } if protocol_version == PROTOCOL_VERSION && epoch == endpoint.identity().epoch() => {
                Ok(caller)
            }
            EventFrame::Hello { .. } => Err(ClientError::Incompatible),
            _ => Err(ClientError::Protocol),
        }
    }
    async fn read(&mut self) -> Result<Option<EventFrame>, ClientError> {
        // Taking the owned socket into this future makes external cancellation retire it.
        let mut socket = self.socket.take().ok_or(ClientError::Unavailable)?;
        loop {
            let message = match socket.next().await {
                None => return Ok(None),
                Some(Err(e)) => return Err(map_error(e)),
                Some(Ok(m)) => m,
            };
            match message {
                Message::Text(text) => {
                    socket.get_mut().message_complete();
                    self.limits.check_add(Resource::Message, 0, text.len())?;
                    let frame = decode_frame(&text, &self.cursors)?;
                    self.socket = Some(socket);
                    return Ok(Some(frame));
                }
                Message::Ping(_) | Message::Pong(_) => {
                    socket.get_mut().message_complete();
                    timeout(self.limits.config().request_timeout, socket.flush())
                        .await
                        .map_err(|_| ClientError::Deadline)?
                        .map_err(map_error)?;
                }
                Message::Close(_) => {
                    let _ = timeout(self.limits.config().connect_timeout, socket.flush()).await;
                    return Ok(None);
                }
                _ => return Err(ClientError::Protocol),
            }
        }
    }
    pub async fn next(&mut self) -> Result<Option<EventFrame>, ClientError> {
        let frame = self.read().await?;
        if matches!(
            frame,
            Some(EventFrame::Hello { .. } | EventFrame::Subscribe { .. })
        ) {
            self.socket = None;
            return Err(ClientError::Protocol);
        }
        Ok(frame)
    }
    pub async fn close(&mut self) -> Result<(), ClientError> {
        if let Some(mut socket) = self.socket.take() {
            timeout(self.limits.config().connect_timeout, socket.close(None))
                .await
                .map_err(|_| ClientError::Deadline)?
                .map_err(map_error)?;
        }
        Ok(())
    }
}
fn map_error(error: Error) -> ClientError {
    match error {
        Error::Capacity(_) => ClientError::Limit(LimitError::Exceeded(Resource::Message)),
        Error::Io(e)
            if e.get_ref()
                .is_some_and(|inner| inner.is::<frame_io::FrameQuota>()) =>
        {
            ClientError::Limit(LimitError::Exceeded(Resource::Frame))
        }
        Error::Io(e) if e.kind() == std::io::ErrorKind::TimedOut => ClientError::Deadline,
        Error::Io(_) | Error::ConnectionClosed | Error::AlreadyClosed => ClientError::Unavailable,
        _ => ClientError::Protocol,
    }
}
fn decode_frame(
    text: &str,
    cursors: &BTreeMap<String, StreamCursor>,
) -> Result<EventFrame, ClientError> {
    let value: Value = serde_json::from_str(text).map_err(|_| ClientError::Protocol)?;
    let object = value.as_object().ok_or(ClientError::Protocol)?;
    let fields: &[&str] = match value["type"].as_str() {
        Some("hello") => &["type", "protocolVersion", "epoch"],
        Some("event") => &["type", "event"],
        Some("gap") => &[
            "type",
            "streamId",
            "epoch",
            "reason",
            "firstSequence",
            "lastSequence",
        ],
        Some("fault") => &["type", "fault"],
        _ => return Err(ClientError::Protocol),
    };
    if object.keys().any(|k| !fields.contains(&k.as_str())) {
        return Err(ClientError::Protocol);
    }
    let frame: EventFrame = serde_json::from_value(value).map_err(|_| ClientError::Protocol)?;
    match &frame {
        EventFrame::Event { event } => {
            let cursor = cursors.get(&event.stream_id).ok_or(ClientError::Protocol)?;
            if event.epoch != cursor.epoch
                || event.sequence == 0
                || event.event_id.is_empty()
                || event.occurred_at.is_empty()
            {
                return Err(ClientError::Protocol);
            }
            validate_body(event)?;
        }
        EventFrame::Gap { gap }
            if !cursors.contains_key(&gap.stream_id)
                || gap.epoch.is_empty()
                || gap
                    .first_sequence
                    .zip(gap.last_sequence)
                    .is_some_and(|(a, b)| a > b) =>
        {
            return Err(ClientError::Protocol);
        }
        _ => {}
    }
    Ok(frame)
}
fn validate_body(event: &workbench_protocol::workbench::EventEnvelope) -> Result<(), ClientError> {
    use workbench_protocol::events::*;
    let kind = parse_stream_id(&event.stream_id)
        .ok_or(ClientError::Protocol)?
        .0;
    if !EVENT_SCHEMAS
        .iter()
        .any(|s| s.schema == event.schema && s.stream_kind == kind)
    {
        return Err(ClientError::Protocol);
    }
    let body = event.body.clone();
    match event.schema.as_str() {
        RUN_EVENT_V1 => serde_json::from_value::<run::RunEventDto>(body).map(|_| ()),
        WORKTREE_CHANGED_V1 => {
            serde_json::from_value::<worktree::WorktreeChangedDto>(body).map(|_| ())
        }
        ORCHESTRATION_WORKSPACE_UPDATED_V1 => {
            serde_json::from_value::<orchestration::OrchestrationEventDto>(body).map(|_| ())
        }
        EXCHANGE_REQUESTED_V1 => {
            serde_json::from_value::<exchange::ExchangeRequestedDto>(body).map(|_| ())
        }
        EXCHANGE_STATUS_V1 => serde_json::from_value::<
            workbench_protocol::operations::exchange::AgentExchangeDto,
        >(body)
        .map(|_| ()),
        BENCH_TITLE_REQUESTED_V1 => {
            serde_json::from_value::<bench::TitleRequestedDto>(body).map(|_| ())
        }
        _ => return Err(ClientError::Protocol),
    }
    .map_err(|_| ClientError::Protocol)
}

fn encode_query(secret: &str) -> String {
    let mut encoded = String::new();
    for byte in secret.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(&mut encoded, "%{byte:02X}");
        }
    }
    encoded
}
