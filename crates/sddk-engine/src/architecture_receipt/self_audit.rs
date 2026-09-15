// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_receipt/self_audit.rs — A3-S9 / AC8 historical-class coverage.
//
// AC-041-002 requires the native analysis to reproduce a representative set of
// historical A0/A1 finding classes. This module does not detect anything: it
// maps the outputs of AC5 (audit) and AC6 (mutations) onto the five classes and
// reports whether each was reproduced, with its evidence.

use crate::architecture_debverify::{DebVerifyAudit, DebVerifyFindingKind};
use crate::architecture_mutation::MutationSuiteReceipt;

use super::types::{ClassCoverage, HistoricalClass};

/// Whether the native analysis reproduced each historical class.
pub fn evaluate_class_coverage(
    audit: &DebVerifyAudit,
    mutations: &MutationSuiteReceipt,
) -> Vec<ClassCoverage> {
    let mut out = Vec::with_capacity(HistoricalClass::ALL.len());

    // 1. duplicate/shadow authority <- AC5 ShadowAuthority
    out.push(class_from_findings(
        HistoricalClass::DuplicateAuthority,
        audit,
        &[DebVerifyFindingKind::ShadowAuthority],
    ));

    // 2. dependency boundary <- AC5 AuthorityBypass
    out.push(class_from_findings(
        HistoricalClass::DependencyBoundary,
        audit,
        &[DebVerifyFindingKind::AuthorityBypass],
    ));

    // 3. bounded compatibility <- AC5 StaleCompatibility or MissingOwner
    out.push(class_from_findings(
        HistoricalClass::BoundedCompatibility,
        audit,
        &[
            DebVerifyFindingKind::StaleCompatibility,
            DebVerifyFindingKind::MissingOwner,
        ],
    ));

    // 4. projection/authority confusion <- AC5 Contradiction
    out.push(class_from_findings(
        HistoricalClass::ProjectionOnly,
        audit,
        &[DebVerifyFindingKind::Contradiction],
    ));

    // 5. missing negative evidence <- AC6 mutation suite proves the guards fire
    let reproduced = !mutations.probes.is_empty() && mutations.all_detected;
    let evidence: Vec<String> = mutations
        .probes
        .iter()
        .filter(|p| p.detected)
        .map(|p| format!("mutation:{}", p.spec_id))
        .collect();
    out.push(ClassCoverage {
        class: HistoricalClass::MissingNegativeEvidence,
        reproduced,
        evidence,
    });

    out
}

fn class_from_findings(
    class: HistoricalClass,
    audit: &DebVerifyAudit,
    kinds: &[DebVerifyFindingKind],
) -> ClassCoverage {
    let mut evidence: Vec<String> = Vec::new();
    for kind in kinds {
        for f in audit.by_kind(*kind) {
            for subject in &f.subjects {
                evidence.push(format!("{}:{subject}", kind.canonical_tag()));
            }
            for id in &f.contract_ids {
                evidence.push(format!("{}:{}", kind.canonical_tag(), id.as_str()));
            }
        }
    }
    evidence.sort();
    evidence.dedup();
    ClassCoverage {
        class,
        reproduced: !evidence.is_empty(),
        evidence,
    }
}
