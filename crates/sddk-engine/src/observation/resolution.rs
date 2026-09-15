//! Contradiction-preserving resolution (REQ-A4S0-010..013).
//!
//! A pure function over an immutable set. It never mutates the set, never sorts the
//! evidence away, and never picks a winner.

use serde::Serialize;

use super::types::{ObservationId, ObservationSet, ObservationStance, RelationId};

/// What the evidence for one relation amounts to.
///
/// A closed ADT, never a confidence number (REQ-A4S0-012).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "resolution")]
pub enum EvidenceResolution {
    /// At least one observation affirms and none denies.
    Supported {
        /// The relation.
        relation: RelationId,
        /// The affirming observations.
        supporting: Vec<ObservationId>,
    },
    /// At least one observation denies and none affirms.
    Contradicted {
        /// The relation.
        relation: RelationId,
        /// The denying observations.
        contradicting: Vec<ObservationId>,
    },
    /// Both exist. **Not a tie to break**: the disagreement is the information.
    Conflicted {
        /// The relation.
        relation: RelationId,
        /// The affirming observations.
        supporting: Vec<ObservationId>,
        /// The denying observations.
        contradicting: Vec<ObservationId>,
    },
    /// Nothing covers the relation, or provenance is missing. An explicit gap.
    Insufficient {
        /// The relation.
        relation: RelationId,
        /// Why there is no resolution.
        gap: String,
    },
}

impl EvidenceResolution {
    /// Canonical short tag.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            EvidenceResolution::Supported { .. } => "supported",
            EvidenceResolution::Contradicted { .. } => "contradicted",
            EvidenceResolution::Conflicted { .. } => "conflicted",
            EvidenceResolution::Insufficient { .. } => "insufficient",
        }
    }

    /// Whether any observation covered the relation.
    pub fn is_covered(&self) -> bool {
        !matches!(self, EvidenceResolution::Insufficient { .. })
    }
}

/// Resolve the evidence for one relation. Pure and deterministic.
///
/// Order-independent by construction: the inputs are partitioned by stance and the
/// ids are already in canonical order, so insertion order cannot reach the result
/// (REQ-A4S0-011).
pub fn resolve_relation(set: &ObservationSet, relation: &RelationId) -> EvidenceResolution {
    let covering = set.for_relation(relation);
    if covering.is_empty() {
        return EvidenceResolution::Insufficient {
            relation: relation.clone(),
            gap: "no observation covers this relation".to_string(),
        };
    }

    let mut supporting: Vec<ObservationId> = Vec::new();
    let mut contradicting: Vec<ObservationId> = Vec::new();
    for o in covering {
        match o.stance {
            ObservationStance::Affirms => supporting.push(o.id.clone()),
            ObservationStance::Denies => contradicting.push(o.id.clone()),
        }
    }
    supporting.sort();
    supporting.dedup();
    contradicting.sort();
    contradicting.dedup();

    match (supporting.is_empty(), contradicting.is_empty()) {
        (false, true) => EvidenceResolution::Supported {
            relation: relation.clone(),
            supporting,
        },
        (true, false) => EvidenceResolution::Contradicted {
            relation: relation.clone(),
            contradicting,
        },
        (false, false) => EvidenceResolution::Conflicted {
            relation: relation.clone(),
            supporting,
            contradicting,
        },
        // Cannot happen: `covering` is non-empty, so at least one side has an entry.
        (true, true) => EvidenceResolution::Insufficient {
            relation: relation.clone(),
            gap: "observations carry no stance".to_string(),
        },
    }
}
