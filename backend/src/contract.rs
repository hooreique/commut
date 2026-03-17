//! Pure protocol and contract helpers for the backend wire format.
//!
//! These helpers are intentionally implementation-agnostic:
//! - they do not start a server
//! - they do not perform network I/O
//! - they encode and decode the wire shapes the server exposes
//!
//! Keeping these rules in ordinary Rust code makes them easy to unit test and
//! gives the HTTP/WebSocket handlers a single place to share parsing logic with
//! the spec tests.
//!
//! Protocol summary:
//! - HTTP handshake request/response bodies use dot-separated plain-text pairs
//! - WebSocket binary type `0` is encrypted PTY data
//! - WebSocket binary type `1` is plaintext resize data
//! - invalid or missing initial dimensions fall back to `100,30`
//! - close code `4000` is normal client close
//! - close code `4001` indicates PTY exit

use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};

/// Nonces issued by `POST /api/nonce` remain valid for 3000 ms.
pub const NONCE_TTL_MS: u64 = 3_000;

/// Invalid or missing initial dimensions fall back to this pair.
pub const DEFAULT_COLS: u16 = 100;
pub const DEFAULT_ROWS: u16 = 30;

/// HKDF `info` label for traffic from browser to server.
pub const HKDF_INFO_UP: &[u8] = b"client -> server";

/// HKDF `info` label for traffic from server to browser.
pub const HKDF_INFO_DOWN: &[u8] = b"server -> client";

/// Normal client-initiated close code.
pub const WS_CLOSE_NORMAL: u16 = 4000;

/// Server close code used when the PTY exits first.
pub const WS_CLOSE_PTY_EXIT: u16 = 4001;

/// Known binary message type for encrypted PTY data.
pub const WS_TYPE_PTY_DATA: u8 = 0;

/// Known binary message type for plaintext resize.
pub const WS_TYPE_RESIZE: u8 = 1;

/// Positive PTY dimensions accepted by the protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub cols: u16,
    pub rows: u16,
}

impl Dimensions {
    pub const DEFAULT: Self = Self {
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
    };
}

/// Parsed `POST /api/ticket` success body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketBody<'a> {
    pub id: &'a str,
    pub server_ephemeral_public_key_base64: &'a str,
}

/// Parsed `POST /api/salt` success body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaltBody<'a> {
    pub id: &'a str,
    pub salt_base64: &'a str,
}

/// Parsed WebSocket application message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsFrame<'a> {
    PtyData {
        iv: [u8; 12],
        ciphertext_and_tag: &'a [u8],
    },
    Resize {
        dimensions: Dimensions,
        raw_text: &'a str,
    },
    Unknown {
        message_type: u8,
        payload: &'a [u8],
    },
}

/// PTY exit details carried in the close reason for code `4001`.
///
/// The close reason uses camelCase keys
/// `{ "exitCode": ..., "signal": ... }` to match the wire format documented in
/// the source comments in this module and `routes.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyExitReason {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
}

/// Split a plain-text request or response body into exactly two non-empty
/// dot-separated fields.
///
/// # Errors
///
/// Returns an error when the input is missing a separator, contains empty
/// members, or contains more than one separator.
pub fn split_dot_pair(input: &str) -> Result<(&str, &str)> {
    let (left, right) = input
        .split_once('.')
        .ok_or_else(|| anyhow!("expected exactly one dot-separated pair"))?;

    if left.is_empty() || right.is_empty() {
        bail!("dot-separated pair members must not be empty");
    }

    if right.contains('.') {
        bail!("expected exactly one dot-separated pair");
    }

    Ok((left, right))
}

/// Parse the success body returned by `POST /api/ticket`.
///
/// # Errors
///
/// Returns an error when the body is not a valid two-part dot-separated pair.
pub fn parse_ticket_body(input: &str) -> Result<TicketBody<'_>> {
    let (id, server_ephemeral_public_key_base64) = split_dot_pair(input)?;
    Ok(TicketBody {
        id,
        server_ephemeral_public_key_base64,
    })
}

/// Parse the success body returned by `POST /api/salt`.
///
/// # Errors
///
/// Returns an error when the body is not a valid two-part dot-separated pair.
pub fn parse_salt_body(input: &str) -> Result<SaltBody<'_>> {
    let (id, salt_base64) = split_dot_pair(input)?;
    Ok(SaltBody { id, salt_base64 })
}

/// Parse `<cols>,<rows>` or return the protocol default dimensions.
#[must_use]
pub fn parse_dimensions_or_default(input: Option<&str>) -> Dimensions {
    input
        .and_then(|raw| parse_dimensions(raw).ok())
        .unwrap_or(Dimensions::DEFAULT)
}

/// Parse strictly positive PTY dimensions from `<cols>,<rows>`.
///
/// # Errors
///
/// Returns an error when the input is malformed, contains non-numeric values,
/// extra fields, or zero dimensions.
pub fn parse_dimensions(input: &str) -> Result<Dimensions> {
    let mut parts = input.split(',');

    let cols = parts
        .next()
        .ok_or_else(|| anyhow!("missing cols"))?
        .parse::<u16>()
        .map_err(|_| anyhow!("invalid cols"))?;
    let rows = parts
        .next()
        .ok_or_else(|| anyhow!("missing rows"))?
        .parse::<u16>()
        .map_err(|_| anyhow!("invalid rows"))?;

    if parts.next().is_some() {
        bail!("too many dimension parts");
    }

    if cols == 0 || rows == 0 {
        bail!("dimensions must be positive");
    }

    Ok(Dimensions { cols, rows })
}

/// Build the normalized binary payload for a successful type `1` resize echo.
#[must_use]
pub fn build_resize_payload(dimensions: Dimensions) -> Vec<u8> {
    let mut payload = Vec::with_capacity(1 + 11);
    payload.push(WS_TYPE_RESIZE);
    payload.extend_from_slice(format!("{},{}", dimensions.cols, dimensions.rows).as_bytes());
    payload
}

/// Parse a binary WebSocket application frame.
///
/// Type `0` is parsed as `message-type || 12-byte-iv || ciphertext-and-tag`.
/// Type `1` is parsed as `message-type || utf8("<cols>,<rows>")`.
///
/// # Errors
///
/// Returns an error when the payload is empty, a known frame type is malformed,
/// or a resize payload is not valid UTF-8 dimensions text.
pub fn parse_ws_frame(payload: &[u8]) -> Result<WsFrame<'_>> {
    let (&message_type, rest) = payload
        .split_first()
        .ok_or_else(|| anyhow!("WebSocket payload must not be empty"))?;

    match message_type {
        WS_TYPE_PTY_DATA => {
            if rest.len() < 12 {
                bail!("type 0 frame must contain a 12-byte IV");
            }

            let (iv_bytes, ciphertext_and_tag) = rest.split_at(12);
            let iv: [u8; 12] = iv_bytes
                .try_into()
                .map_err(|_| anyhow!("type 0 frame IV must be 12 bytes"))?;

            Ok(WsFrame::PtyData {
                iv,
                ciphertext_and_tag,
            })
        }
        WS_TYPE_RESIZE => {
            let raw_text = std::str::from_utf8(rest)
                .map_err(|_| anyhow!("type 1 payload must be valid UTF-8"))?;
            let dimensions = parse_dimensions(raw_text)?;
            Ok(WsFrame::Resize {
                dimensions,
                raw_text,
            })
        }
        other => Ok(WsFrame::Unknown {
            message_type: other,
            payload: rest,
        }),
    }
}

/// Parse the JSON close reason used with WebSocket close code `4001`.
///
/// # Errors
///
/// Returns an error when the close reason is not valid JSON in the expected
/// shape.
pub fn parse_pty_exit_reason(input: &str) -> Result<PtyExitReason> {
    Ok(serde_json::from_str(input)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_dot_pair_accepts_exactly_two_non_empty_parts() {
        let (left, right) = split_dot_pair("abc.def").expect("valid pair");
        assert_eq!(left, "abc");
        assert_eq!(right, "def");
    }

    #[test]
    fn split_dot_pair_rejects_missing_or_extra_parts() {
        assert!(split_dot_pair("abc").is_err());
        assert!(split_dot_pair(".def").is_err());
        assert!(split_dot_pair("abc.").is_err());
        assert!(split_dot_pair("a.b.c").is_err());
    }

    #[test]
    fn parse_ticket_and_salt_bodies_preserve_the_two_wire_fields() {
        let ticket = parse_ticket_body("ticket-id.pubkey-b64").expect("valid ticket body");
        assert_eq!(ticket.id, "ticket-id");
        assert_eq!(ticket.server_ephemeral_public_key_base64, "pubkey-b64");

        let salt = parse_salt_body("ticket-id.salt-b64").expect("valid salt body");
        assert_eq!(salt.id, "ticket-id");
        assert_eq!(salt.salt_base64, "salt-b64");
    }

    #[test]
    fn parse_dimensions_accepts_positive_integer_pairs_only() {
        assert_eq!(
            parse_dimensions("100,30").expect("valid dimensions"),
            Dimensions {
                cols: 100,
                rows: 30
            }
        );

        assert!(parse_dimensions("0,30").is_err());
        assert!(parse_dimensions("100,0").is_err());
        assert!(parse_dimensions("100").is_err());
        assert!(parse_dimensions("100,30,1").is_err());
        assert!(parse_dimensions("100,nope").is_err());
    }

    #[test]
    fn parse_dimensions_or_default_falls_back_as_the_spec_requires() {
        assert_eq!(parse_dimensions_or_default(None), Dimensions::DEFAULT);
        assert_eq!(
            parse_dimensions_or_default(Some("100,nope")),
            Dimensions::DEFAULT
        );
        assert_eq!(
            parse_dimensions_or_default(Some("120,50")),
            Dimensions {
                cols: 120,
                rows: 50
            }
        );
    }

    #[test]
    fn resize_payload_builder_matches_the_wire_format() {
        let payload = build_resize_payload(Dimensions {
            cols: 120,
            rows: 40,
        });
        assert_eq!(payload, b"\x01120,40");
    }

    #[test]
    fn parse_ws_frame_decodes_type_zero_frames() {
        let payload = b"\x00abcdefghijklrest";
        let frame = parse_ws_frame(payload).expect("valid type 0 frame");

        match frame {
            WsFrame::PtyData {
                iv,
                ciphertext_and_tag,
            } => {
                assert_eq!(&iv, b"abcdefghijkl");
                assert_eq!(ciphertext_and_tag, b"rest");
            }
            other => panic!("expected type 0 frame, got {other:?}"),
        }
    }

    #[test]
    fn parse_ws_frame_decodes_type_one_frames() {
        let payload = b"\x01120,40";
        let frame = parse_ws_frame(payload).expect("valid type 1 frame");

        match frame {
            WsFrame::Resize {
                dimensions,
                raw_text,
            } => {
                assert_eq!(
                    dimensions,
                    Dimensions {
                        cols: 120,
                        rows: 40
                    }
                );
                assert_eq!(raw_text, "120,40");
            }
            other => panic!("expected type 1 frame, got {other:?}"),
        }
    }

    #[test]
    fn parse_ws_frame_keeps_unknown_message_types_non_fatal() {
        let payload = b"\xffignored payload";
        let frame = parse_ws_frame(payload).expect("unknown frame should still parse");

        assert_eq!(
            frame,
            WsFrame::Unknown {
                message_type: 0xff,
                payload: b"ignored payload",
            }
        );
    }

    #[test]
    fn parse_pty_exit_reason_accepts_the_current_camel_case_json_shape() {
        let reason = parse_pty_exit_reason(r#"{"exitCode":0,"signal":null}"#)
            .expect("valid PTY exit reason");
        assert_eq!(
            reason,
            PtyExitReason {
                exit_code: Some(0),
                signal: None,
            }
        );
    }
}
