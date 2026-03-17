//! PTY launch contract integration checks.
//!
//! These tests go one step beyond the pure helper tests in `src/pty.rs`.
//! They boot a real authenticated session, let the shell start inside the PTY,
//! and then ask the shell what environment and working directory it actually
//! sees. That keeps the compatibility-sensitive launch contract pinned at the
//! real process boundary on both target Unix platforms.

mod common;

use anyhow::{Result, anyhow};
use tokio::time::Duration;

use common::spawn_harness;

#[tokio::test]
async fn spawned_shell_observes_the_required_working_directory_and_environment() -> Result<()> {
    // Spec coverage:
    // - section 10.1: the PTY shell starts in `$HOME`
    // - section 10.1: PATH must be `$HOME/.nix-profile/bin:/usr/bin`
    // - section 10.1: TERM must be `xterm-256color`
    // - section 10.1: COLORTERM must be `truecolor`
    // - section 10.1: NODE_PTY must be `1`
    //
    // Why this exists:
    // Pure unit tests already pin the launch helper logic, but this test proves
    // that the actual spawned shell observes the same values through the real
    // `portable-pty` path used by the server.
    let home = std::env::var("HOME").map_err(|_| anyhow!("$HOME must exist for PTY tests"))?;
    let expected_pwd_marker = format!("COMMUT_PWD={home}");

    let harness = spawn_harness().await?;
    let session = harness.establish_handshake().await?;
    let mut ws = harness
        .connect_ws(&session.id, Some(&session.ws_token_base64), Some("100,30"))
        .await?;

    ws.send_encrypted_pty_input(
        b"printf 'COMMUT_PWD=%s\\nCOMMUT_TERM=%s\\nCOMMUT_COLORTERM=%s\\nCOMMUT_NODE_PTY=%s\\n' \"$PWD\" \"$TERM\" \"$COLORTERM\" \"$NODE_PTY\"; case \"$PATH\" in *\"$HOME/.nix-profile/bin:/usr/bin\"*) echo COMMUT_PATH_OK;; *) echo COMMUT_PATH_BAD:$PATH;; esac\r",
    )
    .await?;

    let transcript = ws
        .recv_decrypted_text_until_markers(
            &[
                expected_pwd_marker.as_str(),
                "COMMUT_TERM=xterm-256color",
                "COMMUT_COLORTERM=truecolor",
                "COMMUT_NODE_PTY=1",
                "COMMUT_PATH_OK",
            ],
            Duration::from_secs(5),
        )
        .await?;

    assert!(
        transcript.contains(&expected_pwd_marker),
        "spawned shell should start in $HOME, got transcript: {transcript:?}"
    );
    assert!(
        transcript.contains("COMMUT_TERM=xterm-256color"),
        "spawned shell should observe TERM=xterm-256color, got transcript: {transcript:?}"
    );
    assert!(
        transcript.contains("COMMUT_COLORTERM=truecolor"),
        "spawned shell should observe COLORTERM=truecolor, got transcript: {transcript:?}"
    );
    assert!(
        transcript.contains("COMMUT_NODE_PTY=1"),
        "spawned shell should observe NODE_PTY=1, got transcript: {transcript:?}"
    );
    assert!(
        transcript.contains("COMMUT_PATH_OK"),
        "spawned shell should preserve the required PATH segment, got transcript: {transcript:?}"
    );

    ws.close_normally().await?;
    harness.shutdown().await
}
