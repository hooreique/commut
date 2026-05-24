use std::env;

use anyhow::{Context, Result};
use commut::{
    app::{AppConfig, RuntimeConfig, run},
    runtime::{load_authorized_public_key_pem, load_static_asset_roots, parse_cli_args},
};

#[tokio::main]
async fn main() -> Result<()> {
    parse_cli_args()?;
    let runtime = RuntimeConfig {
        host: env::var("COMMUT_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
        port: env::var("COMMUT_PORT")
            .ok()
            .map(|value| value.parse())
            .transpose()
            .context("COMMUT_PORT must be a valid u16")?
            .unwrap_or(3000),
    };

    let authorized_public_key_pem = load_authorized_public_key_pem()?;
    let config = AppConfig {
        authorized_public_key_pem,
        static_assets: load_static_asset_roots()?,
    };

    run(config, runtime).await
}
