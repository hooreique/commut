//! HTTP and handshake contract tests for the server's public surface.

mod common;

use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use common::{spawn_harness, support::TestHarness};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend crate should live directly under the repository root")
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("commut-{name}-{unique}"))
}

fn base64_of_zeroes(len: usize) -> String {
    BASE64_STANDARD.encode(vec![0_u8; len])
}

fn expected_backend_source_digest() -> Result<String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hasher = Sha256::new();

    for path in backend_source_files(manifest_dir)? {
        let relative_path = normalized_backend_path(&path, manifest_dir);
        let contents = fs::read(path)?;

        hasher.update(relative_path.as_bytes());
        hasher.update([0]);
        hasher.update(contents.len().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(contents);
        hasher.update([0]);
    }

    Ok(BASE64_STANDARD.encode(hasher.finalize()))
}

fn backend_source_files(manifest_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![
        manifest_dir.join("Cargo.toml"),
        manifest_dir.join("Cargo.lock"),
        manifest_dir.join("build.rs"),
    ];
    collect_rust_sources(&manifest_dir.join("src"), &mut files)?;
    files.sort_by_key(|path| normalized_backend_path(path, manifest_dir));
    Ok(files)
}

fn collect_rust_sources(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_rust_sources(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }

    Ok(())
}

fn normalized_backend_path(path: &Path, manifest_dir: &Path) -> String {
    path.strip_prefix(manifest_dir)
        .expect("backend source path should be inside backend crate")
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

async fn spawn_harness_with_static_assets(
    public_dir: PathBuf,
    pages_dir: PathBuf,
    dist_dir: PathBuf,
) -> Result<TestHarness> {
    TestHarness::spawn_with_static_assets(public_dir, pages_dir, dist_dir).await
}

#[tokio::test]
async fn build_info_endpoint_returns_version_and_embedded_source_digest() -> Result<()> {
    // Contract coverage:
    // - `GET /api/build-info` returns plaintext `<version> <digest>`
    // - digest is the build-time SHA-256 of the backend source and dependency lock inputs
    let harness = spawn_harness().await?;

    let (status, body) = harness.get_text("/api/build-info").await?;

    assert_eq!(status, 200, "response body: {body}");
    assert_eq!(
        body,
        format!(
            "{} {}",
            env!("CARGO_PKG_VERSION"),
            commut::build_info::BACKEND_SOURCE_DIGEST
        )
    );
    assert_eq!(body.matches(' ').count(), 1);

    let (_, digest) = body
        .split_once(' ')
        .expect("build-info body should contain one space separator");
    assert_eq!(digest, commut::build_info::BACKEND_SOURCE_DIGEST);
    assert_eq!(digest, expected_backend_source_digest()?);
    assert_eq!(
        BASE64_STANDARD.decode(digest.as_bytes())?.len(),
        32,
        "digest should be a Base64-encoded SHA-256 value"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn static_favicon_is_served_from_public_root() -> Result<()> {
    // Contract coverage:
    // - static assets are served from the configured public root
    let harness = spawn_harness().await?;

    let (status, body) = harness.get_bytes("/favicon.ico").await?;
    let expected = fs::read(repo_root().join("frontend/public/favicon.ico"))?;

    assert_eq!(status, 200);
    assert_eq!(
        body, expected,
        "favicon bytes should match the public asset"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn static_manifest_pages_and_dist_assets_are_served_from_their_specified_roots() -> Result<()>
{
    // Contract coverage:
    // - static assets are served from the configured public, pages, and dist roots
    let root = unique_temp_dir("static-assets");
    let public_dir = root.join("public");
    let pages_dir = root.join("build");
    let dist_dir = root.join("dist");
    fs::create_dir_all(&public_dir)?;
    fs::create_dir_all(pages_dir.join("app"))?;
    fs::create_dir_all(&dist_dir)?;
    fs::write(public_dir.join("manifest.json"), "{\"name\":\"fixture\"}\n")?;
    fs::write(pages_dir.join("app/index.html"), "<!doctype html>\n")?;
    fs::write(
        dist_dir.join("app.12345678.mjs"),
        "console.log('fixture dist');\n",
    )?;

    let harness =
        spawn_harness_with_static_assets(public_dir.clone(), pages_dir.clone(), dist_dir.clone())
            .await?;

    let (manifest_status, manifest_body) = harness.get_text("/manifest.json").await?;
    let (page_status, page_body) = harness.get_text("/app/index.html").await?;
    let (dist_status, dist_body) = harness.get_text("/dist/app.12345678.mjs").await?;

    assert_eq!(manifest_status, 200);
    assert_eq!(page_status, 200);
    assert_eq!(dist_status, 200);
    assert_eq!(
        manifest_body,
        fs::read_to_string(public_dir.join("manifest.json"))?
    );
    assert_eq!(
        page_body,
        fs::read_to_string(pages_dir.join("app/index.html"))?
    );
    assert_eq!(
        dist_body,
        fs::read_to_string(dist_dir.join("app.12345678.mjs"))?
    );

    harness.shutdown().await?;
    if let Err(error) = fs::remove_dir_all(root) {
        eprintln!("[api_spec] failed to remove temp static-assets dir: {error}");
    }
    Ok(())
}

#[tokio::test]
async fn unknown_routes_return_plain_404_not_found() -> Result<()> {
    // Contract coverage:
    // - unknown routes return plain `404 Not Found`
    let harness = spawn_harness().await?;

    let (status, body) = harness.get_text("/definitely-missing").await?;

    assert_eq!(status, 404);
    assert_eq!(body, "404 Not Found");

    harness.shutdown().await
}

#[tokio::test]
async fn nonce_endpoint_returns_base64_and_stores_it() -> Result<()> {
    // Contract coverage:
    // - `POST /api/nonce` returns a Base64 nonce
    // - issued nonce is accepted by `POST /api/ticket`
    //
    // Acceptance intent:
    // 1. call POST /api/nonce
    // 2. assert the response is a base64 string
    // 3. prove the nonce was stored by successfully exchanging it in /api/ticket
    let harness = spawn_harness().await?;

    let nonce = harness.issue_nonce().await?;
    BASE64_STANDARD.decode(nonce.as_bytes())?;

    let signature = harness.sign_authorized_base64_value(&nonce)?;
    harness.exchange_ticket(&nonce, &signature).await?;

    harness.shutdown().await
}

#[tokio::test]
async fn issued_nonces_expire_after_the_specified_ttl() -> Result<()> {
    // Contract coverage:
    // - nonce validity is bounded by `contract::NONCE_TTL_MS`
    // - expired nonce behaves as missing in `POST /api/ticket`
    let harness = spawn_harness().await?;

    let nonce = harness.issue_nonce().await?;
    harness.wait_for_nonce_expiry().await;

    let signature = harness.sign_authorized_base64_value(&nonce)?;
    let (status, body) = harness
        .exchange_ticket_raw(&format!("{nonce}.{signature}"))
        .await?;

    assert_eq!(status, 500);
    assert!(
        body.contains("nonce"),
        "expected an error body mentioning nonce expiration or absence, got: {body}"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn ticket_endpoint_rejects_invalid_signatures() -> Result<()> {
    // Contract coverage:
    // - `POST /api/ticket` rejects invalid signatures
    // - application failures are returned as plain-text 500 responses
    let harness = spawn_harness().await?;

    let nonce = harness.issue_nonce().await?;
    let bad_signature = harness.invalid_signature();
    let (status, body) = harness
        .exchange_ticket_raw(&format!("{nonce}.{bad_signature}"))
        .await?;

    assert_eq!(status, 500);
    assert!(
        body.contains("wrong signature") || body.contains("signature"),
        "expected signature failure text, got: {body}"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn successful_ticket_exchange_consumes_the_nonce_exactly_once() -> Result<()> {
    // Contract coverage:
    // - nonce state is consumed exactly once on successful ticket exchange
    let harness = spawn_harness().await?;

    let nonce = harness.issue_nonce().await?;
    let signature = harness.sign_authorized_base64_value(&nonce)?;

    harness.exchange_ticket(&nonce, &signature).await?;
    let (status, body) = harness
        .exchange_ticket_raw(&format!("{nonce}.{signature}"))
        .await?;

    assert_eq!(status, 500);
    assert!(
        body.contains("nonce"),
        "expected a consumed nonce to behave as missing, got: {body}"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn ticket_response_contains_id_and_server_ephemeral_public_key() -> Result<()> {
    // Contract coverage:
    // - successful `POST /api/ticket` returns `<id>.<server-ephemeral-public-key-base64>`
    let harness = spawn_harness().await?;

    let nonce = harness.issue_nonce().await?;
    let signature = harness.sign_authorized_base64_value(&nonce)?;
    let ticket = harness.exchange_ticket(&nonce, &signature).await?;

    assert!(!ticket.id.is_empty(), "ticket id must not be empty");
    assert!(
        BASE64_STANDARD
            .decode(ticket.server_ephemeral_public_key_base64.as_bytes())
            .is_ok(),
        "server ephemeral public key must be syntactically valid base64"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn salt_endpoint_rejects_unknown_checkpoint_ids() -> Result<()> {
    // Contract coverage:
    // - `POST /api/salt` rejects unknown checkpoint ids
    let harness = spawn_harness().await?;

    let client_keys = harness.generate_client_ephemeral_key_pair()?;
    let unknown_id = base64_of_zeroes(8);
    let (status, body) = harness
        .exchange_salt_raw(&format!(
            "{unknown_id}.{}",
            client_keys.public_key_spki_base64
        ))
        .await?;

    assert_eq!(status, 500);
    assert!(
        body.contains("checkpoint"),
        "expected a checkpoint lookup failure, got: {body}"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn successful_salt_exchange_consumes_checkpoint_and_returns_salt() -> Result<()> {
    // Contract coverage:
    // - successful `POST /api/salt` returns `<id>.<salt-base64>`
    // - checkpoint state is consumed exactly once
    let harness = spawn_harness().await?;

    let nonce = harness.issue_nonce().await?;
    let signature = harness.sign_authorized_base64_value(&nonce)?;
    let ticket = harness.exchange_ticket(&nonce, &signature).await?;
    let client_keys = harness.generate_client_ephemeral_key_pair()?;

    let salt = harness
        .exchange_salt(&ticket.id, &client_keys.public_key_spki_base64)
        .await?;

    let raw_salt = BASE64_STANDARD.decode(salt.salt_base64.as_bytes())?;
    assert_eq!(salt.id, ticket.id);
    assert_eq!(raw_salt.len(), 32, "salt must be exactly 32 random bytes");

    let (status, body) = harness
        .exchange_salt_raw(&format!(
            "{}.{}",
            ticket.id, client_keys.public_key_spki_base64
        ))
        .await?;

    assert_eq!(status, 500);
    assert!(
        body.contains("checkpoint"),
        "successful /api/salt must consume the checkpoint entry, got: {body}"
    );

    harness.shutdown().await
}
