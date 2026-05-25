//! Backend-owned browser interoperability checks.
//!
//! The backend contract is the source of truth for browser wire behavior.
//! These tests pin the documented backend spec and runtime effects clients must
//! mirror. They intentionally do not read client source files.

mod common;

use anyhow::Result;
use commut::contract::{
    DEFAULT_COLS, DEFAULT_ROWS, HKDF_INFO_DOWN, HKDF_INFO_UP, WS_CLOSE_NORMAL, WS_TYPE_PTY_DATA,
    WS_TYPE_RESIZE,
};
use tokio::time::Duration;

use common::spawn_harness;

const ROUTES_RS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/routes.rs"));

#[test]
fn backend_contract_constants_match_the_browser_wire_spec() {
    // Browser clients must mirror the protocol summary in `src/contract.rs`.
    assert_eq!(HKDF_INFO_UP, b"client -> server");
    assert_eq!(HKDF_INFO_DOWN, b"server -> client");
    assert_eq!((DEFAULT_COLS, DEFAULT_ROWS), (100, 30));
    assert_eq!(WS_CLOSE_NORMAL, 4000);
    assert_eq!(WS_TYPE_PTY_DATA, 0);
    assert_eq!(WS_TYPE_RESIZE, 1);
}

#[test]
fn backend_socket_input_path_does_not_rewrite_terminal_bytes_or_intercept_exit() {
    // Backend coverage:
    // - decrypted PTY input must be forwarded to the PTY without CR/LF rewriting
    // - the WebSocket layer must not special-case `exit`
    assert!(
        !ROUTES_RS.contains("normalize_terminal_input"),
        "backend socket input path must not normalize carriage returns before PTY write"
    );
    assert!(
        !ROUTES_RS.contains("is_exit_command"),
        "backend socket input path must not intercept `exit` before the PTY sees it"
    );
    assert!(
        ROUTES_RS.contains("pty.write(&decrypted).await"),
        "backend socket input path should write decrypted PTY bytes directly"
    );
}

#[tokio::test]
async fn compact_initial_dimensions_reach_the_real_pty() -> Result<()> {
    // Browser compatibility coverage:
    // - clients may request compact initial dimensions with `dimensions=40,16`
    //
    // Spec coverage:
    // - section 9.3: initial dimensions come from the query parameter
    // - section 10.1: the PTY should be spawned with those dimensions
    //
    // This test avoids shell-specific prompts. Instead it asks the shell for
    // its visible PTY size and waits for the deterministic `rows cols` output.
    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("40,16"))
        .await?;

    ws.send_encrypted_pty_input(b"stty size\r").await?;
    let transcript = ws
        .recv_decrypted_text_until_markers(&["16 40"], Duration::from_secs(5))
        .await?;

    assert!(
        transcript.contains("16 40"),
        "compact initial dimensions should reach the PTY, got transcript: {transcript:?}"
    );

    ws.close_normally().await?;
    harness.shutdown().await
}
