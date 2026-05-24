use std::{
    fs,
    net::TcpListener,
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, anyhow};
use p256::{
    ecdsa::SigningKey,
    elliptic_curve::rand_core::OsRng,
    pkcs8::{EncodePublicKey, LineEnding},
};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend crate should live directly under the repository root")
}

fn reserve_loopback_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn authorized_public_key_pem() -> Result<String> {
    let signing_key = SigningKey::random(&mut OsRng);
    Ok(signing_key
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)?)
}

#[test]
fn run_errors_with_frontend_asset_guidance_when_dirs_are_missing() -> Result<()> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock should be after unix epoch")
        .as_nanos();
    let missing_root = std::env::temp_dir().join(format!("commut-missing-assets-{unique}"));

    let output = Command::new(env!("CARGO_BIN_EXE_commut"))
        .current_dir(repo_root())
        .env("COMMUT_HOST", "127.0.0.1")
        .env("COMMUT_PORT", reserve_loopback_port()?.to_string())
        .env(
            "COMMUT_AUTHORIZED_PUBLIC_KEY_PEM",
            authorized_public_key_pem()?,
        )
        .env("COMMUT_PUBLIC_DIR", missing_root.join("public"))
        .env("COMMUT_BUILD_DIR", missing_root.join("build"))
        .env("COMMUT_DIST_DIR", missing_root.join("dist"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| anyhow!("failed to spawn commut binary: {error}"))?;

    assert!(!output.status.success(), "commut should exit with an error");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("frontend assets are not ready"));
    assert!(stderr.contains("COMMUT_PUBLIC_DIR"));
    assert!(stderr.contains("COMMUT_BUILD_DIR"));
    assert!(stderr.contains("COMMUT_DIST_DIR"));
    assert!(stderr.contains("pnpm --dir frontend install"));
    assert!(stderr.contains("pnpm --dir frontend run build"));

    Ok(())
}

#[test]
fn run_errors_with_font_guidance_when_public_fonts_have_no_woff2() -> Result<()> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock should be after unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("commut-empty-fonts-{unique}"));
    let public_dir = root.join("public");
    let build_dir = root.join("build");
    let dist_dir = root.join("dist");
    fs::create_dir_all(public_dir.join("fonts"))?;
    fs::create_dir_all(&build_dir)?;
    fs::create_dir_all(&dist_dir)?;
    fs::write(public_dir.join("fonts/not-a-font.txt"), "not a font")?;

    let output = Command::new(env!("CARGO_BIN_EXE_commut"))
        .current_dir(repo_root())
        .env("COMMUT_HOST", "127.0.0.1")
        .env("COMMUT_PORT", reserve_loopback_port()?.to_string())
        .env(
            "COMMUT_AUTHORIZED_PUBLIC_KEY_PEM",
            authorized_public_key_pem()?,
        )
        .env("COMMUT_PUBLIC_DIR", &public_dir)
        .env("COMMUT_BUILD_DIR", &build_dir)
        .env("COMMUT_DIST_DIR", &dist_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| anyhow!("failed to spawn commut binary: {error}"))?;

    assert!(!output.status.success(), "commut should exit with an error");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("frontend fonts are not ready"));
    assert!(stderr.contains("Expected at least one .woff2 file"));
    assert!(stderr.contains("nix run .#cp-fonts frontend/public/fonts"));

    fs::remove_dir_all(root)?;

    Ok(())
}
