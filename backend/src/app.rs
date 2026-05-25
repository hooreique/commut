//! Top-level application assembly for the Rust port.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::Router;
use tokio::net::TcpListener;

use crate::{
    crypto::AuthorizedKeySet,
    routes::{RouteDeps, build_router},
    state::AppState,
};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub authorized_public_key_pem: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub host: String,
    pub port: u16,
}

impl RuntimeConfig {
    /// Convert the configured host and port into a socket address.
    ///
    /// # Errors
    ///
    /// Returns an error when the host and port do not form a valid listen
    /// address.
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .with_context(|| format!("invalid listen address {}:{}", self.host, self.port))
    }
}

/// Build the application router and its shared dependencies.
///
/// # Errors
///
/// Returns an error when the configured authorized public key cannot be parsed.
pub fn build_app(config: AppConfig) -> Result<Router> {
    let state = AppState::new();
    let authorized_keys = AuthorizedKeySet::from_public_pem(&config.authorized_public_key_pem)?;
    Ok(build_router(RouteDeps {
        state,
        authorized_keys,
    }))
}

/// Run the HTTP server until shutdown.
///
/// # Errors
///
/// Returns an error when the router cannot be built, the TCP listener cannot be
/// bound, or axum terminates with an error.
pub async fn run(config: AppConfig, runtime: RuntimeConfig) -> Result<()> {
    let app = build_app(config)?;
    let listener = TcpListener::bind(runtime.socket_addr()?)
        .await
        .context("failed to bind TCP listener")?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server exited with error")?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler should install");
        let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler should install");

        tokio::select! {
            _ = sigint.recv() => {}
            _ = sigterm.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("[signal] failed to wait for ctrl-c: {error}");
        }
    }
}
