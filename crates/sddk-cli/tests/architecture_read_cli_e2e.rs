//! E2E tests for the `sddk architecture` read surfaces (arch-spec-A3-S11).
//!
//! Each test writes one declaration into a temp directory and invokes the real
//! binary, asserting the surface's rows, its JSON shape, and its fail-closed
//! behaviour.

use std::fs;
use std::path::Path;
use std::process::Command;

const FILE: &str = ".sddk/architecture/contracts.yaml";

/// A declaration exercising all five surfaces.
const DECL: &str = r#"
revision: read-rev-1
knowledge_basis: test/read
units:
  - id: comp:canonical-event-log
    kind: module
    locator: crates/sddk-engine/src/canonical_event_log.rs
  - id: comp:graph
    kind: module
    locator: crates/sddk-engine/src/architecture_graph
  - id: entity:ledger
    kind: schema
    locator: db/ledger.sql
relations:
  - from: comp:graph
    to: comp:canonical-event-log
    kind: reads
contracts:
  - id: cal-authority
    kind: single_authority
    component: comp:canonical-event-log
    decided_by: decision:ADR-0094
    specified_by: spec:arch-spec-001
    revision: rev:1
  - id: ledger-owner
    kind: unique_owner
    entity: entity:ledger
    decided_by: decision:ADR-0094
    specified_by: spec:arch-spec-001
    revision: rev:1
  - id: window-open
    kind: bounded_compatibility
    deprecated_after_ms: 9999999999999
    decided_by: decision:ADR-0001
    specified_by: spec:arch-spec-001
    revision: rev:1
  - id: window-stale
    kind: bounded_compatibility
    deprecated_after_ms: 1
    decided_by: decision:ADR-0001
    specified_by: spec:arch-spec-001
    revision: rev:1
"#;

fn write_declaration(root: &Path, body: &str) {
    fs::create_dir_all(root.join(".sddk/architecture")).expect("dir");
    fs::write(root.join(FILE), body).expect("write");
}

fn run(root: &Path, surface: &[&str], extra: &[&str]) -> std::process::Output {
    let mut args = vec!["architecture"];
    args.extend_from_slice(surface);
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

fn fixture() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(tmp.path(), DECL);
    tmp
}

#[test]
fn architecture_contracts_lists_declared() {
    // REQ-A3S11-007
    let tmp = fixture();
    let out = run(tmp.path(), &["contracts"], &[]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("architecture contracts — revision read-rev-1"));
    for id in [
        "cal-authority",
        "ledger-owner",
        "window-open",
        "window-stale",
    ] {
        assert!(text.contains(id), "missing {id} in:\n{text}");
    }
    assert!(text.contains("single_authority"));
    assert!(text.contains("bounded_compatibility"));
    // Clean rendering: no Debug wrappers leak into the surface.
    assert!(!text.contains("Revision(\""), "{text}");
    assert!(!text.contains("DecisionRef::"), "{text}");
}

#[test]
fn architecture_authorities_filters_single_authority() {
    // REQ-A3S11-008
    let tmp = fixture();
    let out = run(tmp.path(), &["authorities"], &[]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        text.contains("comp:canonical-event-log: cal-authority"),
        "{text}"
    );
    assert!(
        !text.contains("ledger-owner"),
        "ownership leaked in:\n{text}"
    );
    assert!(!text.contains("window-open"), "{text}");
}

#[test]
fn architecture_ownership_filters_unique_owner() {
    // REQ-A3S11-009
    let tmp = fixture();
    let out = run(tmp.path(), &["ownership"], &[]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0));
    assert!(text.contains("entity:ledger: ledger-owner"), "{text}");
    assert!(!text.contains("cal-authority"), "{text}");
}

#[test]
fn architecture_compatibility_reports_window() {
    // REQ-A3S11-010: one open window and one stale window.
    let tmp = fixture();
    let out = run(
        tmp.path(),
        &["compatibility"],
        &["--now-ms", "1000", "--format", "json"],
    );
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json rows");
    let rows = v.as_array().expect("array");
    assert_eq!(rows.len(), 2);
    let by_id = |id: &str| rows.iter().find(|r| r["id"] == id).expect("row").clone();
    assert_eq!(by_id("window-open")["window_status"], "open");
    assert_eq!(by_id("window-open")["replaced_by"], serde_json::Value::Null);
    assert_eq!(by_id("window-stale")["window_status"], "stale");

    // And the text form names both.
    let text_out = both(&run(tmp.path(), &["compatibility"], &["--now-ms", "1000"]));
    assert!(text_out.contains("window-open"));
    assert!(text_out.contains("window-stale"));
    assert!(text_out.contains("window_status=stale"));
}

#[test]
fn architecture_graph_lists_units_and_relations() {
    // REQ-A3S11-011
    let tmp = fixture();
    let out = run(tmp.path(), &["graph"], &[]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0));
    assert!(text.contains("comp:canonical-event-log"), "{text}");
    assert!(text.contains("entity:ledger"), "{text}");
    assert!(
        text.contains("comp:graph -> comp:canonical-event-log"),
        "relation must render with inner ids:\n{text}"
    );
    assert!(text.contains("Reads"), "{text}");
    assert!(text.contains("overlay_digest_len:"), "{text}");
    // No Debug wrapper leakage.
    assert!(!text.contains("SoftwareUnitRef("), "{text}");
}

#[test]
fn architecture_graph_scope_filters_units() {
    // REQ-A3S11-012
    let tmp = fixture();
    let all = both(&run(tmp.path(), &["graph"], &[]));
    let scoped = both(&run(tmp.path(), &["graph"], &["--scope", "crates/"]));
    assert!(all.contains("entity:ledger"));
    assert!(!scoped.contains("entity:ledger"), "scope failed:\n{scoped}");
    assert!(scoped.contains("comp:graph"));
}

#[test]
fn architecture_read_surfaces_are_score_free() {
    // REQ-A3S11-013: no numeric aggregate anywhere in any surface.
    let tmp = fixture();
    for surface in [
        "contracts",
        "authorities",
        "ownership",
        "compatibility",
        "graph",
    ] {
        for fmt in [vec![], vec!["--format", "json"]] {
            let mut fmt_args = vec!["--now-ms", "1000"];
            fmt_args.extend_from_slice(&fmt);
            let out = run(tmp.path(), &[surface], &fmt_args);
            let text = both(&out);
            for needle in ["score", "weight", "grade", "rating", "percentage"] {
                assert!(
                    !text.to_ascii_lowercase().contains(needle),
                    "{surface} leaked `{needle}`:\n{text}"
                );
            }
            // No verdict is emitted by a read surface.
            assert!(!text.contains("verdict:"), "{surface}:\n{text}");
        }
    }
    // JSON shapes are objects/arrays, never a bare number.
    let json = both(&run(tmp.path(), &["contracts"], &["--format", "json"]));
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v.is_array());
}

#[test]
fn architecture_read_surfaces_fail_closed() {
    // REQ-A3S11-014
    let tmp = tempfile::tempdir().unwrap();
    for surface in [
        "contracts",
        "authorities",
        "ownership",
        "compatibility",
        "graph",
    ] {
        let out = run(tmp.path(), &[surface], &[]);
        assert_eq!(
            out.status.code(),
            Some(2),
            "surface `{surface}` must fail closed on a missing declaration"
        );
        assert!(out.stdout.is_empty(), "{surface} printed a row on error");
    }

    // An invalid declaration (unknown relation kind) also fails closed.
    write_declaration(
        tmp.path(),
        r#"
revision: bad
units:
  - id: a
  - id: b
relations:
  - from: a
    to: b
    kind: contradicts_by
"#,
    );
    let out = run(tmp.path(), &["graph"], &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(both(&out).contains("non-declarable kind"));
}

#[test]
fn architecture_subcommands_are_registered() {
    // REQ-A3S11-015
    let out = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["agent-help", "agent"])
        .output()
        .expect("sddk binary not found");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    for surface in [
        "architecture.contracts",
        "architecture.authorities",
        "architecture.ownership",
        "architecture.compatibility",
        "architecture.graph",
    ] {
        assert!(text.contains(surface), "registry missing `{surface}`");
    }
}

#[test]
fn architecture_declared_bypass_reaches_the_receipt() {
    // REQ-A3S11-005 at the CLI boundary: declared relations make AC5's
    // dependency-boundary class reproducible.
    let tmp = tempfile::tempdir().unwrap();
    write_declaration(
        tmp.path(),
        r#"
revision: bypass
units:
  - id: comp:domain
  - id: comp:provider
relations:
  - from: comp:domain
    to: comp:provider
    kind: depends_on
contracts:
  - id: no-edge
    kind: forbidden_dependency
    from: comp:domain
    to: comp:provider
    reason: forbidden
    decided_by: d
    specified_by: s
    revision: r
"#,
    );
    let out = run(tmp.path(), &["receipt"], &["--format", "json"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("receipt json");
    assert_eq!(v["verdict"], "Blocked");
    let classes = v["class_coverage"].as_array().expect("classes");
    let dep = classes
        .iter()
        .find(|c| c["class"] == "DependencyBoundary")
        .expect("class row");
    assert_eq!(
        dep["reproduced"], true,
        "declared relation must reproduce the dependency-boundary class"
    );
}
