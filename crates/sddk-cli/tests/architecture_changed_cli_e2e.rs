//! E2E tests for `sddk architecture receipt --changed` (arch-spec-A3-S12).
//!
//! Each test builds a real git repository in a temp directory, commits a
//! baseline, touches one declared locator, and asserts the change-scoped
//! receipt.

use std::fs;
use std::path::Path;
use std::process::Command;

const FILE: &str = ".sddk/architecture/contracts.yaml";

const DECL: &str = r#"
revision: changed-rev
knowledge_basis: test/changed
units:
  - id: comp:touched
    kind: module
    locator: src/a.rs
  - id: comp:untouched
    kind: module
    locator: src/b.rs
contracts:
  - id: c-touched
    kind: single_authority
    component: comp:touched
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: c-untouched
    kind: single_authority
    component: comp:untouched
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
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

/// Initialise a repo with the declaration and two unit sources; return the base
/// commit sha.
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

fn run(root: &Path, extra: &[&str]) -> std::process::Output {
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

#[test]
fn architecture_changed_scopes_to_touched_units() {
    // REQ-A3S12-004 / REQ-A3S12-008
    let tmp = tempfile::tempdir().unwrap();
    let base = init_repo(tmp.path());
    fs::write(tmp.path().join("src/a.rs"), "pub fn a() -> u64 { 42 }\n").unwrap();

    let out = run(
        tmp.path(),
        &["--changed", "--base", &base, "--format", "json"],
    );
    let text = both(&out);
    // Exit 1 is the honest verdict: the scoped contract carries no evidence, so
    // its Unknown state is an unresolved MUST finding. A receipt is still
    // emitted; only `blocked` maps to 1 (usage errors are 2).
    assert_eq!(out.status.code(), Some(1), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["verdict"], "Blocked");

    let units = v["change_basis"]["changed_units"]
        .as_array()
        .expect("changed_units")
        .iter()
        .map(|u| u.as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert_eq!(units, vec!["comp:touched".to_string()]);

    let claims: Vec<String> = v["claim_results"]
        .as_array()
        .expect("claim_results")
        .iter()
        .map(|c| c["contract"].as_str().unwrap().to_string())
        .collect();
    assert!(claims.contains(&"c-touched".to_string()), "{claims:?}");
    assert!(
        !claims.contains(&"c-untouched".to_string()),
        "an untouched unit's contract entered the delta: {claims:?}"
    );
}

#[test]
fn architecture_changed_records_basis() {
    // REQ-A3S12-005
    let tmp = tempfile::tempdir().unwrap();
    let base = init_repo(tmp.path());
    fs::write(tmp.path().join("src/b.rs"), "pub fn b() -> u64 { 3 }\n").unwrap();

    let out = run(
        tmp.path(),
        &["--changed", "--base", &base, "--format", "json"],
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["change_basis"]["base"], base);
    assert_eq!(
        v["change_basis"]["changed_units"].as_array().unwrap().len(),
        1
    );
    // And the touched unit is b's contract this time.
    let claims: Vec<String> = v["claim_results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["contract"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(claims, vec!["c-untouched".to_string()]);
}

#[test]
fn architecture_changed_excludes_untouched() {
    // REQ-A3S12-008: a real change with no declared impact scopes nothing.
    // Touching an undeclared file must yield an empty basis, not a blanket
    // scope over every declared contract.
    let tmp = tempfile::tempdir().unwrap();
    let base = init_repo(tmp.path());
    fs::write(tmp.path().join("README.md"), "docs only\n").unwrap();
    fs::write(tmp.path().join("src/undeclared.rs"), "pub fn z() {}\n").unwrap();

    let out = run(
        tmp.path(),
        &["--changed", "--base", &base, "--format", "json"],
    );
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");

    assert_eq!(
        v["change_basis"]["changed_units"]
            .as_array()
            .expect("changed_units")
            .len(),
        0,
        "an undeclared change must not scope units: {text}"
    );
    assert_eq!(
        v["claim_results"].as_array().unwrap().len(),
        0,
        "no declared contract may enter the delta: {text}"
    );
    // The honest zero is distinguishable from a global run: the basis is
    // present (and names the base) while changed_units is empty.
    assert_eq!(v["change_basis"]["base"], base);
    assert!(!v["change_basis"].is_null());
}

#[test]
fn architecture_changed_text_names_base() {
    // REQ-A3S12-006
    let tmp = tempfile::tempdir().unwrap();
    let base = init_repo(tmp.path());
    fs::write(tmp.path().join("src/a.rs"), "pub fn a() -> u64 { 7 }\n").unwrap();

    let out = run(tmp.path(), &["--changed", "--base", &base]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(
        text.contains(&format!("change_basis:      base={base}")),
        "{text}"
    );
    assert!(text.contains("changed_units=1"), "{text}");
}

#[test]
fn architecture_changed_requires_resolvable_base() {
    // REQ-A3S12-003: a bogus base must fail closed, never silently report zero.
    let tmp = tempfile::tempdir().unwrap();
    let _base = init_repo(tmp.path());

    let out = run(tmp.path(), &["--changed", "--base", "no-such-revision"]);
    assert_eq!(out.status.code(), Some(2), "{}", both(&out));
    assert!(out.stdout.is_empty(), "no receipt on a bad base");
    assert!(both(&out).contains("does not resolve"));
}

#[test]
fn architecture_changed_requires_git_when_no_base_resolves() {
    // REQ-A3S12-003: a non-git directory with --changed and no base fails closed.
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".sddk/architecture")).unwrap();
    fs::write(tmp.path().join(FILE), DECL).unwrap();

    let out = run(tmp.path(), &["--changed"]);
    assert_eq!(out.status.code(), Some(2), "{}", both(&out));
    assert!(both(&out).contains("no base revision could be resolved"));
}

#[test]
fn architecture_changed_reports_unknown_without_evidence() {
    // REQ-A3S12-009: the scoped contract has no evidence, so it is Unknown.
    let tmp = tempfile::tempdir().unwrap();
    let base = init_repo(tmp.path());
    fs::write(tmp.path().join("src/a.rs"), "pub fn a() -> u64 { 9 }\n").unwrap();

    let out = run(
        tmp.path(),
        &["--changed", "--base", &base, "--format", "json"],
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    let row = v["claim_results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["contract"] == "c-touched")
        .expect("touched row");
    assert_eq!(row["status"], "Unknown");
}

#[test]
fn architecture_global_run_unchanged() {
    // REQ-A3S12-007: without --changed the run is global and the basis is absent.
    let tmp = tempfile::tempdir().unwrap();
    let _base = init_repo(tmp.path());
    fs::write(tmp.path().join("src/a.rs"), "pub fn a() -> u64 { 11 }\n").unwrap();

    let out = run(tmp.path(), &["--format", "json"]);
    let text = both(&out);
    // A global run scopes nothing, so it has no change-scoped unknowns and
    // passes cleanly; the scoped run above is what reports the Unknown state.
    assert_eq!(out.status.code(), Some(0), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(
        v["change_basis"].is_null(),
        "global run must omit the basis"
    );
    assert_eq!(
        v["claim_results"].as_array().unwrap().len(),
        0,
        "a global run has no change-scoped claims"
    );
    // The global pass still inspected both declared contracts.
    assert_eq!(v["audited_contracts"], 2);

    // And the text form says so explicitly.
    let text_out = both(&run(tmp.path(), &[]));
    assert!(
        text_out.contains("change_basis:      (global run"),
        "{text_out}"
    );
}
