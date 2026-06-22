use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    process::ChildStdin,
    sync::{Mutex, oneshot},
};

use crate::{
    domain::events::RunEvent,
    infrastructure::acp::{client::AcpClient, util::RpcError},
    ports::{event_sink::RunEventSink, permission::PermissionDecisionPort},
};

type PendingMap = HashMap<u64, oneshot::Sender<std::result::Result<Value, RpcError>>>;

/// JSON-RPC 2.0 peer over the agent's stdin/stdout.
///
/// Owns outbound writes, an id counter, and the map of in-flight
/// requests so response frames read from stdout can be correlated
/// back to their waiting callers.
#[derive(Clone)]
pub struct RpcPeer {
    writer: Arc<Mutex<ChildStdin>>,
    pub(crate) pending: Arc<Mutex<PendingMap>>,
    next_id: Arc<Mutex<u64>>,
}

impl RpcPeer {
    pub fn new(stdin: ChildStdin) -> Self {
        Self {
            writer: Arc::new(Mutex::new(stdin)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn request(
        &self,
        method: &str,
        params: Value,
    ) -> std::result::Result<Value, RpcError> {
        let id = {
            let mut next_id = self.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let payload = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        if let Err(err) = self.send_value(&payload).await {
            let _ = self.pending.lock().await.remove(&id);
            return Err(RpcError {
                code: -32603,
                message: err.to_string(),
                data: None,
            });
        }
        match rx.await {
            Ok(result) => result,
            Err(err) => Err(RpcError {
                code: -32603,
                message: format!("connection closed while waiting for {method}: {err}"),
                data: None,
            }),
        }
    }

    pub async fn request_typed<R>(&self, request: R) -> std::result::Result<R::Response, RpcError>
    where
        R: JsonRpcRequest + Serialize,
    {
        let method = request.method().to_string();
        let params = serde_json::to_value(&request).map_err(|err| RpcError {
            code: -32603,
            message: err.to_string(),
            data: None,
        })?;
        let result = self.request(&method, params).await?;
        R::Response::from_value(&method, result).map_err(|err| RpcError {
            code: i32::from(err.code) as i64,
            message: err.message,
            data: err.data,
        })
    }

    pub(crate) async fn respond_ok(&self, id: Value, result: Value) -> Result<()> {
        self.send_value(&json!({"jsonrpc": "2.0", "id": id, "result": result}))
            .await
    }

    pub(crate) async fn respond_error(
        &self,
        id: Value,
        code: i64,
        message: &str,
        data: Option<Value>,
    ) -> Result<()> {
        let mut error = json!({"code": code, "message": message});
        if let Some(data) = data {
            error["data"] = data;
        }
        self.send_value(&json!({"jsonrpc": "2.0", "id": id, "error": error}))
            .await
    }

    pub(crate) async fn fail_pending(&self, message: impl Into<String>) {
        let message = message.into();
        let pending = {
            let mut pending = self.pending.lock().await;
            std::mem::take(&mut *pending)
        };
        for (_, tx) in pending {
            let _ = tx.send(Err(RpcError {
                code: -32603,
                message: message.clone(),
                data: None,
            }));
        }
    }

    async fn send_value(&self, value: &Value) -> Result<()> {
        let mut writer = self.writer.lock().await;
        let mut bytes = serde_json::to_vec(value)?;
        bytes.push(b'\n');
        writer.write_all(&bytes).await?;
        writer.flush().await?;
        Ok(())
    }
}

/// Drive the read side of the JSON-RPC connection.
///
/// Parses each newline-terminated message, dispatches incoming requests
/// to `AcpClient::handle_request`, incoming notifications to
/// `AcpClient::handle_notification`, and resolves outstanding request
/// responses on the `RpcPeer`. Exits cleanly on EOF or fails fast if
/// a message exceeds `limit` bytes.
pub async fn read_loop<R, S, P>(
    mut reader: BufReader<R>,
    peer: RpcPeer,
    client: Arc<AcpClient<S, P>>,
    limit: usize,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    S: RunEventSink,
    P: PermissionDecisionPort,
{
    loop {
        let mut bytes = Vec::new();
        let read = reader.read_until(b'\n', &mut bytes).await?;
        if read == 0 {
            peer.fail_pending("ACP connection closed").await;
            break;
        }
        if bytes.len() > limit {
            peer.fail_pending("ACP message exceeded stdio buffer limit")
                .await;
            bail!("ACP message exceeded stdio buffer limit of {limit} bytes");
        }
        let message: Value = match serde_json::from_slice(&bytes) {
            Ok(message) => message,
            Err(err) => {
                client.emit_raw(RunEvent::Diagnostic {
                    message: format!("failed to parse JSON-RPC message: {err}"),
                });
                continue;
            }
        };
        client.record_raw_rpc_message(&message);
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_string);
        let id = message.get("id").cloned();
        match (method, id) {
            (Some(method), Some(id)) => {
                let params = message.get("params").cloned().unwrap_or(Value::Null);
                let peer = peer.clone();
                let client = Arc::clone(&client);
                tokio::spawn(async move {
                    client.handle_request(peer, id, method, params).await;
                });
            }
            (Some(method), None) => {
                let params = message.get("params").cloned().unwrap_or(Value::Null);
                client.handle_notification(&method, params).await;
            }
            (None, Some(id)) => {
                if let Some(request_id) = id.as_u64() {
                    if let Some(tx) = peer.pending.lock().await.remove(&request_id) {
                        if let Some(result) = message.get("result") {
                            let _ = tx.send(Ok(result.clone()));
                        } else {
                            let error = message.get("error").cloned().unwrap_or(Value::Null);
                            let _ = tx.send(Err(RpcError {
                                code: error.get("code").and_then(Value::as_i64).unwrap_or(-32603),
                                message: error
                                    .get("message")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Error")
                                    .to_string(),
                                data: error.get("data").cloned(),
                            }));
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::RpcPeer;
    use std::process::Stdio;
    use tokio::process::Command;

    #[tokio::test]
    async fn fail_pending_releases_waiting_requests() {
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn cat");
        let stdin = child.stdin.take().expect("stdin");
        let peer = RpcPeer::new(stdin);
        let (tx, pending) = tokio::sync::oneshot::channel();
        peer.pending.lock().await.insert(1, tx);

        peer.fail_pending("closed for test").await;

        let result = pending.await.expect("pending response");
        assert!(result.is_err());
        assert_eq!(result.err().expect("error").message, "closed for test");
        let _ = child.kill().await;
    }
}
