//! `dev check-architecture` — live architecture rule evaluator.
//!
//! Runs the Phase 1 evaluators against the live workspace baseline and prints
//! a tabular summary:
//!
//! ```text
//! ARCH001  FAIL    engine→storage: N edge(s)
//! ARCH002  PASS
//! ARCH003  PASS
//! ARCH004  N/A     kernel repo
//! ARCH005  N/A     Phase 5 not shipped
//! ```
//!
//! Exit code 1 if any rule with `severity = Error` has `status = Fail`.
//! Exit code 0 otherwise (Pass, Waived, N/A all exit 0).
//!
//! JSON output (when `--out` is specified) follows the same shape as `sddk rules check`.

use crate::CommandOutput;
use sddk_domain::{RuleRegistry, RuleSeverity, RuleStatus};
use sddk_engine::rules::{BaselineConsumer, evaluate_all};
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
    exit_status: i32,
}

pub(super) fn run_check_architecture(args: super::CheckArchitectureArgs) -> CommandOutput {
    let root = args.root.as_path();
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_owned());

    // ── Resolve rules path ─────────────────────────────────────────────────
    let rules_path = args.rules.unwrap_or_else(|| {
        root.join("docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml")
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
    let evaluations = evaluate_all(&registry, &baseline, &now);

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

    let exit_status = if has_error_fail { 1 } else { 0 };
    let rows_count = rows.len();

    // ── JSON output (optional) ──────────────────────────────────────────────
    if let Some(out_path) = &args.out {
        let output = ArchCheckOutput {
            schema_version: "1.0.0",
            evaluator_version: sddk_engine::rules::EVALUATOR_VERSION,
            baseline_sha256: baseline.ref_.sha256.clone(),
            head_anchor: baseline.ref_.head_anchor.clone(),
            evaluated_at: now,
            rows: rows.clone(),
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

#[cfg(test)]
#[path = "tests/check_architecture_tests.rs"]
mod check_architecture_tests;
