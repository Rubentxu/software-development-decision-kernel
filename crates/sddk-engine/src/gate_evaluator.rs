//! Declarative gate evaluator for debt-report findings.
//!
//! Evaluates the 2 gates declared in the cycle-7b workflow contract:
//! - `debt-severity-assigned`: every finding has severity ∈ {critical, high, medium, low}
//! - `debt-priority-assigned`: every finding have priority ∈ {P0, P1, P2, P3}

use sddk_domain::DebtReport;

/// Outcome of a gate evaluation (without persistence).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum GateOutcome {
    Passed {
        notes: String,
    },
    Failed {
        offending_ids: Vec<String>,
        notes: String,
    },
}

/// Valid severity values per schema.
const DEBT_SEVERITY_GATE: &str = "debt-severity-assigned";

/// Valid priority values per schema.
const DEBT_PRIORITY_GATE: &str = "debt-priority-assigned";

/// Evaluates a named gate against a debt report.
///
/// Returns `GateOutcome::Passed` if all findings satisfy the gate contract.
/// Returns `GateOutcome::Failed` with offending finding IDs otherwise.
pub fn evaluate_named_gate(name: &str, report: &DebtReport) -> GateOutcome {
    match name {
        DEBT_SEVERITY_GATE => evaluate_with_predicate(
            report,
            |f| matches!(f.severity.as_str(), "critical" | "high" | "medium" | "low"),
            "severity not in {critical,high,medium,low}",
        ),
        DEBT_PRIORITY_GATE => evaluate_with_predicate(
            report,
            |f| matches!(f.priority.as_str(), "P0" | "P1" | "P2" | "P3"),
            "priority not in {P0,P1,P2,P3}",
        ),
        other => GateOutcome::Failed {
            offending_ids: vec![],
            notes: format!("unknown gate: {other}"),
        },
    }
}

fn evaluate_with_predicate<F>(report: &DebtReport, is_valid: F, fail_message: &str) -> GateOutcome
where
    F: Fn(&sddk_domain::Finding) -> bool,
{
    let invalid: Vec<String> = report
        .findings
        .iter()
        .filter(|f| !is_valid(f))
        .map(|f| f.id.clone())
        .collect();
    if invalid.is_empty() {
        GateOutcome::Passed {
            notes: format!("{} findings checked", report.findings.len()),
        }
    } else {
        GateOutcome::Failed {
            offending_ids: invalid,
            notes: fail_message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{DebtReport, Finding, FindingStatus, Priority, Severity};

    fn valid_report() -> DebtReport {
        DebtReport {
            schema_version: "1.1.0".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            generated_at: "2026-08-21T00:00:00Z".into(),
            findings: vec![
                Finding {
                    id: "FIND-0001".into(),
                    title: "Test".into(),
                    severity: Severity::Medium,
                    priority: Priority::P2,
                    status: FindingStatus::Open,
                    fingerprint: "3ef321c4efe1d87e".into(),
                    fingerprint_aliases: vec![],
                    cluster_id: "CL-01".into(),
                    category: "architecture".into(),
                    description: "Test finding".into(),
                    remediation_cycle: None,
                    remediation_pr: None,
                    evidence_refs: None,
                },
                Finding {
                    id: "FIND-0002".into(),
                    title: "Test 2".into(),
                    severity: Severity::Critical,
                    priority: Priority::P0,
                    status: FindingStatus::Open,
                    fingerprint: "efa9e569e7c7b602".into(),
                    fingerprint_aliases: vec![],
                    cluster_id: "CL-02".into(),
                    category: "risk".into(),
                    description: "Critical finding".into(),
                    remediation_cycle: None,
                    remediation_pr: None,
                    evidence_refs: None,
                },
            ],
        }
    }

    #[test]
    fn test_gate_severity_pass() {
        let report = valid_report();
        let outcome = evaluate_named_gate(DEBT_SEVERITY_GATE, &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
    }

    #[test]
    fn test_gate_priority_pass() {
        let report = valid_report();
        let outcome = evaluate_named_gate(DEBT_PRIORITY_GATE, &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
    }

    #[test]
    fn test_gate_unknown() {
        let report = valid_report();
        let outcome = evaluate_named_gate("unknown-gate", &report);
        match &outcome {
            GateOutcome::Failed { notes, .. } => {
                assert!(notes.contains("unknown gate"));
            }
            _ => panic!("expected Failed for unknown gate"),
        }
    }

    #[test]
    fn test_gate_empty_report() {
        let report = DebtReport {
            schema_version: "1.1.0".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            generated_at: "2026-08-21T00:00:00Z".into(),
            findings: vec![],
        };
        let outcome = evaluate_named_gate(DEBT_SEVERITY_GATE, &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
        let outcome = evaluate_named_gate(DEBT_PRIORITY_GATE, &report);
        assert!(matches!(outcome, GateOutcome::Passed { .. }));
    }
}
