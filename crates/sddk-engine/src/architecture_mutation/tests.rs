// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_mutation/tests.rs — A3-S6 / AC6 acceptance +
// anti-encroachment + boundary tests.
//
// REQ-AC6-001..020 pinned via the cycle-bounded spec
// `docs/architecture/specs/arch-spec-A3-S6-ac6-mutation-probes.md`.

use super::*;
use crate::architectural_contract::ContractId;

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

/// A minimal representative source tree: one clean file per guarded subtree.
///
/// The canonical event log carries exactly one writer so
/// `MaxOccurrences { max_allowed: 1 }` has a meaningful negative control.
fn seed_sandbox() -> MutationSandbox {
    MutationSandbox::from_sources([
        (
            "crates/sddk-engine/src/domain/order.rs",
            "// domain model\npub struct Order { pub id: u64 }\n",
        ),
        (
            "crates/sddk-engine/src/alignment/lens.rs",
            "// alignment lens\npub fn evaluate() -> bool { true }\n",
        ),
        (
            "crates/sddk-engine/src/workbook/plan.rs",
            "// workbook view (projection only)\npub struct PlanView;\n",
        ),
        (
            "crates/sddk-engine/src/canonical_event_log.rs",
            "// canonical fact log\npub fn append_canonical(evt: u64) -> Result<(), ()> { let _ = evt; Ok(()) }\n",
        ),
    ])
}

fn spec(id: &str, injection: MutationInjection, guard: &str) -> MutationSpec {
    MutationSpec {
        id: MutationId::new(id),
        kind: MutationKind::ProviderTypeLeak,
        target_path: "crates/sddk-engine/src/a.rs".to_string(),
        injection,
        expected_guard: GuardId::new(guard),
        expected_contract: None,
    }
}

fn guard(id: &str, scope: GuardScope, check: GuardCheck) -> MutationGuard {
    MutationGuard {
        id: GuardId::new(id),
        scope,
        check,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — vocabulary (REQ-AC6-004, 009, 011)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_injection_closed_three() {
    // REQ-AC6-004: three variants, exhaustively matched here.
    let variants = [
        MutationInjection::AppendLine("x".into()),
        MutationInjection::PrependLine("x".into()),
        MutationInjection::InsertBeforeFirstLineContaining {
            needle: "n".into(),
            line: "x".into(),
        },
    ];
    for v in variants {
        // Exhaustive match proves the closed set at compile time.
        match v {
            MutationInjection::AppendLine(_)
            | MutationInjection::PrependLine(_)
            | MutationInjection::InsertBeforeFirstLineContaining { .. } => {}
        }
    }
}

#[test]
fn acceptance_guard_check_closed_two() {
    // REQ-AC6-009: two variants.
    let variants = [
        GuardCheck::ForbiddenLines {
            forbidden: vec!["x".into()],
        },
        GuardCheck::MaxOccurrences {
            pattern: "x".into(),
            max_allowed: 0,
        },
    ];
    for v in variants {
        match v {
            GuardCheck::ForbiddenLines { .. } | GuardCheck::MaxOccurrences { .. } => {}
        }
    }
}

#[test]
fn acceptance_mutation_kind_closed_four() {
    // REQ-AC6-011
    assert_eq!(MutationKind::ALL.len(), 4);
    let mut tags: Vec<&str> = MutationKind::ALL
        .iter()
        .map(|k| k.canonical_tag())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(tags.len(), 4);
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — sandbox & injection (REQ-AC6-001..005)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_probe_does_not_mutate_input_sandbox() {
    // REQ-AC6-002
    let sandbox = seed_sandbox();
    let before = sandbox.clone();
    let specs = critical_mutations();
    let guards = critical_guards();
    for spec in &specs {
        let g = guards.iter().find(|g| g.id == spec.expected_guard).unwrap();
        let _ = run_mutation_probe(&sandbox, spec, g);
    }
    assert_eq!(
        before, sandbox,
        "the runner must not mutate the input sandbox"
    );
}

#[test]
fn acceptance_inject_creates_absent_target() {
    // REQ-AC6-003
    let mut sandbox = MutationSandbox::new();
    let applied = apply_injection(
        &mut sandbox,
        "crates/sddk-engine/src/new.rs",
        &MutationInjection::AppendLine("fn x() {}".into()),
    );
    assert!(applied);
    assert!(sandbox.contains("crates/sddk-engine/src/new.rs"));
    assert_eq!(
        sandbox.get("crates/sddk-engine/src/new.rs"),
        Some("fn x() {}\n")
    );
}

#[test]
fn acceptance_injection_not_applied_when_needle_absent() {
    // REQ-AC6-005
    let mut sandbox = MutationSandbox::from_sources([("a.rs", "line one\nline two\n")]);
    let before = sandbox.clone();
    let applied = apply_injection(
        &mut sandbox,
        "a.rs",
        &MutationInjection::InsertBeforeFirstLineContaining {
            needle: "NOT PRESENT".into(),
            line: "injected".into(),
        },
    );
    assert!(!applied, "an absent needle must not apply");
    assert_eq!(
        before, sandbox,
        "content must be unchanged when not applied"
    );
}

#[test]
fn acceptance_insert_before_needle_applies_once() {
    let mut sandbox = MutationSandbox::from_sources([("a.rs", "alpha\nbeta\nalpha\n")]);
    let applied = apply_injection(
        &mut sandbox,
        "a.rs",
        &MutationInjection::InsertBeforeFirstLineContaining {
            needle: "alpha".into(),
            line: "HEADER".into(),
        },
    );
    assert!(applied);
    assert_eq!(sandbox.get("a.rs"), Some("HEADER\nalpha\nbeta\nalpha\n"));
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — probe shape (REQ-AC6-006..008)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_probe_shape() {
    // REQ-AC6-006
    let sandbox = seed_sandbox();
    let s = spec(
        "m1",
        MutationInjection::PrependLine("use tonic::x;".into()),
        "g1",
    );
    let g = guard(
        "g1",
        GuardScope::all(),
        GuardCheck::ForbiddenLines {
            forbidden: vec!["use tonic".into()],
        },
    );
    let probe = run_mutation_probe(&sandbox, &s, &g);
    assert!(probe.mutation_applied);
    assert!(probe.detected);
    assert_eq!(probe.expected_guard, GuardId::new("g1"));
    assert_eq!(probe.kind, MutationKind::ProviderTypeLeak);
    assert_eq!(probe.target_path, "crates/sddk-engine/src/a.rs");
    assert!(!probe.evidence.is_empty());
}

#[test]
fn acceptance_detected_iff_guard_hits() {
    // REQ-AC6-007
    let sandbox = seed_sandbox();
    let s = spec(
        "m1",
        MutationInjection::PrependLine("use tonic::x;".into()),
        "g1",
    );

    // Guard that does NOT match the injection → not detected.
    let g_miss = guard(
        "g1",
        GuardScope::all(),
        GuardCheck::ForbiddenLines {
            forbidden: vec!["use totally_different".into()],
        },
    );
    let p_miss = run_mutation_probe(&sandbox, &s, &g_miss);
    assert!(p_miss.mutation_applied);
    assert!(!p_miss.detected);
    assert!(p_miss.evidence.is_empty());

    // Not-applied injection → never detected, even with a matching guard.
    let s_noop = spec(
        "m2",
        MutationInjection::InsertBeforeFirstLineContaining {
            needle: "NOPE".into(),
            line: "use tonic::x;".into(),
        },
        "g1",
    );
    let g_hit = guard(
        "g1",
        GuardScope::all(),
        GuardCheck::ForbiddenLines {
            forbidden: vec!["use tonic".into()],
        },
    );
    let p_noop = run_mutation_probe(&sandbox, &s_noop, &g_hit);
    assert!(!p_noop.mutation_applied);
    assert!(!p_noop.detected);
}

#[test]
fn acceptance_guard_hit_names_path_and_line() {
    // REQ-AC6-008
    let sandbox = MutationSandbox::from_sources([("a.rs", "one\ntwo\nuse tonic::x;\n")]);
    let g = guard(
        "g",
        GuardScope::all(),
        GuardCheck::ForbiddenLines {
            forbidden: vec!["use tonic".into()],
        },
    );
    let hits = evaluate_guard(&sandbox, &g.scope, &g.check);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "a.rs");
    assert_eq!(hits[0].line, 3);
    assert_eq!(hits[0].matched, "use tonic::x;");
}

#[test]
fn acceptance_guard_scope_limits_inspection() {
    // REQ-AC6-010
    let sandbox = MutationSandbox::from_sources([
        ("crates/a/x.rs", "use tonic::x;\n"),
        ("crates/b/y.rs", "use tonic::x;\n"),
    ]);
    let g = guard(
        "g",
        GuardScope::prefixes(&["crates/a/"]),
        GuardCheck::ForbiddenLines {
            forbidden: vec!["use tonic".into()],
        },
    );
    let hits = evaluate_guard(&sandbox, &g.scope, &g.check);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "crates/a/x.rs");
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — the four critical mutations (REQ-AC6-012..014, AC-UAT-010)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_critical_mutations_four_and_well_formed() {
    // REQ-AC6-012
    let specs = critical_mutations();
    assert_eq!(specs.len(), 4);
    let kinds: Vec<MutationKind> = specs.iter().map(|s| s.kind).collect();
    for k in MutationKind::ALL {
        assert!(kinds.contains(&k), "missing critical mutation for {k:?}");
    }
    for s in &specs {
        assert!(!s.target_path.is_empty());
        assert!(!s.expected_guard.as_str().is_empty());
        assert!(
            s.expected_contract.is_some(),
            "critical mutations should map to a contract (AC4 seam)"
        );
    }
    // Ties the catalogue to a stable, well-formed set.
    let ids: Vec<&str> = specs.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "mut-provider-type-leak",
            "mut-alignment-to-governance",
            "mut-workbook-canonical-write",
            "mut-second-canonical-writer",
        ]
    );
}

#[test]
fn acceptance_provider_type_leak_detected() {
    // REQ-AC6-013 / AC-UAT-010
    let sandbox = seed_sandbox();
    let specs = critical_mutations();
    let s = specs
        .iter()
        .find(|s| s.kind == MutationKind::ProviderTypeLeak)
        .unwrap();
    let g = critical_guards()
        .into_iter()
        .find(|g| g.id == s.expected_guard)
        .unwrap();
    let probe = run_mutation_probe(&sandbox, s, &g);
    assert!(probe.mutation_applied);
    assert!(
        probe.detected,
        "AC-UAT-010: an injected domain->provider SDK dependency must be detected"
    );
    assert_eq!(
        probe.evidence[0].path,
        "crates/sddk-engine/src/domain/order.rs"
    );
    assert!(probe.evidence[0].matched.contains("use tonic"));
}

#[test]
fn acceptance_alignment_to_governance_detected() {
    // REQ-AC6-013
    let sandbox = seed_sandbox();
    let s = critical_mutations()
        .into_iter()
        .find(|s| s.kind == MutationKind::AlignmentToGovernance)
        .unwrap();
    let g = critical_guards()
        .into_iter()
        .find(|g| g.id == s.expected_guard)
        .unwrap();
    let probe = run_mutation_probe(&sandbox, &s, &g);
    assert!(probe.detected);
    assert!(probe.evidence[0].matched.contains("authority_engine"));
}

#[test]
fn acceptance_workbook_canonical_write_detected() {
    // REQ-AC6-013
    let sandbox = seed_sandbox();
    let s = critical_mutations()
        .into_iter()
        .find(|s| s.kind == MutationKind::WorkbookCanonicalWrite)
        .unwrap();
    let g = critical_guards()
        .into_iter()
        .find(|g| g.id == s.expected_guard)
        .unwrap();
    let probe = run_mutation_probe(&sandbox, &s, &g);
    assert!(probe.detected);
    assert!(probe.evidence[0].matched.contains("write_canonical"));
}

#[test]
fn acceptance_second_canonical_writer_detected() {
    // REQ-AC6-013
    let sandbox = seed_sandbox();
    let s = critical_mutations()
        .into_iter()
        .find(|s| s.kind == MutationKind::SecondCanonicalWriter)
        .unwrap();
    let g = critical_guards()
        .into_iter()
        .find(|g| g.id == s.expected_guard)
        .unwrap();
    let probe = run_mutation_probe(&sandbox, &s, &g);
    assert!(probe.detected);
    // The excess (second) writer is named, not the pre-existing one.
    assert_eq!(probe.evidence.len(), 1);
    assert!(probe.evidence[0].matched.contains("append_canonical_entry"));
}

#[test]
fn acceptance_all_critical_mutations_detected() {
    // REQ-AC6-013 (AC6 exit: "prove the guard catches the injected violation")
    let sandbox = seed_sandbox();
    let receipt = run_critical_mutations(&sandbox).expect("suite");
    assert_eq!(receipt.probes.len(), 4);
    assert!(
        receipt.all_detected,
        "undetected: {:?}",
        receipt.undetected()
    );
    assert_eq!(receipt.detected_count(), 4);
}

#[test]
fn acceptance_critical_guards_have_no_false_positive() {
    // REQ-AC6-014: on the unmutated seed, no critical guard fires.
    let sandbox = seed_sandbox();
    for g in critical_guards() {
        let hits = evaluate_guard(&sandbox, &g.scope, &g.check);
        assert!(
            hits.is_empty(),
            "guard `{}` fired on the clean seed: {hits:?}",
            g.id
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — suite + determinism (REQ-AC6-015..017)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_suite_is_sorted_and_deterministic() {
    // REQ-AC6-015 / REQ-AC6-017
    let sandbox = seed_sandbox();
    let a = run_critical_mutations(&sandbox).unwrap();
    let b = run_critical_mutations(&sandbox).unwrap();
    assert_eq!(a, b, "two runs over identical inputs must be equal");
    let mut ids: Vec<&str> = a.probes.iter().map(|p| p.spec_id.as_str()).collect();
    let sorted = {
        let mut s = ids.clone();
        s.sort();
        s
    };
    assert_eq!(ids, sorted, "probes must be sorted by spec id");
    ids.dedup();
}

#[test]
fn acceptance_all_detected_semantics() {
    // REQ-AC6-016
    // A deliberately misspecified probe makes `all_detected` false.
    let sandbox = seed_sandbox();
    let mut specs = critical_mutations();
    // Point the provider-leak injection at a path the guard does not scope.
    if let Some(s) = specs
        .iter_mut()
        .find(|s| s.kind == MutationKind::ProviderTypeLeak)
    {
        s.target_path = "outside/scope/file.rs".to_string();
    }
    let receipt = run_mutation_suite(&sandbox, &specs, &critical_guards()).unwrap();
    assert!(!receipt.all_detected);
    assert_eq!(
        receipt.undetected(),
        vec![MutationId::new("mut-provider-type-leak")]
    );
    assert_eq!(receipt.detected_count(), 3);
}

#[test]
fn bonus_empty_suite_is_vacuously_detected() {
    // REQ-AC6-016: an empty suite has nothing undetected.
    let sandbox = seed_sandbox();
    let receipt = run_mutation_suite(&sandbox, &[], &critical_guards()).unwrap();
    assert!(receipt.probes.is_empty());
    assert!(receipt.all_detected);
    assert!(receipt.undetected().is_empty());
}

#[test]
fn bonus_unknown_guard_is_an_error() {
    let sandbox = seed_sandbox();
    let s = spec("m1", MutationInjection::AppendLine("x".into()), "g-missing");
    let err = run_mutation_suite(&sandbox, &[s], &[]).unwrap_err();
    match err {
        MutationError::UnknownGuard { spec, guard } => {
            assert_eq!(spec, MutationId::new("m1"));
            assert_eq!(guard, GuardId::new("g-missing"));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — AC4 seam (REQ-AC6-018)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_witnesses_from_detected_probes() {
    // REQ-AC6-018
    let sandbox = seed_sandbox();
    let receipt = run_critical_mutations(&sandbox).unwrap();
    let w = receipt.witnesses();
    assert_eq!(w.len(), 4, "one witness per detected critical probe");
    // Sorted + deduplicated.
    let mut sorted = w.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(w, sorted);
    assert!(w.contains(&ContractId::new("ac6.contract.provider_boundary").unwrap()));

    // Undetected probes contribute no witness.
    let mut specs = critical_mutations();
    specs[0].target_path = "outside/scope/file.rs".to_string();
    let partial = run_mutation_suite(&sandbox, &specs, &critical_guards()).unwrap();
    assert!(partial.witnesses().len() < 4);
}

// ─────────────────────────────────────────────────────────────────────────────
// anti-encroachment (REQ-AC6-019, 020)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_no_forbidden_imports() {
    // REQ-AC6-019
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("sandbox.rs", include_str!("sandbox.rs")),
        ("run.rs", include_str!("run.rs")),
    ];
    let forbidden = [
        "use crate::paradigm_profile",
        "use crate::alignment",
        "use crate::debverify",
        "use crate::provider",
        "use crate::host_sdk",
        "use crate::agent_host",
        "use crate::effective_instructions",
        "use crate::capability",
    ];
    let mut offenders: Vec<(String, String)> = Vec::new();
    for (file, src) in sources {
        for (idx, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("use ") {
                continue;
            }
            for f in forbidden {
                if trimmed.starts_with(f) {
                    offenders.push((file.to_string(), format!("line {}: {}", idx + 1, trimmed)));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "architecture_mutation imports forbidden surfaces: {offenders:?}"
    );
}

#[test]
fn anti_encroachment_no_filesystem_writes() {
    // REQ-AC6-001 / REQ-AC6-020
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("sandbox.rs", include_str!("sandbox.rs")),
        ("run.rs", include_str!("run.rs")),
    ];
    for (file, src) in sources {
        for forbidden in [
            "use std::fs",
            "use std::io::Write",
            "std::fs::",
            "File::create(",
            "File::open(",
            "std::process::Command",
            "ArchitectureClaim {",
            "ArchitectureClaim::",
        ] {
            assert!(
                !src.contains(forbidden),
                "{file} must not contain `{forbidden}` (no IO, no claim construction)"
            );
        }
    }
    // The sandbox exposes no persistence method.
    let sandbox = seed_sandbox();
    let _ = sandbox.len();
    let _ = sandbox.is_empty();
    let _ = sandbox.contains("x");
    let _ = sandbox.get("x");
    let _ = sandbox.iter().count();
}

#[test]
fn anti_encroachment_sandbox_has_no_write_surface() {
    // REQ-AC6-001: the only mutating method is `insert`, used by the runner on
    // a clone. There is no path from the sandbox to durable storage.
    let src = include_str!("sandbox.rs");
    for forbidden in ["save", "flush", "persist", "commit", "write_to_disk"] {
        assert!(
            !src.contains(&format!("fn {forbidden}")),
            "MutationSandbox must not expose `{forbidden}`"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// boundary — MaxOccurrences
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bonus_max_occurrences_boundary() {
    // REQ-AC6-009: exactly max_allowed → no hit; max_allowed + 1 → one hit.
    let check = GuardCheck::MaxOccurrences {
        pattern: "fn append_canonical".into(),
        max_allowed: 1,
    };
    let ok = MutationSandbox::from_sources([("a.rs", "fn append_canonical() {}\n")]);
    assert!(evaluate_guard(&ok, &GuardScope::all(), &check).is_empty());

    let bad = MutationSandbox::from_sources([(
        "a.rs",
        "fn append_canonical() {}\nfn append_canonical_entry() {}\n",
    )]);
    let hits = evaluate_guard(&bad, &GuardScope::all(), &check);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 2);
}

#[test]
fn bonus_suite_digest_changes_with_evidence() {
    // REQ-AC6-015: the digest covers the evidence and the verdict, not just
    // the spec identity.
    let sandbox = MutationSandbox::from_sources([("a.rs", "clean\n")]);
    let g = guard(
        "g1",
        GuardScope::all(),
        GuardCheck::ForbiddenLines {
            forbidden: vec!["use tonic".into()],
        },
    );
    // Detected: the injection matches the guard.
    let hit = spec(
        "m1",
        MutationInjection::AppendLine("use tonic::x;".into()),
        "g1",
    );
    // Not detected: the injection does not match the guard.
    let miss = spec(
        "m1",
        MutationInjection::AppendLine("use harmless::x;".into()),
        "g1",
    );
    let d_hit = suite_digest(&[run_mutation_probe(&sandbox, &hit, &g)]);
    let d_miss = suite_digest(&[run_mutation_probe(&sandbox, &miss, &g)]);
    assert_ne!(
        d_hit, d_miss,
        "digest must reflect the detection verdict/evidence"
    );
}
