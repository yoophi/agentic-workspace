//! Test-only owner authority for the private empty-workspace trigger.
//! This module is absent from the production client and cannot change its admission.
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{client::conn::http1::SendRequest, Request};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpStream, task::JoinHandle};
use workbench_client::{
    infrastructure::{identity::verify_proof, locator::LocatedEndpoint},
    ports::CredentialProvider,
};
pub struct HarnessConnection {
    sender: SendRequest<Full<Bytes>>,
    driver: Option<JoinHandle<()>>,
    endpoint: Arc<LocatedEndpoint>,
}
impl Drop for HarnessConnection {
    fn drop(&mut self) {
        if let Some(driver) = &self.driver {
            driver.abort();
        }
    }
}
impl HarnessConnection {
    pub async fn connect(endpoint: Arc<LocatedEndpoint>) -> Self {
        tokio::time::timeout(Duration::from_secs(2), async {
            let socket = TcpStream::connect(endpoint.address()).await.unwrap();
            let (sender, driver) = hyper::client::conn::http1::handshake(TokioIo::new(socket))
                .await
                .unwrap();
            let mut connection = Self {
                sender,
                driver: Some(tokio::spawn(async move {
                    let _ = driver.await;
                })),
                endpoint,
            };
            let nonce = uuid::Uuid::new_v4().to_string();
            let identify = connection
                .post("/v1/system/identify", json!({"nonce":nonce}), false)
                .await;
            let instance = identify["instanceId"].as_str().unwrap();
            assert_eq!(instance, connection.endpoint.identity().instance());
            verify_proof(
                connection.endpoint.credential(),
                &nonce,
                instance,
                identify["proof"].as_str().unwrap(),
            )
            .unwrap();
            let handshake = connection
                .post(
                    "/v1/system/handshake",
                    json!({"protocolVersions":[1],"clientName":"047-private-wire-harness"}),
                    true,
                )
                .await;
            assert_eq!(
                handshake["serverEpoch"],
                connection.endpoint.identity().epoch()
            );
            assert_eq!(handshake["instanceId"], instance);
            connection
        })
        .await
        .expect("private harness connection deadline")
    }
    pub async fn post(&mut self, path: &str, value: Value, authenticated: bool) -> Value {
        tokio::time::timeout(Duration::from_secs(2), async {
            let mut request = Request::post(path)
                .header("host", self.endpoint.address().to_string())
                .header("content-type", "application/json");
            if authenticated {
                request = request.header(
                    "authorization",
                    format!("Bearer {}", self.endpoint.credential().expose()),
                );
            }
            let response = self
                .sender
                .send_request(
                    request
                        .body(Full::new(Bytes::from(serde_json::to_vec(&value).unwrap())))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            let mut body = response.into_body();
            let mut bytes = Vec::new();
            while let Some(frame) = body.frame().await {
                if let Ok(data) = frame.unwrap().into_data() {
                    assert!(bytes.len() + data.len() <= 8 * 1024 * 1024);
                    bytes.extend_from_slice(&data);
                }
            }
            serde_json::from_slice(&bytes).unwrap()
        })
        .await
        .expect("private harness request deadline")
    }
    pub async fn close(mut self) {
        if let Some(driver) = self.driver.take() {
            driver.abort();
            let result = tokio::time::timeout(Duration::from_secs(1), driver)
                .await
                .expect("private harness driver cleanup deadline");
            assert!(result.is_ok() || result.unwrap_err().is_cancelled());
        }
    }
}

// Keep the handle while a wait future is cancellable. Timeout/panic boundaries must
// explicitly cancel/join; Drop is a final abort fallback rather than detachment.
pub struct OwnedTask<T>(Option<JoinHandle<T>>);
impl<T> Default for OwnedTask<T> {
    fn default() -> Self {
        Self(None)
    }
}
impl<T> Drop for OwnedTask<T> {
    fn drop(&mut self) {
        if let Some(handle) = &self.0 {
            handle.abort();
        }
    }
}
impl<T: Send + 'static> OwnedTask<T> {
    pub fn start(&mut self, future: impl std::future::Future<Output = T> + Send + 'static) {
        assert!(self.0.is_none());
        self.0 = Some(tokio::spawn(future));
    }
    pub fn is_finished(&self) -> bool {
        self.0.as_ref().unwrap().is_finished()
    }
    pub async fn wait(&mut self) -> T {
        let result = self.0.as_mut().unwrap().await;
        self.0.take();
        result.unwrap()
    }
    pub async fn cancel_join(&mut self) -> Result<(), &'static str> {
        if let Some(handle) = &mut self.0 {
            handle.abort();
            let result = tokio::time::timeout(Duration::from_secs(1), handle)
                .await
                .map_err(|_| "fixture task cleanup deadline")?;
            self.0.take();
            if let Err(error) = result {
                if !error.is_cancelled() {
                    return Err("fixture task cleanup panic");
                }
            }
        }
        Ok(())
    }
}
