use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as base64_standard;
use commut_rust_spec_tests::support::TestHarness;
use std::path::PathBuf;

/// Produce syntactically valid base64 bytes for negative-path tests.
///
/// This helper is intentionally shared across integration test files, so it may
/// be unused in a subset of test binaries.
#[allow(dead_code)]
pub fn base64_of_zeroes(len: usize) -> String {
    base64_standard.encode(vec![0u8; len])
}

/// Spawn a fresh ignored-test harness.
///
/// Keeping this helper in one place makes it easy to add common setup later,
/// such as loading fixture keys or enabling verbose tracing for ignored runs.
pub async fn spawn_harness() -> Result<TestHarness> {
    TestHarness::spawn().await
}

#[allow(dead_code)]
pub async fn spawn_harness_with_static_assets(
    public_dir: PathBuf,
    build_dir: PathBuf,
) -> Result<TestHarness> {
    TestHarness::spawn_with_static_assets(public_dir, build_dir).await
}
