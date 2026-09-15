// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_mutation/run.rs — A3-S6 / AC6 probe runner.
//
// `run_mutation_probe` applies one mutation to a *clone* of the sandbox and
// evaluates the expected guard against the clone. `run_mutation_suite` runs a
// set of specs and summarises. Both are pure: no IO, no clock reads.

use sha2::{Digest, Sha256};

use super::sandbox::{MutationSandbox, apply_injection, evaluate_guard};
use super::types::{
    GuardCheck, GuardId, GuardScope, MutationError, MutationGuard, MutationId, MutationInjection,
    MutationKind, MutationProbe, MutationSpec, MutationSuiteReceipt,
};

/// Domain prefix for the suite digest.
pub const SUITE_DIGEST_DOMAIN: &str = "sddk.architecture_mutation.suite.v1|";

/// Run one mutation probe.
///
/// The input `sandbox` is never mutated (REQ-AC6-002).
pub fn run_mutation_probe(
    sandbox: &MutationSandbox,
    spec: &MutationSpec,
    guard: &MutationGuard,
) -> MutationProbe {
    let mut candidate = sandbox.clone();
    let applied = apply_injection(&mut candidate, &spec.target_path, &spec.injection);
    let hits = if applied {
        evaluate_guard(&candidate, &guard.scope, &guard.check)
    } else {
        Vec::new()
    };
    MutationProbe {
        spec_id: spec.id.clone(),
        kind: spec.kind,
        target_path: spec.target_path.clone(),
        mutation_applied: applied,
        expected_guard: spec.expected_guard.clone(),
        // `detected` is true only when the mutation actually applied AND the
        // guard produced evidence (REQ-AC6-007).
        detected: applied && !hits.is_empty(),
        evidence: hits,
        expected_contract: spec.expected_contract.clone(),
    }
}

/// Run every spec against its guard. Probes are sorted by `spec_id`.
pub fn run_mutation_suite(
    sandbox: &MutationSandbox,
    specs: &[MutationSpec],
    guards: &[MutationGuard],
) -> Result<MutationSuiteReceipt, MutationError> {
    let mut probes: Vec<MutationProbe> = Vec::with_capacity(specs.len());
    for spec in specs {
        let guard = guards
            .iter()
            .find(|g| g.id == spec.expected_guard)
            .ok_or_else(|| MutationError::UnknownGuard {
                spec: spec.id.clone(),
                guard: spec.expected_guard.clone(),
            })?;
        probes.push(run_mutation_probe(sandbox, spec, guard));
    }
    probes.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));
    let all_detected = probes.iter().all(|p| p.detected);
    let digest = suite_digest(&probes);
    Ok(MutationSuiteReceipt {
        probes,
        all_detected,
        digest,
    })
}

/// sha256 over the canonical receipt payload (REQ-AC6-015/017).
pub fn suite_digest(probes: &[MutationProbe]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(SUITE_DIGEST_DOMAIN.as_bytes());
    for p in probes {
        h.update(p.spec_id.as_str().as_bytes());
        h.update(b"|kind|");
        h.update(p.kind.canonical_tag().as_bytes());
        h.update(b"|target|");
        h.update(p.target_path.as_bytes());
        h.update(b"|applied|");
        h.update(if p.mutation_applied { b"1" } else { b"0" });
        h.update(b"|guard|");
        h.update(p.expected_guard.as_str().as_bytes());
        h.update(b"|detected|");
        h.update(if p.detected { b"1" } else { b"0" });
        h.update(b"|evidence|");
        for hit in &p.evidence {
            h.update(hit.path.as_bytes());
            h.update(b":");
            h.update(hit.line.to_string().as_bytes());
            h.update(b":");
            h.update(hit.matched.as_bytes());
            h.update(b";");
        }
        h.update(b"\n");
    }
    h.finalize().into()
}

// ─────────────────────────────────────────────────────────────────────────────
// The four critical mutations (REQ-AC6-011..013, AC-038-003)
// ─────────────────────────────────────────────────────────────────────────────

fn contract(id: &str) -> Option<crate::architectural_contract::ContractId> {
    crate::architectural_contract::ContractId::new(id).ok()
}

/// The closed set of initial critical mutations named by upstream
/// `arch-spec-038` AC-038-003.
pub fn critical_mutations() -> Vec<MutationSpec> {
    vec![
        // 1. inject domain → provider SDK dependency
        MutationSpec {
            id: MutationId::new("mut-provider-type-leak"),
            kind: MutationKind::ProviderTypeLeak,
            target_path: "crates/sddk-engine/src/domain/order.rs".to_string(),
            injection: MutationInjection::PrependLine("use tonic::transport::Channel;".to_string()),
            expected_guard: GuardId::new("guard.provider_boundary"),
            expected_contract: contract("ac6.contract.provider_boundary"),
        },
        // 2. inject Alignment → AuthorityEngine dependency
        MutationSpec {
            id: MutationId::new("mut-alignment-to-governance"),
            kind: MutationKind::AlignmentToGovernance,
            target_path: "crates/sddk-engine/src/alignment/lens.rs".to_string(),
            injection: MutationInjection::PrependLine(
                "use crate::authority_engine::AuthorityEngine;".to_string(),
            ),
            expected_guard: GuardId::new("guard.alignment_governance_boundary"),
            expected_contract: contract("ac6.contract.alignment_governance_boundary"),
        },
        // 3. inject Workbook → canonical write path
        MutationSpec {
            id: MutationId::new("mut-workbook-canonical-write"),
            kind: MutationKind::WorkbookCanonicalWrite,
            target_path: "crates/sddk-engine/src/workbook/plan.rs".to_string(),
            injection: MutationInjection::AppendLine(
                "fn __probe() { self.write_canonical(&entry).ok(); }".to_string(),
            ),
            expected_guard: GuardId::new("guard.workbook_projection_only"),
            expected_contract: contract("ac6.contract.workbook_projection_only"),
        },
        // 4. inject a second canonical event writer
        MutationSpec {
            id: MutationId::new("mut-second-canonical-writer"),
            kind: MutationKind::SecondCanonicalWriter,
            target_path: "crates/sddk-engine/src/canonical_event_log.rs".to_string(),
            injection: MutationInjection::AppendLine(
                "fn append_canonical_entry(evt: Event) -> Result<()> { Ok(()) }".to_string(),
            ),
            expected_guard: GuardId::new("guard.single_canonical_writer"),
            expected_contract: contract("ac6.contract.single_canonical_writer"),
        },
    ]
}

/// The guards paired with [`critical_mutations`].
pub fn critical_guards() -> Vec<MutationGuard> {
    vec![
        MutationGuard {
            id: GuardId::new("guard.provider_boundary"),
            scope: GuardScope::prefixes(&["crates/sddk-engine/src/"]),
            check: GuardCheck::ForbiddenLines {
                forbidden: vec![
                    "use tonic".to_string(),
                    "use prost".to_string(),
                    "use regen".to_string(),
                    "use host_sdk::".to_string(),
                    "use crate::provider".to_string(),
                    "use crate::agent_host".to_string(),
                ],
            },
        },
        MutationGuard {
            id: GuardId::new("guard.alignment_governance_boundary"),
            scope: GuardScope::prefixes(&["crates/sddk-engine/src/alignment/"]),
            check: GuardCheck::ForbiddenLines {
                forbidden: vec![
                    "use crate::authority_engine".to_string(),
                    "use crate::capability".to_string(),
                    "use crate::effective_instructions".to_string(),
                ],
            },
        },
        MutationGuard {
            id: GuardId::new("guard.workbook_projection_only"),
            scope: GuardScope::prefixes(&["crates/sddk-engine/src/workbook/"]),
            check: GuardCheck::ForbiddenLines {
                forbidden: vec![
                    "write_canonical(".to_string(),
                    "store_canonical(".to_string(),
                    "append_canonical(".to_string(),
                ],
            },
        },
        MutationGuard {
            id: GuardId::new("guard.single_canonical_writer"),
            scope: GuardScope::prefixes(&["crates/sddk-engine/src/canonical_event_log"]),
            check: GuardCheck::MaxOccurrences {
                pattern: "fn append_canonical".to_string(),
                max_allowed: 1,
            },
        },
    ]
}

/// Convenience: run the four critical mutations against a sandbox.
pub fn run_critical_mutations(
    sandbox: &MutationSandbox,
) -> Result<MutationSuiteReceipt, MutationError> {
    run_mutation_suite(sandbox, &critical_mutations(), &critical_guards())
}
