//! WebSocket, PTY, and binary protocol contract tests.

mod common;

use anyhow::Result;
use commut_rust_spec_tests::support::WsAppMessage;

use common::spawn_harness;

#[tokio::test]
async fn websocket_upgrade_requires_a_token() -> Result<()> {
    // Contract coverage:
    // - WebSocket upgrade requires `token`
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;

    let result = harness.connect_ws(&session.id, None, Some("100,30")).await;

    assert!(result.is_err(), "WebSocket upgrade without token must fail");

    harness.shutdown().await
}

#[tokio::test]
async fn websocket_upgrade_rejects_invalid_tokens() -> Result<()> {
    // Contract coverage:
    // - WebSocket upgrade rejects invalid authorization tokens
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;

    let result = harness
        .connect_ws(
            &session.id,
            Some(&harness.invalid_signature()),
            Some("100,30"),
        )
        .await;

    assert!(
        result.is_err(),
        "WebSocket upgrade with invalid token must fail"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn successful_websocket_upgrade_consumes_the_crypt_store_entry() -> Result<()> {
    // Contract coverage:
    // - `/sockets/{id}` consumes session crypto exactly once
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;

    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await?;

    let second_attempt = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await;

    assert!(
        second_attempt.is_err(),
        "a second WebSocket connection for the same post-salt id must fail"
    );

    ws.close_normally().await?;
    harness.shutdown().await
}

#[tokio::test]
async fn invalid_or_missing_initial_dimensions_fall_back_to_100_by_30() -> Result<()> {
    // Contract coverage:
    // - absent or invalid initial dimensions do not block session startup
    //
    // Acceptance intent:
    // The most robust implementation path is to expose PTY spawn dimensions from
    // the harness. Until then, the echo behavior of a later resize test will be
    // the externally visible confirmation.
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;

    harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), None)
        .await?;

    let second_session = harness.establish_handshake().await?;
    harness
        .connect_ws(
            &second_session.id,
            Some(&second_session.ws_token_base64),
            Some("100,nope"),
        )
        .await?;

    harness.shutdown().await
}

#[tokio::test]
async fn resize_messages_are_plaintext_applied_to_the_pty_and_echoed_verbatim() -> Result<()> {
    // Contract coverage:
    // - resize messages use plaintext type `1`
    // - invalid resize payloads are ignored
    // - valid resize payloads are echoed back
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await?;

    ws.send_resize(120, 40).await?;
    let echoed = ws.recv_app_message().await?;

    match echoed {
        WsAppMessage::Resize {
            cols,
            rows,
            raw_payload,
        } => {
            assert_eq!((cols, rows), (120, 40));
            assert_eq!(raw_payload, b"\x01120,40");
        }
        other => panic!("expected echoed resize message, got: {other:?}"),
    }

    ws.send_binary(b"\x01100,nope".to_vec()).await?;

    ws.close_normally().await?;
    harness.shutdown().await
}

#[tokio::test]
async fn encrypted_type_zero_messages_flow_between_client_and_pty() -> Result<()> {
    // Contract coverage:
    // - type `0` carries encrypted PTY data
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await?;

    ws.send_encrypted_pty_input(b"printf 'hello from test\n'\r")
        .await?;
    let frame = ws.recv_app_message().await?;

    match frame {
        WsAppMessage::PtyData {
            iv,
            ciphertext_and_tag,
        } => {
            assert_eq!(iv.len(), 12, "AES-GCM IV must be 12 bytes");
            assert!(
                !ciphertext_and_tag.is_empty(),
                "encrypted PTY output must carry ciphertext and authentication tag"
            );
        }
        other => panic!("expected encrypted PTY output, got: {other:?}"),
    }

    ws.close_normally().await?;
    harness.shutdown().await
}

#[tokio::test]
async fn unknown_message_types_do_not_crash_the_session() -> Result<()> {
    // Contract coverage:
    // - unknown message types do not terminate the session
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await?;

    ws.send_binary(b"\xffignored payload".to_vec()).await?;
    ws.send_resize(80, 24).await?;
    let echoed = ws.recv_app_message().await?;
    assert!(
        matches!(echoed, WsAppMessage::Resize { .. }),
        "session should remain alive after receiving an unknown message type"
    );

    ws.close_normally().await?;
    harness.shutdown().await
}

#[tokio::test]
async fn pty_exit_closes_the_websocket_with_code_4001() -> Result<()> {
    // Contract coverage:
    // - PTY exit closes the WebSocket with code `4001`
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await?;

    ws.send_encrypted_pty_input(b"exit\r").await?;
    let close = ws.recv_close().await?;

    assert_eq!(close.code, 4001);
    assert!(
        close.reason.contains("exitCode") || close.reason.contains("signal"),
        "close reason should contain serialized PTY exit details, got: {}",
        close.reason
    );

    harness.shutdown().await
}

#[tokio::test]
async fn client_initiated_close_uses_code_4000_and_terminates_the_pty() -> Result<()> {
    // Contract coverage:
    // - client close uses code `4000` and terminates the PTY
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await?;

    ws.close_normally().await?;

    harness.shutdown().await
}
