//! E2E tests for `sddk architecture findings` (arch-spec-A3-S14).
//!
//! The AC5 DebVerify audit runs on every receipt but only its per-kind counts
//! reached the caller. This surface exposes the findings themselves: severity,
//! the contract ids involved, and the message.

use std::fs;
use std::path::Path;
use std::process::Command;

const FILE: &str = ".sddk/architecture/contracts.yaml";

/// Two `single_authority` contracts on `comp:b` (a shadow authority) plus one
/// compatibility window that elapsed at 500ms.
const DECL: &str = r#"
revision: findings-rev
knowledge_basis: test/findings
units:
  - id: comp:a
    kind: module
    locator: src/a.rs
  - id: comp:b
    kind: module
    locator: src/b.rs
contracts:
  - id: c-stale
    kind: bounded_compatibility
    component: comp:a
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
    deprecated_after_ms: 500
  - id: c-auth-1
    kind: single_authority
    component: comp:b
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: c-auth-2
    kind: single_authority
    component: comp:b
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
"#;

/// One authority over one component, no elapsed window: a clean declaration.
const DECL_CLEAN: &str = r#"
revision: findings-rev
knowledge_basis: test/findings
units:
  - id: comp:a
    kind: module
    locator: src/a.rs
contracts:
  - id: c-auth
    kind: single_authority
    component: comp:a
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
"#;

fn write_decl(root: &Path, decl: &str) {
    fs::create_dir_all(root.join(".sddk/architecture")).unwrap();
    fs::write(root.join(FILE), decl).unwrap();
}

fn sddk(root: &Path, family: &[&str]) -> std::process::Output {
    let mut args = vec!["architecture"];
    args.extend_from_slice(family);
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

fn json_rows(out: &std::process::Output) -> Vec<serde_json::Value> {
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("json: {e}: {}", String::from_utf8_lossy(&out.stdout)))
}

fn find<'a>(rows: &'a [serde_json::Value], kind: &str) -> &'a serde_json::Value {
    rows.iter()
        .find(|r| r["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} row in {rows:?}"))
}

#[test]
fn architecture_findings_lists_full_shape() {
    // REQ-A3S14-001: the cycle's reason to exist. The receipt kept counts and a
    // subject; severity, the conflicting contracts and the message were
    // computed then discarded.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);

    let out = sddk(
        tmp.path(),
        &["findings", "--now-ms", "5000", "--format", "json"],
    );
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let rows = json_rows(&out);
    assert_eq!(rows.len(), 2, "{text}");

    let shadow = find(&rows, "ShadowAuthority");
    assert_eq!(shadow["severity"], "Critical");
    assert_eq!(shadow["subjects"][0], "comp:b");
    // The actionable fact the receipt withheld: which contracts conflict.
    let ids: Vec<&str> = shadow["contract_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["c-auth-1", "c-auth-2"]);
    assert!(
        shadow["message"].as_str().unwrap().contains("comp:b"),
        "{shadow:?}"
    );
}

#[test]
fn architecture_findings_kind_filter() {
    // REQ-A3S14-002
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);

    let out = sddk(
        tmp.path(),
        &[
            "findings",
            "--now-ms",
            "5000",
            "--kind",
            "shadow_authority",
            "--format",
            "json",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", both(&out));
    let rows = json_rows(&out);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["kind"], "ShadowAuthority");

    // The filtered kind is genuinely excluded, not merely unmentioned.
    let out = sddk(
        tmp.path(),
        &[
            "findings",
            "--now-ms",
            "5000",
            "--kind",
            "stale_compatibility",
            "--format",
            "json",
        ],
    );
    let rows = json_rows(&out);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["kind"], "StaleCompatibility");

    // The text header states the filter, so a short listing is attributable.
    let text = both(&sddk(
        tmp.path(),
        &["findings", "--now-ms", "5000", "--kind", "shadow_authority"],
    ));
    assert!(text.contains("kind=shadow_authority"), "{text}");
}

#[test]
fn architecture_findings_unknown_kind_fails_closed() {
    // REQ-A3S14-003: an unknown tag must not read as "this declaration is clean".
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);

    let out = sddk(
        tmp.path(),
        &["findings", "--now-ms", "5000", "--kind", "nope"],
    );
    let text = both(&out);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(out.stdout.is_empty(), "no listing may be emitted");
    assert!(text.contains("unknown --kind `nope`"), "{text}");
    // The accepted set is echoed, and derived from the closed enum.
    for tag in [
        "shadow_authority",
        "authority_bypass",
        "missing_owner",
        "contradiction",
        "stale_compatibility",
    ] {
        assert!(text.contains(tag), "accepted set must name {tag}: {text}");
    }
}

#[test]
fn architecture_findings_empty_is_ok() {
    // REQ-A3S14-004: a clean declaration is stated, not implied by silence.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_CLEAN);

    let out = sddk(tmp.path(), &["findings", "--now-ms", "5000"]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("(no findings)"), "{text}");
    assert!(text.contains("0 finding(s)"), "{text}");

    let out = sddk(
        tmp.path(),
        &["findings", "--now-ms", "5000", "--format", "json"],
    );
    assert_eq!(out.status.code(), Some(0));
    assert!(json_rows(&out).is_empty());
}

#[test]
fn architecture_findings_reports_severity() {
    // REQ-A3S14-005: severity is the kind's deterministic severity, not a
    // constant and not a fabricated score.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);

    let out = sddk(
        tmp.path(),
        &["findings", "--now-ms", "5000", "--format", "json"],
    );
    let rows = json_rows(&out);
    assert_eq!(find(&rows, "ShadowAuthority")["severity"], "Critical");
    assert_eq!(find(&rows, "StaleCompatibility")["severity"], "Medium");

    // Three closed severities, no numeric weight anywhere.
    for r in &rows {
        let s = r["severity"].as_str().unwrap();
        assert!(
            ["Critical", "High", "Medium"].contains(&s),
            "unexpected severity {s}"
        );
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(!text.to_lowercase().contains("score"), "{text}");
}

#[test]
fn architecture_findings_agree_with_receipt() {
    // REQ-A3S14-006: the two surfaces cannot drift. Both call the same
    // load/overlay/audit path, and this asserts the invariant from outside.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);

    let receipt = sddk(
        tmp.path(),
        &["receipt", "--now-ms", "5000", "--format", "json"],
    );
    let receipt: serde_json::Value = serde_json::from_slice(&receipt.stdout).unwrap();
    let counts = receipt["audit_findings"].as_object().unwrap().clone();

    let listing = sddk(
        tmp.path(),
        &["findings", "--now-ms", "5000", "--format", "json"],
    );
    let mut mine: std::collections::BTreeMap<String, usize> = Default::default();
    for r in json_rows(&listing) {
        let tag = match r["kind"].as_str().unwrap() {
            "ShadowAuthority" => "shadow_authority",
            "MissingOwner" => "missing_owner",
            "AuthorityBypass" => "authority_bypass",
            "StaleCompatibility" => "stale_compatibility",
            "Contradiction" => "contradiction",
            other => panic!("unmapped kind {other}"),
        };
        *mine.entry(tag.to_string()).or_insert(0) += 1;
    }
    let theirs: std::collections::BTreeMap<String, usize> = counts
        .iter()
        .map(|(k, v)| (k.clone(), v.as_u64().unwrap() as usize))
        .collect();
    assert_eq!(mine, theirs, "findings and receipt disagree");
}

#[test]
fn architecture_findings_clock_is_honest() {
    // REQ-A3S14-007: a stale window is time-dependent. Pinning the clock must
    // change the answer, and the default must be the wall clock, not zero.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);

    let early = sddk(
        tmp.path(),
        &[
            "findings",
            "--now-ms",
            "100",
            "--kind",
            "stale_compatibility",
            "--format",
            "json",
        ],
    );
    assert!(
        json_rows(&early).is_empty(),
        "the window is open at 100ms, before it elapsed at 500ms"
    );

    let late = sddk(
        tmp.path(),
        &[
            "findings",
            "--now-ms",
            "5000",
            "--kind",
            "stale_compatibility",
            "--format",
            "json",
        ],
    );
    assert_eq!(json_rows(&late).len(), 1, "the window is stale at 5000ms");

    // No --now-ms: the wall clock is used, so this must not be a pinned zero.
    let default = sddk(
        tmp.path(),
        &[
            "findings",
            "--kind",
            "stale_compatibility",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        json_rows(&default).len(),
        1,
        "without --now-ms the run must use the wall clock, not 0"
    );
}

#[test]
fn architecture_findings_is_inspection_not_a_gate() {
    // REQ-A3S14-008: the receipt blocks on the mandatory finding; the listing
    // reports it and exits 0. Inspection has no verdict.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);

    let receipt = sddk(tmp.path(), &["receipt", "--now-ms", "5000"]);
    assert_eq!(
        receipt.status.code(),
        Some(1),
        "the receipt must still gate: {}",
        both(&receipt)
    );
    assert!(both(&receipt).contains("verdict:           blocked"));

    let listing = sddk(tmp.path(), &["findings", "--now-ms", "5000"]);
    let text = both(&listing);
    assert_eq!(listing.status.code(), Some(0), "{text}");
    for forbidden in ["verdict", "blocked", "pass", "score"] {
        assert!(
            !text.to_lowercase().contains(forbidden),
            "a read surface must not emit `{forbidden}`: {text}"
        );
    }
}

#[test]
fn architecture_findings_fails_closed_on_bad_declaration() {
    // REQ-A3S14-009
    let tmp = tempfile::tempdir().unwrap();
    // Missing declaration.
    let out = sddk(tmp.path(), &["findings", "--now-ms", "5000"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(both(&out).contains("cannot read declaration"));

    // Malformed declaration.
    fs::create_dir_all(tmp.path().join(".sddk/architecture")).unwrap();
    fs::write(tmp.path().join(FILE), "revision: [not a revision\n").unwrap();
    let out = sddk(tmp.path(), &["findings", "--now-ms", "5000"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(both(&out).contains("not a valid declaration"));
}

#[test]
fn architecture_findings_writes_nothing() {
    // REQ-A3S14-008 (cont.): the surface has no side effects, unlike
    // `receipt --out`. Checked by snapshotting the tree.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL);
    let before = fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();

    let out = sddk(tmp.path(), &["findings", "--now-ms", "5000"]);
    assert_eq!(out.status.code(), Some(0));

    let after = fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(before, after, "findings must not create or remove anything");
}

#[test]
fn architecture_findings_carries_the_full_kind_variety() {
    // REQ-A3S14-001/005 (broadened in verify): the surface must render every
    // kind AC5 can produce, not just the two that happen to be easy to trigger.
    // Each kind has its own severity and its own contract_ids shape, so a
    // renderer that special-cases one kind would pass the narrow tests and fail
    // here.
    let decl = r#"
revision: five
knowledge_basis: test/findings
units:
  - id: comp:domain
  - id: comp:provider
  - id: comp:dup
  - id: ent:twin
relations:
  - from: comp:domain
    to: comp:provider
    kind: depends_on
  - from: comp:dup
    to: ent:twin
    kind: owns
contracts:
  - id: no-edge
    kind: forbidden_dependency
    from: comp:domain
    to: comp:provider
    reason: forbidden
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: auth-1
    kind: single_authority
    component: comp:dup
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: auth-2
    kind: single_authority
    component: comp:dup
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: proj-1
    kind: projection_only
    source_kind: comp:dup
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: orphan
    kind: single_authority
    component: comp:nowhere
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
  - id: window
    kind: bounded_compatibility
    component: comp:domain
    decided_by: decision:d
    specified_by: spec:s
    revision: rev:1
    deprecated_after_ms: 500
"#;
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), decl);

    let out = sddk(
        tmp.path(),
        &["findings", "--now-ms", "5000", "--format", "json"],
    );
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let rows = json_rows(&out);

    let pairs: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| (r["kind"].as_str().unwrap(), r["severity"].as_str().unwrap()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            // DebVerifyAudit orders findings by (kind, subjects, contract_ids),
            // which is the canonical order the audit documents.
            ("ShadowAuthority", "Critical"),
            ("MissingOwner", "High"),
            ("AuthorityBypass", "Critical"),
            ("StaleCompatibility", "Medium"),
            ("Contradiction", "High"),
        ],
        "all five kinds, in the audit's canonical order: {text}"
    );

    // Each kind names the contracts involved, which is what the receipt dropped.
    assert_eq!(find(&rows, "AuthorityBypass")["contract_ids"][0], "no-edge");
    assert_eq!(find(&rows, "MissingOwner")["contract_ids"][0], "orphan");
    // The contradiction involves both sides of the conflict.
    let contra: Vec<&str> = find(&rows, "Contradiction")["contract_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(
        contra.contains(&"auth-1") && contra.contains(&"proj-1"),
        "{contra:?}"
    );

    // Every kind is filterable, which is what `--kind` derives from the enum.
    for tag in [
        "shadow_authority",
        "missing_owner",
        "authority_bypass",
        "stale_compatibility",
        "contradiction",
    ] {
        let one = sddk(
            tmp.path(),
            &[
                "findings", "--now-ms", "5000", "--kind", tag, "--format", "json",
            ],
        );
        assert_eq!(
            json_rows(&one).len(),
            1,
            "--kind {tag} must isolate its own finding"
        );
    }
}
