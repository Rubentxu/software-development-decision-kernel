//! E2E tests for `sddk why architecture <CONTRACT_OR_FINDING>` (arch-spec-A3-S15).
//!
//! The workflow this suite pins end to end:
//!
//! ```text
//! sddk architecture findings            -> shadow_authority <finding-id>
//! sddk why architecture <finding-id>     -> explanation
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const FILE: &str = ".sddk/architecture/contracts.yaml";

/// 1 → 1: a `single_authority` contract on a component that is not declared, so
/// AC5 reports `missing_owner` naming exactly one contract.
const DECL_ONE: &str = r#"
revision: why-rev
knowledge_basis: test/why
units:
  - id: comp:a
contracts:
  - id: c-orphan
    kind: single_authority
    component: comp:nowhere
    decided_by: ADR-0112
    specified_by: arch-spec-032
    revision: rev:1
"#;

/// 1 → 2: two `single_authority` contracts on one component.
const DECL_TWO: &str = r#"
revision: why-rev
knowledge_basis: test/why
units:
  - id: comp:dup
contracts:
  - id: c-auth-1
    kind: single_authority
    component: comp:dup
    decided_by: ADR-0112
    specified_by: arch-spec-032
    revision: rev:1
  - id: c-auth-2
    kind: single_authority
    component: comp:dup
    decided_by: ADR-0112
    specified_by: arch-spec-032
    revision: rev:1
"#;

/// 1 → 3: a contradiction names both authority sides plus the projection side.
/// Also produces a shadow_authority on the same subject, so two findings share
/// `comp:dup`.
const DECL_THREE: &str = r#"
revision: why-rev
knowledge_basis: test/why
units:
  - id: comp:dup
contracts:
  - id: c-auth-1
    kind: single_authority
    component: comp:dup
    decided_by: ADR-0112
    specified_by: arch-spec-032
    revision: rev:1
  - id: c-auth-2
    kind: single_authority
    component: comp:dup
    decided_by: ADR-0112
    specified_by: arch-spec-032
    revision: rev:1
  - id: c-proj
    kind: projection_only
    source_kind: comp:dup
    decided_by: ADR-0113
    specified_by: arch-spec-033
    revision: rev:1
"#;

fn write_decl(root: &Path, decl: &str) {
    fs::create_dir_all(root.join(".sddk/architecture")).unwrap();
    fs::write(root.join(FILE), decl).unwrap();
}

fn sddk(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(args)
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

/// `architecture findings --format json` rows.
fn finding_rows(root: &Path, now: &str) -> Vec<serde_json::Value> {
    let out = sddk(&[
        "architecture",
        "findings",
        "--root",
        root.to_str().unwrap(),
        "--now-ms",
        now,
        "--format",
        "json",
    ]);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "findings json: {e}: {}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

fn id_for(root: &Path, kind: &str) -> String {
    finding_rows(root, "5000")
        .into_iter()
        .find(|r| r["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} finding"))
        .get("id")
        .and_then(|v| v.as_str())
        .expect("row carries an id")
        .to_string()
}

fn why_json(root: &Path, query: &str, now: &str) -> serde_json::Value {
    let out = sddk(&[
        "why",
        "architecture",
        query,
        "--root",
        root.to_str().unwrap(),
        "--now-ms",
        now,
        "--format",
        "json",
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", both(&out));
    serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("why json: {e}: {}", String::from_utf8_lossy(&out.stdout)))
}

fn contracts_of(v: &serde_json::Value) -> Vec<String> {
    v["contracts"]
        .as_array()
        .expect("contracts")
        .iter()
        .map(|c| c["contract"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn findings_prints_ids() {
    // REQ-A3S15-004: an id printed by `findings` must be feedable to `why`.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let out = sddk(&[
        "architecture",
        "findings",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let rows = finding_rows(tmp.path(), "5000");
    let id = rows[0]["id"].as_str().expect("json carries the id");
    assert!(
        text.contains(id),
        "the text rendering must show the id: {text}"
    );
    assert_eq!(id.len(), 64, "finding ids are 64 hex: {id}");
}

#[test]
fn why_finding_ids_are_stable_across_clocks() {
    // REQ-A3S15-002: the basis carries no clock, so an id read at one time
    // resolves at another. With the default wall clock these are two separate
    // invocations, which is the primary workflow.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let early = id_for(tmp.path(), "shadow_authority");
    let rows_late = finding_rows(tmp.path(), "99999999");
    let late = rows_late
        .iter()
        .find(|r| r["kind"] == "shadow_authority")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(early, late, "ids must not move with the clock");

    // And the id resolved at one clock works at another.
    let v = why_json(tmp.path(), &early, "1");
    assert_eq!(v["finding"]["id"], early);
    let v = why_json(tmp.path(), &early, "99999999");
    assert_eq!(v["finding"]["id"], early);
}

#[test]
fn why_resolves_contract() {
    // REQ-A3S15-005: the other namespace, and the answer says which won.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let v = why_json(tmp.path(), "c-auth-1", "5000");
    assert_eq!(v["resolved_as"], "contract");
    assert!(v["finding"].is_null(), "no finding for a contract query");
    assert_eq!(contracts_of(&v), vec!["c-auth-1".to_string()]);
}

#[test]
fn why_resolves_finding() {
    // REQ-A3S15-005
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let id = id_for(tmp.path(), "shadow_authority");
    let v = why_json(tmp.path(), &id, "5000");
    assert_eq!(v["resolved_as"], "finding");
    assert_eq!(v["finding"]["kind"], "shadow_authority");
    assert_eq!(v["finding"]["severity"], "critical");
    assert_eq!(v["query"], id);
}

#[test]
fn why_unknown_id_fails_closed() {
    // REQ-A3S15-007: neither namespace matches → an error that says so.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let out = sddk(&[
        "why",
        "architecture",
        "no-such-thing",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(2), "{text}");
    assert!(out.stdout.is_empty(), "no answer may be emitted");
    assert!(text.contains("unknown"), "{text}");
    assert!(
        text.contains("contract") && text.contains("finding"),
        "both namespaces must be named: {text}"
    );
}

#[test]
fn why_empty_argument_fails_closed() {
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);
    let out = sddk(&[
        "why",
        "architecture",
        "",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]);
    assert_eq!(out.status.code(), Some(2), "{}", both(&out));
}

#[test]
fn why_declaration_invalid_fails_closed() {
    // REQ-A3S15-008
    let tmp = tempfile::tempdir().unwrap();
    // Missing.
    let out = sddk(&[
        "why",
        "architecture",
        "c-a",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(both(&out).contains("cannot read declaration"));

    // Malformed.
    fs::create_dir_all(tmp.path().join(".sddk/architecture")).unwrap();
    fs::write(tmp.path().join(FILE), "revision: [nope\n").unwrap();
    let out = sddk(&[
        "why",
        "architecture",
        "c-a",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]);
    assert_eq!(out.status.code(), Some(2));
    assert!(both(&out).contains("not a valid declaration"));
}

#[test]
fn why_finding_one_contract() {
    // REQ-A3S15-009: 1 finding → 1 contract.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_ONE);

    let id = id_for(tmp.path(), "missing_owner");
    let v = why_json(tmp.path(), &id, "5000");
    assert_eq!(contracts_of(&v), vec!["c-orphan".to_string()]);
    assert_eq!(v["finding"]["contract_ids"][0], "c-orphan");
}

#[test]
fn why_shadow_authority_two_contracts() {
    // REQ-A3S15-009: 1 finding → 2 contracts, cardinality preserved.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let id = id_for(tmp.path(), "shadow_authority");
    let v = why_json(tmp.path(), &id, "5000");
    let legs = contracts_of(&v);
    assert_eq!(legs.len(), 2, "both sides of the conflict: {legs:?}");
    assert_eq!(legs, vec!["c-auth-1".to_string(), "c-auth-2".to_string()]);
    assert_eq!(
        v["finding"]["contract_ids"].as_array().unwrap().len(),
        2,
        "the finding's own contract list is reported in full"
    );
}

#[test]
fn why_finding_three_contracts() {
    // REQ-A3S15-009: 1 finding → 3 contracts. `contradiction` names both
    // authority sides plus the projection side.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_THREE);

    let id = id_for(tmp.path(), "contradiction");
    let v = why_json(tmp.path(), &id, "5000");
    assert_eq!(
        contracts_of(&v),
        vec![
            "c-auth-1".to_string(),
            "c-auth-2".to_string(),
            "c-proj".to_string()
        ],
        "no collapse"
    );
}

#[test]
fn why_two_findings_same_subject_keep_distinct_ids() {
    // REQ-A3S15-003 end to end: `comp:dup` yields a shadow_authority and a
    // contradiction, and neither answer may absorb the other.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_THREE);

    let rows = finding_rows(tmp.path(), "5000");
    let shadow = rows
        .iter()
        .find(|r| r["kind"] == "shadow_authority")
        .unwrap();
    let contra = rows.iter().find(|r| r["kind"] == "contradiction").unwrap();
    assert_eq!(shadow["subjects"][0], "comp:dup");
    assert_eq!(contra["subjects"][0], "comp:dup");
    assert_ne!(shadow["id"], contra["id"], "distinct ids on one subject");

    let a = why_json(tmp.path(), shadow["id"].as_str().unwrap(), "5000");
    let b = why_json(tmp.path(), contra["id"].as_str().unwrap(), "5000");
    assert_eq!(a["finding"]["kind"], "shadow_authority");
    assert_eq!(b["finding"]["kind"], "contradiction");
    assert_ne!(contracts_of(&a), contracts_of(&b), "different legs");
}

#[test]
fn why_leg_carries_assessment_intent_evidence() {
    // REQ-A3S15-010: each leg carries the assessment, the declared intent and
    // the evidence — in separate fields, so provenance class is never merged.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let id = id_for(tmp.path(), "shadow_authority");
    let v = why_json(tmp.path(), &id, "5000");
    let leg = &v["contracts"][0];

    // ASSESSMENT
    assert_eq!(leg["assessment"]["outcome"], "unknown");
    assert_eq!(leg["assessment"]["evidence_present"], false);
    assert_eq!(leg["assessment"]["missing_evidence"][0], "not_provided");
    // DECLARED
    assert_eq!(leg["intent"]["decisions"][0], "ADR-0112");
    assert_eq!(leg["intent"]["specs"][0], "arch-spec-032");
    // OBSERVED
    assert!(leg["evidence"].as_array().unwrap().is_empty());
    // And the participation text is payload-derived, not the finding message.
    let p = leg["participates_because"].as_str().unwrap();
    assert!(p.contains("comp:dup"), "{p}");
    assert!(
        !p.contains("2 contracts claim"),
        "must not reuse the message: {p}"
    );
}

#[test]
fn why_reports_unresolved_evidence_leg() {
    // REQ-A3S15-012: the substrate has no evidence→software edge. It is
    // reported, and it is the same in every answer.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let v = why_json(tmp.path(), &id_for(tmp.path(), "shadow_authority"), "5000");
    let edges = v["unresolved_edges"].as_array().expect("unresolved_edges");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["edge"], "evidence→observes→software_relation");
    let reason = edges[0]["reason"].as_str().unwrap();
    assert!(reason.contains("metadata"), "{reason}");
    assert!(
        reason.contains("no evidence node"),
        "the reason must name the substrate gap: {reason}"
    );

    // It is also represented in the prepared WHY-NOT ADT.
    let reasons = v["why_not"].as_array().unwrap();
    assert!(
        reasons.iter().any(|r| r["reason"] == "unknown_relation"),
        "{reasons:?}"
    );
}

#[test]
fn why_reaches_software_units() {
    // REQ-A3S15-013: the answer reaches software where the substrate allows it.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);

    let v = why_json(tmp.path(), &id_for(tmp.path(), "shadow_authority"), "5000");
    assert_eq!(v["contracts"][0]["software_units"][0], "comp:dup");
    let text = both(&sddk(&[
        "why",
        "architecture",
        "c-auth-1",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]));
    assert!(text.contains("software:      comp:dup"), "{text}");
}

#[test]
fn why_is_deterministic() {
    // REQ-A3S15-016
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_THREE);

    let id = id_for(tmp.path(), "contradiction");
    let a = sddk(&[
        "why",
        "architecture",
        &id,
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
        "--format",
        "json",
    ]);
    let b = sddk(&[
        "why",
        "architecture",
        &id,
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
        "--format",
        "json",
    ]);
    assert_eq!(a.stdout, b.stdout, "byte-identical answers");
}

#[test]
fn why_text_and_json_agree() {
    // REQ-A3S15-016: same facts in both renderings, not merely the same verdict.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_THREE);

    let id = id_for(tmp.path(), "contradiction");
    let v = why_json(tmp.path(), &id, "5000");
    let text = both(&sddk(&[
        "why",
        "architecture",
        &id,
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]));

    assert!(text.contains(&v["finding"]["id"].as_str().unwrap().to_string()));
    assert!(text.contains(v["finding"]["kind"].as_str().unwrap()));
    assert!(text.contains(v["resolved_as"].as_str().unwrap()));
    for c in v["contracts"].as_array().unwrap() {
        assert!(text.contains(c["contract"].as_str().unwrap()), "{text}");
        assert!(text.contains(c["kind"].as_str().unwrap()));
        assert!(text.contains(c["assessment"]["outcome"].as_str().unwrap()));
        assert!(text.contains(c["intent"]["decisions"][0].as_str().unwrap()));
        assert!(text.contains(c["intent"]["specs"][0].as_str().unwrap()));
    }
    for e in v["unresolved_edges"].as_array().unwrap() {
        assert!(text.contains(e["edge"].as_str().unwrap()));
    }
    // No verdict and no score in either rendering.
    for forbidden in ["verdict", "score"] {
        assert!(!text.to_lowercase().contains(forbidden), "{text}");
    }
}

#[test]
fn why_mutates_nothing() {
    // REQ-A3S15-017: READ/EXPLAIN. No writes, no canonical append.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_TWO);
    let id = id_for(tmp.path(), "shadow_authority");

    let snapshot = |root: &Path| -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for e in fs::read_dir(&dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    m.insert(
                        p.to_string_lossy().into_owned(),
                        fs::read_to_string(&p).unwrap_or_default(),
                    );
                }
            }
        }
        m
    };

    let before = snapshot(tmp.path());
    let out = sddk(&[
        "why",
        "architecture",
        &id,
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(before, snapshot(tmp.path()), "why must write nothing");
}

#[test]
fn surfaces_share_the_audit_seam() {
    // REQ-A3S19-019: `receipt`, `findings` and `why` cannot disagree about what
    // the audit found, because they build one context. Asserted from outside at
    // a clock, on the same declaration.
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_THREE);

    let receipt = sddk(&[
        "architecture",
        "receipt",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
        "--format",
        "json",
    ]);
    let receipt: serde_json::Value = serde_json::from_slice(&receipt.stdout).unwrap();
    let counts = receipt["audit_findings"].as_object().unwrap().clone();

    let rows = finding_rows(tmp.path(), "5000");
    let mut mine: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rows {
        // Both surfaces now carry the canonical tag, so no mapping is needed.
        let tag = r["kind"].as_str().unwrap();
        *mine.entry(tag.to_string()).or_insert(0) += 1;
    }
    let theirs: BTreeMap<String, usize> = counts
        .iter()
        .map(|(k, v)| (k.clone(), v.as_u64().unwrap() as usize))
        .collect();
    assert_eq!(mine, theirs, "findings and receipt disagree");

    // `why` explains exactly the findings the other two surfaces report.
    for r in &rows {
        let v = why_json(tmp.path(), r["id"].as_str().unwrap(), "5000");
        assert_eq!(v["finding"]["kind"], r["kind"]);
        assert_eq!(v["finding"]["severity"], r["severity"]);
        assert_eq!(
            v["finding"]["contract_ids"].as_array().unwrap().len(),
            r["contract_ids"].as_array().unwrap().len()
        );
    }
}

#[test]
fn why_reports_where_receipt_errors_out() {
    // REQ-A3S15-018, and A3-S14's recorded observation: `why` computes no delta,
    // so it explains an unevaluable contract where `receipt` fails. The point is
    // that it reports `not_evaluated` with a reason — never an empty answer and
    // never an implied "fine".
    let tmp = tempfile::tempdir().unwrap();
    write_decl(tmp.path(), DECL_ONE);

    let out = sddk(&[
        "why",
        "architecture",
        "c-orphan",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
        "--format",
        "json",
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", both(&out));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["contracts"][0]["assessment"]["outcome"], "not_evaluated");
    assert!(
        v["why_not"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["reason"] == "contract_not_evaluable"),
        "{:?}",
        v["why_not"]
    );
}

#[test]
fn why_does_not_decide_a_namespace_by_shape() {
    // REQ-A3S15-006, honestly scoped: a true namespace collision cannot be
    // constructed — making a contract carry a finding's id changes that finding's
    // basis and therefore its id, so `id = hash(basis(id))` would need a fixed
    // point. What *is* representable, and what the shape heuristic would get
    // wrong, is a contract whose id merely looks like a finding id: it must
    // resolve as a contract, because shape is not identity.
    let tmp = tempfile::tempdir().unwrap();
    let hex_id = "a".repeat(64);
    let decl = format!(
        "revision: why-rev\nknowledge_basis: test/why\nunits:\n  - id: comp:dup\ncontracts:\n  - id: {hex_id}\n    kind: single_authority\n    component: comp:dup\n    decided_by: ADR-0112\n    specified_by: arch-spec-032\n    revision: rev:1\n"
    );
    write_decl(tmp.path(), &decl);

    let out = sddk(&[
        "why",
        "architecture",
        &hex_id,
        "--root",
        tmp.path().to_str().unwrap(),
        "--now-ms",
        "5000",
        "--format",
        "json",
    ]);
    let text = both(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        v["resolved_as"], "contract",
        "a 64-hex contract id must not be mistaken for a finding id"
    );

    // The ambiguity branch itself is a source-level invariant: it must exist and
    // must not be silently ordered. Pinned structurally because no fixture can
    // reach it (see the note above).
    let src = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/why_cmd.rs"))
        .expect("why_cmd.rs");
    assert!(
        src.contains("(true, Some(_))"),
        "the ambiguity arm must remain reachable and explicit"
    );
    assert!(
        src.contains("is ambiguous"),
        "the ambiguity arm must name the problem"
    );
}
