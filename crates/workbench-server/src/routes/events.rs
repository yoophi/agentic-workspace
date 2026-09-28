//! 구독 표 발급과 WebSocket(contracts §3·§4). upgrade 전에 표를 원자적으로 꺼내 소모하고, 연결 뒤 서버가 hello를
//! 보낸 다음 표의 cursor로 `Workbench.events`를 부른다. 판정(권한·cursor 0개·hub 상한)은 모두 거기서 난다(D1).

use std::{sync::Arc, time::Instant};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, Request, State,
    },
    response::Response,
};
use futures_util::StreamExt;
use serde::Deserialize;
use workbench_protocol::{
    events::EventFrame, AuthenticatedPrincipal, CallRequest, EventItem, FaultCode, OperationId,
    RequestId, StreamCursor, Subscription, Workbench, WorkbenchFault, PROTOCOL_VERSION,
};

use super::{
    authenticate, calls::MESSAGE_BAD_BODY, json_response, origin, problem, read_body, record,
    unauthenticated,
};
use crate::{
    drain::{CallGuard, DetachedCalls},
    tickets::{IssueError, TakeError, MESSAGE_TICKETS_EXHAUSTED, MESSAGE_TOO_MANY_CURSORS},
    AppState, MESSAGE_ORIGIN_NOT_ALLOWED, WS_MAX_MESSAGE,
};

#[derive(Deserialize)]
struct TicketRequest {
    cursors: Vec<StreamCursor>,
}

pub async fn issue_ticket(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let started = Instant::now();
    let request_id = RequestId::random();
    let headers = request.headers().clone();
    let principal = match authenticate(&state, &headers) {
        Some(principal) => principal,
        None => {
            let response = unauthenticated(&request_id);
            record(&state, started, None, "event-tickets", None, &response);
            return response;
        }
    };
    let body = match read_body(&state, request, &request_id).await {
        Ok(body) => body,
        Err(response) => {
            record(
                &state,
                started,
                None,
                "event-tickets",
                Some(&principal),
                &response,
            );
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
            Err(IssueError::Retired) => unauthenticated(&request_id),
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
    headers: axum::http::HeaderMap,
    Query(query): Query<EventsQuery>,
    upgrade: WebSocketUpgrade,
) -> Response {
    let started = Instant::now();
    // 열린 구독은 추적한다 — 종료 때 닫고 `serve` 반환 전에 모두 끝나기를 기다린다. 종료 중이면 새 구독은 `503`.
    let Some(guard) = state.subscriptions.accept() else {
        let response = problem(&WorkbenchFault::unavailable(
            RequestId::random(),
            crate::drain::MESSAGE_SHUTTING_DOWN,
        ));
        record(&state, started, None, "events", None, &response);
        return response;
    };
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
    let subscriptions = Arc::clone(&state.subscriptions);
    let principal = ticket.principal.clone();
    // upgrade가 끝나지 않으면 이 closure가 버려지며 guard도 함께 풀린다.
    let response = upgrade
        .max_message_size(WS_MAX_MESSAGE)
        .on_upgrade(move |socket| {
            serve(
                guard,
                subscriptions,
                workbench,
                ticket.principal,
                ticket.cursors,
                socket,
            )
        });
    record(&state, started, None, "events", Some(&principal), &response);
    response
}

/// 구독 → hello(준비 완료) → event/gap 프레임. 연결 종료 = 구독 해제(스트림 drop). 서버 종료가 시작되면 보내던 프레임도
/// 멈추고 close를 보낸 뒤(제한 시간 안에) 스트림과 소켓을 놓는다. `_guard`는 매개변수라 스트림·소켓보다 늦게 풀린다 —
/// 추적 수가 0이 되면 정리가 끝난 것이다.
async fn serve(
    _guard: CallGuard,
    subscriptions: Arc<DetachedCalls>,
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
    // 구독을 먼저 등록하고 hello를 보낸다: hello는 "구독 준비 완료" 신호다. 재생 없는 알림 스트림(worktree)은 등록
    // 전의 변경을 받을 수 없으므로, hello를 먼저 보내면 클라이언트가 준비됐다고 믿은 뒤의 변경을 잃는다(042 구현 리뷰
    // 중 발견). 등록 뒤 쌓인 항목은 EventStream이 담아 둔다. 거절이면 hello 뒤 fault(프레임 순서는 같다).
    let subscribed = workbench.events(principal, Subscription { cursors });
    let hello = EventFrame::Hello {
        protocol_version: PROTOCOL_VERSION,
        epoch,
    };
    if !send_unless_closing(&subscriptions, &mut socket, &hello).await {
        close(&mut socket).await;
        return;
    }
    let mut stream = match subscribed {
        Ok(stream) => stream,
        Err(fault) => {
            let _ = send_unless_closing(&subscriptions, &mut socket, &EventFrame::Fault { fault })
                .await;
            close(&mut socket).await;
            return;
        }
    };
    loop {
        tokio::select! {
            biased;
            () = subscriptions.closed() => break,
            item = stream.next() => {
                let Some(item) = item else { break };
                let frame = match item {
                    EventItem::Event { event } => EventFrame::Event { event },
                    EventItem::Gap { gap } => EventFrame::Gap { gap },
                };
                if !send_unless_closing(&subscriptions, &mut socket, &frame).await {
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
    drop(stream);
    close(&mut socket).await;
}

/// 프레임 하나를 보낸다. 종료가 시작되면(읽지 않는 클라이언트로 전송이 막혀 있어도) 멈추고 `false`.
async fn send_unless_closing(
    subscriptions: &DetachedCalls,
    socket: &mut WebSocket,
    frame: &EventFrame,
) -> bool {
    let text = serde_json::to_string(frame).expect("frame json");
    tokio::select! {
        biased;
        () = subscriptions.closed() => false,
        sent = socket.send(Message::Text(text)) => sent.is_ok(),
    }
}

async fn close(socket: &mut WebSocket) {
    let _ = tokio::time::timeout(
        crate::SUBSCRIPTION_CLOSE_TIMEOUT,
        socket.send(Message::Close(None)),
    )
    .await;
}
