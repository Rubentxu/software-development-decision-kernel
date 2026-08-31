//! Real evaluators for ARCH001..015.

// `missing_docs` is allowed across this file because the Phase 1 ARCH
// evaluators were introduced before the workspace-wide
// `#![warn(missing_docs)]` activation. A future docs-pass cycle should
// restore the per-item `///` doc comments and remove this allow.
#![allow(missing_docs)]

use regex::RegexSet;
use sddk_domain::{EvaluatorKind, RuleEvaluation, RuleRegistry, RuleStatus};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use super::Baseline;

pub const EVALUATOR_VERSION: &str = "0.1.0";

/// Evaluates every registered rule against the baseline (Phase 1).
///
/// Waiver precedence: if a waiver exists and `baseline.ref_.head_anchor <=
/// w.granted_until_sha`, the evaluation is overridden to `Waived`.
/// Expired waivers (head_anchor > granted_until_sha) result in `NotApplicable`
/// to preserve Phase 0 backward compatibility with existing waivers in the registry.
pub fn evaluate_all(
    registry: &RuleRegistry,
    baseline: &Baseline,
    evaluated_at: &str,
) -> Vec<RuleEvaluation> {
    registry
        .iter()
        .map(|rule| {
            // ── Waiver pre-check ──────────────────────────────────────────────
            if let Some(w) = registry.waiver_for(&rule.id) {
                if baseline.ref_.head_anchor <= w.granted_until_sha {
                    return RuleEvaluation {
                        rule_id: rule.id.clone(),
                        status: RuleStatus::Waived,
                        observed: json!({
                            "waiver_id": w.id,
                            "reason": w.reason
                        }),
                        baseline_sha256: baseline.ref_.sha256.clone(),
                        evaluated_at: evaluated_at.to_owned(),
                        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                        waiver_id: Some(w.id.clone()),
                        evaluator_kind: EvaluatorKind::Schema,
                        evaluator_version: EVALUATOR_VERSION.to_owned(),
                        provenance: None,
                    };
                }
                // Waiver expired → NotApplicable (Phase 0 backward compat)
                return RuleEvaluation {
                    rule_id: rule.id.clone(),
                    status: RuleStatus::NotApplicable,
                    observed: json!({ "phase": "phase0", "rule_id": rule.id }),
                    baseline_sha256: baseline.ref_.sha256.clone(),
                    evaluated_at: evaluated_at.to_owned(),
                    evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                    waiver_id: None,
                    evaluator_kind: EvaluatorKind::Schema,
                    evaluator_version: EVALUATOR_VERSION.to_owned(),
                    provenance: Some(format!(
                        "waiver {} expired at baseline {}",
                        w.id, baseline.ref_.head_anchor
                    )),
                };
            }

            // ── Rule-specific evaluation ─────────────────────────────────────
            match rule.id.as_str() {
                "ARCH001" => evaluate_arch001(rule, baseline, evaluated_at),
                "ARCH002" => evaluate_arch002(rule, baseline, evaluated_at),
                "ARCH003" => evaluate_arch003(rule, baseline, evaluated_at),
                "ARCH004" => evaluate_arch004(rule, baseline, evaluated_at),
                "ARCH005" => evaluate_arch005(rule, baseline, evaluated_at),
                "ARCH008" => evaluate_arch008(rule, baseline, evaluated_at),
                "ARCH013" => evaluate_arch013(rule, baseline, evaluated_at),
                "ARCH014" => evaluate_arch014(rule, baseline, evaluated_at),
                "ARCH015" => evaluate_arch015(rule, baseline, evaluated_at),
                _ => RuleEvaluation {
                    rule_id: rule.id.clone(),
                    status: RuleStatus::NotApplicable,
                    observed: json!({}),
                    baseline_sha256: baseline.ref_.sha256.clone(),
                    evaluated_at: evaluated_at.to_owned(),
                    evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
                    waiver_id: None,
                    evaluator_kind: EvaluatorKind::Schema,
                    evaluator_version: EVALUATOR_VERSION.to_owned(),
                    provenance: Some(format!("evaluator not implemented for {}", rule.id)),
                },
            }
        })
        .collect()
}

// ── ARCH001 ──────────────────────────────────────────────────────────────────

/// engine_must_not_depend_on_storage: Fail if any edge from sddk-engine to sddk-storage
/// exists in the baseline (Cargo dep or use statement).
fn evaluate_arch001(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == "sddk-engine" && e.to_crate == "sddk-storage")
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH001 live evaluator: checks sddk-engine→sddk-storage edges in \
             cross_crate_imports (Cargo deps + use statements)"
                .to_owned(),
        ),
    }
}

// ── ARCH002 ──────────────────────────────────────────────────────────────────

/// domain_must_not_depend_on_adapters: Fail if any edge from sddk-domain to
/// sddk-storage, sddk-gateway, or sddk-cli exists.
fn evaluate_arch002(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    let forbidden = ["sddk-storage", "sddk-gateway", "sddk-cli"];
    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| e.from_crate == "sddk-domain" && forbidden.contains(&e.to_crate.as_str()))
        .map(|e| {
            json!({
                "from_file": e.from_file,
                "line": e.line,
                "kind": e.kind,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH002 live evaluator: checks sddk-domain→{storage,gateway,cli} edges".to_owned(),
        ),
    }
}

// ── ARCH003 ──────────────────────────────────────────────────────────────────

/// production_crates_must_not_depend_on_storage_directly: Fail if any production
/// crate has a source-level `use` edge to `sddk-storage`, unless the edge is
/// from `sddk-storage` itself (internal use is fine) or the crate provides
/// `LedgerFactory` (P1-FIX-005).
///
/// This is the extended form of the original "cli must not own persistence logic"
/// rule: it covers ALL crates, not just CLI.  The exception for `LedgerFactory`
/// providers allows a crate to use `sddk-storage` when it has been explicitly
/// composed through the factory port (P1-FIX-002 / ADR-0021).
fn evaluate_arch003(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    use crate::rules::baseline::CrossCrateImportKind;

    // Known crates that provide LedgerFactory (implement or re-export the trait).
    // These may import from sddk-storage without triggering a violation.
    const LEDGER_FACTORY_PROVIDERS: &[&str] = &["sddk-domain", "sddk-storage"];

    let violating: Vec<_> = baseline
        .cross_crate_imports
        .iter()
        .filter(|e| {
            // Only source-level edges
            if e.kind != CrossCrateImportKind::Use {
                return false;
            }
            // Must be an edge to sddk-storage
            if e.to_crate != "sddk-storage" {
                return false;
            }
            // sddk-storage using itself is always fine
            if e.from_crate == "sddk-storage" {
                return false;
            }
            // Crates that provide LedgerFactory are allowed
            if LEDGER_FACTORY_PROVIDERS.contains(&e.from_crate.as_str()) {
                return false;
            }
            true
        })
        .map(|e| {
            json!({
                "from_crate": e.from_crate,
                "from_file": e.from_file,
                "line": e.line,
            })
        })
        .collect();

    let status = if violating.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "edges": violating,
            "count": violating.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH003 live evaluator: any production crate using sddk-storage directly \
             is a violation unless the crate provides LedgerFactory (P1-FIX-005)"
                .to_owned(),
        ),
    }
}

// ── ARCH004 ──────────────────────────────────────────────────────────────────

/// packs_must_declare_dependencies: NotApplicable in the kernel repo
/// (Phase 4 pack-host substrate not shipped here).
fn evaluate_arch004(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "kernel repo, not a pack host (Phase 4 substrate not shipped here)".to_owned(),
        ),
    }
}

// ── ARCH005 ──────────────────────────────────────────────────────────────────

/// reactive_behaviors_must_not_execute_governed_effects_directly:
/// NotApplicable until Phase 5 reactive runtime ships.
fn evaluate_arch005(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some("Phase 5 reactive runtime not yet shipped".to_owned()),
    }
}

// ── ARCH008 ──────────────────────────────────────────────────────────────────

/// SDD-agnostic kernel: workflow_ir and workflow_run must not reference Phase or
/// CyclePath as Rust enum identifiers.
///
/// Uses a 4-pattern RegexSet over the scoped source files:
///   - \bPhase::        — type-qualified match
///   - \bCyclePath::     — type-qualified match
///   - \b(Explore|Specify|Design|Tasks|Apply|Verify|Archive)\s*::  — variant-qualified
///   - match\s+phase\s*\{  — match-on-string anti-pattern
///
/// YAML/JSON literals are excluded by file-extension filtering (scope only covers .rs files).
fn evaluate_arch008(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    use sddk_domain::EvaluatorKind;

    // 4-pattern RegexSet for Phase/CyclePath coupling
    static ARCH008_PATTERNS: std::sync::LazyLock<RegexSet> = std::sync::LazyLock::new(|| {
        RegexSet::new([
            r"\bPhase::",                                                  // type-qualified
            r"\bCyclePath::",                                              // type-qualified
            r"\b(Explore|Specify|Design|Tasks|Apply|Verify|Archive)\s*::", // variant-qualified
            r"match\s+phase\s*\{",                                         // match-on-string
        ])
        .expect("static regex")
    });

    let mut violations = Vec::new();

    // Walk each scope glob from the rule
    for glob_pattern in &rule.scope {
        let matching: Vec<_> = glob_match_files(glob_pattern);
        for file_path in matching {
            if let Ok(content) = fs::read_to_string(&file_path) {
                for (line_no, line) in content.lines().enumerate() {
                    let line_num = (line_no + 1) as u32;
                    if ARCH008_PATTERNS.is_match(line) {
                        violations.push(json!({
                            "file": file_path.to_string_lossy(),
                            "line": line_num,
                            "text": line,
                        }));
                    }
                }
            }
        }
    }

    let status = if violations.is_empty() {
        RuleStatus::Pass
    } else {
        RuleStatus::Fail
    };

    RuleEvaluation {
        rule_id: rule.id.clone(),
        status,
        observed: json!({
            "violations": violations,
            "count": violations.len(),
        }),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH008 heuristic evaluator: scans scoped .rs files for Phase::/CyclePath:: patterns"
                .to_owned(),
        ),
    }
}

/// Simple glob matcher: expands "**/prefix" patterns recursively.
/// Returns absolute paths matching the pattern.
fn glob_match_files(pattern: &str) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();

    // Only handle **/prefix patterns for now (simple form)
    if let Some(prefix) = pattern.strip_prefix("**/") {
        let prefix = prefix.trim_end_matches('/');
        // Walk the repo root looking for files ending with prefix
        if let Ok(cwd) = std::env::current_dir() {
            walk_matching_files(&cwd, prefix, &mut results);
        }
    }
    results
}

fn walk_matching_files(dir: &Path, suffix: &str, results: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip target/, .git/, node_modules/
                let skip = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n == "target" || n == ".git" || n == "node_modules" || n == ".cargo")
                    .unwrap_or(false);
                if skip {
                    continue;
                }
                walk_matching_files(&path, suffix, results);
            } else if path.is_file() {
                let matches_suffix = path.to_str().map(|p| p.ends_with(suffix)).unwrap_or(false);
                if matches_suffix {
                    results.push(path);
                }
            }
        }
    }
}

// ── ARCH013 ──────────────────────────────────────────────────────────────────

/// dynamic_operators_require_capability_contract: stub for v1.29.0.
fn evaluate_arch013(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH013 substance (dynamic operator contracts) deferred to cycle 3".to_owned(),
        ),
    }
}

// ── ARCH014 ──────────────────────────────────────────────────────────────────

/// expansion_proposals_require_approval_receipt: stub for v1.29.0.
fn evaluate_arch014(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH014 substance (expansion proposal receipts) deferred to cycle 3".to_owned(),
        ),
    }
}

// ── ARCH015 ──────────────────────────────────────────────────────────────────

/// ir_events_must_not_emit_phase_strings: stub for v1.29.0.
fn evaluate_arch015(
    rule: &sddk_domain::ArchitectureRule,
    baseline: &Baseline,
    evaluated_at: &str,
) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Heuristic,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some(
            "ARCH015 substance (IR event phase strings) deferred to cycle 3".to_owned(),
        ),
    }
}
