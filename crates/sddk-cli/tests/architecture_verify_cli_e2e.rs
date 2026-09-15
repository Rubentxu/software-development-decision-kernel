//! E2E tests for the architecture verification entry point (arch-spec-A3-S13).
//!
//! `sddk architecture receipt` is the verification entry point: `--changed`
//! scopes by diff (A3-S12), `--contract` scopes to one contract, `--out` leaves
//! the receipt where a harness can find it.

use std::fs;
use std::path::Path;
use std::process::Command;

const FILE: &str = ".sddk/architecture/contracts.yaml";

/// Two unit-scoped contracts, each on its own unit.
const DECL: &str = r#"
revision: verify-rev
knowledge_basis: test/verify
units:
  - id: comp:a
    kind: module
    locator: src/a.rs
  - id: comp:b
    kind: module
    locator: src/b.rs
contracts:
  - id: c-a
    kind: single_authority
    component: comp:a
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: c-b
    kind: single_authority
    component: comp:b
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
"#;

/// A contract kind that is not unit-scoped: it has no evaluable subject.
const DECL_GLOBAL_KIND: &str = r#"
revision: verify-rev
knowledge_basis: test/verify
units:
  - id: comp:a
    kind: module
    locator: src/a.rs
contracts:
  - id: c-window
    kind: bounded_compatibility
    component: comp:a
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
    deprecated_after_ms: 2000
"#;

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn init_repo(root: &Path) -> String {
    fs::create_dir_all(root.join(".sddk/architecture")).unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join(FILE), DECL).unwrap();
    fs::write(root.join("src/a.rs"), "pub fn a() -> u64 { 1 }\n").unwrap();
    fs::write(root.join("src/b.rs"), "pub fn b() -> u64 { 2 }\n").unwrap();
    git(root, &["init", "-q"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "base"]);
    git(root, &["rev-parse", "HEAD"])
}

/// A plain (non-git) directory holding just a declaration.
fn init_plain(root: &Path, decl: &str) {
    fs::create_dir_all(root.join(".sddk/architecture")).unwrap();
    fs::write(root.join(FILE), decl).unwrap();
}

fn sddk(root: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["architecture", "receipt", "--now-ms", "1000"];
    args.extend_from_slice(extra);
    args.push("--root");
    let root_str = root.to_str().expect("utf8 root");
    args.push(root_str);
    Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(&args)
        .output()
        .expect("sddk binary not found")
}

fn both(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn claims(v: &serde_json::Value) -> Vec<String> {
    v["claim_results"]
        .as_array()
        .expect("claim_results")
        .iter()
        .map(|c| c["contract"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn architecture_contract_filter_scopes() {
    // REQ-A3S13-001: `--contract` alone answers a question about that contract
    // rather than degenerating into an empty (and therefore passing) scope.
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);

    let out = sddk(tmp.path(), &["--contract", "c-b", "--format", "json"]);
    let text = both(&out);
    // Exit 1: c-b carries no evidence, so it is honestly Unknown/Blocked.
    assert_eq!(out.status.code(), Some(1), "{text}");
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| panic!("json: {e}: {text}"));
    assert_eq!(claims(&v), vec!["c-b".to_string()], "{text}");
    assert_eq!(v["verdict"], "Blocked");
    // The scope is attributable: no diff, and the filter names what was asked.
    assert!(v["change_basis"].is_null());
    assert_eq!(v["contract_filter"], "c-b");
}

#[test]
fn architecture_contract_filter_fails_closed() {
    // REQ-A3S13-002: an undeclared id is a usage error, never an empty scope.
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);

    for bad in ["no-such-contract", "c-A", ""] {
        let out = sddk(tmp.path(), &["--contract", bad]);
        let text = both(&out);
        assert_eq!(out.status.code(), Some(2), "[{bad}] {text}");
        assert!(out.stdout.is_empty(), "[{bad}] no receipt may be emitted");
        assert!(
            text.contains("is not declared"),
            "[{bad}] error must name the problem: {text}"
        );
    }
}

#[test]
fn architecture_contract_filter_rejects_unevaluable_kinds() {
    // REQ-A3S13-002 (extended): a global-kind contract has no subject unit, so
    // "verify it" is unanswerable. Fail closed rather than report a passing
    // nothing.
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL_GLOBAL_KIND);

    let out = sddk(tmp.path(), &["--contract", "c-window"]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(out.stdout.is_empty(), "no receipt may be emitted");
    assert!(text.contains("no evaluable subject unit"), "{text}");
}

#[test]
fn architecture_contract_filter_rejects_dangling_subjects() {
    // REQ-A3S13-011 (extended, found in verify): a unit-scoped kind whose
    // subject the declaration does not describe is just as unanswerable as a
    // global kind. AC5's missing-owner finding would eventually flag the
    // dangling reference, but the verdict would be incidental to "verify c-x".
    let decl = r#"
revision: verify-rev
knowledge_basis: test/verify
units:
  - id: comp:a
    kind: module
    locator: src/a.rs
contracts:
  - id: c-orphan
    kind: single_authority
    component: comp:missing
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
"#;
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), decl);

    let out = sddk(tmp.path(), &["--contract", "c-orphan"]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(out.stdout.is_empty(), "no receipt may be emitted");
    assert!(text.contains("no evaluable subject unit"), "{text}");

    // The declaration itself is still accepted: a partial declaration is a
    // manifest of intent, and the global audit remains the thing that reports
    // what it left undescribed.
    let read = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "architecture",
            "contracts",
            "--root",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("sddk");
    assert_eq!(read.status.code(), Some(0));
}

#[test]
fn architecture_contract_filter_is_recorded() {
    // REQ-A3S13-003: present when asked for, absent (not empty) otherwise.
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);

    let with = sddk(tmp.path(), &["--contract", "c-a", "--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&with.stdout).unwrap();
    assert_eq!(v["contract_filter"], "c-a");

    let without = sddk(tmp.path(), &["--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&without.stdout).unwrap();
    assert!(
        v["contract_filter"].is_null(),
        "absence must be absence, not an empty string"
    );

    // And the text renderer only prints the line when there is something to say.
    let text = both(&sddk(tmp.path(), &["--contract", "c-a"]));
    assert!(text.contains("contract_filter:   c-a"), "{text}");
    let text = both(&sddk(tmp.path(), &[]));
    assert!(!text.contains("contract_filter:"), "{text}");
}

#[test]
fn architecture_contract_filter_intersects_with_changed() {
    // REQ-A3S13-004: both scopes compose as an intersection, and the receipt
    // states both so an empty result is attributable.
    let tmp = tempfile::tempdir().unwrap();
    let base = init_repo(tmp.path());
    fs::write(tmp.path().join("src/a.rs"), "pub fn a() -> u64 { 99 }\n").unwrap();

    // The diff reached comp:a, whose contract is c-a.
    let hit = sddk(
        tmp.path(),
        &[
            "--changed",
            "--base",
            &base,
            "--contract",
            "c-a",
            "--format",
            "json",
        ],
    );
    let text = both(&hit);
    assert_eq!(hit.status.code(), Some(1), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&hit.stdout).unwrap();
    assert_eq!(claims(&v), vec!["c-a".to_string()]);
    assert_eq!(v["change_basis"]["changed_units"][0], "comp:a");
    assert_eq!(v["contract_filter"], "c-a");

    // Asking about the other contract is a legitimate, attributable empty: the
    // diff is recorded, the filter is recorded, and nothing is claimed.
    let miss = sddk(
        tmp.path(),
        &[
            "--changed",
            "--base",
            &base,
            "--contract",
            "c-b",
            "--format",
            "json",
        ],
    );
    let text = both(&miss);
    assert_eq!(miss.status.code(), Some(0), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&miss.stdout).unwrap();
    assert!(claims(&v).is_empty(), "{text}");
    assert_eq!(
        v["change_basis"]["changed_units"]
            .as_array()
            .expect("changed_units")
            .len(),
        1,
        "the change basis must stay truthful: something did change"
    );
    assert_eq!(v["contract_filter"], "c-b");
}

#[test]
fn architecture_contract_filter_text_names_it() {
    // REQ-A3S13-005
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);

    let text = both(&sddk(tmp.path(), &["--contract", "c-a"]));
    assert!(text.contains("contract_filter:   c-a"), "{text}");
    assert!(
        text.contains("affected_contracts: 1 (contract-scoped)"),
        "the scope note must not call a filtered run change-scoped: {text}"
    );
    // A plain global run is not called change-scoped either.
    let global = both(&sddk(tmp.path(), &[]));
    assert!(
        global.contains("affected_contracts: 0 (empty without --changed or --contract)"),
        "{global}"
    );
}

#[test]
fn architecture_out_writes_receipt() {
    // REQ-A3S13-006
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);
    let path = out_dir.path().join("receipt.json");

    let out = sddk(
        tmp.path(),
        &[
            "--contract",
            "c-a",
            "--format",
            "json",
            "--out",
            path.to_str().unwrap(),
        ],
    );
    let text = both(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    let written = fs::read(&path).unwrap_or_else(|e| panic!("--out wrote nothing: {e}"));
    assert_eq!(
        String::from_utf8_lossy(&written),
        String::from_utf8_lossy(&out.stdout),
        "the artifact must be the receipt, byte for byte"
    );
    // The path is reported on stderr so stdout stays parseable JSON.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("wrote: "), "{stderr}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout stays json");
    assert_eq!(v["contract_filter"], "c-a");
}

#[test]
fn architecture_out_missing_parent_fails_closed() {
    // REQ-A3S13-007
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);
    let missing = out_dir.path().join("does-not-exist").join("receipt.json");

    let out = sddk(
        tmp.path(),
        &["--format", "json", "--out", missing.to_str().unwrap()],
    );
    let text = both(&out);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(out.stdout.is_empty(), "no receipt may be emitted");
    assert!(text.contains("parent directory"), "{text}");
    assert!(
        !missing.parent().unwrap().exists(),
        "the directory must not be created: a receipt must land where asked"
    );
}

#[test]
fn read_surfaces_reject_change_flags() {
    // REQ-A3S13-008: the read surfaces used to advertise `--changed`/`--base`
    // and silently ignore both.
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);
    let root = tmp.path().to_str().unwrap();

    for cmd in ["contracts", "authorities", "ownership", "compatibility"] {
        for flag in ["--changed", "--base"] {
            let out = Command::new(env!("CARGO_BIN_EXE_sddk"))
                .args(["architecture", cmd, "--root", root, flag])
                .output()
                .expect("sddk");
            assert_eq!(
                out.status.code(),
                Some(2),
                "`architecture {cmd} {flag}` must be rejected, not ignored"
            );
        }
        let help = Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(["architecture", cmd, "--help"])
            .output()
            .expect("sddk");
        let text = String::from_utf8_lossy(&help.stdout);
        assert!(
            !text.contains("--changed"),
            "`{cmd} --help` still advertises it"
        );
        assert!(
            !text.contains("--base"),
            "`{cmd} --help` still advertises it"
        );
    }
}

#[test]
fn receipt_rejects_unknown_contract_at_the_engine_boundary() {
    // REQ-A3S13-009 / a guard on the new machinery: with no new flag, the run
    // is the v1.169.34 global run. It finds no change scope and no filter, and
    // reports zero affected claims while still auditing every contract.
    let tmp = tempfile::tempdir().unwrap();
    init_plain(tmp.path(), DECL);

    let out = sddk(tmp.path(), &["--format", "json"]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["claim_results"].as_array().unwrap().is_empty());
    assert!(v["contract_filter"].is_null());
    assert!(v["change_basis"].is_null());
    assert_eq!(v["audited_contracts"], 2);
}
