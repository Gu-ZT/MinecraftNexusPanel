//! WebSocket 票据签发、连接升级和 Core/实例事件订阅路由。
//!
//! 客户端先通过 HTTP 鉴权获取一次性票据，再使用票据升级连接；订阅主题限制为
//! Core 状态、实例控制台和任务快照，控制台订阅通过游标避免重复读取。

use std::collections::HashMap;

use axum::Error;
use axum::Json;
use axum::Router;
use axum::extract::Query;
use axum::extract::State;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use nexus_domain::CoreId;
use nexus_domain::InstanceId;
use nexus_domain::RequestId;
use nexus_storage::StoredSession;
use serde_json::Value;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::AuthError;
use crate::PanelState;
use crate::RequestCredential;
use crate::auth_routes::auth_error_response;
use crate::auth_routes::authenticate;
use crate::auth_routes::error_response;
use crate::auth_routes::header_text;
use crate::auth_routes::request_credential;
use crate::auth_routes::run_blocking;

const HEARTBEAT_SECONDS: u64 = 20;
const MAX_SUBSCRIPTIONS: u16 = 100;

pub(crate) fn websocket_routes() -> Router<PanelState> {
    Router::new()
        .route("/api/v1/ws/tickets", post(create_ticket))
        .route("/api/v1/ws", get(connect_websocket))
}

async fn create_ticket(State(state): State<PanelState>, headers: HeaderMap) -> Response {
    let request_id = RequestId::new();
    let credential = match request_credential(&headers) {
        Some(credential) => credential,
        None => return auth_error_response(AuthError::InvalidSession, request_id),
    };
    if let Err(response) =
        authenticate_websocket_ticket_request(&state, &headers, &credential, request_id)
            .await
            .map(|_| ())
    {
        return response;
    }

    match state.websocket_tickets().issue(credential) {
        Ok((ticket, expires_at)) => (
            StatusCode::CREATED,
            Json(json!({
                "ticket": ticket,
                "expiresAt": timestamp(expires_at),
            })),
        )
            .into_response(),
        Err(error) => auth_error_response(error, request_id),
    }
}

async fn connect_websocket(
    State(state): State<PanelState>,
    Query(query): Query<HashMap<String, String>>,
    websocket: WebSocketUpgrade,
) -> Response {
    let request_id = RequestId::new();
    let ticket = match query.get("ticket") {
        Some(ticket) => ticket,
        None => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "WS_TICKET_INVALID",
                "WebSocket ticket is invalid or expired",
                request_id,
            );
        }
    };
    let ticket = match state.websocket_tickets().consume(ticket) {
        Ok(Some(ticket)) => ticket,
        Ok(None) => {
            return error_response(
                StatusCode::UNAUTHORIZED,
                "WS_TICKET_INVALID",
                "WebSocket ticket is invalid or expired",
                request_id,
            );
        }
        Err(error) => return auth_error_response(error, request_id),
    };
    let credential = ticket.credential().clone();
    if let Err(response) = authenticate_websocket_credential(&state, &credential, request_id).await
    {
        return response;
    }

    websocket.on_upgrade(move |socket| websocket_session(socket, state))
}

async fn websocket_session(mut socket: WebSocket, state: PanelState) {
    if send_json(&mut socket, ready_message()).await.is_err() {
        return;
    }

    while let Some(message) = socket.recv().await {
        let Ok(message) = message else {
            return;
        };
        match message {
            Message::Text(text) => handle_client_text(&mut socket, &state, text.as_str()).await,
            Message::Ping(payload) => {
                if socket.send(Message::Pong(payload)).await.is_err() {
                    return;
                }
            }
            Message::Close(_) => return,
            Message::Binary(_) | Message::Pong(_) => {}
        }
    }
}

async fn handle_client_text(socket: &mut WebSocket, state: &PanelState, text: &str) {
    let Ok(message) = serde_json::from_str::<Value>(text) else {
        let _ = send_json(socket, protocol_error(None, "WS_MESSAGE_INVALID")).await;
        return;
    };
    match message.get("type").and_then(Value::as_str) {
        Some("ping") => {
            let _ = send_json(socket, pong_message(&message)).await;
        }
        Some("subscribe") => subscribe(socket, state, &message).await,
        Some("unsubscribe") => {
            let _ = send_json(socket, unsubscribe_ack(&message)).await;
        }
        _ => {
            let _ = send_json(
                socket,
                protocol_error(message_id(&message), "WS_MESSAGE_INVALID"),
            )
            .await;
        }
    }
}

async fn subscribe(socket: &mut WebSocket, state: &PanelState, message: &Value) {
    let subscription_id = RequestId::new();
    if send_json(socket, subscribe_ack(message, subscription_id))
        .await
        .is_err()
    {
        return;
    }
    let Some(topic) = message.get("topic").and_then(Value::as_str) else {
        let _ = send_json(
            socket,
            protocol_error(message_id(message), "WS_TOPIC_INVALID"),
        )
        .await;
        return;
    };

    match topic_parts(topic).as_slice() {
        ["core", core_id, "status"] => {
            send_core_status(socket, state, subscription_id, topic, core_id).await
        }
        ["instance", core_id, instance_id, "console"] => {
            send_console_snapshot(
                socket,
                state,
                subscription_id,
                topic,
                core_id,
                instance_id,
                message,
            )
            .await;
        }
        ["task", task_id] => {
            let _ = send_json(socket, task_snapshot(subscription_id, topic, task_id)).await;
        }
        _ => {
            let _ = send_json(
                socket,
                protocol_error(message_id(message), "WS_TOPIC_FORBIDDEN"),
            )
            .await;
        }
    }
}

async fn send_core_status(
    socket: &mut WebSocket,
    state: &PanelState,
    subscription_id: RequestId,
    topic: &str,
    core_id: &str,
) {
    let Some(core_id) = parse_core_id(core_id) else {
        let _ = send_json(socket, protocol_error(None, "WS_TOPIC_INVALID")).await;
        return;
    };
    match state.cores().get(core_id).await {
        Ok(core) => {
            let _ = send_json(socket, event_message(subscription_id, topic, None, core)).await;
        }
        Err(_) => {
            let _ = send_json(socket, protocol_error(None, "WS_TOPIC_FORBIDDEN")).await;
        }
    }
}

async fn send_console_snapshot(
    socket: &mut WebSocket,
    state: &PanelState,
    subscription_id: RequestId,
    topic: &str,
    core_id: &str,
    instance_id: &str,
    message: &Value,
) {
    let Some(core_id) = parse_core_id(core_id) else {
        let _ = send_json(
            socket,
            protocol_error(message_id(message), "WS_TOPIC_INVALID"),
        )
        .await;
        return;
    };
    let Some(instance_id) = parse_instance_id(instance_id) else {
        let _ = send_json(
            socket,
            protocol_error(message_id(message), "WS_TOPIC_INVALID"),
        )
        .await;
        return;
    };
    let cursor = message.get("cursor").and_then(Value::as_str);
    match state
        .cores()
        .get_instance_logs(core_id, &instance_id, cursor, None, Some(200))
        .await
    {
        Ok(page) => {
            if let Some(items) = page.get("items").and_then(Value::as_array) {
                for item in items {
                    let cursor = item.get("cursor").and_then(Value::as_str);
                    let data = json!({
                        "stream": item.get("stream").cloned().unwrap_or(Value::Null),
                        "line": item.get("line").cloned().unwrap_or(Value::Null),
                    });
                    if send_json(socket, event_message(subscription_id, topic, cursor, data))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
        Err(_) => {
            let _ = send_json(
                socket,
                protocol_error(message_id(message), "WS_TOPIC_FORBIDDEN"),
            )
            .await;
        }
    }
}

async fn send_json(socket: &mut WebSocket, value: Value) -> Result<(), Error> {
    socket.send(Message::Text(value.to_string().into())).await
}

async fn authenticate_websocket_ticket_request(
    state: &PanelState,
    headers: &HeaderMap,
    credential: &RequestCredential,
    request_id: RequestId,
) -> Result<(), Response> {
    let browser_session = matches!(credential, RequestCredential::Browser(_));
    let csrf_token = header_text(headers, "x-csrf-token").map(str::to_owned);
    let session = authenticate_websocket_credential(state, credential, request_id).await?;
    if browser_session {
        state
            .auth()
            .verify_csrf(
                &session,
                csrf_token
                    .as_deref()
                    .ok_or_else(|| auth_error_response(AuthError::InvalidCsrfToken, request_id))?,
            )
            .map_err(|error| auth_error_response(error, request_id))?;
    }

    Ok(())
}

async fn authenticate_websocket_credential(
    state: &PanelState,
    credential: &RequestCredential,
    request_id: RequestId,
) -> Result<StoredSession, Response> {
    let auth = state.auth().clone();
    let credential = credential.clone();
    run_blocking(move || authenticate(&auth, &credential))
        .await
        .map_err(|error| auth_error_response(error, request_id))
}

fn ready_message() -> Value {
    json!({
        "type": "ready",
        "connectionId": RequestId::new(),
        "heartbeatSeconds": HEARTBEAT_SECONDS,
        "maxSubscriptions": MAX_SUBSCRIPTIONS,
        "serverTime": current_timestamp(),
    })
}

fn subscribe_ack(message: &Value, subscription_id: RequestId) -> Value {
    json!({
        "type": "ack",
        "messageId": message_id(message),
        "subscriptionId": subscription_id,
        "acceptedCursor": message.get("cursor").cloned().unwrap_or(Value::Null),
    })
}

fn unsubscribe_ack(message: &Value) -> Value {
    json!({
        "type": "ack",
        "messageId": message_id(message),
        "subscriptionId": message.get("subscriptionId").cloned().unwrap_or(Value::Null),
    })
}

fn event_message(
    subscription_id: RequestId,
    topic: &str,
    cursor: Option<&str>,
    data: Value,
) -> Value {
    json!({
        "type": "event",
        "subscriptionId": subscription_id,
        "topic": topic,
        "eventId": RequestId::new(),
        "sequence": 1,
        "occurredAt": current_timestamp(),
        "cursor": cursor,
        "data": data,
    })
}

fn task_snapshot(subscription_id: RequestId, topic: &str, task_id: &str) -> Value {
    event_message(
        subscription_id,
        topic,
        None,
        json!({
            "taskId": task_id,
            "state": "UNKNOWN",
            "progress": null,
        }),
    )
}

fn pong_message(message: &Value) -> Value {
    json!({
        "type": "pong",
        "messageId": message_id(message),
        "receivedAt": current_timestamp(),
    })
}

fn protocol_error(message_id: Option<&str>, code: &str) -> Value {
    json!({
        "type": "error",
        "messageId": message_id,
        "error": {
            "code": code,
            "message": "WebSocket message could not be processed",
            "retryable": false,
        }
    })
}

fn message_id(message: &Value) -> Option<&str> {
    message.get("messageId").and_then(Value::as_str)
}

fn topic_parts(topic: &str) -> Vec<&str> {
    topic.split('/').collect()
}

fn parse_core_id(value: &str) -> Option<CoreId> {
    value.parse().ok()
}

fn parse_instance_id(value: &str) -> Option<InstanceId> {
    value.parse().ok()
}

fn current_timestamp() -> String {
    timestamp(OffsetDateTime::now_utc())
}

fn timestamp(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
