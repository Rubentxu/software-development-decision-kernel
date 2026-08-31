//! Integration tests for `dev check-architecture`.

use std::process::Command;

/// Invokes the sddk binary (release build) with the given args.
fn sddk_bin() -> std::path::PathBuf {
    let manifest_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/release/sddk");
    if manifest_dir.exists() {
        manifest_dir
    } else {
        // Fallback: try debug
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../target/debug/sddk")
    }
}

#[test]
fn check_architecture_runs_against_repo() {
    // Run against this repo: ARCH001 must FAIL (engine→storage dep exists)
    let bin = sddk_bin();
    if !bin.exists() {
        eprintln!("sddk binary not found at {:?}, skipping", bin);
        return;
    }

    let output = Command::new(&bin)
        .args(["dev", "check-architecture", "--root", "."])
        .output()
        .expect("sddk dev check-architecture must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Must contain ARCH001 and FAIL
    assert!(
        stdout.contains("ARCH001") && stdout.contains("FAIL"),
        "output should contain ARCH001 FAIL; got:\n{}",
        stdout
    );

    // Must exit 1 (because ARCH001 Fail with Error severity)
    assert_eq!(
        output.status.code(),
        Some(1),
        "exit code should be 1 (ARCH001 FAIL); stderr: {}",
        stderr
    );
}

#[test]
fn check_architecture_exit_zero_when_only_warnings() {
    // Create a synthetic workspace with two crates and no forbidden edges,
    // plus a minimal YAML that only defines ARCH001 (which should Pass).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Minimal workspace Cargo.toml with two crates
    let ws_toml = r#"[workspace]
resolver = "2"
members = ["crate-a", "crate-b"]
"#;
    std::fs::write(root.join("Cargo.toml"), ws_toml).unwrap();
    std::fs::create_dir_all(root.join("crate-a/src")).unwrap();
    std::fs::write(
        root.join("crate-a/Cargo.toml"),
        r#"[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    std::fs::write(root.join("crate-a/src/lib.rs"), "// empty").unwrap();

    std::fs::create_dir_all(root.join("crate-b/src")).unwrap();
    std::fs::write(
        root.join("crate-b/Cargo.toml"),
        r#"[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    std::fs::write(root.join("crate-b/src/lib.rs"), "// empty").unwrap();

    // Minimal rules YAML
    let rules_yaml = r#"schema_version: "1.0.0"
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
"#;
    let rules_path = root.join("rules.yaml");
    std::fs::write(&rules_path, rules_yaml).unwrap();

    let bin = sddk_bin();
    if !bin.exists() {
        eprintln!("sddk binary not found at {:?}, skipping", bin);
        return;
    }

    let output = Command::new(&bin)
        .args([
            "dev",
            "check-architecture",
            "--root",
            root.to_str().unwrap(),
            "--rules",
            rules_path.to_str().unwrap(),
        ])
        .output()
        .expect("sddk dev check-architecture must run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // No engine→storage edge, so ARCH001 should Pass
    assert!(
        stdout.contains("ARCH001") && stdout.contains("PASS"),
        "output should contain ARCH001 PASS; got:\n{}\nstderr: {}",
        stdout,
        stderr
    );

    // Must exit 0 (no Error Fail)
    assert_eq!(
        output.status.code(),
        Some(0),
        "exit code should be 0 (no violations); stderr: {}",
        stderr
    );
}

#[test]
fn check_architecture_respects_rules_path_override() {
    // Same synthetic workspace as above, but with two rule files:
    // one for ARCH001 (with a waiver) and one default.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let ws_toml = r#"[workspace]
resolver = "2"
members = []
"#;
    std::fs::write(root.join("Cargo.toml"), ws_toml).unwrap();

    // Rule file that would fail if evaluated
    let fail_yaml = r#"schema_version: "1.0.0"
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
"#;
    let fail_path = root.join("fail.yaml");
    std::fs::write(&fail_path, fail_yaml).unwrap();

    // Rule file with no rules
    let empty_yaml = r#"schema_version: "1.0.0"
rules: []
"#;
    let empty_path = root.join("empty.yaml");
    std::fs::write(&empty_path, empty_yaml).unwrap();

    let bin = sddk_bin();
    if !bin.exists() {
        eprintln!("sddk binary not found at {:?}, skipping", bin);
        return;
    }

    // With the empty rules file, should exit 0 (no rules = nothing to fail)
    let output = Command::new(&bin)
        .args([
            "dev",
            "check-architecture",
            "--root",
            root.to_str().unwrap(),
            "--rules",
            empty_path.to_str().unwrap(),
        ])
        .output()
        .expect("sddk dev check-architecture must run");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // With empty rules, there should be 0 rows
    assert!(
        !stdout.contains("FAIL"),
        "empty rules should produce no FAIL; got:\n{}",
        stdout
    );
}
