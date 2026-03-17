use std::{
    fs,
    net::TcpListener,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Result, anyhow, bail};
use p256::{
    ecdsa::SigningKey,
    elliptic_curve::rand_core::OsRng,
    pkcs8::{EncodePublicKey, LineEnding},
};
use tokio::time::{Instant, sleep};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend crate should live directly under the repository root")
}

fn reserve_loopback_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

#[tokio::test]
async fn debug_run_builds_and_serves_client_assets_by_default() -> Result<()> {
    let signing_key = SigningKey::random(&mut OsRng);
    let public_key_pem = signing_key
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)?;
    let port = reserve_loopback_port()?;
    let base_url = format!("http://127.0.0.1:{port}");

    let mut child = Command::new(env!("CARGO_BIN_EXE_commut"))
        .current_dir(repo_root())
        .env("COMMUT_HOST", "127.0.0.1")
        .env("COMMUT_PORT", port.to_string())
        .env("COMMUT_AUTHORIZED_PUBLIC_KEY_PEM", public_key_pem)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| anyhow!("failed to spawn commut binary: {error}"))?;

    let client = reqwest::Client::builder().build()?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let expected_manifest = fs::read_to_string(repo_root().join("frontend/public/manifest.json"))?;

    loop {
        if let Some(status) = child.try_wait()? {
            let output = child.wait_with_output()?;
            bail!(
                "commut exited early with status {status}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let manifest = client.get(format!("{base_url}/manifest.json")).send().await;
        let build = client.get(format!("{base_url}/build/app.mjs")).send().await;

        if let (Ok(manifest), Ok(build)) = (manifest, build)
            && manifest.status().as_u16() == 200
            && build.status().as_u16() == 200
        {
            let manifest_body = manifest.text().await?;
            let build_body = build.text().await?;
            assert_eq!(manifest_body, expected_manifest);
            assert!(
                !build_body.trim().is_empty(),
                "built frontend asset should not be empty"
            );
            break;
        }

        if Instant::now() >= deadline {
            bail!("timed out waiting for commut to serve built frontend assets");
        }

        sleep(Duration::from_secs(1)).await;
    }

    if child.try_wait()?.is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }

    Ok(())
}
