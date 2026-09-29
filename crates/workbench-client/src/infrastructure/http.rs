//! One owned HTTP/1 connection. No pool, proxy, redirects or implicit reconnect.
use crate::{
    domain::{
        attempt::EndpointIdentity,
        limits::{Limits, Resource},
    },
    infrastructure::{
        identity::verify_proof,
        locator::{LocatedEndpoint, STORAGE_SCHEMA_VERSION},
    },
    ports::{CallTransport, ClientError, CredentialProvider},
};
use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{body::Incoming, client::conn::http1::SendRequest, Request, Response};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{net::TcpStream, task::JoinHandle, time::timeout};
use workbench_protocol::{CallReply, CallRequest, WorkbenchFault, PROTOCOL_VERSION};

pub struct HttpConnection {
    sender: SendRequest<Full<Bytes>>,
    driver: Option<JoinHandle<()>>,
    usable: bool,
    endpoint: Arc<LocatedEndpoint>,
    limits: Limits,
}
impl Drop for HttpConnection {
    fn drop(&mut self) {
        if let Some(driver) = &self.driver {
            driver.abort();
        }
    }
}
impl HttpConnection {
    pub async fn connect(
        endpoint: Arc<LocatedEndpoint>,
        limits: Limits,
    ) -> Result<Self, ClientError> {
        timeout(
            limits.config().request_timeout,
            Self::connect_inner(endpoint, limits.clone()),
        )
        .await
        .map_err(|_| ClientError::Deadline)?
    }
    async fn connect_inner(
        endpoint: Arc<LocatedEndpoint>,
        limits: Limits,
    ) -> Result<Self, ClientError> {
        let socket = timeout(
            limits.config().connect_timeout,
            TcpStream::connect(endpoint.address()),
        )
        .await
        .map_err(|_| ClientError::Deadline)?
        .map_err(|_| ClientError::Unavailable)?;
        let mut builder = hyper::client::conn::http1::Builder::new();
        builder.max_headers(64).max_buf_size(16384);
        let (sender, connection) = builder
            .handshake(TokioIo::new(socket))
            .await
            .map_err(|_| ClientError::Unavailable)?;
        let driver = tokio::spawn(async move {
            let _ = connection.with_upgrades().await;
        });
        let mut caller = Self {
            sender,
            driver: Some(driver),
            usable: true,
            endpoint,
            limits,
        };
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let (status, identity) = caller
            .json_request("/v1/system/identify", json!({"nonce":nonce}), false)
            .await?;
        if status != 200
            || identity["instanceId"].as_str() != Some(caller.endpoint.identity().instance())
        {
            return Err(ClientError::Identity);
        }
        verify_proof(
            caller.endpoint.credential(),
            &nonce,
            caller.endpoint.identity().instance(),
            identity["proof"].as_str().ok_or(ClientError::Identity)?,
        )?;
        // Every credential-bearing request uses this exact sender; failed proof drops it.
        let (status,handshake)=caller.json_request("/v1/system/handshake",json!({"supportedProtocolVersions":[PROTOCOL_VERSION],"client":{"name":"aw-rust-client","version":env!("CARGO_PKG_VERSION")}}),true).await?;
        if status != 200
            || handshake["instanceId"].as_str() != Some(caller.endpoint.identity().instance())
            || handshake["serverEpoch"].as_str() != Some(caller.endpoint.identity().epoch())
            || handshake["selectedProtocolVersion"].as_u64() != Some(PROTOCOL_VERSION as u64)
            || handshake["apiMajor"].as_u64() != Some(1)
            || handshake["storageSchemaVersion"].as_i64() != Some(STORAGE_SCHEMA_VERSION)
            || handshake["state"].as_str() != Some("serving")
        {
            return Err(ClientError::Incompatible);
        }
        Ok(caller)
    }
    pub(crate) async fn json_request(
        &mut self,
        path: &str,
        body: Value,
        authenticated: bool,
    ) -> Result<(u16, Value), ClientError> {
        let bytes = serde_json::to_vec(&body).map_err(|_| ClientError::Protocol)?;
        self.limits.check_add(Resource::Input, 0, bytes.len())?;
        let mut builder = Request::builder()
            .method("POST")
            .uri(path)
            .header("host", self.endpoint.address().to_string())
            .header("content-type", "application/json")
            .header("accept", "application/json");
        if authenticated {
            builder = builder.header(
                "authorization",
                format!("Bearer {}", self.endpoint.credential().expose()),
            );
        }
        let request = builder
            .body(Full::new(Bytes::from(bytes)))
            .map_err(|_| ClientError::Protocol)?;
        let response = self
            .sender
            .send_request(request)
            .await
            .map_err(|_| ClientError::TransportUnknown)?;
        let status = response.status().as_u16();
        let body = read_response(response, &self.limits).await?;
        Ok((
            status,
            serde_json::from_slice(&body).map_err(|_| ClientError::Protocol)?,
        ))
    }
}
pub(crate) async fn read_response(
    response: Response<Incoming>,
    limits: &Limits,
) -> Result<Vec<u8>, ClientError> {
    if let Some(length) = response.headers().get("content-length") {
        let length = length
            .to_str()
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or(ClientError::Protocol)?;
        limits.check_add(Resource::Body, 0, length)?;
    }
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| ClientError::TransportUnknown)?;
        if let Ok(data) = frame.into_data() {
            limits.check_add(Resource::Body, bytes.len(), data.len())?;
            bytes.extend_from_slice(&data);
        }
    }
    Ok(bytes)
}

pub fn decode_reply(
    status: u16,
    value: Value,
    request: &CallRequest,
) -> Result<CallReply, ClientError> {
    if status == 200 {
        let object = value.as_object().ok_or(ClientError::Protocol)?;
        match value["kind"].as_str() {
            Some("complete")
                if object.contains_key("output")
                    && object.keys().all(|k| {
                        matches!(k.as_str(), "kind" | "output" | "revision" | "replayed")
                    }) => {}
            Some("accepted")
                if value["executionId"]
                    .as_str()
                    .is_some_and(|id| !id.is_empty())
                    && object
                        .keys()
                        .all(|k| matches!(k.as_str(), "kind" | "executionId" | "revision")) => {}
            _ => return Err(ClientError::Protocol),
        }
        return serde_json::from_value(value).map_err(|_| ClientError::Protocol);
    }
    if !(400..=599).contains(&status) {
        return Err(ClientError::Protocol);
    }
    let fault: WorkbenchFault =
        serde_json::from_value(value.clone()).map_err(|_| ClientError::Protocol)?;
    if fault.request_id != request.request_id
        || fault.code.http_status() != status
        || value["status"].as_u64() != Some(status as u64)
    {
        return Err(ClientError::Protocol);
    }
    Err(ClientError::Fault(fault))
}
// Dropping just a call future must also retire its socket: no late reply can be
// reused by a new attempt through the retained connection object.
struct Flight<'a> {
    connection: &'a mut HttpConnection,
    resolved: bool,
}
impl Drop for Flight<'_> {
    fn drop(&mut self) {
        if !self.resolved {
            self.connection.usable = false;
            if let Some(driver) = &self.connection.driver {
                driver.abort();
            }
        }
    }
}
#[async_trait]
impl CallTransport for HttpConnection {
    fn identity(&self) -> &EndpointIdentity {
        self.endpoint.identity()
    }
    async fn call(&mut self, request: &CallRequest) -> Result<CallReply, ClientError> {
        crate::application::call::validate_request(request)?;
        if !self.usable {
            return Err(ClientError::Unavailable);
        }
        let limits = self.limits.clone();
        let mut flight = Flight {
            connection: self,
            resolved: false,
        };
        let result = timeout(limits.config().request_timeout, async {
            let (status, value) = flight
                .connection
                .json_request(
                    "/v1/calls",
                    serde_json::to_value(request).map_err(|_| ClientError::Protocol)?,
                    true,
                )
                .await?;
            decode_reply(status, value, request)
        })
        .await
        .unwrap_or(Err(ClientError::Deadline));
        if result.is_ok() || matches!(result, Err(ClientError::Fault(_))) {
            flight.resolved = true;
        }
        result
    }
    async fn close(&mut self) -> Result<(), ClientError> {
        self.usable = false;
        if let Some(driver) = self.driver.take() {
            driver.abort();
            timeout(self.limits.config().connect_timeout, driver)
                .await
                .map_err(|_| ClientError::Deadline)?
                .ok();
        }
        Ok(())
    }
}
