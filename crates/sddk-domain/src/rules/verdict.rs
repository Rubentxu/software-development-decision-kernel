//! Gate verdict for architecture conformance — C3l.7.
//!
//! `RuleStatus` answers "what did the evaluator observe about one rule?".
//! It does **not** answer "is this repository architecturally conformant?",
//! and conflating the two is the defect this module fixes: a repository with
//! a known open violation reported `exit 0` through the same branch as a clean
//! one, so the gate could be cited as evidence of conformance while the
//! violation was still on the books.
//!
//! The package policy (C3l.7) is explicit: a violation of `severity = error`
//! may only be in one of `OPEN_DEBT`, `WAIVED_UNTIL(date)` or `FIXED`. It
//! "cannot count as architecture gate PASS". This module makes that
//! distinction a typed value instead of a reading of the exit code.
//!
//! The classification is deliberately conservative:
//!
//! - `OpenDebt` — a real, unwaived `Fail` at error severity. The violation
//!   exists and nobody has excepted it. The gate must not be green.
//! - `Waived` — a `Waived` status. Conformant *for now*, under an explicit
//!   human exception, and therefore expirable by construction.
//! - `NotEvaluated` — a `NotApplicable` status. The rule was not checked, so
//!   it can neither confirm nor deny conformance. Counting silence as
//!   evidence is exactly how a gate becomes theatre.
//! - `Conformant` — every rule that was actually evaluated either passed or
//!   is covered by a live waiver, and nothing is outstanding.
//!
//! `Verdict::is_conformant()` is false for everything except `Conformant`,
//! so a caller cannot accidentally treat debt, an exception, or silence as
//! evidence of a clean architecture.

use super::types::{RuleSeverity, RuleStatus};
use serde::{Deserialize, Serialize};

/// Aggregate gate verdict, computed from the per-rule outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Every applicable rule was checked and holds. The only state that
    /// licenses the claim "this repository conforms architecturally".
    Conformant,
    /// A rule with error severity failed and no waiver covers it. The
    /// violation is on the books as open debt.
    OpenDebt,
    /// At least one error-severity rule is covered by a live waiver. The
    /// repository conforms only while those waivers hold.
    Waived,
    /// At least one rule was not evaluated (target absent, wrong phase, or
    /// an expired waiver). Conformance is undetermined, not proven.
    NotEvaluated,
}

impl Verdict {
    /// Whether this verdict licenses claiming architectural conformance.
    ///
    /// Only [`Verdict::Conformant`] returns true. `OpenDebt`, `Waived` and
    /// `NotEvaluated` all return false, so the claim is unavailable by
    /// default and has to be earned.
    pub fn is_conformant(self) -> bool {
        matches!(self, Verdict::Conformant)
    }

    /// Wire form used in `--out` JSON and in the human table.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Conformant => "CONFORMANT",
            Verdict::OpenDebt => "OPEN_DEBT",
            Verdict::Waived => "WAIVED",
            Verdict::NotEvaluated => "NOT_EVALUATED",
        }
    }

    /// Merge two partial observations into the most severe one.
    ///
    /// Severity order, strongest first: `OpenDebt` outranks everything because
    /// an unwaived error-severity violation is the only state that means work
    /// is outstanding *and* unexcepted. `Waived` outranks `NotEvaluated`
    /// because a live waiver is positive evidence about the tree, while
    /// `NotEvaluated` is silence.
    fn merge(self, other: Verdict) -> Verdict {
        use Verdict::*;
        match (self, other) {
            (OpenDebt, _) | (_, OpenDebt) => OpenDebt,
            (Waived, _) | (_, Waived) => Waived,
            (NotEvaluated, _) | (_, NotEvaluated) => NotEvaluated,
            (Conformant, Conformant) => Conformant,
        }
    }
}

/// One rule's contribution to the verdict.
#[derive(Debug, Clone, Copy)]
pub struct RuleOutcome<'a> {
    /// The evaluator's per-rule status.
    pub status: RuleStatus,
    /// The rule's declared severity, used to decide whether a `Fail` is
    /// debt (error) or an advisory (warning).
    pub severity: RuleSeverity,
    /// The rule identifier, kept for diagnostics.
    pub rule_id: &'a str,
}

/// Compute the aggregate verdict for a set of per-rule outcomes.
///
/// Empty input is **not** `Conformant`: an evaluation that checked nothing
/// has not demonstrated anything, and returning `Conformant` here would let
/// an empty rule file mint a clean architectural claim.
pub fn verdict_for(outcomes: &[RuleOutcome<'_>]) -> Verdict {
    if outcomes.is_empty() {
        return Verdict::NotEvaluated;
    }
    outcomes
        .iter()
        .map(|o| match (o.status, o.severity) {
            // A real, unwaived error-severity violation: open debt.
            (RuleStatus::Fail, RuleSeverity::Error) => Verdict::OpenDebt,
            // An error-severity violation under a live human exception.
            (RuleStatus::Waived, RuleSeverity::Error) => Verdict::Waived,
            // The rule was not checked, so it cannot testify either way.
            (RuleStatus::NotApplicable, _) => Verdict::NotEvaluated,
            // A warning-severity failure is reported but never blocks CI, so
            // it is not outstanding work. `WarningThenRatchet` is a warning
            // whose existing violations are deliberately grandfathered in
            // place; blocking on one would defeat its entire purpose, which
            // is to freeze the status quo while new code complies.
            (RuleStatus::Pass, _)
            | (RuleStatus::Fail, RuleSeverity::Warning)
            | (RuleStatus::Fail, RuleSeverity::WarningThenRatchet)
            | (RuleStatus::Waived, RuleSeverity::Warning)
            | (RuleStatus::Waived, RuleSeverity::WarningThenRatchet) => Verdict::Conformant,
        })
        .reduce(Verdict::merge)
        .unwrap_or(Verdict::NotEvaluated)
}

/// Process exit code implied by a verdict.
///
/// `0` only for [`Verdict::Conformant`]; `1` for a real unwaived violation
/// (unchanged from the historical contract, so existing callers keep their
/// meaning); `2` for "not proven" — live waivers or unevaluated rules. The
/// split matters: a caller that must fail closed on *unproven* conformance
/// can distinguish it from a proven violation.
pub fn exit_code_for(verdict: Verdict) -> i32 {
    match verdict {
        Verdict::Conformant => 0,
        Verdict::OpenDebt => 1,
        Verdict::Waived | Verdict::NotEvaluated => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(status: RuleStatus, severity: RuleSeverity) -> RuleOutcome<'static> {
        RuleOutcome {
            status,
            severity,
            rule_id: "ARCH001",
        }
    }

    #[test]
    fn all_pass_is_conformant() {
        let v = verdict_for(&[
            out(RuleStatus::Pass, RuleSeverity::Error),
            out(RuleStatus::Pass, RuleSeverity::Warning),
        ]);
        assert_eq!(v, Verdict::Conformant);
        assert!(v.is_conformant());
        assert_eq!(exit_code_for(v), 0);
    }

    #[test]
    fn unwaived_error_fail_is_open_debt_and_never_green() {
        let v = verdict_for(&[out(RuleStatus::Fail, RuleSeverity::Error)]);
        assert_eq!(v, Verdict::OpenDebt);
        assert!(!v.is_conformant(), "open debt must not be conformant");
        assert_eq!(exit_code_for(v), 1);
    }

    #[test]
    fn open_debt_outranks_a_passing_rule() {
        // The regression this module exists for: one violation among several
        // passes must dominate the verdict, not be averaged away.
        let v = verdict_for(&[
            out(RuleStatus::Pass, RuleSeverity::Error),
            out(RuleStatus::Pass, RuleSeverity::Error),
            out(RuleStatus::Fail, RuleSeverity::Error),
            out(RuleStatus::Pass, RuleSeverity::Error),
        ]);
        assert_eq!(v, Verdict::OpenDebt);
        assert!(!v.is_conformant());
    }

    #[test]
    fn warning_fail_does_not_block_conformance() {
        let v = verdict_for(&[out(RuleStatus::Fail, RuleSeverity::Warning)]);
        assert_eq!(v, Verdict::Conformant);
        assert_eq!(exit_code_for(v), 0);
    }

    #[test]
    fn live_waiver_is_not_conformance() {
        let v = verdict_for(&[out(RuleStatus::Waived, RuleSeverity::Error)]);
        assert_eq!(v, Verdict::Waived);
        assert!(!v.is_conformant(), "a waiver is an exception, not proof");
        assert_eq!(exit_code_for(v), 2);
    }

    #[test]
    fn not_applicable_is_not_evidence_of_conformance() {
        let v = verdict_for(&[out(RuleStatus::NotApplicable, RuleSeverity::Error)]);
        assert_eq!(v, Verdict::NotEvaluated);
        assert!(!v.is_conformant(), "silence is not conformance");
        assert_eq!(exit_code_for(v), 2);
    }

    #[test]
    fn waived_outranks_not_evaluated() {
        // A live waiver is positive evidence about the tree: somebody checked
        // this rule and excepted it. `NotEvaluated` is silence. When both are
        // present the verdict must name the more informative one, so a report
        // says "under waiver" rather than burying the exception in "unknown".
        // Neither is conformant, which the assertions below pin.
        let v = verdict_for(&[
            out(RuleStatus::Waived, RuleSeverity::Error),
            out(RuleStatus::NotApplicable, RuleSeverity::Error),
        ]);
        assert_eq!(v, Verdict::Waived);
        assert!(!v.is_conformant());
        assert_eq!(exit_code_for(v), 2);
    }

    #[test]
    fn empty_evaluation_is_not_conformant() {
        // An empty rule file must not be able to mint a clean claim.
        let v = verdict_for(&[]);
        assert_eq!(v, Verdict::NotEvaluated);
        assert!(!v.is_conformant());
        assert_eq!(exit_code_for(v), 2);
    }
}
