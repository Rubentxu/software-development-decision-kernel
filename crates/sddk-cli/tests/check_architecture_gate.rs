//! Integration tests for `dev check-architecture`.

use std::path::PathBuf;
use std::process::Command;

/// Invokes the real `sddk` binary built by cargo for this test run.
///
/// **Fail-closed (C3l.7).** This used to resolve `<crate>/../../../target/{release,debug}/sddk`
/// and `return` early when the file was absent, reporting `ok` without ever
/// running the gate — an absent toolchain silently produced a green test. That
/// is the same "absence is not a pass" defect C3l.4 fixed elsewhere, and it is
/// observable whenever `CARGO_TARGET_DIR` points outside the repo, which is the
/// case on this machine. `CARGO_BIN_EXE_sddk` is resolved by cargo at compile
/// time and is always the binary under test.
fn sddk_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sddk"))
}

/// The repository root.
///
/// An integration test's CWD is the *package* directory (`crates/sddk-cli`),
/// not the workspace root, so `--root .` silently resolved a non-existent
/// rules file and the gate exited 1 with empty stdout. Resolving from
/// `CARGO_MANIFEST_DIR` makes the target explicit instead of ambient.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root must resolve")
}

/// Runs `dev check-architecture` against `--root` and returns
/// (exit code, stdout, stderr).
fn run_gate(root: &str, extra: &[&str]) -> (Option<i32>, String, String) {
    let mut args = vec!["dev", "check-architecture", "--root", root];
    args.extend_from_slice(extra);
    let output = Command::new(sddk_bin())
        .args(&args)
        .current_dir(repo_root())
        .output()
        .unwrap_or_else(|e| panic!("sddk dev check-architecture must run: {e}"));
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn check_architecture_gate_does_not_certify_conformance_on_this_repo() {
    // The repository carries live waivers (ARCH003, ARCH008) and rules whose
    // evaluators are not implemented, so the gate can only report WAIVED /
    // NOT_EVALUATED. That is the whole point of C3l.7: exit 0 is reserved for
    // a genuinely conformant tree, and this one is not conformant.
    //
    // Before this slice the gate exited 0 here, which is how a repository with
    // open architectural debt could be cited as architecturally conformant.
    let root = repo_root();
    let (code, stdout, stderr) = run_gate(root.to_str().unwrap(), &[]);

    assert!(
        stdout.contains("VERDICT:"),
        "the gate must print an explicit VERDICT line; got:\n{stdout}"
    );
    assert!(
        stdout.contains("does NOT certify architectural conformance"),
        "a non-conformant verdict must say so in plain words; got:\n{stdout}"
    );
    assert_ne!(
        code,
        Some(0),
        "exit 0 must be reserved for a conformant tree; this repo has live \
         waivers and unimplemented evaluators, so it cannot be conformant.\n\
         stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_architecture_reports_verdict_in_json_output() {
    // A programmatic consumer must be able to tell "proven clean" from "not
    // proven" without parsing the human table or guessing from the exit code.
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("arch.json");
    let root = repo_root();
    let (code, _, stderr) = run_gate(
        root.to_str().unwrap(),
        &["--out", out_path.to_str().unwrap()],
    );

    let raw = std::fs::read_to_string(&out_path)
        .unwrap_or_else(|e| panic!("--out must write the JSON report: {e}\nstderr:\n{stderr}"));
    let json: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("report is not JSON: {e}\n{raw}"));

    let verdict = json["verdict"]
        .as_str()
        .unwrap_or_else(|| panic!("report must carry a `verdict` field:\n{raw}"));
    assert!(
        ["CONFORMANT", "OPEN_DEBT", "WAIVED", "NOT_EVALUATED"].contains(&verdict),
        "verdict must be one of the four typed values; got {verdict}"
    );
    // exit_status and verdict must agree: only CONFORMANT is exit 0.
    let exit = json["exit_status"]
        .as_i64()
        .expect("exit_status must be an integer");
    assert_eq!(
        exit == 0,
        verdict == "CONFORMANT",
        "exit 0 must imply CONFORMANT and vice versa; got verdict={verdict} exit={exit}"
    );
    assert_eq!(
        Some(exit as i32),
        code,
        "JSON exit_status must match the process exit code"
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
    let rules_yaml = r#"schema_version: "1.2.0"
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
"#;
    let rules_path = root.join("rules.yaml");
    std::fs::write(&rules_path, rules_yaml).unwrap();

    // Fail-closed (C3l.7): no `skip` on a missing binary. These tests exist to
    // certify the gate; without the binary they certify nothing, and a green
    // "ok" is a false claim. `CARGO_BIN_EXE_sddk` is always built for an
    // integration target, so its absence is a real failure, not an environment
    // condition to tolerate.
    // Fail-closed (C3l.7): no `skip` on a missing binary. These tests exist to
    // certify the gate; without the binary they certify nothing, and a green
    // "ok" is a false claim. `CARGO_BIN_EXE_sddk` is always built for an
    // integration target, so its absence is a real failure, not an environment
    // condition to tolerate.
    let bin = sddk_bin();
    assert!(
        bin.is_file(),
        "sddk binary missing at {bin:?}; the gate would go unexercised"
    );

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
    let fail_yaml = r#"schema_version: "1.2.0"
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
"#;
    let fail_path = root.join("fail.yaml");
    std::fs::write(&fail_path, fail_yaml).unwrap();

    // Rule file with no rules
    let empty_yaml = r#"schema_version: "1.2.0"
rules: []
"#;
    let empty_path = root.join("empty.yaml");
    std::fs::write(&empty_path, empty_yaml).unwrap();

    // Fail-closed (C3l.7): no `skip` on a missing binary. These tests exist to
    // certify the gate; without the binary they certify nothing, and a green
    // "ok" is a false claim. `CARGO_BIN_EXE_sddk` is always built for an
    // integration target, so its absence is a real failure, not an environment
    // condition to tolerate.
    // Fail-closed (C3l.7): no `skip` on a missing binary. These tests exist to
    // certify the gate; without the binary they certify nothing, and a green
    // "ok" is a false claim. `CARGO_BIN_EXE_sddk` is always built for an
    // integration target, so its absence is a real failure, not an environment
    // condition to tolerate.
    let bin = sddk_bin();
    assert!(
        bin.is_file(),
        "sddk binary missing at {bin:?}; the gate would go unexercised"
    );

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
