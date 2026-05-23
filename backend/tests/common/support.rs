//! Shared test support types for the `commut` server specification tests.
//!
//! The helpers in this module wrap the current server implementation with a
//! compact harness for HTTP and WebSocket contract tests.
#![allow(dead_code)]

use aes_gcm::{
    Aes128Gcm,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use futures_util::{SinkExt, StreamExt};
use hkdf::Hkdf;
use p256::{
    PublicKey, SecretKey,
    ecdh::diffie_hellman,
    ecdsa::{SigningKey, signature::Signer},
    elliptic_curve::rand_core::OsRng,
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
};
use rand::random;
use reqwest::{Client, Method, StatusCode};
use sha2::Sha256;
use std::path::PathBuf;
use tokio::time::{Duration, Instant, sleep, timeout};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use commut_rust_spec_tests::{
    app::{AppConfig, build_app},
    contract,
};

/// Plain-text response from `POST /api/ticket`.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketResponse {
    pub id: String,
    pub server_ephemeral_public_key_base64: String,
}

impl TicketResponse {
    /// Parse the plain-text `<id>.<pubkey>` wire body used by `POST /api/ticket`.
    ///
    /// # Errors
    ///
    /// Returns an error when the wire body is not a valid ticket response.
    pub fn parse(input: &str) -> Result<Self> {
        let parsed = contract::parse_ticket_body(input)?;
        Ok(Self {
            id: parsed.id.to_owned(),
            server_ephemeral_public_key_base64: parsed
                .server_ephemeral_public_key_base64
                .to_owned(),
        })
    }
}

/// Plain-text response from `POST /api/salt`.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaltResponse {
    pub id: String,
    pub salt_base64: String,
}

impl SaltResponse {
    /// Parse the plain-text `<id>.<salt>` wire body used by `POST /api/salt`.
    ///
    /// # Errors
    ///
    /// Returns an error when the wire body is not a valid salt response.
    pub fn parse(input: &str) -> Result<Self> {
        let parsed = contract::parse_salt_body(input)?;
        Ok(Self {
            id: parsed.id.to_owned(),
            salt_base64: parsed.salt_base64.to_owned(),
        })
    }
}

/// Supported WebSocket application message kinds.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsAppMessage {
    PtyData {
        iv: [u8; 12],
        ciphertext_and_tag: Vec<u8>,
    },
    Resize {
        cols: u16,
        rows: u16,
        raw_payload: Vec<u8>,
    },
    Unknown {
        message_type: u8,
        raw_payload: Vec<u8>,
    },
}

/// WebSocket close event captured by the test harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WsCloseEvent {
    pub code: u16,
    pub reason: String,
}

/// Test harness for the current server implementation.
///
/// This remains an inherent-method API so tests can exercise the server
/// through its public HTTP and WebSocket surface.
#[derive(Debug, Clone)]
pub struct TestHarness {
    base_url: String,
    client: Client,
    signing_key: SigningKey,
    client_session_crypts:
        std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, ClientSessionCrypto>>>,
    shutdown_tx: std::sync::Arc<std::sync::Mutex<Option<oneshot::Sender<()>>>>,
    server_task: std::sync::Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
}

impl TestHarness {
    /// Boot a fresh server instance configured for tests.
    ///
    /// # Errors
    ///
    /// Returns an error when the test server or HTTP client cannot be created.
    pub async fn spawn() -> Result<Self> {
        let defaults = commut_rust_spec_tests::app::StaticAssetRoots::repo_root_default();
        Self::spawn_with_static_assets(defaults.public_dir, defaults.pages_dir, defaults.dist_dir)
            .await
    }

    /// Boot a fresh server instance configured for tests with explicit static
    /// asset roots.
    ///
    /// # Errors
    ///
    /// Returns an error when the authorized key cannot be encoded, the app
    /// cannot be built, or the local listener/client cannot be created.
    pub async fn spawn_with_static_assets(
        public_dir: PathBuf,
        pages_dir: PathBuf,
        dist_dir: PathBuf,
    ) -> Result<Self> {
        let signing_key = SigningKey::random(&mut OsRng);
        let authorized_public_key_pem = signing_key
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)?;
        let app = build_app(AppConfig {
            authorized_public_key_pem,
            static_assets: commut_rust_spec_tests::app::StaticAssetRoots {
                public_dir,
                pages_dir,
                dist_dir,
            },
        })?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            if shutdown_rx.await.is_err() {
                eprintln!("[support] graceful shutdown channel dropped before signal");
            }
        });
        let server_task = tokio::spawn(async move {
            if let Err(error) = server.await {
                eprintln!("[support] server task exited with error: {error}");
            }
        });

        Ok(Self {
            base_url: format!("http://{address}"),
            client: Client::builder().build()?,
            signing_key,
            client_session_crypts: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            shutdown_tx: std::sync::Arc::new(std::sync::Mutex::new(Some(shutdown_tx))),
            server_task: std::sync::Arc::new(std::sync::Mutex::new(Some(server_task))),
        })
    }

    /// Tear the server down and release all resources.
    ///
    /// # Errors
    ///
    /// Returns an error when the shutdown coordination mutexes are poisoned.
    pub async fn shutdown(self) -> Result<()> {
        let shutdown_tx = self
            .shutdown_tx
            .lock()
            .map_err(|_| anyhow!("poisoned shutdown mutex"))?
            .take();
        if let Some(tx) = shutdown_tx
            && tx.send(()).is_err()
        {
            eprintln!("[support] shutdown signal could not be delivered");
        }

        let server_task = self
            .server_task
            .lock()
            .map_err(|_| anyhow!("poisoned server task mutex"))?
            .take();
        if let Some(task) = server_task
            && let Err(error) = task.await
        {
            eprintln!("[support] server task join failed: {error}");
        }
        Ok(())
    }

    /// Issue a nonce from `POST /api/nonce`.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the server does not return a
    /// successful nonce response.
    pub async fn issue_nonce(&self) -> Result<String> {
        let (status, body) = self
            .request_text(Method::POST, "/api/nonce", String::new())
            .await?;
        ensure_status_ok(status, &body)?;
        Ok(body)
    }

    /// Exchange a signed nonce for a ticket.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the server does not return a
    /// successful ticket response.
    pub async fn exchange_ticket(
        &self,
        nonce: &str,
        signature_base64: &str,
    ) -> Result<TicketResponse> {
        let body = format!("{nonce}.{signature_base64}");
        let (status, response_body) = self.request_text(Method::POST, "/api/ticket", body).await?;
        ensure_status_ok(status, &response_body)?;
        TicketResponse::parse(&response_body)
    }

    /// Return status code and body from `POST /api/ticket` without assuming success.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request itself fails.
    pub async fn exchange_ticket_raw(&self, body: &str) -> Result<(u16, String)> {
        self.request_text(Method::POST, "/api/ticket", body.to_owned())
            .await
    }

    /// Exchange client ephemeral key material for salt.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the server does not return a
    /// successful salt response.
    pub async fn exchange_salt(
        &self,
        id: &str,
        client_ephemeral_public_key_base64: &str,
    ) -> Result<SaltResponse> {
        let body = format!("{id}.{client_ephemeral_public_key_base64}");
        let (status, response_body) = self.request_text(Method::POST, "/api/salt", body).await?;
        ensure_status_ok(status, &response_body)?;
        SaltResponse::parse(&response_body)
    }

    /// Return status code and body from `POST /api/salt` without assuming success.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request itself fails.
    pub async fn exchange_salt_raw(&self, body: &str) -> Result<(u16, String)> {
        self.request_text(Method::POST, "/api/salt", body.to_owned())
            .await
    }

    /// Issue a raw HTTP GET and return status plus UTF-8 body text.
    ///
    /// This is used for static-file and fallback route checks where the spec
    /// defines only the plain HTTP surface.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request fails or the response body cannot
    /// be decoded as UTF-8 text.
    pub async fn get_text(&self, uri: &str) -> Result<(u16, String)> {
        self.request_text(Method::GET, uri, String::new()).await
    }

    /// Issue a raw HTTP GET and return status plus raw response bytes.
    ///
    /// Binary assets such as `favicon.ico` need byte-level checks rather than
    /// lossy UTF-8 decoding.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request fails or the response body cannot
    /// be read.
    pub async fn get_bytes(&self, uri: &str) -> Result<(u16, Vec<u8>)> {
        let response = self
            .client
            .request(Method::GET, format!("{}{}", self.base_url, uri))
            .send()
            .await
            .map_err(|error| anyhow!("request failed: {error}"))?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| anyhow!("failed to read response body: {error}"))?;

        Ok((status.as_u16(), body.to_vec()))
    }

    /// Produce an authorization signature over base64-decoded input bytes.
    ///
    /// This helper stands in for the browser-side ECDSA signing step described
    /// in the spec and current frontend implementation.
    ///
    /// # Errors
    ///
    /// Returns an error when the provided Base64 value cannot be decoded.
    pub fn sign_authorized_base64_value(&self, base64_value: &str) -> Result<String> {
        let message = BASE64_STANDARD.decode(base64_value.as_bytes())?;
        let signature: p256::ecdsa::Signature = self.signing_key.sign(&message);
        Ok(BASE64_STANDARD.encode(signature.to_bytes()))
    }

    /// Generate a deliberately invalid authorization token.
    #[must_use]
    pub fn invalid_signature(&self) -> String {
        BASE64_STANDARD.encode("invalid-stub-signature")
    }

    /// Generate a browser-compatible ephemeral ECDH key pair for tests.
    ///
    /// # Errors
    ///
    /// Returns an error when the generated public key cannot be serialized.
    pub fn generate_client_ephemeral_key_pair(&self) -> Result<ClientEphemeralKeyPair> {
        let private_key = SecretKey::random(&mut OsRng);
        let public_key_spki_base64 =
            BASE64_STANDARD.encode(private_key.public_key().to_public_key_der()?.as_bytes());
        Ok(ClientEphemeralKeyPair {
            public_key_spki_base64,
            private_key,
        })
    }

    /// Complete the full HTTP handshake defined by sections 8.1 to 8.3.
    ///
    /// # Errors
    ///
    /// Returns an error when any handshake step fails or the derived session
    /// crypto cannot be stored.
    pub async fn establish_handshake(&self) -> Result<EstablishedSession> {
        let nonce = self.issue_nonce().await?;
        let signature = self.sign_authorized_base64_value(&nonce)?;
        let ticket = self.exchange_ticket(&nonce, &signature).await?;
        let client_keys = self.generate_client_ephemeral_key_pair()?;
        let salt = self
            .exchange_salt(&ticket.id, &client_keys.public_key_spki_base64)
            .await?;
        let client_session_crypto = derive_client_session_crypto(
            &client_keys.private_key,
            &ticket.server_ephemeral_public_key_base64,
            &salt.salt_base64,
        )?;
        self.client_session_crypts
            .lock()
            .map_err(|_| anyhow!("poisoned session crypto mutex"))?
            .insert(ticket.id.clone(), client_session_crypto);
        let ws_token_base64 = self.sign_authorized_base64_value(&ticket.id)?;

        Ok(EstablishedSession {
            id: ticket.id,
            salt_base64: salt.salt_base64,
            ws_token_base64,
            server_ephemeral_public_key_base64: ticket.server_ephemeral_public_key_base64,
            client_ephemeral_public_key_base64: client_keys.public_key_spki_base64,
        })
    }

    /// Connect to `GET /sockets/:id` with the provided auth token and dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error when the WebSocket connection attempt fails or the
    /// client-side session crypto store is poisoned.
    pub async fn connect_ws(
        &self,
        id: &str,
        token: Option<&str>,
        dimensions: Option<&str>,
    ) -> Result<TestWebSocket> {
        let mut endpoint = self.base_url.replace("http://", "ws://");
        endpoint.push_str("/sockets/");
        endpoint.push_str(&urlencoding::encode(id));

        let mut separator = '?';
        if let Some(token) = token {
            endpoint.push(separator);
            endpoint.push_str("token=");
            endpoint.push_str(&urlencoding::encode(token));
            separator = '&';
        }
        if let Some(dimensions) = dimensions {
            endpoint.push(separator);
            endpoint.push_str("dimensions=");
            endpoint.push_str(&urlencoding::encode(dimensions));
        }

        let (stream, _) = connect_async(endpoint).await?;
        let session_crypto = self
            .client_session_crypts
            .lock()
            .map_err(|_| anyhow!("poisoned session crypto mutex"))?
            .get(id)
            .cloned();
        Ok(TestWebSocket {
            stream,
            session_crypto,
        })
    }

    /// Wait long enough for a nonce issued by this server to expire.
    pub async fn wait_for_nonce_expiry(&self) {
        sleep(Duration::from_millis(contract::NONCE_TTL_MS + 25)).await;
    }

    async fn request_text(&self, method: Method, uri: &str, body: String) -> Result<(u16, String)> {
        let response = self
            .client
            .request(method, format!("{}{}", self.base_url, uri))
            .body(body)
            .send()
            .await
            .map_err(|error| anyhow!("request failed: {error}"))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| anyhow!("failed to read response body: {error}"))?;

        Ok((status.as_u16(), body))
    }
}

/// Client ephemeral key pair used during the `/api/salt` phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientEphemeralKeyPair {
    pub public_key_spki_base64: String,
    pub private_key: SecretKey,
}

/// Fully established session state after the three HTTP handshake steps.
///
/// The test harness can compute more derived fields later, but these are the
/// minimum pieces needed by the WebSocket and crypto tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstablishedSession {
    pub id: String,
    pub salt_base64: String,
    pub ws_token_base64: String,
    pub server_ephemeral_public_key_base64: String,
    pub client_ephemeral_public_key_base64: String,
}

/// Test handle for an authenticated WebSocket session.
///
/// The harness exposes protocol-level operations rather than raw tungstenite or
/// hyper internals.
#[derive(Debug)]
pub struct TestWebSocket {
    stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    session_crypto: Option<ClientSessionCrypto>,
}

impl TestWebSocket {
    /// Send a raw binary WebSocket frame payload.
    ///
    /// # Errors
    ///
    /// Returns an error when the underlying WebSocket send fails.
    pub async fn send_binary(&mut self, payload: Vec<u8>) -> Result<()> {
        self.stream
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                payload.into(),
            ))
            .await?;
        Ok(())
    }

    /// Receive and classify one application-level binary message.
    ///
    /// # Errors
    ///
    /// Returns an error when the WebSocket closes unexpectedly or the received
    /// binary frame does not match the expected protocol shape.
    pub async fn recv_app_message(&mut self) -> Result<WsAppMessage> {
        while let Some(message) = self.stream.next().await {
            match message? {
                tokio_tungstenite::tungstenite::Message::Binary(payload) => {
                    let payload_vec = payload.to_vec();
                    if payload_vec.is_empty() {
                        continue;
                    }
                    match payload_vec[0] {
                        0 => {
                            if payload_vec.len() < 13 {
                                return Err(anyhow!("type 0 payload shorter than 13 bytes"));
                            }
                            let mut iv = [0_u8; 12];
                            iv.copy_from_slice(&payload_vec[1..13]);
                            let ciphertext_and_tag = payload_vec[13..].to_vec();
                            if let Some(session_crypto) = &self.session_crypto {
                                session_crypto.decrypt(&iv, &ciphertext_and_tag)?;
                            }
                            return Ok(WsAppMessage::PtyData {
                                iv,
                                ciphertext_and_tag,
                            });
                        }
                        1 => {
                            let text = String::from_utf8(payload_vec[1..].to_vec())?;
                            let dimensions = contract::parse_dimensions(&text)?;
                            return Ok(WsAppMessage::Resize {
                                cols: dimensions.cols,
                                rows: dimensions.rows,
                                raw_payload: payload_vec,
                            });
                        }
                        message_type => {
                            return Ok(WsAppMessage::Unknown {
                                message_type,
                                raw_payload: payload_vec,
                            });
                        }
                    }
                }
                tokio_tungstenite::tungstenite::Message::Close(Some(close)) => {
                    return Err(anyhow!(
                        "received close frame instead of app message: {}",
                        close.reason
                    ));
                }
                _ => {}
            }
        }

        Err(anyhow!(
            "websocket closed before an application message arrived"
        ))
    }

    /// Receive one decrypted PTY output chunk from a type `0` server frame.
    ///
    /// This exists for end-to-end transcript tests that need to assert on the
    /// browser-visible terminal bytes, not only on encrypted frame structure.
    ///
    /// # Errors
    ///
    /// Returns an error when the WebSocket closes unexpectedly or the frame
    /// cannot be decrypted or parsed.
    pub async fn recv_decrypted_pty_output(&mut self) -> Result<Vec<u8>> {
        while let Some(message) = self.stream.next().await {
            match message? {
                tokio_tungstenite::tungstenite::Message::Binary(payload) => {
                    let payload_vec = payload.to_vec();
                    match contract::parse_ws_frame(&payload_vec)? {
                        contract::WsFrame::PtyData {
                            iv,
                            ciphertext_and_tag,
                        } => {
                            if let Some(session_crypto) = &self.session_crypto {
                                return session_crypto.decrypt(&iv, ciphertext_and_tag);
                            }

                            return Ok(ciphertext_and_tag.to_vec());
                        }
                        contract::WsFrame::Resize { .. } | contract::WsFrame::Unknown { .. } => {}
                    }
                }
                tokio_tungstenite::tungstenite::Message::Close(Some(close)) => {
                    return Err(anyhow!(
                        "received close frame instead of PTY output: {}",
                        close.reason
                    ));
                }
                _ => {}
            }
        }

        Err(anyhow!(
            "websocket closed before a PTY output frame arrived"
        ))
    }

    /// Accumulate decrypted PTY text until every marker appears in order.
    ///
    /// The transcript may include prompts, echoed input, or chunking artifacts.
    /// Tests therefore pin a sequence of distinctive markers rather than an
    /// exact full transcript.
    ///
    /// # Errors
    ///
    /// Returns an error when the timeout elapses, the WebSocket closes, or a
    /// PTY data frame cannot be decrypted.
    pub async fn recv_decrypted_text_until_markers(
        &mut self,
        ordered_markers: &[&str],
        max_wait: Duration,
    ) -> Result<String> {
        let deadline = Instant::now() + max_wait;
        let mut transcript = String::new();
        let mut search_from = 0usize;
        let mut next_marker_index = 0usize;

        while next_marker_index < ordered_markers.len() {
            let now = Instant::now();
            if now >= deadline {
                return Err(anyhow!(
                    "timed out while waiting for marker {:?} in transcript: {}",
                    ordered_markers[next_marker_index],
                    transcript
                ));
            }

            let remaining = deadline - now;
            let chunk = timeout(remaining, self.recv_decrypted_pty_output())
                .await
                .map_err(|_| {
                    anyhow!(
                        "timed out while waiting for marker {:?} in transcript: {}",
                        ordered_markers[next_marker_index],
                        transcript
                    )
                })??;

            transcript.push_str(&String::from_utf8_lossy(&chunk));

            while next_marker_index < ordered_markers.len() {
                let marker = ordered_markers[next_marker_index];
                if let Some(relative_index) = transcript[search_from..].find(marker) {
                    search_from += relative_index + marker.len();
                    next_marker_index += 1;
                } else {
                    break;
                }
            }
        }

        Ok(transcript)
    }

    /// Read the next close event, if any.
    ///
    /// # Errors
    ///
    /// Returns an error when the WebSocket ends without an explicit close frame
    /// or the underlying stream reports an error.
    pub async fn recv_close(&mut self) -> Result<WsCloseEvent> {
        while let Some(message) = self.stream.next().await {
            if let tokio_tungstenite::tungstenite::Message::Close(close) = message? {
                let close = close.unwrap_or(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code:
                        tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: tokio_tungstenite::tungstenite::Utf8Bytes::default(),
                });
                return Ok(WsCloseEvent {
                    code: u16::from(close.code),
                    reason: close.reason.to_string(),
                });
            }
        }
        Err(anyhow!("websocket closed without an explicit close frame"))
    }

    /// Perform a client-initiated close with code `4000`.
    ///
    /// # Errors
    ///
    /// Returns an error when the close frame cannot be sent.
    pub async fn close_normally(&mut self) -> Result<()> {
        self.stream
            .send(tokio_tungstenite::tungstenite::Message::Close(Some(
                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code:
                        tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Library(
                            contract::WS_CLOSE_NORMAL,
                        ),
                    reason: "normal".into(),
                },
            )))
            .await?;
        Ok(())
    }

    /// Send a protocol type `1` resize message using the canonical UTF-8 format.
    ///
    /// # Errors
    ///
    /// Returns an error when the resize frame cannot be sent.
    pub async fn send_resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.send_binary(contract::build_resize_payload(contract::Dimensions {
            cols,
            rows,
        }))
        .await
    }

    /// Send a protocol type `0` PTY data message.
    ///
    /// The harness is expected to encrypt this in the same way as the browser
    /// once the real crypto code exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the plaintext cannot be encrypted or the frame
    /// cannot be sent.
    pub async fn send_encrypted_pty_input(&mut self, plaintext: &[u8]) -> Result<()> {
        let iv = random_iv();
        let ciphertext_and_tag = if let Some(session_crypto) = &self.session_crypto {
            session_crypto.encrypt(&iv, plaintext)?
        } else {
            plaintext.to_vec()
        };
        let mut payload = Vec::with_capacity(13 + ciphertext_and_tag.len());
        payload.push(0);
        payload.extend_from_slice(&iv);
        payload.extend_from_slice(&ciphertext_and_tag);
        self.send_binary(payload).await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClientSessionCrypto {
    encrypt_key_bytes: [u8; 16],
    decrypt_key_bytes: [u8; 16],
}

impl ClientSessionCrypto {
    fn encrypt(&self, iv: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes128Gcm::new_from_slice(&self.encrypt_key_bytes)?;
        cipher
            .encrypt(iv.into(), plaintext)
            .map_err(|_| anyhow!("aes-gcm encrypt failure"))
    }

    fn decrypt(&self, iv: &[u8; 12], ciphertext_and_tag: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes128Gcm::new_from_slice(&self.decrypt_key_bytes)?;
        cipher
            .decrypt(iv.into(), ciphertext_and_tag)
            .map_err(|_| anyhow!("aes-gcm decrypt failure"))
    }
}

fn derive_client_session_crypto(
    client_private_key: &SecretKey,
    server_public_key_base64: &str,
    salt_base64: &str,
) -> Result<ClientSessionCrypto> {
    let server_public_key_der = BASE64_STANDARD.decode(server_public_key_base64.as_bytes())?;
    let server_public_key = PublicKey::from_public_key_der(&server_public_key_der)?;
    let salt = BASE64_STANDARD.decode(salt_base64.as_bytes())?;
    let shared_secret = diffie_hellman(
        client_private_key.to_nonzero_scalar(),
        server_public_key.as_affine(),
    );
    let hkdf = Hkdf::<Sha256>::new(Some(&salt), shared_secret.raw_secret_bytes().as_slice());
    Ok(ClientSessionCrypto {
        encrypt_key_bytes: derive_aes128_key(&hkdf, contract::HKDF_INFO_UP)?,
        decrypt_key_bytes: derive_aes128_key(&hkdf, contract::HKDF_INFO_DOWN)?,
    })
}

fn derive_aes128_key(hkdf: &Hkdf<Sha256>, info: &[u8]) -> Result<[u8; 16]> {
    let mut key = [0_u8; 16];
    hkdf.expand(info, &mut key)
        .map_err(|_| anyhow!("hkdf expand failure"))?;
    Ok(key)
}

fn random_iv() -> [u8; 12] {
    random::<[u8; 12]>()
}

fn ensure_status_ok(status: u16, body: &str) -> Result<()> {
    if status == StatusCode::OK.as_u16() {
        Ok(())
    } else {
        Err(anyhow!("expected 200 OK, got {status} with body: {body}"))
    }
}
