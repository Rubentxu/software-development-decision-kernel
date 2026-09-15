// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_receipt/compose.rs — A3-S9 / AC8 aggregation.
//
// `compose_receipt` aggregates the outputs of the native capabilities
// (AC2 graph, AC4 delta, AC5 audit, AC6 mutations, AC7 lenses) into the named
// `ArchitectureConformanceReceipt`. It derives nothing: every value comes from
// an input. Pure: no IO, no clock.
//
// Verdict (AC-041-003):
//   any mandatory unresolved finding not covered by a waiver -> Blocked
//   all mandatory findings covered by waivers               -> PassWithWaivers
//   no mandatory findings                                   -> Pass

use crate::architectural_contract::ContractId;
use crate::architecture_conformance::{ArchitectureConformanceDelta, DeltaContractStatus};
use crate::architecture_debverify::{DebVerifyAudit, FindingSeverity};
use crate::architecture_mutation::MutationSuiteReceipt;
use crate::knowledge::EventTime;
use crate::paradigm_lens::LensEvaluation;

use super::self_audit::evaluate_class_coverage;
use super::types::{
    ArchitectureConformanceReceipt, ClaimResult, CompatibilityEntry, LensResult, MutationResult,
    ReceiptBasis, ReceiptId, ReceiptVerdict, UnresolvedFinding,
};

/// The inputs the receipt aggregates. Every value is produced elsewhere.
pub struct ReceiptInputs<'a> {
    /// Exact named revision (AC-041-001).
    pub revision: String,
    /// Human-readable knowledge basis.
    pub knowledge_basis: String,
    /// AC4's change-scoped delta (digests, claims, witnesses).
    pub delta: &'a ArchitectureConformanceDelta,
    /// AC5's global audit.
    pub audit: &'a DebVerifyAudit,
    /// AC6's mutation results (negative evidence).
    pub mutations: &'a MutationSuiteReceipt,
    /// AC7's lens evaluations (advisory).
    pub lenses: &'a [LensEvaluation],
    /// Governed waivers supplied by the caller.
    pub waivers: &'a [String],
    /// Provider contributions; empty in Base mode (AC-041-005).
    pub provider_basis: &'a [String],
}

fn waiver_for(kind: &str, subjects: &[String], waivers: &[String]) -> Vec<String> {
    // A waiver covers a finding iff it names the finding's kind or one of its
    // subjects. The composer never invents a waiver.
    let mut out: Vec<String> = waivers
        .iter()
        .filter(|w| {
            let lower = w.to_ascii_lowercase();
            lower.contains(&kind.to_ascii_lowercase())
                || subjects
                    .iter()
                    .any(|s| lower.contains(&s.to_ascii_lowercase()))
        })
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Aggregate the capabilities into a receipt.
pub fn compose_receipt(
    inputs: ReceiptInputs<'_>,
    now: EventTime,
) -> ArchitectureConformanceReceipt {
    let basis = ReceiptBasis {
        revision: inputs.revision.clone(),
        contract_set_digest: inputs.delta.contract_set_digest,
        semantic_graph_digest: inputs.audit.graph_digest.clone(),
        verification_plan_digest: inputs.delta.plan_digest,
        knowledge_basis: inputs.knowledge_basis.clone(),
    };

    // ── claim results + compatibility rows ─────────────────────────────────
    let mut claim_results: Vec<ClaimResult> = inputs
        .delta
        .statuses()
        .into_iter()
        .map(|(contract, status)| ClaimResult { contract, status })
        .collect();
    claim_results.sort_by(|a, b| a.contract.cmp(&b.contract));

    let mut compatibility_delta: Vec<CompatibilityEntry> = claim_results
        .iter()
        .filter(|c| {
            matches!(
                c.status,
                DeltaContractStatus::Stale | DeltaContractStatus::Unknown
            )
        })
        .map(|c| CompatibilityEntry {
            contract: c.contract.clone(),
            status: c.status,
        })
        .collect();
    compatibility_delta.sort_by(|a, b| a.contract.cmp(&b.contract));

    let unknowns: Vec<ContractId> = inputs.delta.unknowns();
    let contradictions: Vec<ContractId> = inputs.delta.contradictions().to_vec();

    // ── mutation results (negative evidence) ────────────────────────────────
    let mut mutation_results: Vec<MutationResult> = inputs
        .mutations
        .probes
        .iter()
        .map(|p| MutationResult {
            mutation: p.spec_id.clone(),
            kind: p.kind,
            detected: p.detected,
        })
        .collect();
    mutation_results.sort_by(|a, b| a.mutation.cmp(&b.mutation));

    // ── lens results (advisory) ─────────────────────────────────────────────
    let mut lens_results: Vec<LensResult> = inputs
        .lenses
        .iter()
        .map(|l| LensResult {
            lens: l.assessment.lens,
            anchor: l.assessment.anchor.locator(),
            status: l.assessment.status,
            basis: l.assessment.basis,
        })
        .collect();
    lens_results.sort_by(|a, b| {
        a.lens
            .domain_tag()
            .cmp(b.lens.domain_tag())
            .then_with(|| a.anchor.cmp(&b.anchor))
    });

    // ── audit findings ─────────────────────────────────────────────────────
    let mut audit_findings: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for (kind, n) in inputs.audit.counts() {
        audit_findings.insert(kind.canonical_tag().to_string(), n);
    }

    // ── evidence citations (sorted by ordering key) ─────────────────────────
    let mut evidence_refs: Vec<crate::evidence_ref::EvidenceRef> = Vec::new();
    for l in inputs.lenses {
        evidence_refs.extend(l.assessment.evidence_refs.iter().cloned());
    }
    for f in &inputs.audit.findings {
        for id in &f.contract_ids {
            evidence_refs.push(crate::evidence_ref::EvidenceRef::new(
                crate::evidence_ref::EvidenceKind::Planning,
                format!("audit:{}:{}", f.kind.canonical_tag(), id.as_str()),
            ));
        }
    }
    evidence_refs.sort();
    evidence_refs.dedup();

    // ── unresolved findings (AC-041-003) ────────────────────────────────────
    let mut unresolved: Vec<UnresolvedFinding> = Vec::new();

    for cid in &unknowns {
        let subjects = vec![cid.as_str().to_string()];
        unresolved.push(UnresolvedFinding {
            kind: "unknown".to_string(),
            waiver_refs: waiver_for("unknown", &subjects, inputs.waivers),
            subjects,
            mandatory: true,
        });
    }
    for cid in &contradictions {
        let subjects = vec![cid.as_str().to_string()];
        unresolved.push(UnresolvedFinding {
            kind: "contradicted".to_string(),
            waiver_refs: waiver_for("contradicted", &subjects, inputs.waivers),
            subjects,
            mandatory: true,
        });
    }
    for f in &inputs.audit.findings {
        let mandatory = matches!(
            f.severity,
            FindingSeverity::Critical | FindingSeverity::High
        );
        unresolved.push(UnresolvedFinding {
            kind: f.kind.canonical_tag().to_string(),
            waiver_refs: waiver_for(f.kind.canonical_tag(), &f.subjects, inputs.waivers),
            subjects: f.subjects.clone(),
            mandatory,
        });
    }
    for p in inputs.mutations.probes.iter().filter(|p| !p.detected) {
        let subjects = vec![p.spec_id.as_str().to_string()];
        unresolved.push(UnresolvedFinding {
            kind: "negative_evidence_gap".to_string(),
            waiver_refs: waiver_for("negative_evidence_gap", &subjects, inputs.waivers),
            subjects,
            mandatory: true,
        });
    }
    unresolved.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.subjects.cmp(&b.subjects))
    });

    let mandatory_uncovered = unresolved
        .iter()
        .any(|f| f.mandatory && f.waiver_refs.is_empty());
    let mandatory_total = unresolved.iter().filter(|f| f.mandatory).count();
    let verdict = if mandatory_uncovered {
        ReceiptVerdict::Blocked
    } else if mandatory_total > 0 {
        ReceiptVerdict::PassWithWaivers
    } else {
        ReceiptVerdict::Pass
    };

    let class_coverage = evaluate_class_coverage(inputs.audit, inputs.mutations);

    let mut waivers: Vec<String> = inputs.waivers.to_vec();
    waivers.sort();
    waivers.dedup();

    let id = ReceiptId::derive(&basis, verdict);

    ArchitectureConformanceReceipt {
        id,
        basis,
        claim_results,
        evidence_refs,
        provider_basis: inputs.provider_basis.to_vec(),
        unknowns,
        contradictions,
        compatibility_delta,
        mutation_results,
        lens_results,
        audit_findings,
        vector: inputs.delta.vector,
        unresolved,
        class_coverage,
        waivers,
        verdict,
        created_at: now,
    }
}
