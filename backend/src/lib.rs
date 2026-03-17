//! Rust backend crate for the `commut` server.
//!
//! The crate contains both the server implementation and the test support used
//! to verify the HTTP and WebSocket contract documented directly in the source
//! modules.

pub mod app;
pub mod contract;
pub mod crypto;
pub mod error;
pub mod pty;
pub mod routes;
pub mod runtime;
pub mod state;
pub mod support;
