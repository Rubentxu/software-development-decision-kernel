//! Evidence-Backed Promotion / Tuning / Rollback
//!
//! Pure authority that, given a [`StrategyExperimentSummary`] and the
//! previously-deployed [`StrategyKind`], emits a single plain-data
//! decision (Promote / Tune / Rollback / Hold) with auditable
//! clauses. Read-only with respect to canonical state.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::strategy_comparison::PromotionPolicy;
use crate::strategy_experiments::{
    StrategyExperimentSummary, StrategyExperimentVerdict, StrategyKind,
};

// ── Audit guard constant ────────────────────────────────────────────────

/// Closed-set size of [`PromotionAction`]. Bump when a variant is added.
pub const PROMOTION_ACTION_VARIANT_COUNT: usize = 4;

// ── Enums ────────────────────────────────────────────────────────────────

/// Closed-set taxonomy of promotion actions.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum PromotionAction {
    Promote {
        kind: StrategyKind,
        match_rate_bps: u32,
    },
    Tune {
        kind: StrategyKind,
        delta_bps: i32,
    },
    Rollback {
        previous: StrategyKind,
        reason: String,
    },
    Hold {
        reason: String,
    },
}

// ── Records ─────────────────────────────────────────────────────────────

/// Aggregate of the authority's decision.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct PromotionDecision {
    pub experiment_id: String,
    pub action: PromotionAction,
    /// Sorted.
    pub clauses_satisfied: Vec<String>,
    /// Sorted.
    pub clauses_violated: Vec<String>,
    pub issued_at: String,
}

// ── Trait + default authority ──────────────────────────────────────────

/// Decides promotion based on experiment evidence.
pub trait EvidenceBackedPromotionAuthority {
    fn decide(
        &self,
        summary: &StrategyExperimentSummary,
        previous: Option<&StrategyKind>,
        promotion_policy: PromotionPolicy,
        issuer: &str,
        issued_at: &str,
        previous_match_rate_bps: Option<u32>,
    ) -> PromotionDecision;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultEvidenceBackedPromotionAuthority;

impl EvidenceBackedPromotionAuthority for DefaultEvidenceBackedPromotionAuthority {
    fn decide(
        &self,
        summary: &StrategyExperimentSummary,
        previous: Option<&StrategyKind>,
        _promotion_policy: PromotionPolicy,
        _issuer: &str,
        issued_at: &str,
        previous_match_rate_bps: Option<u32>,
    ) -> PromotionDecision {
        // Evaluate clauses.
        let experiment_passed = matches!(summary.verdict, StrategyExperimentVerdict::AcceptAll);
        let experiment_mixed = matches!(summary.verdict, StrategyExperimentVerdict::Mixed { .. });
        let experiment_failed =
            matches!(summary.verdict, StrategyExperimentVerdict::RejectAll { .. });
        let respects_budget = !summary.over_budget;
        let matching_candidate = summary
            .results
            .iter()
            .all(|r| r.candidate.label() == summary.candidate.label());
        let min_match_rate_bps = summary
            .results
            .iter()
            .map(|r| r.match_rate_bps)
            .min()
            .unwrap_or(0);
        let rollback_requested = match (previous, previous_match_rate_bps) {
            (Some(_), Some(prev_rate)) => min_match_rate_bps < prev_rate,
            _ => false,
        };

        // Build satisfied/violated clause lists.
        let mut satisfied: Vec<String> = Vec::new();
        let mut violated: Vec<String> = Vec::new();
        if experiment_passed {
            satisfied.push("experiment_passed".to_string());
        } else {
            violated.push("experiment_passed".to_string());
        }
        if respects_budget {
            satisfied.push("respects_budget".to_string());
        } else {
            violated.push("respects_budget".to_string());
        }
        if matching_candidate {
            satisfied.push("matching_candidate".to_string());
        } else {
            violated.push("matching_candidate".to_string());
        }
        if experiment_mixed {
            satisfied.push("experiment_mixed".to_string());
        }
        if experiment_failed {
            violated.push("experiment_failed".to_string());
        }
        if rollback_requested {
            satisfied.push("rollback_requested".to_string());
        }

        // Action selection.
        let action = if !respects_budget {
            PromotionAction::Hold {
                reason: "over_budget".to_string(),
            }
        } else if experiment_failed {
            PromotionAction::Hold {
                reason: "experiment_failed".to_string(),
            }
        } else if rollback_requested {
            if let Some(prev) = previous {
                PromotionAction::Rollback {
                    previous: prev.clone(),
                    reason: "match_rate_drop".to_string(),
                }
            } else {
                PromotionAction::Hold {
                    reason: "rollback_without_previous".to_string(),
                }
            }
        } else if experiment_passed && matching_candidate {
            PromotionAction::Promote {
                kind: summary.candidate.clone(),
                match_rate_bps: min_match_rate_bps,
            }
        } else if experiment_mixed {
            PromotionAction::Tune {
                kind: summary.candidate.clone(),
                delta_bps: 0,
            }
        } else {
            PromotionAction::Hold {
                reason: "no_actionable_evidence".to_string(),
            }
        };

        satisfied.sort();
        violated.sort();

        PromotionDecision {
            experiment_id: summary.experiment_id.clone(),
            action,
            clauses_satisfied: satisfied,
            clauses_violated: violated,
            issued_at: issued_at.to_string(),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy_experiments::{
        StrategyExperimentCase, StrategyExperimentResult, StrategyExperimentSummary,
        StrategyExperimentVerdict,
    };

    fn auth() -> DefaultEvidenceBackedPromotionAuthority {
        DefaultEvidenceBackedPromotionAuthority
    }

    fn summary(
        verdict: StrategyExperimentVerdict,
        candidate: StrategyKind,
        over_budget: bool,
    ) -> StrategyExperimentSummary {
        StrategyExperimentSummary {
            experiment_id: "exp-1".to_string(),
            candidate: candidate.clone(),
            results: vec![StrategyExperimentResult {
                case_id: "c1".to_string(),
                candidate,
                matched_clauses: 10,
                total_clauses: 10,
                match_rate_bps: 10_000,
                accepted: true,
                recorded_at: "t0".to_string(),
            }],
            verdict,
            min_match_rate_bps: 10_000,
            generated_at: "t0".to_string(),
            over_budget,
        }
    }

    // ── S-1: Promote on AcceptAll + respects_budget + matching ────────

    #[test]
    fn s1_promote_on_accept_all() {
        let s = summary(
            StrategyExperimentVerdict::AcceptAll,
            StrategyKind::BestFirst,
            false,
        );
        let d = auth().decide(
            &s,
            None,
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-issued",
            None,
        );
        match d.action {
            PromotionAction::Promote {
                kind,
                match_rate_bps,
            } => {
                assert_eq!(kind, StrategyKind::BestFirst);
                assert_eq!(match_rate_bps, 10_000);
            }
            other => panic!("expected Promote, got {other:?}"),
        }
        assert!(
            d.clauses_satisfied
                .contains(&"experiment_passed".to_string())
        );
        assert!(
            d.clauses_satisfied
                .contains(&"matching_candidate".to_string())
        );
        assert!(d.clauses_satisfied.contains(&"respects_budget".to_string()));
    }

    // ── S-2: Rollback when worse than previous ────────────────────────

    #[test]
    fn s2_rollback_when_worse_than_previous() {
        let s = summary(
            StrategyExperimentVerdict::Mixed {
                accepted: 1,
                rejected: 1,
                failing: vec!["c2".to_string()],
            },
            StrategyKind::BestFirst,
            false,
        );
        // min match_rate = 10_000 (only 1 result). To force rollback we
        // need previous_match_rate > 10_000, but bps are 0..=10_000. Use
        // None for previous_match_rate to disable rollback.
        let d = auth().decide(
            &s,
            Some(&StrategyKind::Mcts {
                rollouts: 1,
                exploration_c: 1.41,
                max_depth: 1,
            }),
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-issued",
            Some(10_000), // equal, NOT strictly less; use 10_001 to force.
        );
        // Now try with 10_001:
        let d2 = auth().decide(
            &s,
            Some(&StrategyKind::Mcts {
                rollouts: 1,
                exploration_c: 1.41,
                max_depth: 1,
            }),
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-issued",
            Some(10_001),
        );
        // First decision: not rollback (equal).
        match d.action {
            PromotionAction::Tune { .. } | PromotionAction::Hold { .. } => {}
            other => panic!("expected Tune/Hold, got {other:?}"),
        }
        // Second decision: rollback.
        match d2.action {
            PromotionAction::Rollback { previous, .. } => {
                assert!(matches!(previous, StrategyKind::Mcts { .. }));
            }
            other => panic!("expected Rollback, got {other:?}"),
        }
    }

    // ── S-3: Tune on Mixed verdict ───────────────────────────────────

    #[test]
    fn s3_tune_on_mixed() {
        let s = summary(
            StrategyExperimentVerdict::Mixed {
                accepted: 1,
                rejected: 1,
                failing: vec!["c2".to_string()],
            },
            StrategyKind::BestFirst,
            false,
        );
        let d = auth().decide(
            &s,
            None,
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-issued",
            None,
        );
        match d.action {
            PromotionAction::Tune { kind, delta_bps } => {
                assert_eq!(kind, StrategyKind::BestFirst);
                assert_eq!(delta_bps, 0);
            }
            other => panic!("expected Tune, got {other:?}"),
        }
    }

    // ── S-4: Hold on RejectAll ───────────────────────────────────────

    #[test]
    fn s4_hold_on_reject_all() {
        let s = summary(
            StrategyExperimentVerdict::RejectAll {
                failing_cases: vec!["c1".to_string()],
            },
            StrategyKind::BestFirst,
            false,
        );
        let d = auth().decide(
            &s,
            None,
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-issued",
            None,
        );
        match d.action {
            PromotionAction::Hold { reason } => {
                assert_eq!(reason, "experiment_failed");
            }
            other => panic!("expected Hold, got {other:?}"),
        }
    }

    // ── S-5: Hold on over-budget ─────────────────────────────────────

    #[test]
    fn s5_hold_on_over_budget() {
        let s = summary(
            StrategyExperimentVerdict::AcceptAll,
            StrategyKind::BestFirst,
            true, // over budget
        );
        let d = auth().decide(
            &s,
            None,
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-issued",
            None,
        );
        match d.action {
            PromotionAction::Hold { reason } => {
                assert_eq!(reason, "over_budget");
            }
            other => panic!("expected Hold, got {other:?}"),
        }
    }

    // ── S-6: Closed-set audit ────────────────────────────────────────

    #[test]
    fn s6_closed_set_audit() {
        assert_eq!(PROMOTION_ACTION_VARIANT_COUNT, 4);
        // Each variant constructed and labelled.
        let actions = [
            PromotionAction::Promote {
                kind: StrategyKind::BestFirst,
                match_rate_bps: 10_000,
            },
            PromotionAction::Tune {
                kind: StrategyKind::BestFirst,
                delta_bps: 0,
            },
            PromotionAction::Rollback {
                previous: StrategyKind::BestFirst,
                reason: "x".to_string(),
            },
            PromotionAction::Hold {
                reason: "x".to_string(),
            },
        ];
        assert_eq!(actions.len(), PROMOTION_ACTION_VARIANT_COUNT);
    }

    // ── Bonus: determinism ───────────────────────────────────────────

    #[test]
    fn s7_deterministic_output() {
        let s = summary(
            StrategyExperimentVerdict::AcceptAll,
            StrategyKind::BestFirst,
            false,
        );
        let d1 = auth().decide(
            &s,
            None,
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-A",
            None,
        );
        let d2 = auth().decide(
            &s,
            None,
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-B",
            None,
        );
        assert_eq!(d1.action, d2.action);
        assert_eq!(d1.clauses_satisfied, d2.clauses_satisfied);
        assert_ne!(d1.issued_at, d2.issued_at);
    }

    // ── Bonus: clause sort order ─────────────────────────────────────

    #[test]
    fn s8_clauses_sorted() {
        let s = summary(
            StrategyExperimentVerdict::AcceptAll,
            StrategyKind::BestFirst,
            false,
        );
        let d = auth().decide(
            &s,
            None,
            PromotionPolicy::NotRegressing,
            "issuer",
            "t-issued",
            None,
        );
        let mut sorted = d.clauses_satisfied.clone();
        sorted.sort();
        assert_eq!(d.clauses_satisfied, sorted);
        let mut sorted = d.clauses_violated.clone();
        sorted.sort();
        assert_eq!(d.clauses_violated, sorted);
    }

    // ── Silence unused-import warning for StrategyExperimentCase.
    #[allow(dead_code)]
    fn _case_used(_c: StrategyExperimentCase) {}
}
