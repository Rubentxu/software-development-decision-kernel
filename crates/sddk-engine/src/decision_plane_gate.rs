//! Decision Plane Gates Backed by Assurance + UAT Evidence.
//!
//! Cycle: EA-UAT-001 (H7, order 430, context pack
//! `engineering-assurance`).
//!
//! Pure, deterministic evaluator that consumes
//! `AssuranceEvidence` (engineering) plus `UatSignoff` records
//! (human) and emits a typed `DecisionPlaneGateVerdict` that
//! the Decision Plane runtime can read to decide whether to
//! advance a workflow.
//!
//! ## Design
//!
//! - **Closed-set `DecisionPlaneGate`.** Three values: `Advance`,
//!   `Hold { reason }`, `Block { reason }`.
//! - **Pure evaluator.** `evaluate` is a pure function of
//!   `(input, profile, evaluated_at)`.
//! - **Priority ordering.** `Block` > `Hold` > `Advance`.
//! - **Reuses `AssuranceEngine`.** The assurance verdict is
//!   computed internally; the gate aggregates it.
//! - **Read-only.** The gater never mutates evidence.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-DecisionPlaneAssuranceGating` (spec, accepted)
//! - `ADR-104` (architecture decision, accepted)
//! - `REQ-EngineeringAssuranceProfiles`,
//!   `REQ-EngineeringAssuranceResolvers`, `REQ-UatLifecycle`
//!   (dependencies).

use serde::{Deserialize, Serialize};

use crate::engineering_assurance::{
    AssuranceEngine, AssuranceEvidence, AssuranceProfile, AssuranceStatus,
};
use crate::uat_lifecycle::{UatDefect, UatSignoff};

// ── DecisionPlaneGate ─────────────────────────────────────────────────────

/// Closed-set gate answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecisionPlaneGate {
    /// Workflow can advance.
    Advance,
    /// Workflow must hold until evidence improves.
    Hold {
        /// Reason.
        reason: String,
    },
    /// Workflow must block; critical evidence missing.
    Block {
        /// Reason.
        reason: String,
    },
}

impl DecisionPlaneGate {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            DecisionPlaneGate::Advance => "advance".to_string(),
            DecisionPlaneGate::Hold { reason } => format!("hold(reason={reason})"),
            DecisionPlaneGate::Block { reason } => {
                format!("block(reason={reason})")
            }
        }
    }
}

// ── DecisionPlaneGateInput ───────────────────────────────────────────────

/// Input to the gate evaluator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DecisionPlaneGateInput {
    /// Engineering evidence.
    pub engineering: Vec<AssuranceEvidence>,
    /// UAT signoff records.
    pub uat_signoffs: Vec<UatSignoff>,
    /// Open defects currently affecting the workflow.
    pub open_defects: Vec<UatDefect>,
    /// Pending human checks (None observed) for the workflow.
    pub pending_human_checks: usize,
    /// Minimum human signoff count required.
    pub required_signoffs: usize,
}

impl DecisionPlaneGateInput {
    /// Empty input.
    pub fn empty() -> Self {
        Self {
            engineering: Vec::new(),
            uat_signoffs: Vec::new(),
            open_defects: Vec::new(),
            pending_human_checks: 0,
            required_signoffs: 0,
        }
    }
}

// ── DecisionPlaneGateVerdict ──────────────────────────────────────────────

/// Verdict emitted by the gater.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DecisionPlaneGateVerdict {
    /// Final gate answer.
    pub gate: DecisionPlaneGate,
    /// Assurance engine status (mirror).
    pub assurance_verdict_status: AssuranceStatus,
    /// Number of signoffs satisfied (across all signoff records).
    pub satisfied_required_signoffs: usize,
    /// Total required signoffs.
    pub total_required_signoffs: usize,
    /// RFC-3339 timestamp supplied by the caller.
    pub evaluated_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl DecisionPlaneGateVerdict {
    /// Constant schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── DecisionPlaneGater ───────────────────────────────────────────────────

/// Pure gater.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DecisionPlaneGater {
    profile: AssuranceProfile,
}

impl DecisionPlaneGater {
    /// Construct a gater bound to an assurance profile.
    pub fn new(profile: AssuranceProfile) -> Self {
        Self { profile }
    }

    /// Profile in use.
    pub fn profile(&self) -> &AssuranceProfile {
        &self.profile
    }

    /// Evaluate the input against the assurance profile and emit
    /// a typed gate verdict.
    pub fn evaluate(
        &self,
        input: &DecisionPlaneGateInput,
        evaluated_at: String,
    ) -> DecisionPlaneGateVerdict {
        let assurance_engine = AssuranceEngine::new(self.profile.clone());
        let av = assurance_engine.evaluate(&input.engineering, evaluated_at.clone());
        let assurance_status = av.status.clone();

        // Priority: Block > Hold > Advance.
        let gate = if matches!(assurance_status, AssuranceStatus::Fail { .. }) {
            DecisionPlaneGate::Block {
                reason: "engineering_evidence_fail".to_string(),
            }
        } else if !input.open_defects.is_empty() {
            DecisionPlaneGate::Hold {
                reason: "open_uat_defects".to_string(),
            }
        } else if input.pending_human_checks > 0 {
            DecisionPlaneGate::Hold {
                reason: "pending_human_checks".to_string(),
            }
        } else if input.uat_signoffs.len() < input.required_signoffs {
            DecisionPlaneGate::Hold {
                reason: "missing_signoffs".to_string(),
            }
        } else {
            DecisionPlaneGate::Advance
        };

        DecisionPlaneGateVerdict {
            gate,
            assurance_verdict_status: assurance_status,
            satisfied_required_signoffs: input.uat_signoffs.len(),
            total_required_signoffs: input.required_signoffs,
            evaluated_at,
            schema_version: DecisionPlaneGateVerdict::SCHEMA_VERSION,
        }
    }
}

// ── Audit guard ──────────────────────────────────────────────────────────

#[allow(unused)]
const DECISION_PLANE_GATE_VARIANT_LIST: &[DecisionPlaneGate] = &[
    DecisionPlaneGate::Advance,
    DecisionPlaneGate::Hold {
        reason: String::new(),
    },
    DecisionPlaneGate::Block {
        reason: String::new(),
    },
];

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engineering_assurance::EvidenceTag;
    use crate::uat_lifecycle::{UatDefectSeverity, UatDefectStatus};

    fn pass_engineering() -> Vec<AssuranceEvidence> {
        vec![
            AssuranceEvidence {
                tag: EvidenceTag::EngineTest {
                    count: 100,
                    passed: 99,
                },
                collected_at: "t".to_string(),
                source: "cargo".to_string(),
            },
            AssuranceEvidence {
                tag: EvidenceTag::ClippyClean,
                collected_at: "t".to_string(),
                source: "clippy".to_string(),
            },
            AssuranceEvidence {
                tag: EvidenceTag::Receipt {
                    sha256: "abc".to_string(),
                },
                collected_at: "t".to_string(),
                source: "release".to_string(),
            },
        ]
    }

    fn signoff(actor: &str) -> UatSignoff {
        UatSignoff {
            actor: actor.to_string(),
            at: "t".to_string(),
            summary: "ok".to_string(),
            evidence_refs: vec!["ref".to_string()],
        }
    }

    fn defect() -> UatDefect {
        UatDefect {
            defect_id: "d1".to_string(),
            opened_at: "t".to_string(),
            severity: UatDefectSeverity::Major,
            summary: "x".to_string(),
            status: UatDefectStatus::Open,
        }
    }

    // ── S-1: Advance when everything passes ─────────────────────────────

    #[test]
    fn s1_advance_when_everything_passes() {
        let gater = DecisionPlaneGater::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let mut input = DecisionPlaneGateInput::empty();
        input.engineering = pass_engineering();
        input.uat_signoffs = vec![signoff("alice")];
        input.required_signoffs = 1;
        let verdict = gater.evaluate(&input, "t".to_string());
        assert_eq!(verdict.gate, DecisionPlaneGate::Advance);
        assert_eq!(verdict.satisfied_required_signoffs, 1);
        assert_eq!(verdict.total_required_signoffs, 1);
    }

    // ── S-2: Block on engineering Fail ─────────────────────────────────

    #[test]
    fn s2_block_on_engineering_fail() {
        let gater = DecisionPlaneGater::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let mut input = DecisionPlaneGateInput::empty();
        // Only EngineTest, no clippy / receipt — promotion profile
        // requires them.
        input.engineering = vec![AssuranceEvidence {
            tag: EvidenceTag::EngineTest {
                count: 100,
                passed: 99,
            },
            collected_at: "t".to_string(),
            source: "cargo".to_string(),
        }];
        input.uat_signoffs = vec![signoff("alice")];
        input.required_signoffs = 1;
        let verdict = gater.evaluate(&input, "t".to_string());
        assert!(matches!(verdict.gate, DecisionPlaneGate::Block { .. }));
    }

    // ── S-3: Hold on open defects ──────────────────────────────────────

    #[test]
    fn s3_hold_on_open_defects() {
        let gater = DecisionPlaneGater::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let mut input = DecisionPlaneGateInput::empty();
        input.engineering = pass_engineering();
        input.uat_signoffs = vec![signoff("alice")];
        input.required_signoffs = 1;
        input.open_defects = vec![defect()];
        let verdict = gater.evaluate(&input, "t".to_string());
        assert!(matches!(verdict.gate, DecisionPlaneGate::Hold { .. }));
        if let DecisionPlaneGate::Hold { reason } = &verdict.gate {
            assert_eq!(reason, "open_uat_defects");
        }
    }

    // ── S-4: Hold on pending human checks ──────────────────────────────

    #[test]
    fn s4_hold_on_pending_human_checks() {
        let gater = DecisionPlaneGater::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let mut input = DecisionPlaneGateInput::empty();
        input.engineering = pass_engineering();
        input.uat_signoffs = vec![signoff("alice")];
        input.required_signoffs = 1;
        input.pending_human_checks = 1;
        let verdict = gater.evaluate(&input, "t".to_string());
        if let DecisionPlaneGate::Hold { reason } = &verdict.gate {
            assert_eq!(reason, "pending_human_checks");
        } else {
            panic!("expected Hold");
        }
    }

    // ── S-5: Hold on missing signoffs ──────────────────────────────────

    #[test]
    fn s5_hold_on_missing_signoffs() {
        let gater = DecisionPlaneGater::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let mut input = DecisionPlaneGateInput::empty();
        input.engineering = pass_engineering();
        input.uat_signoffs = vec![signoff("alice")];
        input.required_signoffs = 2;
        let verdict = gater.evaluate(&input, "t".to_string());
        if let DecisionPlaneGate::Hold { reason } = &verdict.gate {
            assert_eq!(reason, "missing_signoffs");
        } else {
            panic!("expected Hold");
        }
    }

    // ── S-6: Determinism ───────────────────────────────────────────────

    #[test]
    fn s6_determinism_byte_equal() {
        let gater = DecisionPlaneGater::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let mut input = DecisionPlaneGateInput::empty();
        input.engineering = pass_engineering();
        input.uat_signoffs = vec![signoff("alice")];
        input.required_signoffs = 1;
        let a = gater.evaluate(&input, "t".to_string());
        let b = gater.evaluate(&input, "t".to_string());
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── S-7: Block wins over Hold ──────────────────────────────────────

    #[test]
    fn s7_block_wins_over_hold() {
        let gater = DecisionPlaneGater::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let mut input = DecisionPlaneGateInput::empty();
        input.engineering = vec![AssuranceEvidence {
            tag: EvidenceTag::EngineTest {
                count: 100,
                passed: 99,
            },
            collected_at: "t".to_string(),
            source: "cargo".to_string(),
        }];
        input.uat_signoffs = vec![signoff("alice")];
        input.required_signoffs = 1;
        input.open_defects = vec![defect()];
        input.pending_human_checks = 1;
        let verdict = gater.evaluate(&input, "t".to_string());
        assert!(matches!(verdict.gate, DecisionPlaneGate::Block { .. }));
    }

    // ── Extra: gate ids ────────────────────────────────────────────────

    #[test]
    fn gate_ids() {
        assert_eq!(DecisionPlaneGate::Advance.id(), "advance".to_string());
        assert_eq!(
            DecisionPlaneGate::Hold {
                reason: "r".to_string()
            }
            .id(),
            "hold(reason=r)".to_string()
        );
        assert_eq!(
            DecisionPlaneGate::Block {
                reason: "r".to_string()
            }
            .id(),
            "block(reason=r)".to_string()
        );
    }
}
