//! `dev check-architecture` — live architecture rule evaluator.
//!
//! Runs the Phase 1 evaluators against the live workspace baseline and prints
//! a tabular summary, followed by an explicit aggregate verdict:
//!
//! ```text
//! RULE      STATUS    DETAIL
//! ARCH001   PASS
//! ARCH003   WAIVED    waived: …
//! ARCH004   N/A       kernel repo
//!
//! VERDICT: WAIVED (exit 2)
//! This gate does NOT certify architectural conformance. See VERDICT above.
//! ```
//!
//! ## Exit codes and the verdict (C3l.7)
//!
//! The per-rule table says what each evaluator observed; it does **not** say
//! whether the repository conforms. Those were conflated: `Waived` and
//! `NotApplicable` used to reach `exit 0` through the same branch as a clean
//! tree, so a repository with open architectural debt could be cited as
//! conformant. The verdict is now a typed value (`sddk_domain::rules::verdict`)
//! and `exit 0` is reserved for it:
//!
//! - `0` — `CONFORMANT`: every applicable rule was checked and holds.
//! - `1` — `OPEN_DEBT`: a real unwaived `severity = error` violation.
//! - `2` — `WAIVED` / `NOT_EVALUATED`: conformance is **not proven** — live
//!   waivers, or rules whose evaluators do not exist yet. A caller that must
//!   fail closed on unproven conformance can tell this apart from a proven
//!   violation.
//!
//! JSON output (when `--out` is specified) carries both `verdict` and
//! `exit_status`; a programmatic consumer should read `verdict`, since
//! `exit_status` alone conflates "proven clean" with "not proven".

use crate::CommandOutput;
use sddk_domain::{
    RuleOutcome, RuleRegistry, RuleSeverity, RuleStatus, Verdict, exit_code_for, verdict_for,
};
use sddk_engine::rules::{BaselineConsumer, evaluate_all_with_resolver, git_ancestry_resolver};
use serde::Serialize;

/// Architecture check result rendered as a single table row.
#[derive(Debug, Clone, Serialize)]
pub struct ArchCheckRow {
    pub rule_id: String,
    pub status: String,
    pub detail: String,
}

/// JSON output written when `--out <path>` is specified.
#[derive(Serialize)]
struct ArchCheckOutput {
    schema_version: &'static str,
    evaluator_version: &'static str,
    baseline_sha256: String,
    head_anchor: String,
    evaluated_at: String,
    rows: Vec<ArchCheckRow>,
    /// Aggregate gate verdict (C3l.7). A consumer reading this JSON must use
    /// `verdict` to decide whether conformance is claimed — `exit_status` alone
    /// conflates "proven clean" with "not proven".
    verdict: &'static str,
    exit_status: i32,
}

pub(super) fn run_check_architecture(args: super::CheckArchitectureArgs) -> CommandOutput {
    let root = args.root.as_path();
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_owned());

    // ── Resolve rules path ─────────────────────────────────────────────────
    let rules_path = args.rules.unwrap_or_else(|| {
        root.join("docs/history/legacy-packages/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml")
    });

    let rules_yaml = match std::fs::read_to_string(&rules_path) {
        Ok(s) => s,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!(
                    "error: failed to read rules file {}: {e}\n",
                    rules_path.display()
                ),
            };
        }
    };

    let registry = match RuleRegistry::from_yaml_str(&rules_yaml) {
        Ok(r) => r,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error: failed to parse rules YAML: {e}\n"),
            };
        }
    };

    // ── Live baseline capture ───────────────────────────────────────────────
    let baseline = match BaselineConsumer::capture_live(root) {
        Ok(b) => b,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error: live baseline capture failed: {e}\n"),
            };
        }
    };

    // ── Evaluate ────────────────────────────────────────────────────────────
    // INC-DEBT-018: decide waiver expiry by real git ancestry, not lexicographic
    // SHA compare (a fixed granted_until_sha expired on every commit otherwise).
    let resolver = git_ancestry_resolver(root);
    let evaluations = evaluate_all_with_resolver(&registry, &baseline, &now, resolver, Some(root));

    // ── Render tabular output ──────────────────────────────────────────────
    let mut rows: Vec<ArchCheckRow> = Vec::new();
    let mut has_error_fail = false;
    let mut stdout = String::new();

    // Column widths for alignment
    const RULE_W: usize = 8;
    const STATUS_W: usize = 8;
    const DETAIL_W: usize = 60;

    // Header
    stdout.push_str(&format!(
        "{:RULE_W$}  {:STATUS_W$}  {:DETAIL_W$}\n",
        "RULE", "STATUS", "DETAIL"
    ));
    stdout.push_str(&format!(
        "{:-<RULE_W$}  {:-<STATUS_W$}  {:-<DETAIL_W$}\n",
        "", "", ""
    ));

    for eval in &evaluations {
        let rule = registry.iter().find(|r| r.id == eval.rule_id);
        let severity = rule.map(|r| r.severity).unwrap_or(RuleSeverity::Error);
        let detail = detail_for(&eval.status, &eval.observed, eval.provenance.as_deref());

        let status_str = match eval.status {
            RuleStatus::Pass => "PASS",
            RuleStatus::Fail => "FAIL",
            RuleStatus::Waived => "WAIVED",
            RuleStatus::NotApplicable => "N/A",
        };

        rows.push(ArchCheckRow {
            rule_id: eval.rule_id.clone(),
            status: status_str.to_owned(),
            detail: detail.clone(),
        });

        // Truncate detail for display
        let detail_display = if detail.len() > DETAIL_W {
            format!("{}…", &detail[..DETAIL_W - 1])
        } else {
            detail
        };

        stdout.push_str(&format!(
            "{:RULE_W$}  {:STATUS_W$}  {}\n",
            eval.rule_id, status_str, detail_display
        ));

        if eval.status == RuleStatus::Fail && severity == RuleSeverity::Error {
            has_error_fail = true;
        }
    }

    // ── Gate verdict (C3l.7) ────────────────────────────────────────────────
    // The per-rule table says what each evaluator observed; it does not say
    // whether the repository conforms. Before this, `exit 0` was reachable
    // through the same branch for a clean tree, a waived violation and an
    // unevaluated rule alike, so the gate could be cited as proof of
    // conformance while debt was still open. The verdict is the typed answer
    // to that question, and only CONFORMANT licenses the claim.
    let outcomes: Vec<_> = evaluations
        .iter()
        .map(|eval| {
            let severity = registry
                .iter()
                .find(|r| r.id == eval.rule_id)
                .map(|r| r.severity)
                .unwrap_or(RuleSeverity::Error);
            RuleOutcome {
                status: eval.status,
                severity,
                rule_id: eval.rule_id.as_str(),
            }
        })
        .collect();
    let verdict = verdict_for(&outcomes);
    let exit_status = exit_code_for(verdict);
    // `has_error_fail` is now implied by the verdict: a Fail at error
    // severity is exactly what produces OpenDebt. Kept as an assertion of
    // that invariant rather than a second source of truth for the exit code.
    debug_assert_eq!(has_error_fail, verdict == Verdict::OpenDebt);
    let rows_count = rows.len();

    stdout.push_str(&format!(
        "\nVERDICT: {} (exit {})\n",
        verdict.as_str(),
        exit_status
    ));
    if !verdict.is_conformant() {
        stdout
            .push_str("This gate does NOT certify architectural conformance. See VERDICT above.\n");
    }

    // ── JSON output (optional) ──────────────────────────────────────────────
    if let Some(out_path) = &args.out {
        let output = ArchCheckOutput {
            schema_version: "1.0.0",
            evaluator_version: sddk_engine::rules::EVALUATOR_VERSION,
            baseline_sha256: baseline.ref_.sha256.clone(),
            head_anchor: baseline.ref_.head_anchor.clone(),
            evaluated_at: now,
            rows: rows.clone(),
            verdict: verdict.as_str(),
            exit_status,
        };
        let json = match serde_json::to_string_pretty(&output) {
            Ok(s) => s,
            Err(e) => {
                return CommandOutput {
                    status: 1,
                    stdout,
                    stderr: format!("error: failed to serialize JSON: {e}\n"),
                };
            }
        };
        if let Err(e) = std::fs::write(out_path, json) {
            return CommandOutput {
                status: 1,
                stdout,
                stderr: format!("error: failed to write {}: {e}\n", out_path.display()),
            };
        }
        stdout.push_str(&format!(
            "\n[wrote {} rule rows to {}]\n",
            rows_count,
            out_path.display()
        ));
    }

    CommandOutput {
        status: exit_status,
        stdout,
        stderr: String::new(),
    }
}

/// Builds a human-readable detail string from an evaluation.
fn detail_for(
    status: &RuleStatus,
    observed: &serde_json::Value,
    provenance: Option<&str>,
) -> String {
    match status {
        RuleStatus::Pass => String::new(),
        RuleStatus::Fail => {
            // An evaluator that knows what its violations ARE says so. Without
            // this, a manifest rule rendered as "3 edge(s) detected" because
            // the only fallback counted things: "edge" is ARCH001's noun, not
            // the vocabulary of a pack manifest or a governed-effect scan.
            if let Some(summary) = observed.get("summary").and_then(|v| v.as_str()) {
                return summary.to_owned();
            }
            if let Some(count) = observed.get("count").and_then(|v| v.as_u64()) {
                if count == 0 {
                    return "violation detected".to_owned();
                }
                return format!("{} edge(s) detected", count);
            }
            provenance.unwrap_or("violation detected").to_owned()
        }
        RuleStatus::Waived => {
            if let Some(reason) = observed.get("reason").and_then(|v| v.as_str()) {
                format!("waived: {}", reason)
            } else {
                provenance.unwrap_or("waived").to_owned()
            }
        }
        RuleStatus::NotApplicable => provenance.unwrap_or("not applicable").to_owned(),
    }
}
