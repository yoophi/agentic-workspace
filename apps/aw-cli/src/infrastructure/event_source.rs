//! Read-only snapshot construction. Stream binding IDs are resolved from server DTOs, never guessed.
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use workbench_client::{
    application::admission::{admit_stream, CallerProfile},
    domain::limits::{Limits, Resource},
    infrastructure::{
        http::HttpConnection, locator::LocatedEndpoint, websocket::WebSocketConnection,
    },
    ports::{CallTransport, ClientError, EventSocket, EventSource, Snapshot, SnapshotPort},
};
use workbench_protocol::{
    events::{parse_stream_id, StreamKind},
    operations::{bench::BenchSummaryDto, orchestration_dto::OrchestrationSessionDto},
    workbench::StreamCursor,
    CallRequest, OperationId,
};
pub struct HttpEventSource {
    endpoint: Arc<LocatedEndpoint>,
    limits: Limits,
}
impl HttpEventSource {
    pub fn new(endpoint: Arc<LocatedEndpoint>, limits: Limits) -> Self {
        Self { endpoint, limits }
    }
}
#[async_trait]
impl EventSource for HttpEventSource {
    async fn connect(&self, cursor: &StreamCursor) -> Result<Box<dyn EventSocket>, ClientError> {
        Ok(Box::new(
            WebSocketConnection::connect(
                self.endpoint.clone(),
                self.limits.clone(),
                vec![cursor.clone()],
            )
            .await?,
        ))
    }
    fn snapshot_port(&self) -> Box<dyn SnapshotPort> {
        Box::new(HttpSnapshot {
            endpoint: self.endpoint.clone(),
            limits: self.limits.clone(),
        })
    }
}
struct HttpSnapshot {
    endpoint: Arc<LocatedEndpoint>,
    limits: Limits,
}
async fn query(
    connection: &mut HttpConnection,
    operation: OperationId,
    input: Value,
) -> Result<Value, ClientError> {
    let reply = connection
        .call(&CallRequest::query(operation, input))
        .await?;
    reply.output().cloned().ok_or(ClientError::Protocol)
}
#[async_trait]
impl SnapshotPort for HttpSnapshot {
    async fn snapshot(&mut self, applied: &StreamCursor) -> Result<Snapshot, ClientError> {
        self.snapshot_after(applied, applied).await
    }
    async fn snapshot_after(
        &mut self,
        applied: &StreamCursor,
        live: &StreamCursor,
    ) -> Result<Snapshot, ClientError> {
        admit_stream(&live.stream_id, CallerProfile::Owner)?;
        if applied.stream_id != live.stream_id
            || applied.epoch != live.epoch
            || live.epoch != self.endpoint.identity().epoch()
        {
            return Err(ClientError::Incompatible);
        }
        let (kind, key) = parse_stream_id(&live.stream_id).ok_or(ClientError::InvalidInput)?;
        let mut connection =
            HttpConnection::connect(self.endpoint.clone(), self.limits.clone()).await?;
        let result = async {
            let benches: Vec<BenchSummaryDto> = serde_json::from_value(
                query(&mut connection, OperationId::BenchList, json!({})).await?,
            )
            .map_err(|_| ClientError::Protocol)?;
            self.limits
                .check_add(Resource::QueueItems, 0, benches.len())?;
            let value = match kind {
                StreamKind::Bench => {
                    let owners = benches
                        .into_iter()
                        .filter(|b| b.bench_id == key)
                        .collect::<Vec<_>>();
                    let bench = match owners.as_slice() {
                        [bench] => bench,
                        [] => return Err(ClientError::Unavailable),
                        _ => return Err(ClientError::Protocol),
                    };
                    // Title requests are not retained state. The reset explicitly exposes that loss.
                    json!({"bench":bench,"notificationsRetained":false})
                }
                StreamKind::Orchestration => {
                    let mut found = None;
                    for bench in benches {
                        let value = query(
                            &mut connection,
                            OperationId::OrchestrationGet,
                            json!({"benchId":bench.bench_id}),
                        )
                        .await?;
                        let session: Option<OrchestrationSessionDto> =
                            serde_json::from_value(value.clone())
                                .map_err(|_| ClientError::Protocol)?;
                        if session.as_ref().is_some_and(|s| {
                            s.event_stream_id.as_deref() == Some(live.stream_id.as_str())
                        }) {
                            if found.is_some() {
                                return Err(ClientError::Protocol);
                            }
                            found = Some(value);
                        }
                    }
                    found.ok_or(ClientError::Unavailable)?
                }
                _ => return Err(ClientError::PrerequisiteUnavailable),
            };
            // Reads start after verified live hello. The current state covers that minimum
            // boundary; no sequence is invented from workspace revision, and later live
            // events (including the same revision) remain eligible for delivery.
            Ok(Snapshot {
                cursor: StreamCursor {
                    after_sequence: live.after_sequence.max(applied.after_sequence),
                    ..live.clone()
                },
                value,
            })
        }
        .await;
        let close = connection.close().await;
        match result {
            Ok(snapshot) => {
                close?;
                Ok(snapshot)
            }
            Err(error) => Err(error),
        }
    }
}
