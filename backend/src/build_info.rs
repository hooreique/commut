//! Build-time metadata exposed by `GET /api/build-info`.
//!
//! The digest is computed by `build.rs` from the backend crate's source and
//! dependency lock inputs and embedded in the binary with `cargo:rustc-env`.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BACKEND_SOURCE_DIGEST: &str = env!("COMMUT_BACKEND_SOURCE_DIGEST");

#[must_use]
pub fn current_wire_body() -> String {
    format!("{VERSION} {BACKEND_SOURCE_DIGEST}")
}
