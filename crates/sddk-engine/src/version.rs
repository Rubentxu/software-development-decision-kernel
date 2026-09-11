//! Version module — exposes the crate version as a stable compile-time constant
//! and the workspace-vs-tag lockstep rule used by the release pipeline.
//!
//! The version is read from `env!("CARGO_PKG_VERSION")` which is set at compile
//! time from the `version` field in `Cargo.toml`. For the workspace version
//! (`version.workspace = true`), this resolves to the workspace-level version.
//!
//! The lockstep rule (`ensure_version_lockstep`) was extracted from
//! `sddk-cli::release_cmd` per INC-024 (god-class smell at 906 LOC; now ~1820).
//! Keeping the lockstep check in the engine substrate makes it reusable from
//! any caller (CLI today; future daemon / CI gate tomorrow).

/// Returns the SDDK engine version string (e.g. `"1.42.5"`).
///
/// This function is useful for runtime version reporting where a `&'static str`
/// is needed rather than the const value.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Typed error for version lockstep failures — names both workspace and tag versions.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VersionLockstepError {
    pub workspace_version: String,
    pub tag_version: String,
    pub message: String,
}

impl std::fmt::Display for VersionLockstepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for VersionLockstepError {}

/// Ensure the release tag matches the workspace Cargo.toml version (lockstep rule).
///
/// The lockstep rule: `version tag == workspace Cargo.toml version`.
/// Tags use the "v" prefix (e.g. "v1.42.5") while Cargo.toml uses plain "1.42.5".
///
/// Returns `Ok(())` if the tag matches. Returns `Err(VersionLockstepError)` naming
/// BOTH workspace and tag versions on mismatch.
pub fn ensure_version_lockstep(
    root: &std::path::Path,
    tag: &str,
) -> Result<(), VersionLockstepError> {
    // Read the workspace version from the root Cargo.toml
    let cargo_toml = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).map_err(|e| VersionLockstepError {
        workspace_version: String::new(),
        tag_version: tag.to_string(),
        message: format!(
            "VERSION LOCKSTEP ERROR: could not read {}: {e}",
            cargo_toml.display()
        ),
    })?;
    // Extract `version = "X.Y.Z"` from [workspace] or [workspace.package] section
    let workspace_version = {
        let mut in_workspace = false;
        let mut version = None;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_workspace = trimmed.starts_with("[workspace");
            } else if in_workspace
                && let Some(v) = trimmed.strip_prefix("version").and_then(|rest| {
                    let rest = rest.trim();
                    if !rest.starts_with('=') {
                        return None;
                    }
                    let rest = rest[1..].trim();
                    rest.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                })
            {
                version = Some(v.to_string());
                break;
            }
        }
        version
    }
    .ok_or_else(|| VersionLockstepError {
        workspace_version: String::new(),
        tag_version: tag.to_string(),
        message: format!(
            "VERSION LOCKSTEP ERROR: could not find `version` in [workspace] or [workspace.package] section of {}",
            cargo_toml.display()
        ),
    })?;

    let tag_version = tag.strip_prefix('v').unwrap_or(tag).to_string();
    if workspace_version != tag_version {
        let msg = format!(
            "VERSION LOCKSTEP FAILED: workspace={} vs tag={}. \
             Release planning refused until the lockstep rule is satisfied.",
            workspace_version, tag_version
        );
        return Err(VersionLockstepError {
            workspace_version,
            tag_version,
            message: msg,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_workspace(dir: &std::path::Path, version_line: &str) {
        let mut f = std::fs::File::create(dir.join("Cargo.toml")).unwrap();
        writeln!(f, "[workspace]").unwrap();
        writeln!(f, "{version_line}").unwrap();
        writeln!(f, "[workspace.package]").unwrap();
        writeln!(f, "edition = \"2024\"").unwrap();
    }

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

    #[test]
    fn lockstep_passes_when_tag_matches_workspace_version() {
        let dir = tempfile::tempdir().unwrap();
        write_workspace(dir.path(), "version = \"1.42.5\"");
        ensure_version_lockstep(dir.path(), "v1.42.5").expect("tag matches workspace");
    }

    #[test]
    fn lockstep_passes_when_tag_has_no_v_prefix() {
        let dir = tempfile::tempdir().unwrap();
        write_workspace(dir.path(), "version = \"1.42.5\"");
        ensure_version_lockstep(dir.path(), "1.42.5").expect("tag without v prefix matches");
    }

    #[test]
    fn lockstep_fails_when_tag_diverges_from_workspace() {
        let dir = tempfile::tempdir().unwrap();
        write_workspace(dir.path(), "version = \"1.42.5\"");
        let err = ensure_version_lockstep(dir.path(), "v1.99.0").unwrap_err();
        assert_eq!(err.workspace_version, "1.42.5");
        assert_eq!(err.tag_version, "1.99.0"); // 'v' prefix stripped
        assert!(err.message.contains("LOCKSTEP FAILED"));
    }

    #[test]
    fn lockstep_errors_when_cargo_toml_missing() {
        let dir = tempfile::tempdir().unwrap();
        let err = ensure_version_lockstep(dir.path(), "v1.0.0").unwrap_err();
        assert!(err.message.contains("could not read"));
    }

    #[test]
    fn lockstep_errors_when_version_key_absent_in_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("Cargo.toml")).unwrap();
        writeln!(f, "[workspace]").unwrap();
        writeln!(f, "members = []").unwrap();
        let err = ensure_version_lockstep(dir.path(), "v1.0.0").unwrap_err();
        assert!(err.message.contains("could not find `version`"));
    }
}
