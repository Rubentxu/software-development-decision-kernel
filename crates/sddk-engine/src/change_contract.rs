//! ChangeContract + SHAPE / Adaptive Specialist Selection.
//!
//! Cycle: SDD-ADAPTIVE-001 (H8, order 440, context pack
//! `adaptive-sdd`).
//!
//! Pure, deterministic typed contract for an in-flight adaptation,
//! plus a SHAPE selector that picks a `Specialist` from a roster
//! based on the contract's surface, kind, and constraints.
//!
//! ## Design
//!
//! - **Closed-set `ChangeKind`.** Five surface-level change kinds.
//! - **Closed-set `SpecialistKind`.** Human / LlmAgent /
//!   RuleAdapter / LabAgent.
//! - **Closed-set `ChangeConstraint`/`RejectionReason`.** Keep
//!   audit surface small.
//! - **Pure `ShapeSelector::select`.** Deterministic.
//! - **Read-only** over roster and contract.
//! - **`#[non_exhaustive]`** everywhere.
//!
//! See:
//!
//! - `REQ-ChangeContractShape` (spec, accepted)
//! - `ADR-105` (architecture decision, accepted)
//! - `REQ-DecisionPlaneAssuranceGating`,
//!   `REQ-DecisionLabBaseline`, `REQ-DecisionLabExperimental`
//!   (dependencies).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ── ChangeKind ────────────────────────────────────────────────────────────

/// Closed-set change kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChangeKind {
    /// Behaviour-tweak / config change.
    Tweak,
    /// Refactor within the same surface.
    Refactor,
    /// New capability or surface addition.
    Feature,
    /// Bug fix.
    BugFix,
    /// Performance / cost optimisation.
    Optimisation,
}

impl ChangeKind {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            ChangeKind::Tweak => "tweak".to_string(),
            ChangeKind::Refactor => "refactor".to_string(),
            ChangeKind::Feature => "feature".to_string(),
            ChangeKind::BugFix => "bug_fix".to_string(),
            ChangeKind::Optimisation => "optimisation".to_string(),
        }
    }
}

// ── ChangeConstraint ──────────────────────────────────────────────────────

/// Closed-set change constraints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChangeConstraint {
    /// Must not touch a particular surface.
    MustNotTouch {
        /// Surface id.
        surface: String,
    },
    /// Must touch a particular surface.
    MustTouch {
        /// Surface id.
        surface: String,
    },
    /// Must pass a specific assurance profile.
    MustPassAssuranceProfile {
        /// Profile id.
        profile_id: String,
    },
    /// Must pass the UAT lifecycle (sign-off).
    MustPassUatLifecycle,
    /// Engine match rate must meet a threshold in basis points.
    MustMatchRateAtLeastBps {
        /// Threshold in basis points.
        value_bps: u32,
    },
    /// Outcome must be byte-deterministic.
    MustBeDeterministic,
}

impl ChangeConstraint {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            ChangeConstraint::MustNotTouch { surface } => {
                format!("must_not_touch:{surface}")
            }
            ChangeConstraint::MustTouch { surface } => {
                format!("must_touch:{surface}")
            }
            ChangeConstraint::MustPassAssuranceProfile { profile_id } => {
                format!("must_pass_assurance_profile:{profile_id}")
            }
            ChangeConstraint::MustPassUatLifecycle => "must_pass_uat_lifecycle".to_string(),
            ChangeConstraint::MustMatchRateAtLeastBps { value_bps } => {
                format!("must_match_rate_at_least_bps:{value_bps}")
            }
            ChangeConstraint::MustBeDeterministic => "must_be_deterministic".to_string(),
        }
    }
}

// ── ChangeContract ────────────────────────────────────────────────────────

/// A typed in-flight change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChangeContract {
    /// Contract id.
    pub contract_id: String,
    /// Kind.
    pub kind: ChangeKind,
    /// Surface (e.g. "sddk-engine/strategy_comparison").
    pub surface: String,
    /// Free-form inputs.
    pub inputs: BTreeMap<String, serde_json::Value>,
    /// Constraints on the adaptation.
    pub constraints: Vec<ChangeConstraint>,
    /// RFC-3339 timestamp supplied by the caller.
    pub created_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl ChangeContract {
    /// Schema version.
    pub const SCHEMA_VERSION: u32 = 1;

    /// Construct a new contract.
    pub fn new(
        contract_id: String,
        kind: ChangeKind,
        surface: String,
        constraints: Vec<ChangeConstraint>,
        created_at: String,
    ) -> Self {
        Self {
            contract_id,
            kind,
            surface,
            inputs: BTreeMap::new(),
            constraints,
            created_at,
            schema_version: Self::SCHEMA_VERSION,
        }
    }
}

// ── SpecialistKind ────────────────────────────────────────────────────────

/// Closed-set specialist kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SpecialistKind {
    /// Human operator.
    Human,
    /// LLM-driven sub-agent.
    LlmAgent,
    /// Deterministic rule-based adapter.
    RuleAdapter,
    /// Search-strategy-driven lab agent.
    LabAgent,
}

// ── Specialist ────────────────────────────────────────────────────────────

/// A candidate specialist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Specialist {
    /// Specialist id.
    pub specialist_id: String,
    /// Kind.
    pub kind: SpecialistKind,
    /// Surfaces the specialist can cover (e.g. "sddk-engine/foo").
    pub capabilities: Vec<String>,
    /// Admission criteria required to admit this specialist.
    pub admission_criteria: Vec<ChangeConstraint>,
    /// Max concurrent contracts.
    pub max_concurrent: u32,
}

// ── SpecialistRoster ──────────────────────────────────────────────────────

/// A roster of specialists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SpecialistRoster {
    specialists: Vec<Specialist>,
}

impl SpecialistRoster {
    /// Construct a roster.
    pub fn new(specialists: Vec<Specialist>) -> Self {
        Self { specialists }
    }

    /// Read-only access.
    pub fn registry(&self) -> &[Specialist] {
        &self.specialists
    }

    /// Find specialists that cover the surface.
    pub fn by_surface(&self, surface: &str) -> Vec<&Specialist> {
        let mut out: Vec<&Specialist> = self
            .specialists
            .iter()
            .filter(|s| s.capabilities.iter().any(|c| c == surface))
            .collect();
        out.sort_by(|a, b| a.specialist_id.cmp(&b.specialist_id));
        out
    }
}

// ── RejectionReason ───────────────────────────────────────────────────────

/// Closed-set rejection reasons.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RejectionReason {
    /// No specialist covers the surface.
    NoSurfaceCoverage {
        /// Surface id.
        surface: String,
    },
    /// A constraint cannot be satisfied.
    ConstraintUnsatisfied {
        /// Constraint id.
        constraint: String,
    },
    /// Capabilities don't match the contract's required surfaces.
    CapabilityMismatch {
        /// Required capabilities.
        required: Vec<String>,
        /// Available capabilities in roster.
        available: Vec<String>,
    },
}

impl RejectionReason {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            RejectionReason::NoSurfaceCoverage { surface } => {
                format!("no_surface_coverage:{surface}")
            }
            RejectionReason::ConstraintUnsatisfied { constraint } => {
                format!("constraint_unsatisfied:{constraint}")
            }
            RejectionReason::CapabilityMismatch {
                required,
                available,
            } => format!(
                "capability_mismatch:required={},available={}",
                required.join("+"),
                available.join("+")
            ),
        }
    }
}

// ── ShapeSelectionError ───────────────────────────────────────────────────

/// Errors emitted by `ShapeSelector`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ShapeSelectionError {
    /// Roster is empty.
    #[error("specialist roster is empty")]
    EmptyRoster,
    /// Contract was malformed.
    #[error("invalid contract: {reason}")]
    InvalidContract {
        /// Reason.
        reason: String,
    },
}

// ── ShapeDecision ─────────────────────────────────────────────────────────

/// Decision emitted by `ShapeSelector`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ShapeDecision {
    /// Contract id.
    pub contract_id: String,
    /// Chosen specialist (None if no match).
    pub chosen: Option<Specialist>,
    /// Rejection reasons for the candidates that were considered.
    pub rejected: Vec<RejectionReason>,
    /// Human-readable rationale.
    pub rationale: String,
    /// RFC-3339 timestamp supplied by the caller.
    pub decided_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl ShapeDecision {
    /// Schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── ShapeSelector ─────────────────────────────────────────────────────────

/// Pure SHAPE selector.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ShapeSelector {
    roster: SpecialistRoster,
}

impl ShapeSelector {
    /// Construct a SHAPE selector.
    pub fn new(roster: SpecialistRoster) -> Self {
        Self { roster }
    }

    /// Roster in use.
    pub fn roster(&self) -> &SpecialistRoster {
        &self.roster
    }

    /// Pure selection.
    pub fn select(&self, contract: &ChangeContract) -> Result<ShapeDecision, ShapeSelectionError> {
        if self.roster.specialists.is_empty() {
            return Err(ShapeSelectionError::EmptyRoster);
        }
        if contract.surface.is_empty() {
            return Err(ShapeSelectionError::InvalidContract {
                reason: "empty surface".to_string(),
            });
        }

        let mut rejected: Vec<RejectionReason> = Vec::new();
        let candidates = self.roster.by_surface(&contract.surface);

        if candidates.is_empty() {
            rejected.push(RejectionReason::NoSurfaceCoverage {
                surface: contract.surface.clone(),
            });
            return Ok(ShapeDecision {
                contract_id: contract.contract_id.clone(),
                chosen: None,
                rejected,
                rationale: "no surface coverage".to_string(),
                decided_at: contract.created_at.clone(),
                schema_version: ShapeDecision::SCHEMA_VERSION,
            });
        }

        let mut eligible: Vec<&Specialist> = Vec::new();
        for s in candidates {
            let mut ok = true;
            // Specialist admission criteria must be satisfied by
            // the contract's constraints.
            for crit in &s.admission_criteria {
                let cid = crit.id();
                if !contract.constraints.iter().any(|c| c.id() == cid) {
                    rejected.push(RejectionReason::ConstraintUnsatisfied { constraint: cid });
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }

            // Contract's MustNotTouch constraints must not block
            // the specialist's capabilities (the specialist's
            // capabilities include the contract surface; if the
            // contract forbids touching that exact surface, reject).
            for c in &contract.constraints {
                if let ChangeConstraint::MustNotTouch { surface } = c
                    && surface == &contract.surface
                {
                    rejected.push(RejectionReason::ConstraintUnsatisfied { constraint: c.id() });
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            eligible.push(s);
        }

        let chosen = if eligible.is_empty() {
            None
        } else {
            // Pick the most focused: fewest capabilities; ties by
            // specialist_id ascending.
            let mut sorted: Vec<&Specialist> = eligible.clone();
            sorted.sort_by(|a, b| {
                a.capabilities
                    .len()
                    .cmp(&b.capabilities.len())
                    .then_with(|| a.specialist_id.cmp(&b.specialist_id))
            });
            Some(sorted[0].clone())
        };

        // Deterministic ordering.
        rejected.sort_by_key(|a| a.id());
        rejected.dedup_by(|a, b| a.id() == b.id());

        let rationale = if chosen.is_some() {
            "selected by capability focus + tie-break".to_string()
        } else {
            "no eligible specialist".to_string()
        };

        Ok(ShapeDecision {
            contract_id: contract.contract_id.clone(),
            chosen,
            rejected,
            rationale,
            decided_at: contract.created_at.clone(),
            schema_version: ShapeDecision::SCHEMA_VERSION,
        })
    }
}

// ── Audit guards ──────────────────────────────────────────────────────────

#[allow(unused)]
const CHANGE_KIND_VARIANT_LIST: &[ChangeKind] = &[
    ChangeKind::Tweak,
    ChangeKind::Refactor,
    ChangeKind::Feature,
    ChangeKind::BugFix,
    ChangeKind::Optimisation,
];

#[allow(unused)]
const SPECIALIST_KIND_VARIANT_LIST: &[SpecialistKind] = &[
    SpecialistKind::Human,
    SpecialistKind::LlmAgent,
    SpecialistKind::RuleAdapter,
    SpecialistKind::LabAgent,
];

#[allow(unused)]
const REJECTION_REASON_VARIANT_LIST: &[RejectionReason] = &[
    RejectionReason::NoSurfaceCoverage {
        surface: String::new(),
    },
    RejectionReason::ConstraintUnsatisfied {
        constraint: String::new(),
    },
    RejectionReason::CapabilityMismatch {
        required: vec![],
        available: vec![],
    },
];

#[allow(unused)]
const SHAPE_SELECTION_ERROR_VARIANT_LIST: &[ShapeSelectionError] = &[
    ShapeSelectionError::EmptyRoster,
    ShapeSelectionError::InvalidContract {
        reason: String::new(),
    },
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, caps: Vec<&str>, admission: Vec<ChangeConstraint>) -> Specialist {
        Specialist {
            specialist_id: id.to_string(),
            kind: SpecialistKind::LlmAgent,
            capabilities: caps.into_iter().map(String::from).collect(),
            admission_criteria: admission,
            max_concurrent: 4,
        }
    }

    fn contract(surface: &str, constraints: Vec<ChangeConstraint>) -> ChangeContract {
        ChangeContract::new(
            "C1".to_string(),
            ChangeKind::Feature,
            surface.to_string(),
            constraints,
            "t".to_string(),
        )
    }

    // ── S-1: Single candidate passes ───────────────────────────────────

    #[test]
    fn s1_single_candidate_passes() {
        let s = spec("alice", vec!["sddk-engine/sc"], vec![]);
        let roster = SpecialistRoster::new(vec![s.clone()]);
        let selector = ShapeSelector::new(roster);
        let c = contract("sddk-engine/sc", vec![]);
        let d = selector.select(&c).unwrap();
        assert!(d.chosen.is_some());
        assert_eq!(d.chosen.unwrap().specialist_id, "alice");
        assert!(d.rejected.is_empty());
    }

    // ── S-2: No coverage → chosen = None ───────────────────────────────

    #[test]
    fn s2_no_coverage() {
        let s = spec("alice", vec!["other"], vec![]);
        let roster = SpecialistRoster::new(vec![s]);
        let selector = ShapeSelector::new(roster);
        let c = contract("sddk-engine/sc", vec![]);
        let d = selector.select(&c).unwrap();
        assert!(d.chosen.is_none());
        assert!(
            d.rejected
                .iter()
                .any(|r| matches!(r, RejectionReason::NoSurfaceCoverage { .. }))
        );
    }

    // ── S-3: MustNotTouch blocks specialist ─────────────────────────────

    #[test]
    fn s3_must_not_touch_blocks() {
        let s = spec("alice", vec!["sddk-engine/sc"], vec![]);
        let roster = SpecialistRoster::new(vec![s]);
        let selector = ShapeSelector::new(roster);
        let c = contract(
            "sddk-engine/sc",
            vec![ChangeConstraint::MustNotTouch {
                surface: "sddk-engine/sc".to_string(),
            }],
        );
        let d = selector.select(&c).unwrap();
        assert!(d.chosen.is_none());
        assert!(
            d.rejected
                .iter()
                .any(|r| matches!(r, RejectionReason::ConstraintUnsatisfied { .. }))
        );
    }

    // ── S-4: Tie-break by specialist_id ascending ──────────────────────

    #[test]
    fn s4_tie_break_by_id() {
        let a = spec("alice", vec!["sddk-engine/sc"], vec![]);
        let b = spec("bob", vec!["sddk-engine/sc"], vec![]);
        let roster = SpecialistRoster::new(vec![a, b]);
        let selector = ShapeSelector::new(roster);
        let c = contract("sddk-engine/sc", vec![]);
        let d = selector.select(&c).unwrap();
        assert_eq!(d.chosen.unwrap().specialist_id, "alice");
    }

    // ── S-5: Empty roster fails closed ─────────────────────────────────

    #[test]
    fn s5_empty_roster() {
        let roster = SpecialistRoster::new(vec![]);
        let selector = ShapeSelector::new(roster);
        let c = contract("sddk-engine/sc", vec![]);
        let err = selector.select(&c).unwrap_err();
        assert!(matches!(err, ShapeSelectionError::EmptyRoster));
    }

    // ── S-6: Constraint satisfaction ──────────────────────────────────

    #[test]
    fn s6_constraint_satisfies() {
        let s = spec(
            "alice",
            vec!["sddk-engine/sc"],
            vec![ChangeConstraint::MustBeDeterministic],
        );
        let roster = SpecialistRoster::new(vec![s]);
        let selector = ShapeSelector::new(roster);
        let c = contract(
            "sddk-engine/sc",
            vec![ChangeConstraint::MustBeDeterministic],
        );
        let d = selector.select(&c).unwrap();
        assert!(d.chosen.is_some());
    }

    // ── S-7: Determinism ───────────────────────────────────────────────

    #[test]
    fn s7_determinism_byte_equal() {
        let s = spec("alice", vec!["sddk-engine/sc"], vec![]);
        let roster = SpecialistRoster::new(vec![s]);
        let selector = ShapeSelector::new(roster);
        let c = contract("sddk-engine/sc", vec![]);
        let a = selector.select(&c).unwrap();
        let b = selector.select(&c).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── S-8: Rejected reasons sorted by id ─────────────────────────────

    #[test]
    fn s8_rejected_sorted_by_id() {
        let s1 = spec(
            "alice",
            vec!["sddk-engine/sc"],
            vec![ChangeConstraint::MustBeDeterministic],
        );
        let s2 = spec("bob", vec!["sddk-engine/sc"], vec![]);
        let roster = SpecialistRoster::new(vec![s1, s2]);
        let selector = ShapeSelector::new(roster);
        // Contract has no constraints; alice's admission criterion
        // is unsatisfied, bob is eligible.
        let c = contract("sddk-engine/sc", vec![]);
        let d = selector.select(&c).unwrap();
        // Only one rejection reason from alice.
        assert!(!d.rejected.is_empty());
        // Verify sorted.
        let mut sorted = d.rejected.clone();
        sorted.sort_by_key(|a| a.id());
        assert_eq!(d.rejected, sorted);
        assert_eq!(d.chosen.unwrap().specialist_id, "bob");
    }

    // ── Extra: ChangeKind / ChangeConstraint ids ───────────────────────

    #[test]
    fn ids() {
        assert_eq!(ChangeKind::Tweak.id(), "tweak".to_string());
        assert_eq!(ChangeKind::Feature.id(), "feature".to_string());
        assert_eq!(
            ChangeConstraint::MustMatchRateAtLeastBps { value_bps: 9500 }.id(),
            "must_match_rate_at_least_bps:9500".to_string()
        );
        assert_eq!(
            RejectionReason::NoSurfaceCoverage {
                surface: "s".to_string()
            }
            .id(),
            "no_surface_coverage:s".to_string()
        );
    }

    // ── Extra: invalid contract (empty surface) ────────────────────────

    #[test]
    fn invalid_contract_empty_surface() {
        let s = spec("alice", vec!["x"], vec![]);
        let roster = SpecialistRoster::new(vec![s]);
        let selector = ShapeSelector::new(roster);
        let c = ChangeContract::new(
            "C1".to_string(),
            ChangeKind::Tweak,
            "".to_string(),
            vec![],
            "t".to_string(),
        );
        let err = selector.select(&c).unwrap_err();
        assert!(matches!(err, ShapeSelectionError::InvalidContract { .. }));
    }
}
