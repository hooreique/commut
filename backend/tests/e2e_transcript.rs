//! Browser-style end-to-end transcript coverage.
//!
//! These tests intentionally drive the full public path:
//! - HTTP nonce/ticket/salt handshake
//! - authenticated WebSocket upgrade
//! - encrypted type `0` PTY writes
//! - encrypted type `0` PTY output reads
//! - PTY-exit close semantics
//!
//! The assertions are transcript-oriented rather than frame-oriented. This
//! keeps the test aligned with the route and protocol comments in `src/routes.rs`
//! and `src/contract.rs`.

mod common;

use anyhow::Result;
use commut::contract::WS_CLOSE_PTY_EXIT;
use serde::Deserialize;
use tokio::time::Duration;

use common::spawn_harness;

#[derive(Debug, Deserialize)]
struct TranscriptFixture {
    initial_dimensions: String,
    ordered_markers: Vec<String>,
    commands: Vec<String>,
    exit_command: String,
}

fn fixture() -> TranscriptFixture {
    serde_json::from_str(include_str!("fixtures/session_transcript.json"))
        .expect("fixture json must stay valid")
}

#[tokio::test]
async fn browser_style_handshake_and_interactive_session_follow_the_expected_transcript()
-> Result<()> {
    // Contract coverage:
    // - the full nonce/ticket/salt handshake precedes WebSocket upgrade
    // - interactive encrypted PTY traffic works end-to-end
    // - PTY exit closes the WebSocket with code `4001`
    let fixture = fixture();
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(
            &session.id,
            Some(&session.ws_token_base64),
            Some(&fixture.initial_dimensions),
        )
        .await?;

    // Send the transcript writes without waiting between them. This exercises
    // the server's client->PTY ordering path rather than relying on the test to
    // serialize every round trip.
    for command in &fixture.commands {
        ws.send_encrypted_pty_input(command.as_bytes()).await?;
    }

    let ordered_markers = fixture
        .ordered_markers
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let transcript = ws
        .recv_decrypted_text_until_markers(&ordered_markers, Duration::from_secs(5))
        .await?;

    // The transcript may contain prompts and echoed input, but the distinctive
    // markers from the fixture must appear in order to prove the visible
    // terminal conversation matches the expected round trip.
    let mut previous_end = 0usize;
    for marker in &fixture.ordered_markers {
        let marker_index = transcript[previous_end..]
            .find(marker)
            .expect("all markers should have been observed in order");
        previous_end += marker_index + marker.len();
    }

    ws.send_encrypted_pty_input(fixture.exit_command.as_bytes())
        .await?;
    let close = ws.recv_close().await?;

    assert_eq!(close.code, WS_CLOSE_PTY_EXIT);
    assert!(
        close.reason.contains("exitCode") || close.reason.contains("signal"),
        "close reason should carry PTY exit details, got: {}",
        close.reason
    );

    harness.shutdown().await
}
