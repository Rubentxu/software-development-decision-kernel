//! Version module — exposes the crate version as a stable compile-time constant.
//!
//! The version is read from `env!("CARGO_PKG_VERSION")` which is set at compile
//! time from the `version` field in `Cargo.toml`. For the workspace version
//! (`version.workspace = true`), this resolves to the workspace-level version.

/// Returns the SDDK engine version string (e.g. `"1.42.5"`).
///
/// This function is useful for runtime version reporting where a `&'static str`
/// is needed rather than the const value.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn version_matches_cargo_pkg_version() {
        // CARGO_PKG_VERSION is set at compile time; this test verifies the
        // const and the macro resolve to the same value.
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
