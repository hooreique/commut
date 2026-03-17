use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use commut_rust_spec_tests::{
    app::{AppConfig, RuntimeConfig, StaticAssetRoots, run},
    runtime::{
        build_frontend_assets, load_authorized_public_key_pem, parse_cli_args,
        should_build_frontend_for_run,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_cli_args()?;
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
    let explicit_public_dir = env::var_os("COMMUT_PUBLIC_DIR").map(PathBuf::from);
    let explicit_build_dir = env::var_os("COMMUT_BUILD_DIR").map(PathBuf::from);
    let built_frontend_root = if should_build_frontend_for_run(
        cli,
        explicit_public_dir.is_some(),
        explicit_build_dir.is_some(),
    ) {
        Some(build_frontend_assets()?)
    } else {
        None
    };

    let config = AppConfig {
        authorized_public_key_pem,
        static_assets: StaticAssetRoots::with_overrides(
            explicit_public_dir
                .or_else(|| built_frontend_root.as_ref().map(|path| path.join("public"))),
            explicit_build_dir
                .or_else(|| built_frontend_root.as_ref().map(|path| path.join("build"))),
        ),
    };

    run(config, runtime).await
}
