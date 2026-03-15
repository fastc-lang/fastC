use crate::executor::{ExecutionMessage, Executor};
use crate::server::AppState;
use axum::{
    extract::{
        ConnectInfo, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast, mpsc};
use uuid::Uuid;

/// WebSocket message from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe")]
    Subscribe { session_id: String },
    #[serde(rename = "run")]
    Run { code: String },
}

#[derive(Debug, Deserialize, Default)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

/// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    Query(query): Query<WsAuthQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(message) = state.authorize_ws(&headers, query.token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, message).into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, remote.ip()))
        .into_response()
}

async fn handle_socket(socket: WebSocket, state: AppState, remote_ip: std::net::IpAddr) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    // Channel for coordinating sends to the WebSocket
    let (internal_tx, mut internal_rx) = mpsc::channel::<ExecutionMessage>(100);

    // Task to forward messages to WebSocket
    let sender_clone = sender.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = internal_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                let mut sender = sender_clone.lock().await;
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                match client_msg {
                    ClientMessage::Subscribe { session_id } => {
                        tracing::info!("Client subscribed to session: {}", session_id);

                        // Parse the session ID and subscribe to its broadcast channel
                        if let Ok(uuid) = Uuid::parse_str(&session_id) {
                            if let Some(mut rx) = state.sessions.subscribe(uuid) {
                                let internal_tx = internal_tx.clone();

                                // Spawn task to forward broadcast messages to this client
                                tokio::spawn(async move {
                                    loop {
                                        match rx.recv().await {
                                            Ok(msg) => {
                                                if internal_tx.send(msg).await.is_err() {
                                                    break;
                                                }
                                            }
                                            Err(broadcast::error::RecvError::Closed) => break,
                                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                                        }
                                    }
                                });
                            }
                        }
                    }
                    ClientMessage::Run { code } => {
                        if let Err(message) = state.validate_run_request(remote_ip, code.len()) {
                            let _ = internal_tx.send(ExecutionMessage::Error { message }).await;
                            continue;
                        }

                        let Ok(run_slot) = state.try_acquire_run_slot() else {
                            let _ = internal_tx
                                .send(ExecutionMessage::Error {
                                    message: "Server is busy. Try again shortly.".to_string(),
                                })
                                .await;
                            continue;
                        };

                        // Create a new session and run directly via WebSocket
                        let session_id = state.sessions.create();
                        state.sessions.update_code(session_id, code.clone());

                        // Create broadcast channel for this session
                        let tx = state.sessions.create_channel(session_id);

                        // Subscribe this client to the channel
                        let mut rx = tx.subscribe();
                        let internal_tx = internal_tx.clone();

                        // Forward messages to this client
                        tokio::spawn(async move {
                            loop {
                                match rx.recv().await {
                                    Ok(msg) => {
                                        if internal_tx.send(msg).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(broadcast::error::RecvError::Closed) => break,
                                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                                }
                            }
                        });

                        // Run the executor
                        let sessions = state.sessions.clone();
                        let executor_limits = state.executor_limits.clone();
                        tokio::spawn(async move {
                            let _run_slot = run_slot;
                            let executor = Executor::new(executor_limits);
                            if let Err(e) = executor.run(session_id, &code, tx).await {
                                tracing::error!("Execution failed: {}", e);
                            }
                            sessions.remove_channel(session_id);
                        });
                    }
                }
            }
        }
    }

    send_task.abort();
}
