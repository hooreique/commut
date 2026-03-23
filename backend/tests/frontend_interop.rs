//! Frontend-driven interoperability checks.
//!
//! These tests intentionally pin the Rust port to the existing browser client,
//! not just to the Rust server tests in the abstract.
//!
//! The goal is to catch accidental drift between:
//! - `frontend/src/connect.ts`
//! - `frontend/src/protocol.ts`
//! - the Rust contract and runtime behavior

mod common;

use anyhow::Result;
use commut_rust_spec_tests::contract::{
    DEFAULT_COLS, DEFAULT_ROWS, HKDF_INFO_DOWN, HKDF_INFO_UP, WS_CLOSE_NORMAL, WS_TYPE_PTY_DATA,
    WS_TYPE_RESIZE,
};
use tokio::time::Duration;

use common::spawn_harness;

const CONNECT_TS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../frontend/src/connect.ts"
));
const PROTOCOL_TS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../frontend/src/protocol.ts"
));
const ROUTES_RS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/routes.rs"));

#[test]
fn frontend_source_constants_match_the_rust_contract() {
    // Frontend coverage:
    // - `frontend/src/connect.ts` hard-codes the HKDF info strings
    // - `frontend/src/connect.ts` hard-codes the initial dimensions query values
    // - `frontend/src/connect.ts` closes the socket with code 4000
    // - `frontend/src/protocol.ts` hard-codes message lead bytes 0 and 1
    //
    // Why this exists:
    // The Rust tests already prove the server-side contract. This
    // extra source lock makes frontend wire drift fail fast as soon as someone
    // edits the browser code without updating the Rust contract.
    assert!(
        CONNECT_TS.contains("te.encode('client -> server')"),
        "frontend connect flow must continue to use the client->server HKDF info label"
    );
    assert_eq!(HKDF_INFO_UP, b"client -> server");

    assert!(
        CONNECT_TS.contains("te.encode('server -> client')"),
        "frontend connect flow must continue to use the server->client HKDF info label"
    );
    assert_eq!(HKDF_INFO_DOWN, b"server -> client");

    assert!(
        CONNECT_TS.contains("smallInit() ? '40,16' : '100,30'"),
        "frontend connect flow must continue to send the documented initial dimensions query"
    );
    assert_eq!((DEFAULT_COLS, DEFAULT_ROWS), (100, 30));

    assert!(
        CONNECT_TS.contains("close: () => ws.close(4000)"),
        "frontend close path must continue to use code 4000"
    );
    assert_eq!(WS_CLOSE_NORMAL, 4000);

    assert!(
        PROTOCOL_TS.contains("cat[0] = 0;"),
        "frontend protocol writer must continue to use lead byte 0 for encrypted PTY data"
    );
    assert_eq!(WS_TYPE_PTY_DATA, 0);

    assert!(
        PROTOCOL_TS.contains("cat[0] = 1;"),
        "frontend resize writer must continue to use lead byte 1"
    );
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
async fn frontend_small_init_dimensions_reach_the_real_pty() -> Result<()> {
    // Frontend coverage:
    // - `frontend/src/connect.ts` sends `dimensions=40,16` when `smallInit()` is true
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
        "small-init frontend dimensions should reach the PTY, got transcript: {transcript:?}"
    );

    ws.close_normally().await?;
    harness.shutdown().await
}
