//! 구독 표 발급과 WebSocket(contracts §3·§4). upgrade 전에 표를 원자적으로 꺼내 소모하고, 연결 뒤 서버가 hello를
//! 보낸 다음 표의 cursor로 `Workbench.events`를 부른다. 판정(권한·cursor 0개·hub 상한)은 모두 거기서 난다(D1).

use std::{sync::Arc, time::Instant};

use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::HeaderMap,
    response::Response,
};
use futures_util::StreamExt;
use serde::Deserialize;
use workbench_protocol::{
    events::EventFrame, AuthenticatedPrincipal, CallRequest, EventItem, FaultCode, OperationId,
    RequestId, StreamCursor, Subscription, Workbench, WorkbenchFault, PROTOCOL_VERSION,
};

use super::{
    authenticate, calls::MESSAGE_BAD_BODY, json_response, origin, problem, record, unauthenticated,
};
use crate::{
    tickets::{IssueError, TakeError, MESSAGE_TICKETS_EXHAUSTED, MESSAGE_TOO_MANY_CURSORS},
    AppState, MESSAGE_ORIGIN_NOT_ALLOWED, WS_MAX_MESSAGE,
};

#[derive(Deserialize)]
struct TicketRequest {
    cursors: Vec<StreamCursor>,
}

pub async fn issue_ticket(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let started = Instant::now();
    let request_id = RequestId::random();
    let principal = match authenticate(&state, &headers) {
        Some(principal) => principal,
        None => {
            let response = unauthenticated(&request_id);
            record(&state, started, None, "event-tickets", None, &response);
            return response;
        }
    };
    let response = match serde_json::from_slice::<TicketRequest>(&body) {
        Err(_) => invalid(request_id, MESSAGE_BAD_BODY),
        Ok(request) => match state.config.tickets.issue(
            principal.clone(),
            request.cursors,
            origin(&headers).map(str::to_owned),
        ) {
            Ok((ticket, expires_at)) => json_response(&serde_json::json!({
                "ticket": ticket,
                "expiresAt": expires_at.to_rfc3339(),
            })),
            Err(IssueError::TooManyCursors) => invalid(request_id, MESSAGE_TOO_MANY_CURSORS),
            Err(IssueError::Exhausted) => problem(&WorkbenchFault::unavailable(
                request_id,
                MESSAGE_TICKETS_EXHAUSTED,
            )),
        },
    };
    record(
        &state,
        started,
        None,
        "event-tickets",
        Some(&principal),
        &response,
    );
    response
}

fn invalid(request_id: RequestId, message: &str) -> Response {
    problem(&WorkbenchFault::new(
        FaultCode::InvalidArgument,
        request_id,
        message,
    ))
}

#[derive(Deserialize)]
pub struct EventsQuery {
    ticket: Option<String>,
}

pub async fn connect(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let started = Instant::now();
    let taken = query
        .ticket
        .as_deref()
        .ok_or(TakeError::Invalid)
        .and_then(|ticket| state.config.tickets.take(ticket, origin(&headers)));
    let ticket = match taken {
        Ok(ticket) => ticket,
        Err(error) => {
            let fault = match error {
                TakeError::Invalid => WorkbenchFault::unauthenticated(RequestId::random()),
                TakeError::OriginMismatch => WorkbenchFault::new(
                    FaultCode::Forbidden,
                    RequestId::random(),
                    MESSAGE_ORIGIN_NOT_ALLOWED,
                ),
            };
            let response = problem(&fault);
            record(&state, started, None, "events", None, &response);
            return response;
        }
    };
    let workbench = Arc::clone(&state.workbench);
    let principal = ticket.principal.clone();
    let response = upgrade
        .max_message_size(WS_MAX_MESSAGE)
        .on_upgrade(move |socket| serve(workbench, ticket.principal, ticket.cursors, socket));
    record(&state, started, None, "events", Some(&principal), &response);
    response
}

async fn send(socket: &mut WebSocket, frame: &EventFrame) -> bool {
    let text = serde_json::to_string(frame).expect("frame json");
    socket.send(Message::Text(text)).await.is_ok()
}

/// hello → 구독 → event/gap 프레임. 연결 종료 = 구독 해제(스트림 drop).
async fn serve(
    workbench: Arc<dyn Workbench>,
    principal: AuthenticatedPrincipal,
    cursors: Vec<StreamCursor>,
    mut socket: WebSocket,
) {
    // 세대는 Workbench 계약으로만 얻는다(system.describe).
    let epoch = workbench
        .call(
            principal.clone(),
            CallRequest::query(OperationId::SystemDescribe, serde_json::json!({})),
        )
        .await
        .ok()
        .and_then(|reply| {
            reply
                .output()
                .and_then(|out| out["epoch"].as_str().map(str::to_owned))
        })
        .unwrap_or_default();
    if !send(
        &mut socket,
        &EventFrame::Hello {
            protocol_version: PROTOCOL_VERSION,
            epoch,
        },
    )
    .await
    {
        return;
    }
    let mut stream = match workbench.events(principal, Subscription { cursors }) {
        Ok(stream) => stream,
        Err(fault) => {
            let _ = send(&mut socket, &EventFrame::Fault { fault }).await;
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
    };
    loop {
        tokio::select! {
            item = stream.next() => {
                let Some(item) = item else { break };
                let frame = match item {
                    EventItem::Event { event } => EventFrame::Event { event },
                    EventItem::Gap { gap } => EventFrame::Gap { gap },
                };
                if !send(&mut socket, &frame).await {
                    break;
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}
