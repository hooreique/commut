//! HTTP and WebSocket routes for the Rust `commut` server.
//!
//! API overview:
//! - `GET /api/build-info`
//!   - returns `<version> <digest>` using the build-time backend source digest
//! - `POST /api/nonce`
//!   - issues a Base64 nonce for the authentication handshake
//!   - stores that nonce in memory for a short time
//! - `POST /api/ticket`
//!   - accepts `<nonce-base64>.<signature-base64>`
//!   - verifies the signature over the Base64-decoded nonce bytes
//!   - consumes the nonce on success
//!   - returns `<id>.<server-ephemeral-public-key-base64>`
//! - `POST /api/salt`
//!   - accepts `<id>.<client-ephemeral-public-key-base64>`
//!   - consumes the checkpoint created by `/api/ticket`
//!   - derives session crypto and returns `<id>.<salt-base64>`
//! - `GET /sockets/{id}`
//!   - requires query param `token`, a signature over the Base64-decoded `id`
//!   - accepts optional `dimensions=<cols>,<rows>` and falls back to `100,30`
//!   - consumes the session crypto state before starting the PTY session
//!
//! HTTP bodies are plain text. Most application failures return plain-text
//! `500 Internal Server Error`; missing WebSocket `token` returns
//! `400 Bad Request`.

use axum::{
    Router,
    body::Body,
    extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket},
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use futures_util::SinkExt;
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    app::StaticAssetRoots,
    build_info,
    contract::{
        Dimensions, WS_CLOSE_NORMAL, WS_CLOSE_PTY_EXIT, WS_TYPE_PTY_DATA, build_resize_payload,
        parse_dimensions_or_default, parse_ws_frame, split_dot_pair,
    },
    crypto::AuthorizedKeySet,
    error::AppError,
    pty::{PtySpec, spawn_pty},
    state::SharedAppState,
};

#[derive(Debug, Clone)]
pub struct RouteDeps {
    pub state: SharedAppState,
    pub authorized_keys: AuthorizedKeySet,
}

pub fn build_router(deps: RouteDeps, static_roots: StaticAssetRoots) -> Router {
    Router::new()
        .nest_service(
            "/images",
            ServeDir::new(static_roots.public_dir.join("images")),
        )
        .nest_service(
            "/fonts",
            ServeDir::new(static_roots.public_dir.join("fonts")),
        )
        .nest_service("/app", ServeDir::new(static_roots.pages_dir.join("app")))
        .nest_service("/dist", ServeDir::new(static_roots.dist_dir))
        .nest_service(
            "/favicon.ico",
            ServeFile::new(static_roots.public_dir.join("favicon.ico")),
        )
        .nest_service(
            "/manifest.json",
            ServeFile::new(static_roots.public_dir.join("manifest.json")),
        )
        .route("/api/nonce", post(post_nonce))
        .route("/api/ticket", post(post_ticket))
        .route("/api/salt", post(post_salt))
        .route("/api/build-info", get(get_build_info))
        .route("/sockets/{id}", get(get_socket))
        .fallback(not_found)
        .with_state(deps)
}

async fn get_build_info() -> String {
    build_info::current_wire_body()
}

async fn post_nonce(State(deps): State<RouteDeps>) -> Result<Response<Body>, AppError> {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use rand::random;

    let nonce = BASE64_STANDARD.encode(random::<[u8; 8]>());
    deps.state.insert_nonce(nonce.clone()).await;
    Ok((StatusCode::OK, nonce).into_response())
}

/// Exchange `<nonce-base64>.<signature-base64>` for a one-shot checkpoint.
///
/// On success this consumes the nonce and returns
/// `<id>.<server-ephemeral-public-key-base64>`. The signature is verified over
/// the Base64-decoded nonce bytes.
async fn post_ticket(
    State(deps): State<RouteDeps>,
    body: String,
) -> Result<Response<Body>, AppError> {
    let (nonce, signature_base64) =
        split_dot_pair(&body).map_err(|error| AppError::internal(error.to_string()))?;

    if !deps.state.contains_nonce(nonce).await {
        return Err(AppError::internal("nonce not found"));
    }

    let is_valid = deps
        .authorized_keys
        .verify_authorization_signature(nonce, signature_base64)
        .map_err(AppError::from)?;

    if !is_valid {
        return Err(AppError::internal("wrong signature"));
    }

    let consumed = deps.state.consume_nonce(nonce).await;
    if !consumed {
        return Err(AppError::internal("nonce not found"));
    }

    let (id, checkpoint) = deps
        .authorized_keys
        .generate_checkpoint()
        .map_err(AppError::from)?;
    let response_body = format!("{id}.{}", checkpoint.server_ephemeral_public_key_base64);
    deps.state.insert_checkpoint(id, checkpoint).await;

    Ok((StatusCode::OK, response_body).into_response())
}

/// Complete the HTTP handshake using a stored checkpoint.
///
/// The request body is `<id>.<client-ephemeral-public-key-base64>`. On success
/// the server derives directional AES-128-GCM session keys, consumes the
/// checkpoint, stores session crypto under `id`, and returns `<id>.<salt-base64>`.
async fn post_salt(
    State(deps): State<RouteDeps>,
    body: String,
) -> Result<Response<Body>, AppError> {
    let (id, client_ephemeral_public_key_base64) =
        split_dot_pair(&body).map_err(|error| AppError::internal(error.to_string()))?;

    let checkpoint = deps
        .state
        .checkpoint(id)
        .await
        .ok_or_else(|| AppError::internal("checkpoint not found"))?;

    let (salt_base64, session_crypto) = deps
        .authorized_keys
        .derive_session_crypto(&checkpoint, client_ephemeral_public_key_base64)
        .map_err(AppError::from)?;

    let consumed = deps.state.consume_checkpoint(id).await;
    if consumed.is_none() {
        return Err(AppError::internal("checkpoint not found"));
    }

    deps.state.insert_crypt(id.to_owned(), session_crypto).await;

    Ok((StatusCode::OK, format!("{id}.{salt_base64}")).into_response())
}

#[derive(Debug, Deserialize)]
struct SocketQuery {
    token: Option<String>,
    dimensions: Option<String>,
}

/// Upgrade to the encrypted PTY WebSocket for a fully established session.
///
/// Requirements:
/// - path `id` must reference stored session crypto from `/api/salt`
/// - query `token` must be a valid signature over the Base64-decoded `id`
/// - query `dimensions` is optional and falls back to `100,30`
///
/// The stored session crypto is consumed before the interactive session starts,
/// so the same `id` cannot be upgraded twice.
async fn get_socket(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    Query(query): Query<SocketQuery>,
    State(deps): State<RouteDeps>,
) -> Result<Response<Body>, AppError> {
    let dimensions = parse_dimensions_or_default(query.dimensions.as_deref());
    let token = query
        .token
        .ok_or_else(|| AppError::bad_request("token is required"))?;
    deps.state
        .crypt(&id)
        .await
        .ok_or_else(|| AppError::internal("socket not found"))?;

    let is_valid = deps
        .authorized_keys
        .verify_authorization_signature(&id, &token)
        .map_err(AppError::from)?;

    if !is_valid {
        return Err(AppError::internal("wrong signature"));
    }

    let crypt = deps
        .state
        .consume_crypt(&id)
        .await
        .ok_or_else(|| AppError::internal("socket not found"))?;

    let response = ws.on_upgrade(move |socket| async move {
        handle_socket(socket, dimensions, crypt).await;
    });

    Ok(response.into_response())
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

async fn handle_socket(
    mut socket: WebSocket,
    initial_dimensions: Dimensions,
    crypt: crate::state::SessionCrypto,
) {
    let Ok(mut pty) = spawn_pty(PtySpec {
        dimensions: initial_dimensions,
    })
    .await
    else {
        log_socket_close_failure("spawn failure", socket.close().await);
        return;
    };
    let mut exit_rx = pty.exit_receiver();
    let mut output_stream_open = true;

    loop {
        tokio::select! {
            biased;

            message_result = socket.recv() => {
                let Some(Ok(message)) = message_result else {
                    log_pty_kill_failure("socket receive ended", pty.kill().await);
                    break;
                };

                if !handle_client_message(&mut socket, &mut pty, &crypt, message).await {
                    break;
                }
            }
            maybe_output = pty.next_output(), if output_stream_open => {
                if !forward_pty_output(&mut socket, &mut pty, &crypt, maybe_output, &mut output_stream_open).await {
                    break;
                }
            }
            exit_changed = exit_rx.changed() => {
                if !forward_pty_exit(&mut socket, exit_changed, &exit_rx).await {
                    break;
                }
            }
        }
    }
}

async fn handle_client_message(
    socket: &mut WebSocket,
    pty: &mut crate::pty::PtyHandle,
    crypt: &crate::state::SessionCrypto,
    message: Message,
) -> bool {
    match message {
        Message::Binary(payload) => handle_binary_message(socket, pty, crypt, &payload).await,
        Message::Close(close) => handle_close_message(socket, pty, close).await,
        Message::Text(_) | Message::Pong(_) => true,
        Message::Ping(payload) => {
            if socket.send(Message::Pong(payload)).await.is_err() {
                log_pty_kill_failure("pong send failure", pty.kill().await);
                return false;
            }

            true
        }
    }
}

async fn handle_binary_message(
    socket: &mut WebSocket,
    pty: &mut crate::pty::PtyHandle,
    crypt: &crate::state::SessionCrypto,
    payload: &[u8],
) -> bool {
    match parse_ws_frame(payload) {
        Ok(crate::contract::WsFrame::Resize { dimensions, .. }) => {
            handle_resize_message(socket, pty, dimensions).await
        }
        Ok(crate::contract::WsFrame::PtyData {
            iv,
            ciphertext_and_tag,
        }) => handle_pty_data_message(pty, crypt, &iv, ciphertext_and_tag).await,
        Ok(crate::contract::WsFrame::Unknown { .. }) | Err(_) => true,
    }
}

async fn handle_resize_message(
    socket: &mut WebSocket,
    pty: &mut crate::pty::PtyHandle,
    dimensions: Dimensions,
) -> bool {
    if pty.resize(dimensions).await.is_ok() {
        let echoed = build_resize_payload(dimensions);
        if socket.send(Message::Binary(echoed.into())).await.is_err() {
            log_pty_kill_failure("resize echo send failure", pty.kill().await);
            return false;
        }
    }

    true
}

async fn handle_pty_data_message(
    pty: &mut crate::pty::PtyHandle,
    crypt: &crate::state::SessionCrypto,
    iv: &[u8; 12],
    ciphertext_and_tag: &[u8],
) -> bool {
    let Ok(decrypted) = crypt.decrypt(iv, ciphertext_and_tag) else {
        return true;
    };

    if pty.write(&decrypted).await.is_err() {
        log_pty_kill_failure("pty write failure", pty.kill().await);
        return false;
    }

    true
}

async fn handle_close_message(
    socket: &mut WebSocket,
    pty: &mut crate::pty::PtyHandle,
    close: Option<CloseFrame>,
) -> bool {
    if let Some(close) = close
        && close.code == WS_CLOSE_NORMAL
    {
        log_pty_kill_failure("client normal close", pty.kill().await);
        log_socket_close_failure("client normal close", socket.close().await);
        return false;
    }

    log_pty_kill_failure("client close", pty.kill().await);
    log_socket_close_failure("client close", socket.close().await);
    false
}

async fn forward_pty_output(
    socket: &mut WebSocket,
    pty: &mut crate::pty::PtyHandle,
    crypt: &crate::state::SessionCrypto,
    maybe_output: Option<Vec<u8>>,
    output_stream_open: &mut bool,
) -> bool {
    let Some(output) = maybe_output else {
        *output_stream_open = false;
        return true;
    };

    let iv = rand::random::<[u8; 12]>();
    let Ok(ciphertext_and_tag) = crypt.encrypt(&iv, &output) else {
        log_pty_kill_failure("pty output encrypt failure", pty.kill().await);
        return false;
    };
    let mut payload = Vec::with_capacity(13 + output.len());
    payload.push(WS_TYPE_PTY_DATA);
    payload.extend_from_slice(&iv);
    payload.extend_from_slice(&ciphertext_and_tag);

    if socket.send(Message::Binary(payload.into())).await.is_err() {
        log_pty_kill_failure("pty output send failure", pty.kill().await);
        return false;
    }

    true
}

async fn forward_pty_exit(
    socket: &mut WebSocket,
    exit_changed: Result<(), tokio::sync::watch::error::RecvError>,
    exit_rx: &tokio::sync::watch::Receiver<Option<crate::contract::PtyExitReason>>,
) -> bool {
    if exit_changed.is_err() {
        return false;
    }

    let Some(reason) = exit_rx.borrow().clone() else {
        return true;
    };
    let reason_json = serde_json::to_string(&reason)
        .unwrap_or_else(|_| r#"{"exitCode":null,"signal":null}"#.to_owned());
    log_socket_send_close_failure(
        "pty exit",
        socket
            .send(Message::Close(Some(CloseFrame {
                code: WS_CLOSE_PTY_EXIT,
                reason: Utf8Bytes::from(reason_json),
            })))
            .await,
    );
    false
}

fn log_pty_kill_failure(context: &str, result: anyhow::Result<()>) {
    if let Err(error) = result {
        eprintln!("[routes] failed to kill PTY after {context}: {error}");
    }
}

fn log_socket_close_failure(context: &str, result: Result<(), axum::Error>) {
    if let Err(error) = result {
        eprintln!("[routes] failed to close socket after {context}: {error}");
    }
}

fn log_socket_send_close_failure(context: &str, result: Result<(), axum::Error>) {
    if let Err(error) = result {
        eprintln!("[routes] failed to send close frame after {context}: {error}");
    }
}
