//! Rust backend crate for the `commut` server.
//!
//! The crate contains the server implementation shared by the executable and
//! integration tests.

pub mod app;
pub mod build_info;
pub mod contract;
pub mod crypto;
pub mod error;
pub mod pty;
pub mod routes;
pub mod runtime;
pub mod state;
