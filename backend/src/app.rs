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
    pub pages_dir: PathBuf,
    pub dist_dir: PathBuf,
}

impl StaticAssetRoots {
    /// Resolve the repository-local frontend asset directories from the
    /// current working directory.
    #[must_use]
    pub fn repo_root_default() -> Self {
        let repo_root = find_repo_root_from_current_dir().unwrap_or_else(current_dir_fallback);

        Self {
            public_dir: repo_root.join("frontend/public"),
            pages_dir: repo_root.join("frontend/build"),
            dist_dir: repo_root.join("frontend/dist"),
        }
    }

    /// Build static asset roots from explicit overrides plus repository
    /// defaults.
    #[must_use]
    pub fn with_overrides(
        public_dir: Option<PathBuf>,
        pages_dir: Option<PathBuf>,
        dist_dir: Option<PathBuf>,
    ) -> Self {
        let defaults = Self::repo_root_default();

        Self {
            public_dir: public_dir.unwrap_or(defaults.public_dir),
            pages_dir: pages_dir.unwrap_or(defaults.pages_dir),
            dist_dir: dist_dir.unwrap_or(defaults.dist_dir),
        }
    }
}

fn find_repo_root_from_current_dir() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    current_dir
        .ancestors()
        .find(|path| path.join("backend/Cargo.toml").is_file() && path.join("frontend").is_dir())
        .map(PathBuf::from)
}

fn current_dir_fallback() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
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
        assert!(roots.pages_dir.ends_with(Path::new("frontend/build")));
        assert!(roots.dist_dir.ends_with(Path::new("frontend/dist")));
    }

    #[test]
    fn static_asset_roots_allow_partial_overrides() {
        let roots =
            StaticAssetRoots::with_overrides(Some(PathBuf::from("/tmp/public")), None, None);

        assert_eq!(roots.public_dir, PathBuf::from("/tmp/public"));
        assert!(roots.pages_dir.ends_with(Path::new("frontend/build")));
        assert!(roots.dist_dir.ends_with(Path::new("frontend/dist")));
    }
}
