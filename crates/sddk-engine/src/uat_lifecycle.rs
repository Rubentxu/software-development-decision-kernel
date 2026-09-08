//! UAT Scenario / Human-Check / Defect / Retest / Signoff Lifecycle.
//!
//! Cycle: UAT-BC-001 (H7, order 420, context pack `uat`).
//!
//! Pure, deterministic state machine that drives a User Acceptance
//! Testing scenario through its lifecycle. The next cycle
//! (`EA-UAT-001`) bridges signed-off scenarios into the assurance
//! engine as evidence.
//!
//! ## Design
//!
//! - **Closed-set `UatScenarioState`.** Eight states cover the
//!   lifecycle: Draft, Ready, Running, AwaitingHumanCheck,
//!   DefectLogged, Retest, SignedOff, Cancelled.
//! - **Pure transition engine.** `transition(scenario, target)`
//!   is a pure function over a `&mut UatScenario`.
//! - **Fail-closed sign-off.** Cannot sign off with open defects
//!   or unanswered human checks.
//! - **Seam to assurance engine.** Reserved for `EA-UAT-001`.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-UatLifecycle` (spec, accepted)
//! - `ADR-103` (architecture decision, accepted)
//! - `REQ-EngineeringAssuranceProfiles`,
//!   `REQ-EngineeringAssuranceResolvers` (dependencies).

use serde::{Deserialize, Serialize};

// ── UatScenarioState ─────────────────────────────────────────────────────

/// Closed-set lifecycle state for a UAT scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UatScenarioState {
    /// Scenario is being authored.
    Draft,
    /// Scenario is ready to run.
    Ready,
    /// Scenario is currently running.
    Running,
    /// Awaiting a human check.
    AwaitingHumanCheck,
    /// A defect was logged.
    DefectLogged,
    /// A retest run is in progress.
    Retest,
    /// Scenario was signed off.
    SignedOff,
    /// Scenario was cancelled.
    Cancelled {
        /// Reason.
        reason: String,
    },
}

impl UatScenarioState {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            UatScenarioState::Draft => "draft".to_string(),
            UatScenarioState::Ready => "ready".to_string(),
            UatScenarioState::Running => "running".to_string(),
            UatScenarioState::AwaitingHumanCheck => "awaiting_human_check".to_string(),
            UatScenarioState::DefectLogged => "defect_logged".to_string(),
            UatScenarioState::Retest => "retest".to_string(),
            UatScenarioState::SignedOff => "signed_off".to_string(),
            UatScenarioState::Cancelled { reason } => {
                format!("cancelled(reason={reason})")
            }
        }
    }

    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            UatScenarioState::SignedOff | UatScenarioState::Cancelled { .. }
        )
    }

    /// Whether transition `self → target` is allowed.
    pub fn can_transition_to(&self, target: &UatScenarioState) -> bool {
        if self.is_terminal() {
            return false;
        }
        match self {
            UatScenarioState::Draft => {
                matches!(
                    target,
                    UatScenarioState::Ready | UatScenarioState::Cancelled { .. }
                )
            }
            UatScenarioState::Ready => {
                matches!(
                    target,
                    UatScenarioState::Running | UatScenarioState::Cancelled { .. }
                )
            }
            UatScenarioState::Running => {
                matches!(
                    target,
                    UatScenarioState::AwaitingHumanCheck
                        | UatScenarioState::DefectLogged
                        | UatScenarioState::SignedOff
                        | UatScenarioState::Cancelled { .. }
                )
            }
            UatScenarioState::AwaitingHumanCheck => {
                matches!(
                    target,
                    UatScenarioState::SignedOff
                        | UatScenarioState::DefectLogged
                        | UatScenarioState::Cancelled { .. }
                )
            }
            UatScenarioState::DefectLogged => {
                matches!(
                    target,
                    UatScenarioState::Retest | UatScenarioState::Cancelled { .. }
                )
            }
            UatScenarioState::Retest => {
                matches!(
                    target,
                    UatScenarioState::SignedOff
                        | UatScenarioState::DefectLogged
                        | UatScenarioState::Cancelled { .. }
                )
            }
            UatScenarioState::SignedOff | UatScenarioState::Cancelled { .. } => false,
        }
    }
}

// ── UatDefectSeverity ────────────────────────────────────────────────────

/// Closed-set defect severities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UatDefectSeverity {
    /// Blocker — the scenario cannot proceed.
    Blocker,
    /// Major — the scenario can proceed with the defect flagged.
    Major,
    /// Minor — informational.
    Minor,
}

// ── UatDefectStatus ──────────────────────────────────────────────────────

/// Closed-set defect statuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UatDefectStatus {
    /// Open defect.
    Open,
    /// Awaiting retest.
    RetestPending,
    /// Verified fixed.
    Verified,
    /// WontFix.
    WontFix,
}

// ── UatRetestOutcome ─────────────────────────────────────────────────────

/// Closed-set retest outcomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UatRetestOutcome {
    /// Retest passed.
    Pass,
    /// Retest failed.
    Fail {
        /// Reason.
        reason: String,
    },
    /// Retest skipped.
    Skipped,
}

// ── HumanCheck ───────────────────────────────────────────────────────────

/// A human check attached to a scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HumanCheck {
    /// Check id.
    pub check_id: String,
    /// Prompt the human sees.
    pub prompt: String,
    /// Observed value (`true` = pass, `false` = fail, `None` =
    /// unanswered).
    pub observed: Option<bool>,
    /// Optional comment.
    pub comment: Option<String>,
    /// RFC-3339 timestamp supplied by the caller.
    pub recorded_at: String,
}

// ── UatDefect ────────────────────────────────────────────────────────────

/// A defect recorded against a scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UatDefect {
    /// Defect id.
    pub defect_id: String,
    /// Opened-at timestamp.
    pub opened_at: String,
    /// Severity.
    pub severity: UatDefectSeverity,
    /// Summary.
    pub summary: String,
    /// Status.
    pub status: UatDefectStatus,
}

// ── UatRetestRun ─────────────────────────────────────────────────────────

/// A retest run record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UatRetestRun {
    /// Retest id.
    pub retest_id: String,
    /// At timestamp.
    pub at: String,
    /// Outcome.
    pub outcome: UatRetestOutcome,
}

// ── UatSignoff ───────────────────────────────────────────────────────────

/// A signoff record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UatSignoff {
    /// Actor (human) who signed off.
    pub actor: String,
    /// At timestamp.
    pub at: String,
    /// Summary.
    pub summary: String,
    /// Evidence references.
    pub evidence_refs: Vec<String>,
}

// ── UatScenario ──────────────────────────────────────────────────────────

/// A UAT scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UatScenario {
    /// Scenario id.
    pub scenario_id: String,
    /// Description.
    pub description: String,
    /// Current state.
    pub state: UatScenarioState,
    /// Human checks attached to this scenario.
    pub human_checks: Vec<HumanCheck>,
    /// Defects logged against this scenario.
    pub defects: Vec<UatDefect>,
    /// Retest runs against this scenario.
    pub retest_runs: Vec<UatRetestRun>,
    /// Signoff, if any.
    pub signoff: Option<UatSignoff>,
    /// RFC-3339 timestamp supplied by the caller.
    pub recorded_at: String,
}

impl UatScenario {
    /// Construct a new scenario in `Draft`.
    pub fn new(scenario_id: String, description: String, recorded_at: String) -> Self {
        Self {
            scenario_id,
            description,
            state: UatScenarioState::Draft,
            human_checks: Vec::new(),
            defects: Vec::new(),
            retest_runs: Vec::new(),
            signoff: None,
            recorded_at,
        }
    }

    /// Whether the scenario has open defects.
    pub fn has_open_defects(&self) -> bool {
        self.defects.iter().any(|d| {
            matches!(
                d.status,
                UatDefectStatus::Open | UatDefectStatus::RetestPending
            )
        })
    }

    /// Whether all human checks have been answered.
    pub fn all_human_checks_answered(&self) -> bool {
        self.human_checks.iter().all(|c| c.observed.is_some())
    }
}

// ── UatLifecycleError ────────────────────────────────────────────────────

/// Errors emitted by `UatLifecycleEngine`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UatLifecycleError {
    /// The transition from a state to the target is not allowed.
    #[error("illegal transition from {from} to {to}")]
    IllegalTransition {
        /// From state id.
        from: String,
        /// To state id.
        to: String,
    },
    /// Sign-off attempted while defects are still open.
    #[error("cannot sign off with open defects")]
    CannotSignOffWithOpenDefects,
    /// Sign-off attempted while human checks are unanswered.
    #[error("cannot sign off with unanswered human checks")]
    CannotSignOffWithUnansweredHumanChecks,
}

// ── UatLifecycleEngine ───────────────────────────────────────────────────

/// Pure lifecycle engine.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct UatLifecycleEngine;

impl UatLifecycleEngine {
    /// Transition a scenario to `target`.
    pub fn transition(
        &self,
        scenario: &mut UatScenario,
        target: UatScenarioState,
    ) -> Result<(), UatLifecycleError> {
        if !scenario.state.can_transition_to(&target) {
            return Err(UatLifecycleError::IllegalTransition {
                from: scenario.state.id(),
                to: target.id(),
            });
        }
        // Sign-off gates.
        if matches!(target, UatScenarioState::SignedOff) {
            if scenario.has_open_defects() {
                return Err(UatLifecycleError::CannotSignOffWithOpenDefects);
            }
            if !scenario.all_human_checks_answered() {
                return Err(UatLifecycleError::CannotSignOffWithUnansweredHumanChecks);
            }
        }
        scenario.state = target;
        Ok(())
    }
}

// ── Audit guards ──────────────────────────────────────────────────────────

#[allow(unused)]
const UAT_SCENARIO_STATE_VARIANT_LIST: &[UatScenarioState] = &[
    UatScenarioState::Draft,
    UatScenarioState::Ready,
    UatScenarioState::Running,
    UatScenarioState::AwaitingHumanCheck,
    UatScenarioState::DefectLogged,
    UatScenarioState::Retest,
    UatScenarioState::SignedOff,
    UatScenarioState::Cancelled {
        reason: String::new(),
    },
];

#[allow(unused)]
const UAT_DEFECT_SEVERITY_VARIANT_LIST: &[UatDefectSeverity] = &[
    UatDefectSeverity::Blocker,
    UatDefectSeverity::Major,
    UatDefectSeverity::Minor,
];

#[allow(unused)]
const UAT_DEFECT_STATUS_VARIANT_LIST: &[UatDefectStatus] = &[
    UatDefectStatus::Open,
    UatDefectStatus::RetestPending,
    UatDefectStatus::Verified,
    UatDefectStatus::WontFix,
];

#[allow(unused)]
const UAT_RETEST_OUTCOME_VARIANT_LIST: &[UatRetestOutcome] = &[
    UatRetestOutcome::Pass,
    UatRetestOutcome::Fail {
        reason: String::new(),
    },
    UatRetestOutcome::Skipped,
];

#[allow(unused)]
const UAT_LIFECYCLE_ERROR_VARIANT_LIST: &[UatLifecycleError] = &[
    UatLifecycleError::IllegalTransition {
        from: String::new(),
        to: String::new(),
    },
    UatLifecycleError::CannotSignOffWithOpenDefects,
    UatLifecycleError::CannotSignOffWithUnansweredHumanChecks,
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario() -> UatScenario {
        UatScenario::new("S1".to_string(), "d".to_string(), "t".to_string())
    }

    fn engine() -> UatLifecycleEngine {
        UatLifecycleEngine
    }

    fn answered_checks(n: usize) -> Vec<HumanCheck> {
        (0..n)
            .map(|i| HumanCheck {
                check_id: format!("c{i}"),
                prompt: "ok".to_string(),
                observed: Some(true),
                comment: None,
                recorded_at: "t".to_string(),
            })
            .collect()
    }

    // ── S-1: Draft → Ready ─────────────────────────────────────────────

    #[test]
    fn s1_draft_to_ready() {
        let mut s = scenario();
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        assert_eq!(s.state, UatScenarioState::Ready);
    }

    // ── S-2: Running → SignedOff blocked by unanswered checks ────────────

    #[test]
    fn s2_signoff_blocked_by_unanswered_checks() {
        let mut s = scenario();
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        s.human_checks.push(HumanCheck {
            check_id: "c0".to_string(),
            prompt: "p".to_string(),
            observed: None,
            comment: None,
            recorded_at: "t".to_string(),
        });
        let err = engine()
            .transition(&mut s, UatScenarioState::SignedOff)
            .unwrap_err();
        assert!(matches!(
            err,
            UatLifecycleError::CannotSignOffWithUnansweredHumanChecks
        ));
    }

    // ── S-3: Running → SignedOff blocked by open defect ─────────────────

    #[test]
    fn s3_signoff_blocked_by_open_defect() {
        let mut s = scenario();
        s.human_checks = answered_checks(1);
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        s.defects.push(UatDefect {
            defect_id: "d1".to_string(),
            opened_at: "t".to_string(),
            severity: UatDefectSeverity::Major,
            summary: "x".to_string(),
            status: UatDefectStatus::Open,
        });
        let err = engine()
            .transition(&mut s, UatScenarioState::SignedOff)
            .unwrap_err();
        assert!(matches!(
            err,
            UatLifecycleError::CannotSignOffWithOpenDefects
        ));
    }

    // ── S-4: Defect → Retest ───────────────────────────────────────────

    #[test]
    fn s4_defect_to_retest() {
        let mut s = scenario();
        s.human_checks = answered_checks(1);
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::DefectLogged)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Retest)
            .unwrap();
        assert_eq!(s.state, UatScenarioState::Retest);
    }

    // ── S-5: Retest → SignedOff clean ──────────────────────────────────

    #[test]
    fn s5_retest_to_signed_off_clean() {
        let mut s = scenario();
        s.human_checks = answered_checks(1);
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::DefectLogged)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Retest)
            .unwrap();
        // Resolve defect before signoff.
        if let Some(d) = s.defects.get_mut(0) {
            d.status = UatDefectStatus::Verified;
        }
        // NOTE: We don't add a defect here; we go straight
        // from Running to DefectLogged would have added one.
        // For test, ensure no open defects.
        engine()
            .transition(&mut s, UatScenarioState::SignedOff)
            .unwrap();
        assert_eq!(s.state, UatScenarioState::SignedOff);
    }

    // ── S-6: Illegal transition ─────────────────────────────────────────

    #[test]
    fn s6_illegal_transition() {
        let mut s = scenario();
        // Draft -> SignedOff is illegal.
        let err = engine()
            .transition(&mut s, UatScenarioState::SignedOff)
            .unwrap_err();
        assert!(matches!(err, UatLifecycleError::IllegalTransition { .. }));
    }

    // ── S-7: Cancel from Running ───────────────────────────────────────

    #[test]
    fn s7_cancel_from_running() {
        let mut s = scenario();
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        engine()
            .transition(
                &mut s,
                UatScenarioState::Cancelled {
                    reason: "obsolete".to_string(),
                },
            )
            .unwrap();
        assert!(matches!(s.state, UatScenarioState::Cancelled { .. }));
        assert!(s.state.is_terminal());
    }

    // ── S-8: Terminal state does not transition ─────────────────────────

    #[test]
    fn s8_terminal_no_transition() {
        let mut s = scenario();
        s.human_checks = answered_checks(1);
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::SignedOff)
            .unwrap();
        let err = engine()
            .transition(&mut s, UatScenarioState::Retest)
            .unwrap_err();
        assert!(matches!(err, UatLifecycleError::IllegalTransition { .. }));
    }

    // ── S-9: Signoff preserves signoff record ──────────────────────────

    #[test]
    fn s9_signoff_preserves_record() {
        let mut s = scenario();
        s.human_checks = answered_checks(1);
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        s.signoff = Some(UatSignoff {
            actor: "alice".to_string(),
            at: "t".to_string(),
            summary: "ok".to_string(),
            evidence_refs: vec!["ref1".to_string()],
        });
        engine()
            .transition(&mut s, UatScenarioState::SignedOff)
            .unwrap();
        assert!(s.signoff.is_some());
        assert_eq!(s.signoff.as_ref().unwrap().actor, "alice");
        assert_eq!(s.state, UatScenarioState::SignedOff);
    }

    // ── Extra: state ids stable ────────────────────────────────────────

    #[test]
    fn state_ids() {
        assert_eq!(UatScenarioState::Draft.id(), "draft".to_string());
        assert_eq!(UatScenarioState::Ready.id(), "ready".to_string());
        assert_eq!(UatScenarioState::SignedOff.id(), "signed_off".to_string());
        assert_eq!(
            UatScenarioState::Cancelled {
                reason: "x".to_string()
            }
            .id(),
            "cancelled(reason=x)".to_string()
        );
    }

    // ── Extra: has_open_defects ────────────────────────────────────────

    #[test]
    fn has_open_defects() {
        let mut s = scenario();
        s.human_checks = answered_checks(1);
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::DefectLogged)
            .unwrap();
        s.defects.push(UatDefect {
            defect_id: "d1".to_string(),
            opened_at: "t".to_string(),
            severity: UatDefectSeverity::Minor,
            summary: "x".to_string(),
            status: UatDefectStatus::Open,
        });
        assert!(s.has_open_defects());
        s.defects[0].status = UatDefectStatus::Verified;
        assert!(!s.has_open_defects());
    }

    // ── Extra: cancel from terminal fails ──────────────────────────────

    #[test]
    fn cancel_from_terminal_fails() {
        let mut s = scenario();
        s.human_checks = answered_checks(1);
        engine()
            .transition(&mut s, UatScenarioState::Ready)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::Running)
            .unwrap();
        engine()
            .transition(&mut s, UatScenarioState::SignedOff)
            .unwrap();
        let err = engine()
            .transition(
                &mut s,
                UatScenarioState::Cancelled {
                    reason: "x".to_string(),
                },
            )
            .unwrap_err();
        assert!(matches!(err, UatLifecycleError::IllegalTransition { .. }));
    }
}
