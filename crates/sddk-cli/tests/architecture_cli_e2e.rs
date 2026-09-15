//! E2E tests for `sddk architecture receipt` (arch-spec-A3-S10).
//!
//! Each test writes a declaration into a temp directory and invokes the real
//! binary, asserting the receipt, the verdict, the exit code and the absence of
//! any fabricated score.

use std::fs;
use std::path::Path;
use std::process::Command;

const DIR: &str = ".sddk/architecture";
const FILE: &str = ".sddk/architecture/contracts.yaml";

/// A declaration exhibiting the duplicate-authority class (unwaived).
const DIRTY: &str = r#"
revision: test-rev-1
knowledge_basis: test/kb
units:
  - id: comp:auth
    kind: module
    locator: crates/auth.rs
contracts:
  - id: dup-a
    kind: single_authority
    component: comp:auth
    decided_by: decision:1
    specified_by: spec:1
    revision: rev:1
  - id: dup-b
    kind: single_authority
    component: comp:auth
    decided_by: decision:1
    specified_by: spec:1
    revision: rev:1
"#;

/// The same declaration, waived.
const WAIVED: &str = r#"
revision: test-rev-1
knowledge_basis: test/kb
units:
  - id: comp:auth
    kind: module
    locator: crates/auth.rs
contracts:
  - id: dup-a
    kind: single_authority
    component: comp:auth
    decided_by: decision:1
    specified_by: spec:1
    revision: rev:1
  - id: dup-b
    kind: single_authority
    component: comp:auth
    decided_by: decision:1
    specified_by: spec:1
    revision: rev:1
waivers:
  - waiver:shadow_authority:gov-1
"#;

fn write_declaration(root: &Path, body: &str) {
    let dir = root.join(DIR);
    fs::create_dir_all(&dir).expect("create declaration dir");
    fs::write(root.join(FILE), body).expect("write declaration");
}

fn run(root: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["architecture", "receipt"];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(&args)
        .arg("--root")
        .arg(root)
        .output()
        .expect("sddk binary not found")
}

#[test]
fn architecture_receipt_reads_declaration() {
    // REQ-A3S10-009
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(tmp.path(), DIRTY);
    let out = run(tmp.path(), &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ARCHITECTURE-CONFORMANCE-RECEIPT"),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("revision:          test-rev-1"));
    assert!(stdout.contains("knowledge_basis:   test/kb"));
    // The global pass inspected both declared contracts even though AC4's
    // change-scoped delta is empty without --changed.
    assert!(stdout.contains("audited_contracts: 2"), "stdout: {stdout}");
    assert!(stdout.contains("unresolved_must_findings:"));
}

#[test]
fn architecture_receipt_json_has_verdict() {
    // REQ-A3S10-010
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(tmp.path(), DIRTY);
    let out = run(tmp.path(), &["--format", "json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON receipt");
    assert_eq!(v["verdict"], "Blocked");
    assert!(v["id"].is_string());
    assert!(v["basis"]["revision"] == "test-rev-1");
    // Five historical classes are reported.
    assert_eq!(v["class_coverage"].as_array().map(|a| a.len()), Some(5));
}

#[test]
fn architecture_receipt_exit_codes() {
    // REQ-A3S10-011 / REQ-A3S10-006
    let tmp = tempfile::tempdir().unwrap();

    // Blocked -> 1
    write_declaration(tmp.path(), DIRTY);
    let blocked = run(tmp.path(), &[]);
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "dirty declaration must exit 1"
    );

    // Waived -> 0
    write_declaration(tmp.path(), WAIVED);
    let waived = run(tmp.path(), &[]);
    let stdout = String::from_utf8_lossy(&waived.stdout);
    assert_eq!(waived.status.code(), Some(0), "stdout: {stdout}");
    assert!(stdout.contains("verdict:           pass_with_waivers"));
}

#[test]
fn architecture_receipt_missing_file_fails() {
    // REQ-A3S10-012: a missing declaration is an error (exit 2), never an empty
    // receipt.
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "no receipt on error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot read declaration"),
        "stderr: {stderr}"
    );
}

#[test]
fn architecture_receipt_invalid_yaml_fails_closed() {
    // REQ-A3S10-003: fail closed on a malformed declaration.
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(tmp.path(), "revision: [unclosed\n");
    let out = run(tmp.path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a valid declaration"));
}

#[test]
fn architecture_receipt_unknown_kind_fails_closed() {
    // REQ-A3S10-003
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(
        tmp.path(),
        r#"
revision: r1
contracts:
  - id: x
    kind: not_a_kind
    decided_by: d
    specified_by: s
    revision: r
"#,
    );
    let out = run(tmp.path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown kind"));
}

#[test]
fn architecture_receipt_has_no_score() {
    // REQ-A3S10-013 / AC-UAT-043: no fabricated aggregate quality score.
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(tmp.path(), DIRTY);
    let text = String::from_utf8_lossy(&run(tmp.path(), &[]).stdout).into_owned();
    let json = String::from_utf8_lossy(&run(tmp.path(), &["--format", "json"]).stdout).into_owned();
    for needle in ["score", "weight", "grade", "rating", "percentage"] {
        assert!(
            !text.to_ascii_lowercase().contains(needle),
            "text output must not contain `{needle}`:\n{text}"
        );
        assert!(
            !json.to_ascii_lowercase().contains(needle),
            "json output must not contain `{needle}`"
        );
    }
}

#[test]
fn architecture_command_is_registered() {
    // REQ-A3S10-014: discoverable through the typed command surface.
    let out = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["agent-help", "agent"])
        .output()
        .expect("sddk binary not found");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("architecture"),
        "agent cheat sheet must list the architecture command"
    );
    // The help text works. NOTE: this binary renders clap help on stderr, so
    // the assertion checks both streams.
    let help = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["architecture", "--help"])
        .output()
        .expect("sddk binary not found");
    assert!(help.status.success());
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&help.stdout),
        String::from_utf8_lossy(&help.stderr)
    );
    assert!(
        rendered.contains("receipt"),
        "help:
{rendered}"
    );
    assert!(
        rendered.contains("Architecture conformance"),
        "help must carry the command's own about text:\n{rendered}"
    );
}

#[test]
fn architecture_receipt_is_deterministic() {
    // Two runs over the same declaration produce the same receipt id.
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(tmp.path(), DIRTY);
    // Pin the evaluation time: the receipt records `created_at`.
    let a =
        String::from_utf8_lossy(&run(tmp.path(), &["--now-ms", "1000", "--format", "json"]).stdout)
            .into_owned();
    let b =
        String::from_utf8_lossy(&run(tmp.path(), &["--now-ms", "1000", "--format", "json"]).stdout)
            .into_owned();
    assert_eq!(a, b);
}
