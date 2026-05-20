use anyhow::Result;

pub mod support;

use support::TestHarness;

/// Spawn a fresh ignored-test harness.
///
/// Keeping this helper in one place makes it easy to add common setup later,
/// such as loading fixture keys or enabling verbose tracing for ignored runs.
pub async fn spawn_harness() -> Result<TestHarness> {
    TestHarness::spawn().await
}
