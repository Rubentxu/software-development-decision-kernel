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

/// Extract the project version from a parsed `Cargo.toml`.
///
/// Precedence is fixed and total, and it is NOT "the first `version` key under
/// any table whose name starts with `[workspace`" — that is what the
/// hand-rolled line parser did, and it read `[workspace.dependencies]`
/// (RED measured, session-65i: `left: "9.9.9" / right: "1.42.5"`), because
/// that table also starts with `[workspace`. The failure was silent: a
/// confident number that described a dependency.
///
/// The order below is the one Cargo itself resolves with, most specific
/// first. `[package]` is the single-crate case; a workspace overrides it.
fn extract_project_version(doc: &toml::Value) -> Option<String> {
    let str_at = |table: Option<&toml::Value>, key: &str| -> Option<String> {
        table?
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };

    // 1. `[workspace.package] version` — the workspace declaration.
    if let Some(v) = str_at(
        doc.get("workspace").and_then(|w| w.get("package")),
        "version",
    ) {
        return Some(v);
    }
    // 2. `[workspace] version` — accepted by this codebase's own tests and by
    //    older workspaces, where the key sat directly in the table.
    if let Some(v) = str_at(doc.get("workspace"), "version") {
        return Some(v);
    }
    // 3. `[package] version` — a single-crate repository has no workspace
    //    table at all. Without this, the mirror image of the original bug:
    //    the lockstep would abort on a project whose version is right there.
    if let Some(v) = str_at(doc.get("package"), "version") {
        return Some(v);
    }
    None
}

/// Ensure the release tag matches the project version (lockstep rule).
///
/// The lockstep rule: `version tag == project version`. Tags use the "v"
/// prefix (e.g. "v1.42.5") while the manifest uses plain "1.42.5".
///
/// Returns `Ok(())` if the tag matches. Returns `Err(VersionLockstepError)` naming
/// BOTH the project and tag versions on mismatch.
///
/// ## Why this is not stack-agnostic yet
///
/// This still reads `Cargo.toml`, so a non-Rust project aborts. That is
/// INC-DEBT-051, and it is a missing *contract*, not a missing branch: the
/// concept "where a project declares its version" does not exist anywhere in
/// the engine. Closing it means introducing that concept, not teaching this
/// function about more file formats. Adding a second format here would be
/// the same mistake as a third `cp -r` in the release staging (INC-DEBT-056)
/// — a hard-coded list of places to look, each with its own failure mode.
pub fn ensure_version_lockstep(
    root: &std::path::Path,
    tag: &str,
) -> Result<(), VersionLockstepError> {
    let cargo_toml = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).map_err(|e| VersionLockstepError {
        workspace_version: String::new(),
        tag_version: tag.to_string(),
        message: format!(
            "VERSION LOCKSTEP ERROR: could not read {}: {e}",
            cargo_toml.display()
        ),
    })?;

    let doc: toml::Value = content.parse().map_err(|e| VersionLockstepError {
        workspace_version: String::new(),
        tag_version: tag.to_string(),
        message: format!(
            "VERSION LOCKSTEP ERROR: could not parse {} as TOML: {e}",
            cargo_toml.display()
        ),
    })?;

    let workspace_version = extract_project_version(&doc).ok_or_else(|| VersionLockstepError {
        workspace_version: String::new(),
        tag_version: tag.to_string(),
        message: format!(
            "VERSION LOCKSTEP ERROR: could not find `version` in [workspace.package], \
             [workspace] or [package] of {}",
            cargo_toml.display()
        ),
    })?;

    let tag_version = tag.strip_prefix('v').unwrap_or(tag).to_string();
    if workspace_version != tag_version {
        let msg = format!(
            "VERSION LOCKSTEP FAILED: project={} vs tag={}. \
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
    /// RED measured (session-65i). The lockstep parser matched table headers
    /// with `starts_with("[workspace")`, which also matches
    /// `[workspace.dependencies]`. When that table precedes
    /// `[workspace.package]` in the file — legal TOML, and the order Cargo
    /// itself emits when dependencies are declared first — the parser read the
    /// `version` key of a DEPENDENCY and compared the tag against it.
    ///
    /// The failure was silent: a confident verdict built from the wrong
    /// number. A tag equal to `1.42.5` was REJECTED because the parser saw
    /// `9.9.9`, and a tag equal to `9.9.9` would have been ACCEPTED — a
    /// release authorised on a value describing a dependency.
    ///
    /// The assertion is on the verdict, not on the parsed string. An earlier
    /// draft of this test forced `unwrap_err()` and then inspected
    /// `err.workspace_version`, which couples the test to the failure path:
    /// with the fix in place the lockstep correctly PASSES here, and the test
    /// failed for that reason. The property worth pinning is the one a caller
    /// observes — the project's own version decides.
    #[test]
    fn lockstep_uses_the_project_version_not_a_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("Cargo.toml")).unwrap();
        // `[workspace.dependencies]` FIRST, so a prefix match on the table
        // name would pick up 9.9.9 before ever reaching 1.42.5.
        writeln!(f, "[workspace.dependencies]").unwrap();
        writeln!(f, "version = \"9.9.9\"").unwrap();
        writeln!(f, "[workspace.package]").unwrap();
        writeln!(f, "version = \"1.42.5\"").unwrap();

        // The project's version: the tag is accepted.
        ensure_version_lockstep(dir.path(), "v1.42.5")
            .expect("the tag matches the PROJECT version, not the dependency's");

        // The dependency's version: the tag is refused. This is the half that
        // matters — a release must never be authorised on 9.9.9.
        let err = ensure_version_lockstep(dir.path(), "v9.9.9")
            .expect_err("a dependency's version must never authorise a release");
        assert_eq!(err.workspace_version, "1.42.5");
    }

    /// RED measured (session-65i). A single-quoted string is valid TOML —
    /// `version = '1.42.5'` parses identically to the double-quoted form —
    /// and the hand-rolled line parser only ever stripped `"`. A project
    /// using single quotes aborted its release on a file Cargo itself
    /// accepts.
    #[test]
    fn lockstep_accepts_a_single_quoted_version() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("Cargo.toml")).unwrap();
        writeln!(f, "[workspace.package]").unwrap();
        writeln!(f, "version = '1.42.5'").unwrap();
        ensure_version_lockstep(dir.path(), "v1.42.5")
            .expect("a single-quoted version is valid TOML and must be honoured");
    }

    /// RED measured (session-65i, falsification). A malformed `Cargo.toml`
    /// must be reported AS A PARSE ERROR, not as "this project declares no
    /// version". Those are different facts: the first says the file is
    /// broken, the second says the project has nothing to compare.
    ///
    /// Falsified by degrading the parse to
    /// `unwrap_or(Table::default())` — a change that turns a typed error
    /// naming the file and the cause into a confident "no version here",
    /// and every existing test stayed green. That is the same substitution
    /// as `prompts_count = 0` and `is_empty()`: a failure wearing the
    /// costume of an absence, indistinguishable from the real thing.
    #[test]
    fn lockstep_reports_a_parse_failure_as_such() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("Cargo.toml")).unwrap();
        // Unterminated table header: not valid TOML.
        writeln!(f, "[workspace.package").unwrap();
        writeln!(f, "version = \"1.42.5\"").unwrap();
        let err = ensure_version_lockstep(dir.path(), "v1.42.5").unwrap_err();
        assert!(
            err.message.contains("could not parse"),
            "a malformed manifest must be named as a PARSE failure, not reported \
             as a missing version. Got: {}",
            err.message
        );
        assert!(
            err.message.contains("Cargo.toml"),
            "the error must name the file it could not parse: {}",
            err.message
        );
    }

    /// The mirror of the above: a well-formed file that genuinely declares no
    /// version is a DIFFERENT error, and conflating the two is what the
    /// mutation above exploited. Pinned so the two can never be merged back
    /// into one message.
    #[test]
    fn lockstep_distinguishes_a_missing_version_from_a_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("Cargo.toml")).unwrap();
        writeln!(f, "[workspace]").unwrap();
        writeln!(f, "members = []").unwrap();
        let err = ensure_version_lockstep(dir.path(), "v1.0.0").unwrap_err();
        assert!(
            err.message.contains("could not find `version`"),
            "a valid file with no version is a different fact from a broken one. Got: {}",
            err.message
        );
    }

    /// The same table-prefix confusion, in the shape Cargo emits for a
    /// non-workspace (single-crate) repository. A `[package]` table is not a
    /// workspace table, so a project version declared there must not be
    /// silently ignored — nor must the absence of one be read as "no
    /// version at all" when the workspace table does carry it.
    #[test]
    fn lockstep_does_not_confuse_package_and_workspace_tables() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = std::fs::File::create(dir.path().join("Cargo.toml")).unwrap();
        writeln!(f, "[package]").unwrap();
        writeln!(f, "name = \"demo\"").unwrap();
        writeln!(f, "version = \"1.42.5\"").unwrap();
        writeln!(f, "[workspace]").unwrap();
        writeln!(f, "members = []").unwrap();
        // A single-crate repo declares its version under [package]; refusing
        // here would be the mirror image of the original bug.
        let result = ensure_version_lockstep(dir.path(), "v1.42.5");
        assert!(
            result.is_ok(),
            "a single-crate repo declares version under [package]; the lockstep \
             must honour it rather than abort (message: {:?})",
            result.err().map(|e| e.message)
        );
    }
}
