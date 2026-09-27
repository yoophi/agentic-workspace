//! 043 T005: 창 incarnation에 묶인 데스크톱 토큰. 창이 닫히면(주체 폐기) 그 주체의 토큰으로는 호출·표 발급이 401이고,
//! 폐기 전에 받아 둔 표도 쓸 수 없다. 다른 창의 토큰과 같은 label의 새 incarnation 토큰은 영향이 없다. 토큰은 발급 때
//! 받은 창 주체로 풀려, 다른 창이 연 작업대를 조작하지 못한다(작업대 소유 판정, T004).

use std::sync::Arc;

use reqwest::{header, StatusCode};
use serde_json::{json, Value};
use support::{
    http_harness::{read_reply, Harness, HarnessOptions},
    list_request, TestRuntime,
};
use workbench_protocol::{
    AuthenticatedPrincipal, FaultCode, IdempotencyKey, OperationId, RequestId, StreamCursor,
    Workbench, PROTOCOL_VERSION,
};
use workbench_server::{
    auth::{DesktopTokenIssuer, TokenOrigin, DESKTOP_TOKEN_TTL},
    tickets::EventTicketStore,
};

mod support;

const APP: &str = "http://localhost:1420";

struct Env {
    _rt: TestRuntime,
    harness: Harness,
    issuer: Arc<DesktopTokenIssuer>,
    tickets: Arc<EventTicketStore>,
    dir: String,
    epoch: String,
}

async fn env() -> Env {
    let rt = TestRuntime::new();
    let dir = rt.dir.path().join("wt");
    std::fs::create_dir_all(&dir).unwrap();
    let issuer = Arc::new(DesktopTokenIssuer::default());
    let tickets = Arc::new(EventTicketStore::default());
    let epoch = rt.runtime.epoch().to_owned();
    let harness = Harness::spawn_with(
        rt.runtime.clone() as Arc<dyn Workbench>,
        HarnessOptions {
            origins: vec![APP.into()],
            extra_resolver: Some(issuer.clone()),
            tickets: Some(tickets.clone()),
            ..HarnessOptions::default()
        },
    )
    .await;
    Env {
        _rt: rt,
        harness,
        issuer,
        tickets,
        dir: dir.to_string_lossy().into_owned(),
        epoch,
    }
}

impl Env {
    fn issue(&self, label: &str, incarnation: &str) -> String {
        self.issuer
            .issue_for(
                AuthenticatedPrincipal::desktop_window(label, incarnation),
                TokenOrigin::WebView(APP.into()),
                DESKTOP_TOKEN_TTL,
            )
            .token
    }

    /// 창 주체 폐기(창 Destroyed): 토큰과 남은 표를 모두 지운다.
    fn revoke(&self, label: &str, incarnation: &str) {
        let subject = AuthenticatedPrincipal::desktop_window(label, incarnation).subject;
        self.issuer.revoke_subject(&subject);
        self.tickets.revoke_subject(&subject);
    }

    async fn call(&self, token: &str, operation: OperationId, input: Value) -> reqwest::Response {
        let command = !matches!(
            workbench_protocol::operations::spec_for(operation).kind,
            workbench_protocol::OperationKind::Query
        );
        let body = json!({
            "protocolVersion": PROTOCOL_VERSION,
            "operation": operation.as_str(),
            "requestId": RequestId::random(),
            "input": input,
            "idempotencyKey": if command { Some(IdempotencyKey::random()) } else { None },
        });
        self.harness
            .client()
            .post(self.harness.url())
            .bearer_auth(token)
            .header(header::ORIGIN, APP)
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    async fn list_status(&self, token: &str) -> StatusCode {
        self.harness
            .client()
            .post(self.harness.url())
            .bearer_auth(token)
            .header(header::ORIGIN, APP)
            .json(&list_request())
            .send()
            .await
            .unwrap()
            .status()
    }

    fn cursor(&self, stream: &str) -> StreamCursor {
        StreamCursor {
            stream_id: stream.into(),
            epoch: self.epoch.clone(),
            after_sequence: 0,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoking_a_window_incarnation_kills_its_tokens_and_pending_tickets_only() {
    let env = env().await;
    let a1 = env.issue("session-1", "inc-1");
    let a1_second = env.issue("session-1", "inc-1");
    let b = env.issue("session-2", "inc-1");
    for token in [&a1, &a1_second, &b] {
        assert_eq!(env.list_status(token).await, StatusCode::OK);
    }
    let bench = read_reply(
        env.call(
            &a1,
            OperationId::BenchOpen,
            json!({ "workingDirectory": env.dir }),
        )
        .await,
    )
    .await
    .expect("bench.open")
    .output()
    .unwrap()["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    // 토큰은 창 주체로 풀린다: 다른 창 토큰은 이 작업대를 닫지 못한다.
    let foreign = read_reply(
        env.call(&b, OperationId::BenchClose, json!({ "benchId": bench }))
            .await,
    )
    .await;
    assert_eq!(foreign.unwrap_err().code, FaultCode::Forbidden);

    let pending = env
        .harness
        .issue_ticket(&a1, Some(APP), &[env.cursor(&format!("bench:{bench}"))])
        .await
        .expect("ticket before revocation");

    env.revoke("session-1", "inc-1");

    // 폐기된 incarnation의 토큰 두 개 모두 401.
    for token in [&a1, &a1_second] {
        assert_eq!(env.list_status(token).await, StatusCode::UNAUTHORIZED);
        let ticket = env
            .harness
            .issue_ticket(token, Some(APP), &[env.cursor(&format!("bench:{bench}"))])
            .await;
        assert_eq!(ticket.unwrap_err().code, FaultCode::Unauthenticated);
    }
    // 폐기 전에 받은 표도 쓸 수 없다(upgrade 거절).
    assert!(env
        .harness
        .connect_ticket(&pending, Some(APP))
        .await
        .is_err());

    // 다른 창은 그대로, 같은 label의 새 incarnation은 새 토큰으로 동작하지만 이전 작업대는 남의 것이다.
    assert_eq!(env.list_status(&b).await, StatusCode::OK);
    let a2 = env.issue("session-1", "inc-2");
    assert_eq!(env.list_status(&a2).await, StatusCode::OK);
    let reopened = read_reply(
        env.call(&a2, OperationId::BenchClose, json!({ "benchId": bench }))
            .await,
    )
    .await;
    assert_eq!(reopened.unwrap_err().code, FaultCode::Forbidden);
}
