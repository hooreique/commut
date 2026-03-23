//! Top-level application assembly for the Rust port.

use std::{net::SocketAddr, path::PathBuf};

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
    pub static_assets: StaticAssetRoots,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAssetRoots {
    pub public_dir: PathBuf,
    pub build_dir: PathBuf,
}

impl StaticAssetRoots {
    /// Resolve the repository-local frontend asset directories.
    ///
    /// # Panics
    ///
    /// Panics if the backend crate is no longer located directly under the
    /// repository root.
    #[must_use]
    pub fn repo_root_default() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = crate_dir
            .parent()
            .expect("backend crate should live directly under the repository root");

        Self {
            public_dir: repo_root.join("frontend/public"),
            build_dir: repo_root.join("frontend/build"),
        }
    }

    /// Build static asset roots from explicit overrides plus repository
    /// defaults.
    #[must_use]
    pub fn with_overrides(public_dir: Option<PathBuf>, build_dir: Option<PathBuf>) -> Self {
        let defaults = Self::repo_root_default();

        Self {
            public_dir: public_dir.unwrap_or(defaults.public_dir),
            build_dir: build_dir.unwrap_or(defaults.build_dir),
        }
    }
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
    Ok(build_router(
        RouteDeps {
            state,
            authorized_keys,
        },
        config.static_assets,
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn static_asset_roots_default_to_the_repository_layout() {
        let roots = StaticAssetRoots::repo_root_default();

        assert!(roots.public_dir.ends_with(Path::new("frontend/public")));
        assert!(roots.build_dir.ends_with(Path::new("frontend/build")));
    }

    #[test]
    fn static_asset_roots_allow_partial_overrides() {
        let roots = StaticAssetRoots::with_overrides(Some(PathBuf::from("/tmp/public")), None);

        assert_eq!(roots.public_dir, PathBuf::from("/tmp/public"));
        assert!(roots.build_dir.ends_with(Path::new("frontend/build")));
    }
}
