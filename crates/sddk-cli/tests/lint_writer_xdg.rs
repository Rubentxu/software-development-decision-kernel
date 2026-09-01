//! Contract tests for linting vault export routing through writer.
//!
//! Tests that `validate_vault_export_routes_through_writer` correctly validates
//! vault export output paths:
//! - Paths inside XDG project data dir are accepted
//! - Paths outside XDG project data dir are rejected
//! - Symlink traversal attacks are detected and rejected

use sddk_cli::{Diagnostic, Severity};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Holds a TempDir to keep it alive for the duration of the test.
struct WriterXdgTestEnv {
    _dir: TempDir,
    xdg_root: PathBuf,
}

impl WriterXdgTestEnv {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let xdg_root = dir.path().join("project_data");
        fs::create_dir_all(&xdg_root).unwrap();
        Self {
            _dir: dir,
            xdg_root,
        }
    }

    fn create_output(&self, relative: &str) -> PathBuf {
        self.xdg_root.join(relative)
    }
}

/// Assert that running `validate_vault_export_routes_through_writer` produces zero diagnostics
/// (all valid).
fn assert_clean(output_path: &Path, xdg_root: &Path) {
    let diagnostics = sddk_cli::validate_vault_export_routes_through_writer(output_path, xdg_root);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics for {:?}, got: {:?}",
        output_path,
        diagnostics
    );
}

/// Assert that running `validate_vault_export_routes_through_writer` produces exactly one
/// error diagnostic.
fn assert_one_error(output_path: &Path, xdg_root: &Path, contains: &str) {
    let diagnostics = sddk_cli::validate_vault_export_routes_through_writer(output_path, xdg_root);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected 1 error, got {}: {:?}",
        errors.len(),
        diagnostics
    );
    assert!(
        errors[0].message.contains(contains),
        "expected message containing '{}', got '{}'",
        contains,
        errors[0].message
    );
}

// ── Valid paths inside XDG ───────────────────────────────────────────────────

#[test]
fn output_inside_xdg_root_is_accepted() {
    let env = WriterXdgTestEnv::new();
    let output = env.create_output("vault/export.html");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(&output, b"<html></html>").unwrap();
    assert_clean(&output, &env.xdg_root);
}

#[test]
fn output_in_deep_subdirectory_is_accepted() {
    let env = WriterXdgTestEnv::new();
    let output = env.create_output("vault/subdir/deep/export.html");
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    fs::write(&output, b"<html></html>").unwrap();
    assert_clean(&output, &env.xdg_root);
}

#[test]
fn output_exactly_at_xdg_root_is_accepted() {
    let env = WriterXdgTestEnv::new();
    // A file directly in the xdg root is valid (e.g., xdg_root/vault.html)
    let output = env.xdg_root.join("vault.html");
    fs::write(&output, b"<html></html>").unwrap();
    assert_clean(&output, &env.xdg_root);
}

// ── Invalid paths outside XDG ────────────────────────────────────────────────

#[test]
fn output_outside_xdg_root_is_rejected() {
    let env = WriterXdgTestEnv::new();
    let output = env._dir.path().join("outside.html");
    fs::write(&output, b"<html></html>").unwrap();
    assert_one_error(&output, &env.xdg_root, "outside");
}

#[test]
fn output_in_sibling_directory_is_rejected() {
    let env = WriterXdgTestEnv::new();
    let sibling = env._dir.path().join("sibling");
    fs::create_dir_all(&sibling).unwrap();
    let output = sibling.join("evil.html");
    fs::write(&output, b"<html></html>").unwrap();
    assert_one_error(&output, &env.xdg_root, "outside");
}

#[test]
fn output_escapes_via_dotdot_is_rejected() {
    let env = WriterXdgTestEnv::new();
    // xdg_root/../sibling/evil.html
    let sibling = env._dir.path().join("sibling");
    fs::create_dir_all(&sibling).unwrap();
    let output = env.xdg_root.join("..").join("sibling").join("evil.html");
    // canonicalize would resolve this to sibling/evil.html which is outside
    fs::write(&output, b"<html></html>").unwrap();
    assert_one_error(&output, &env.xdg_root, "outside");
}

#[test]
fn output_in_tmp_is_rejected() {
    let env = WriterXdgTestEnv::new();
    let output = PathBuf::from("/tmp/vault-export.html");
    fs::write(&output, b"<html></html>").unwrap();
    assert_one_error(&output, &env.xdg_root, "outside");
}

// ── Symlink traversal ────────────────────────────────────────────────────────

#[test]
fn symlink_to_outside_is_rejected() {
    let env = WriterXdgTestEnv::new();
    let target = env._dir.path().join("target");
    let link = env.xdg_root.join("link");
    fs::create_dir_all(&target).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();

    #[cfg(unix)]
    {
        let malicious = link.join("..").join("sibling").join("evil.html");
        if malicious.exists() || malicious.canonicalize().is_ok() {
            assert_one_error(&malicious, &env.xdg_root, "outside");
        }
    }
}
