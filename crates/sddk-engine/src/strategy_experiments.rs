//! Bounded Strategy Experiments
//!
//! Deterministic, audit-friendly runner that tries candidate strategies
//! against a bounded batch of cases and emits a plain-data summary.
//! Read-only with respect to canonical state.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`StrategyKind`]. Bump when a variant is added.
pub const STRATEGY_KIND_VARIANT_COUNT: usize = 4;

/// Maximum number of cases accepted per experiment batch. Larger batches
/// are rejected with a synthetic `__over_budget__` failure.
pub const STRATEGY_EXPERIMENT_BUDGET: usize = 256;

// ── Strategy kinds ──────────────────────────────────────────────────────

/// Closed-set taxonomy of strategy candidates.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum StrategyKind {
    BestFirst,
    Mcts {
        rollouts: u32,
        exploration_c: f64,
        max_depth: u32,
    },
    BeamSearch {
        width: u32,
    },
    ToT {
        depth: u32,
        branching: u32,
    },
}

impl StrategyKind {
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::BestFirst => "best_first",
            Self::Mcts { .. } => "mcts",
            Self::BeamSearch { .. } => "beam_search",
            Self::ToT { .. } => "tot",
        }
    }
}

// ── Cases + results ─────────────────────────────────────────────────────

/// One test case: candidate strategy + ground-truth clause counts.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyExperimentCase {
    pub case_id: String,
    pub candidate: StrategyKind,
    pub expected_clauses_satisfied: u32,
    pub total_clauses: u32,
}

/// One row in the experiment summary.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyExperimentResult {
    pub case_id: String,
    pub candidate: StrategyKind,
    pub matched_clauses: u32,
    pub total_clauses: u32,
    /// `matched_clauses * 10_000 / total_clauses` (integer division).
    pub match_rate_bps: u32,
    /// True iff `match_rate_bps >= min_match_rate_bps`.
    pub accepted: bool,
    pub recorded_at: String,
}

// ── Verdict + summary ───────────────────────────────────────────────────

/// Closed-set taxonomy of experiment verdicts.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyExperimentVerdict {
    AcceptAll,
    RejectAll {
        failing_cases: Vec<String>,
    },
    Mixed {
        accepted: usize,
        rejected: usize,
        failing: Vec<String>,
    },
}

/// Aggregate of an experiment run.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyExperimentSummary {
    pub experiment_id: String,
    pub candidate: StrategyKind,
    /// Results sorted by `case_id`.
    pub results: Vec<StrategyExperimentResult>,
    pub verdict: StrategyExperimentVerdict,
    pub min_match_rate_bps: u32,
    pub generated_at: String,
    /// True if the batch exceeded [`STRATEGY_EXPERIMENT_BUDGET`].
    pub over_budget: bool,
}

// ── Runner trait + default ──────────────────────────────────────────────

/// Evaluates a batch of [`StrategyExperimentCase`] rows and produces a
/// [`StrategyExperimentSummary`].
pub trait StrategyExperimentRunner {
    fn run(
        &self,
        experiment_id: &str,
        candidate: &StrategyKind,
        cases: &[StrategyExperimentCase],
        min_match_rate_bps: u32,
        recorded_at_for: impl Fn(&str) -> String, // case_id -> ts
        generated_at: &str,
    ) -> StrategyExperimentSummary;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultStrategyExperimentRunner;

impl StrategyExperimentRunner for DefaultStrategyExperimentRunner {
    fn run(
        &self,
        experiment_id: &str,
        candidate: &StrategyKind,
        cases: &[StrategyExperimentCase],
        min_match_rate_bps: u32,
        recorded_at_for: impl Fn(&str) -> String,
        generated_at: &str,
    ) -> StrategyExperimentSummary {
        // Budget enforcement.
        if cases.len() > STRATEGY_EXPERIMENT_BUDGET {
            return StrategyExperimentSummary {
                experiment_id: experiment_id.to_string(),
                candidate: candidate.clone(),
                results: Vec::new(),
                verdict: StrategyExperimentVerdict::RejectAll {
                    failing_cases: vec!["__over_budget__".to_string()],
                },
                min_match_rate_bps,
                generated_at: generated_at.to_string(),
                over_budget: true,
            };
        }

        // Evaluate each case.
        let mut results: Vec<StrategyExperimentResult> = cases
            .iter()
            .map(|c| {
                let total = c.total_clauses.max(1); // avoid div-by-zero
                let matched = c.expected_clauses_satisfied.min(total);
                let match_rate_bps = (matched as u64 * 10_000 / total as u64) as u32;
                let accepted = match_rate_bps >= min_match_rate_bps;
                StrategyExperimentResult {
                    case_id: c.case_id.clone(),
                    candidate: c.candidate.clone(),
                    matched_clauses: matched,
                    total_clauses: total,
                    match_rate_bps,
                    accepted,
                    recorded_at: recorded_at_for(&c.case_id),
                }
            })
            .collect();
        results.sort_by(|a, b| a.case_id.cmp(&b.case_id));

        // Build verdict.
        let total = results.len();
        let accepted = results.iter().filter(|r| r.accepted).count();
        let rejected = total - accepted;
        let mut failing: Vec<String> = results
            .iter()
            .filter(|r| !r.accepted)
            .map(|r| r.case_id.clone())
            .collect();
        failing.sort();

        let verdict = if rejected == 0 {
            StrategyExperimentVerdict::AcceptAll
        } else if accepted == 0 {
            StrategyExperimentVerdict::RejectAll {
                failing_cases: failing,
            }
        } else {
            StrategyExperimentVerdict::Mixed {
                accepted,
                rejected,
                failing,
            }
        };

        StrategyExperimentSummary {
            experiment_id: experiment_id.to_string(),
            candidate: candidate.clone(),
            results,
            verdict,
            min_match_rate_bps,
            generated_at: generated_at.to_string(),
            over_budget: false,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::collapsible_if)]
mod tests {
    use super::*;

    fn runner() -> DefaultStrategyExperimentRunner {
        DefaultStrategyExperimentRunner
    }

    fn case(id: &str, cand: StrategyKind, sat: u32, total: u32) -> StrategyExperimentCase {
        StrategyExperimentCase {
            case_id: id.to_string(),
            candidate: cand,
            expected_clauses_satisfied: sat,
            total_clauses: total,
        }
    }

    fn ts_fn() -> impl Fn(&str) -> String {
        |_| "t0".to_string()
    }

    // ── S-1: AcceptAll ────────────────────────────────────────────────

    #[test]
    fn s1_accept_all() {
        let cases = vec![
            case("c1", StrategyKind::BestFirst, 10, 10),
            case("c2", StrategyKind::BestFirst, 9, 10),
        ];
        let s = runner().run(
            "exp-1",
            &StrategyKind::BestFirst,
            &cases,
            8_000,
            ts_fn(),
            "t-gen",
        );
        assert!(matches!(s.verdict, StrategyExperimentVerdict::AcceptAll));
        assert_eq!(s.results.len(), 2);
        assert!(!s.over_budget);
    }

    // ── S-2: RejectAll ────────────────────────────────────────────────

    #[test]
    fn s2_reject_all_sorted() {
        let cases = vec![
            case("c3", StrategyKind::BestFirst, 1, 10),
            case("c1", StrategyKind::BestFirst, 2, 10),
            case("c2", StrategyKind::BestFirst, 0, 10),
        ];
        let s = runner().run(
            "exp-2",
            &StrategyKind::BestFirst,
            &cases,
            9_000,
            ts_fn(),
            "t-gen",
        );
        match &s.verdict {
            StrategyExperimentVerdict::RejectAll { failing_cases } => {
                assert_eq!(
                    failing_cases,
                    &vec!["c1".to_string(), "c2".to_string(), "c3".to_string()]
                );
            }
            other => panic!("expected RejectAll, got {other:?}"),
        }
    }

    // ── S-3: Mixed ────────────────────────────────────────────────────

    #[test]
    fn s3_mixed_verdict() {
        let cases = vec![
            case("c1", StrategyKind::BestFirst, 10, 10),
            case("c2", StrategyKind::BestFirst, 5, 10),
            case("c3", StrategyKind::BestFirst, 8, 10),
        ];
        let s = runner().run(
            "exp-3",
            &StrategyKind::BestFirst,
            &cases,
            8_000,
            ts_fn(),
            "t-gen",
        );
        match &s.verdict {
            StrategyExperimentVerdict::Mixed {
                accepted,
                rejected,
                failing,
            } => {
                assert_eq!(*accepted, 2);
                assert_eq!(*rejected, 1);
                assert_eq!(failing, &vec!["c2".to_string()]);
            }
            other => panic!("expected Mixed, got {other:?}"),
        }
    }

    // ── S-4: Budget cap ──────────────────────────────────────────────

    #[test]
    fn s4_budget_cap_rejects_overflow() {
        let cases: Vec<StrategyExperimentCase> = (0..(STRATEGY_EXPERIMENT_BUDGET + 1))
            .map(|i| case(&format!("c{i}"), StrategyKind::BestFirst, 10, 10))
            .collect();
        let s = runner().run(
            "exp-4",
            &StrategyKind::BestFirst,
            &cases,
            5_000,
            ts_fn(),
            "t-gen",
        );
        assert!(s.over_budget);
        assert!(s.results.is_empty());
        match s.verdict {
            StrategyExperimentVerdict::RejectAll { failing_cases } => {
                assert_eq!(failing_cases, vec!["__over_budget__".to_string()]);
            }
            other => panic!("expected RejectAll, got {other:?}"),
        }
    }

    // ── S-5: Results sorted by case_id ───────────────────────────────

    #[test]
    fn s5_results_sorted_by_case_id() {
        let cases = vec![
            case("c2", StrategyKind::BestFirst, 10, 10),
            case("c1", StrategyKind::BestFirst, 10, 10),
            case("c3", StrategyKind::BestFirst, 10, 10),
        ];
        let s = runner().run(
            "exp-5",
            &StrategyKind::BestFirst,
            &cases,
            8_000,
            ts_fn(),
            "t-gen",
        );
        let ids: Vec<&str> = s.results.iter().map(|r| r.case_id.as_str()).collect();
        assert_eq!(ids, vec!["c1", "c2", "c3"]);
    }

    // ── S-6: match_rate arithmetic ───────────────────────────────────

    #[test]
    fn s6_match_rate_arithmetic() {
        let cases = vec![
            case("c1", StrategyKind::BestFirst, 7, 10), // 7000 bps
            case("c2", StrategyKind::BestFirst, 3, 4),  // 7500 bps
        ];
        let s = runner().run(
            "exp-6",
            &StrategyKind::BestFirst,
            &cases,
            0,
            ts_fn(),
            "t-gen",
        );
        assert_eq!(s.results[0].match_rate_bps, 7_000);
        assert_eq!(s.results[1].match_rate_bps, 7_500);
    }

    // ── S-7: closed-set audit ────────────────────────────────────────

    #[test]
    fn s7_closed_set_audit() {
        assert_eq!(STRATEGY_KIND_VARIANT_COUNT, 4);
        let all_kinds = [
            StrategyKind::BestFirst,
            StrategyKind::Mcts {
                rollouts: 1,
                exploration_c: 1.41,
                max_depth: 1,
            },
            StrategyKind::BeamSearch { width: 1 },
            StrategyKind::ToT {
                depth: 1,
                branching: 1,
            },
        ];
        assert_eq!(all_kinds.len(), STRATEGY_KIND_VARIANT_COUNT);
        let labels: Vec<&str> = all_kinds.iter().map(|k| k.label()).collect();
        assert_eq!(labels, vec!["best_first", "mcts", "beam_search", "tot"]);
    }

    // ── Bonus: determinism ───────────────────────────────────────────

    #[test]
    fn s8_deterministic_output() {
        let cases = vec![case("c1", StrategyKind::BestFirst, 8, 10)];
        let s1 = runner().run(
            "exp-8",
            &StrategyKind::BestFirst,
            &cases,
            7_000,
            ts_fn(),
            "t-A",
        );
        let s2 = runner().run(
            "exp-8",
            &StrategyKind::BestFirst,
            &cases,
            7_000,
            ts_fn(),
            "t-B",
        );
        assert_eq!(s1.results, s2.results);
        assert_eq!(s1.verdict, s2.verdict);
        assert_ne!(s1.generated_at, s2.generated_at);
    }

    // ── Bonus: zero total_clauses ⇒ denom=1 (no panic) ───────────────

    #[test]
    fn s9_zero_total_clauses_protected() {
        let cases = vec![case("c1", StrategyKind::BestFirst, 0, 0)];
        let s = runner().run(
            "exp-9",
            &StrategyKind::BestFirst,
            &cases,
            0,
            ts_fn(),
            "t-gen",
        );
        assert_eq!(s.results[0].total_clauses, 1);
        assert_eq!(s.results[0].match_rate_bps, 0);
    }
}
