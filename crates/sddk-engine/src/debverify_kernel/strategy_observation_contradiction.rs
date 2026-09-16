// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// debverify_kernel/strategy_observation_contradiction.rs — A4-2:
// ObservationContradictionChallengeStrategy.
//
// Demonstrates that DebVerify is generic by surfacing contradictions from
// the observation substrate (arch-spec-042). When the same `SoftwareRelation`
// has one observation that affirms and another that denies, this strategy
// emits a `ContradictionSet` per relation. Both observations are kept; no
// latest-wins.

use crate::observation::{ObservationSet, ObservationStance};

use super::strategy::{ChallengeOutcome, ChallengeStrategy};
use super::types::{Baseline, ContradictionSet, ReconciliationScope};

/// Strategy name (stable, used in registry).
pub const STRATEGY_NAME: &str = "observation_contradiction";

/// Surfaces contradictions between affirming and denying observations.
pub struct ObservationContradictionChallengeStrategy;

impl ChallengeStrategy for ObservationContradictionChallengeStrategy {
    fn name(&self) -> &'static str {
        STRATEGY_NAME
    }

    fn applicable(&self, _scope: &ReconciliationScope) -> bool {
        // Always applicable — observation contradictions can be reconciled
        // in any scope. The scope name is irrelevant to the strategy's
        // applicability.
        true
    }

    fn challenge(
        &self,
        _baseline: &Baseline,
        evidence: &ObservationSet,
    ) -> Result<ChallengeOutcome, super::strategy::ChallengeError> {
        // Group observations by their `RelationId`.
        let mut by_relation: std::collections::BTreeMap<
            crate::observation::RelationId,
            (
                Vec<crate::observation::ObservationId>,
                Vec<crate::observation::ObservationId>,
            ),
        > = std::collections::BTreeMap::new();

        for o in evidence.observations() {
            if let crate::observation::ObservationSubject::SoftwareRelation(rel) = &o.subject {
                let entry = by_relation
                    .entry(rel.id())
                    .or_insert_with(|| (Vec::new(), Vec::new()));
                match o.stance {
                    ObservationStance::Affirms => entry.0.push(o.id.clone()),
                    ObservationStance::Denies => entry.1.push(o.id.clone()),
                }
            }
        }

        let mut contradictions: Vec<ContradictionSet> = Vec::new();
        for (relation, (mut supporting, mut contradicting)) in by_relation {
            // Only emit when BOTH sides exist. Otherwise it is not a
            // contradiction (it is a Supported or Contradicted resolution,
            // which the verifier kernel reports, not DebVerify).
            if !supporting.is_empty() && !contradicting.is_empty() {
                supporting.sort();
                contradicting.sort();
                contradictions.push(ContradictionSet {
                    relation,
                    supporting,
                    contradicting,
                });
            }
        }
        // Already in BTreeMap order (sorted by relation id); keep that.
        Ok(ChallengeOutcome::Contradictions(contradictions))
    }
}
