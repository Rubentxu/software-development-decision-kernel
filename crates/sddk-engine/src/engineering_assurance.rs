//! Engineering Assurance: Profiles, Evidence Taxonomy and Rules.
//!
//! Cycle: EA-ASSURANCE-001 (H7, order 400, context pack
//! `engineering-assurance`).
//!
//! Pure, deterministic, replay-friendly subsystem that classifies
//! engineering evidence and emits typed `AssuranceVerdict` records.
//! Downstream cycles (`EA-ASSURANCE-002`, `UAT-BC-001`,
//! `EA-UAT-001`) consume the verdicts to gate promotion.
//!
//! ## Design
//!
//! - **Closed-set profiles.** Four profiles: Smoke, Full,
//!   Promotion, Custom. Adding a profile requires an ADR + audit.
//! - **Closed-set evidence tags.** Nine tags including
//!   `EngineTest`, `ClippyClean`, `Receipt`, `ReplayProof`.
//! - **Pure evaluator.** `evaluate` is a pure function of
//!   `(profile, evidence, evaluated_at)`.
//! - **Severity-aware aggregation.** `Must` failures ⇒
//!   `Fail`; `Should` failures ⇒ `Degraded`; `Info` failures ⇒
//!   `Pass`.
//! - **Read-only over canonical state.** The engine consumes
//!   already-collected evidence.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-EngineeringAssuranceProfiles` (spec, accepted)
//! - `ADR-101` (architecture decision, accepted)
//! - `REQ-WorkflowLabMetrics`, `REQ-StrategyComparison` (dependencies)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ── AssuranceProfile ─────────────────────────────────────────────────────

/// Closed-set assurance profiles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AssuranceProfile {
    /// Smoke: a permissive profile that never fails; emits
    /// `Pass` if profile evaluation completes.
    Smoke,
    /// Full: per-adapter verification of all surfaces.
    Full,
    /// Promotion: minimum-required to promote a cycle.
    Promotion {
        /// Minimum match rate in basis points (1/10000). 9500 = 95%.
        min_match_rate_bps: u32,
    },
    /// Custom: user-supplied rule set.
    Custom {
        /// Explicit rules to evaluate.
        rules: Vec<AssuranceRule>,
    },
}

impl AssuranceProfile {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            AssuranceProfile::Smoke => "smoke".to_string(),
            AssuranceProfile::Full => "full".to_string(),
            AssuranceProfile::Promotion { min_match_rate_bps } => {
                format!("promotion(min_match_rate_bps={min_match_rate_bps})")
            }
            AssuranceProfile::Custom { rules } => {
                format!("custom(rules={})", rules.len())
            }
        }
    }

    /// Get the effective rule list for this profile.
    pub fn rules(&self) -> Vec<AssuranceRule> {
        match self {
            AssuranceProfile::Smoke => vec![],
            AssuranceProfile::Full => full_profile_rules(),
            AssuranceProfile::Promotion { min_match_rate_bps } => {
                promotion_profile_rules(*min_match_rate_bps)
            }
            AssuranceProfile::Custom { rules } => rules.clone(),
        }
    }
}

fn full_profile_rules() -> Vec<AssuranceRule> {
    vec![
        AssuranceRule {
            rule_id: "fp.engine_match_rate".to_string(),
            surface: "engine/tests".to_string(),
            kind: AssuranceRuleKind::CoverageAtLeastBps { value_bps: 9500 },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        },
        AssuranceRule {
            rule_id: "fp.clippy".to_string(),
            surface: "engine/clippy".to_string(),
            kind: AssuranceRuleKind::InvariantHolds {
                invariant: "clippy_clean".to_string(),
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        },
        AssuranceRule {
            rule_id: "fp.fmt".to_string(),
            surface: "engine/fmt".to_string(),
            kind: AssuranceRuleKind::InvariantHolds {
                invariant: "fmt_clean".to_string(),
            },
            severity: AssuranceSeverity::Should,
            scope: AssuranceScope::Global,
        },
        AssuranceRule {
            rule_id: "fp.determinism".to_string(),
            surface: "engine/determinism".to_string(),
            kind: AssuranceRuleKind::DeterminismByteEqual,
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        },
        AssuranceRule {
            rule_id: "fp.receipt".to_string(),
            surface: "engine/receipt".to_string(),
            kind: AssuranceRuleKind::InvariantHolds {
                invariant: "receipt_present".to_string(),
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        },
    ]
}

fn promotion_profile_rules(min_match_rate_bps: u32) -> Vec<AssuranceRule> {
    vec![
        AssuranceRule {
            rule_id: "pp.engine_match_rate".to_string(),
            surface: "engine/tests".to_string(),
            kind: AssuranceRuleKind::CoverageAtLeastBps {
                value_bps: min_match_rate_bps,
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        },
        AssuranceRule {
            rule_id: "pp.clippy".to_string(),
            surface: "engine/clippy".to_string(),
            kind: AssuranceRuleKind::InvariantHolds {
                invariant: "clippy_clean".to_string(),
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        },
        AssuranceRule {
            rule_id: "pp.receipt".to_string(),
            surface: "engine/receipt".to_string(),
            kind: AssuranceRuleKind::InvariantHolds {
                invariant: "receipt_present".to_string(),
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        },
    ]
}

// ── AssuranceRule ────────────────────────────────────────────────────────

/// A single assurance rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssuranceRule {
    /// Deterministic rule id.
    pub rule_id: String,
    /// Surface this rule applies to (e.g. `"engine/strategy_comparison"`).
    pub surface: String,
    /// The rule kind.
    pub kind: AssuranceRuleKind,
    /// Severity.
    pub severity: AssuranceSeverity,
    /// Scope.
    pub scope: AssuranceScope,
}

// ── AssuranceRuleKind ────────────────────────────────────────────────────

/// Closed-set rule kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AssuranceRuleKind {
    /// Engine test pass rate must be at least `value_bps` of
    /// `count`.
    CoverageAtLeastBps {
        /// Threshold in basis points.
        value_bps: u32,
    },
    /// A named invariant must hold. The invariant name is matched
    /// against evidence kinds.
    InvariantHolds {
        /// Invariant name (e.g. "clippy_clean").
        invariant: String,
    },
    /// The system is byte-deterministic.
    DeterminismByteEqual,
    /// A replay proof exists for `cycle`.
    ReplayExistsFor {
        /// Cycle id.
        cycle: String,
    },
    /// A receipt hash matches the expected value.
    ReceiptHashMatches {
        /// Expected SHA-256 hex string.
        expected_sha256: String,
    },
}

// ── AssuranceSeverity ────────────────────────────────────────────────────

/// Closed-set severities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AssuranceSeverity {
    /// Failure ⇒ profile `Fail`.
    Must,
    /// Failure ⇒ profile `Degraded`.
    Should,
    /// Failure ⇒ profile `Pass` (informational).
    Info,
}

// ── AssuranceScope ───────────────────────────────────────────────────────

/// Closed-set scopes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AssuranceScope {
    /// Cycle-scoped.
    Cycle {
        /// Cycle id.
        cycle_id: String,
    },
    /// Module-scoped.
    Module {
        /// Module name.
        module: String,
    },
    /// Global scope.
    Global,
}

// ── EvidenceTag ──────────────────────────────────────────────────────────

/// Closed-set evidence tags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EvidenceTag {
    /// Engine test counts.
    EngineTest {
        /// Total tests.
        count: usize,
        /// Passed tests.
        passed: usize,
    },
    /// Clippy clean (`-D warnings`).
    ClippyClean,
    /// `cargo fmt --check` clean.
    FmtClean,
    /// Replay proof emitted for a cycle.
    ReplayProof {
        /// Cycle id.
        cyc_id: String,
        /// Number of attempts recorded in the proof.
        attempts: usize,
    },
    /// Receipt hash present.
    Receipt {
        /// SHA-256 hex string.
        sha256: String,
    },
    /// Schema version stable since last release.
    SchemaStable,
    /// Byte-determinism proof captured.
    DeterminismByteEqual,
    /// Bridge between authority paths closed.
    BridgeClosed,
    /// Manual override (emergency escape hatch).
    ManualOverride {
        /// Actor id.
        actor: String,
        /// Reason.
        reason: String,
    },
}

impl EvidenceTag {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            EvidenceTag::EngineTest { count, passed } => {
                format!("engine_test(passed={passed}/count={count})")
            }
            EvidenceTag::ClippyClean => "clippy_clean".to_string(),
            EvidenceTag::FmtClean => "fmt_clean".to_string(),
            EvidenceTag::ReplayProof { cyc_id, attempts } => {
                format!("replay_proof(cycle={cyc_id},attempts={attempts})")
            }
            EvidenceTag::Receipt { sha256 } => format!("receipt(sha256={sha256})"),
            EvidenceTag::SchemaStable => "schema_stable".to_string(),
            EvidenceTag::DeterminismByteEqual => "determinism_byte_equal".to_string(),
            EvidenceTag::BridgeClosed => "bridge_closed".to_string(),
            EvidenceTag::ManualOverride { actor, reason } => {
                format!("manual_override(actor={actor},reason={reason})")
            }
        }
    }
}

// ── AssuranceEvidence ────────────────────────────────────────────────────

/// A piece of evidence collected from the system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssuranceEvidence {
    /// The evidence tag.
    pub tag: EvidenceTag,
    /// RFC-3339 timestamp supplied by the caller.
    pub collected_at: String,
    /// Source of the evidence (e.g. `"cargo test -p sddk-engine"`).
    pub source: String,
}

// ── AssuranceRuleResult ──────────────────────────────────────────────────

/// Result of evaluating a single rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssuranceRuleResult {
    /// Rule id.
    pub rule_id: String,
    /// Whether the rule passed.
    pub passed: bool,
    /// Severity.
    pub severity: AssuranceSeverity,
    /// Human-readable detail.
    pub detail: String,
}

// ── AssuranceStatus ──────────────────────────────────────────────────────

/// Closed-set verdict status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AssuranceStatus {
    /// All `Must` rules passed.
    Pass,
    /// All `Must` rules passed; some `Should` rules failed.
    Degraded {
        /// Reason.
        reason: String,
    },
    /// At least one `Must` rule failed.
    Fail {
        /// Reason.
        reason: String,
    },
}

// ── AssuranceVerdict ─────────────────────────────────────────────────────

/// Verdict emitted by the assurance engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssuranceVerdict {
    /// Profile used.
    pub profile: AssuranceProfile,
    /// Final status.
    pub status: AssuranceStatus,
    /// Per-rule results (sorted by `rule_id`).
    pub rule_results: Vec<AssuranceRuleResult>,
    /// Number of `Must` rules that failed.
    pub failing_rules: usize,
    /// Number of `Should` rules that failed.
    pub should_rules: usize,
    /// RFC-3339 timestamp supplied by the caller.
    pub evaluated_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl AssuranceVerdict {
    /// Constant schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── AssuranceEngine ──────────────────────────────────────────────────────

/// Pure assurance engine.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AssuranceEngine {
    profile: AssuranceProfile,
}

impl AssuranceEngine {
    /// Construct an engine for the given profile.
    pub fn new(profile: AssuranceProfile) -> Self {
        Self { profile }
    }

    /// Profile in use.
    pub fn profile(&self) -> &AssuranceProfile {
        &self.profile
    }

    /// Evaluate the supplied evidence against the engine's
    /// profile.
    pub fn evaluate(
        &self,
        evidence: &[AssuranceEvidence],
        evaluated_at: String,
    ) -> AssuranceVerdict {
        let rules = self.profile.rules();
        let mut rule_results: Vec<AssuranceRuleResult> =
            rules.iter().map(|r| evaluate_rule(r, evidence)).collect();
        // Deterministic ordering by rule_id.
        rule_results.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));

        let mut failing = 0_usize;
        let mut should = 0_usize;
        let mut first_must_reason: Option<String> = None;
        let mut first_should_reason: Option<String> = None;
        for rr in &rule_results {
            if rr.passed {
                continue;
            }
            match rr.severity {
                AssuranceSeverity::Must => {
                    failing += 1;
                    if first_must_reason.is_none() {
                        first_must_reason = Some(rr.detail.clone());
                    }
                }
                AssuranceSeverity::Should => {
                    should += 1;
                    if first_should_reason.is_none() {
                        first_should_reason = Some(rr.detail.clone());
                    }
                }
                AssuranceSeverity::Info => {}
            }
        }

        let status = if failing > 0 {
            AssuranceStatus::Fail {
                reason: first_must_reason
                    .unwrap_or_else(|| format!("{failing} Must rule(s) failed")),
            }
        } else if should > 0 {
            AssuranceStatus::Degraded {
                reason: first_should_reason
                    .unwrap_or_else(|| format!("{should} Should rule(s) failed")),
            }
        } else {
            AssuranceStatus::Pass
        };

        AssuranceVerdict {
            profile: self.profile.clone(),
            status,
            rule_results,
            failing_rules: failing,
            should_rules: should,
            evaluated_at,
            schema_version: AssuranceVerdict::SCHEMA_VERSION,
        }
    }
}

// ── Rule evaluation ──────────────────────────────────────────────────────

/// Evaluate a single rule against the evidence collection.
fn evaluate_rule(rule: &AssuranceRule, evidence: &[AssuranceEvidence]) -> AssuranceRuleResult {
    let passed = match &rule.kind {
        AssuranceRuleKind::CoverageAtLeastBps { value_bps } => {
            coverage_passes(evidence, *value_bps)
        }
        AssuranceRuleKind::InvariantHolds { invariant } => invariant_holds(evidence, invariant),
        AssuranceRuleKind::DeterminismByteEqual => evidence
            .iter()
            .any(|e| matches!(e.tag, EvidenceTag::DeterminismByteEqual)),
        AssuranceRuleKind::ReplayExistsFor { cycle } => evidence.iter().any(|e| {
            if let EvidenceTag::ReplayProof { cyc_id, .. } = &e.tag {
                cyc_id == cycle
            } else {
                false
            }
        }),
        AssuranceRuleKind::ReceiptHashMatches { expected_sha256 } => evidence.iter().any(|e| {
            if let EvidenceTag::Receipt { sha256 } = &e.tag {
                sha256 == expected_sha256
            } else {
                false
            }
        }),
    };
    let detail = if passed {
        format!("rule {} passed", rule.rule_id)
    } else {
        format!("rule {} failed", rule.rule_id)
    };
    AssuranceRuleResult {
        rule_id: rule.rule_id.clone(),
        passed,
        severity: rule.severity,
        detail,
    }
}

fn coverage_passes(evidence: &[AssuranceEvidence], value_bps: u32) -> bool {
    let mut by_source: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for e in evidence {
        if let EvidenceTag::EngineTest { count, passed } = e.tag {
            let entry = by_source.entry(e.source.clone()).or_insert((0, 0));
            entry.0 += count;
            entry.1 += passed;
        }
    }
    for (_source, (count, passed)) in by_source {
        if count == 0 {
            return false;
        }
        let rate = (passed as u128 * 10_000) / (count as u128);
        if (rate as u32) < value_bps {
            return false;
        }
    }
    true
}

fn invariant_holds(evidence: &[AssuranceEvidence], invariant: &str) -> bool {
    match invariant {
        "clippy_clean" => evidence
            .iter()
            .any(|e| matches!(e.tag, EvidenceTag::ClippyClean)),
        "fmt_clean" => evidence
            .iter()
            .any(|e| matches!(e.tag, EvidenceTag::FmtClean)),
        "receipt_present" => evidence
            .iter()
            .any(|e| matches!(e.tag, EvidenceTag::Receipt { .. })),
        "schema_stable" => evidence
            .iter()
            .any(|e| matches!(e.tag, EvidenceTag::SchemaStable)),
        "bridge_closed" => evidence
            .iter()
            .any(|e| matches!(e.tag, EvidenceTag::BridgeClosed)),
        _ => false,
    }
}

// ── Audit guards ─────────────────────────────────────────────────────────

#[allow(unused)]
const ASSURANCE_PROFILE_VARIANT_LIST: &[AssuranceProfile] = &[
    AssuranceProfile::Smoke,
    AssuranceProfile::Full,
    AssuranceProfile::Promotion {
        min_match_rate_bps: 0,
    },
    AssuranceProfile::Custom { rules: vec![] },
];

#[allow(unused)]
const ASSURANCE_RULE_KIND_VARIANT_LIST: &[AssuranceRuleKind] = &[
    AssuranceRuleKind::CoverageAtLeastBps { value_bps: 0 },
    AssuranceRuleKind::InvariantHolds {
        invariant: String::new(),
    },
    AssuranceRuleKind::DeterminismByteEqual,
    AssuranceRuleKind::ReplayExistsFor {
        cycle: String::new(),
    },
    AssuranceRuleKind::ReceiptHashMatches {
        expected_sha256: String::new(),
    },
];

#[allow(unused)]
const ASSURANCE_SEVERITY_VARIANT_LIST: &[AssuranceSeverity] = &[
    AssuranceSeverity::Must,
    AssuranceSeverity::Should,
    AssuranceSeverity::Info,
];

#[allow(unused)]
const ASSURANCE_SCOPE_VARIANT_LIST: &[AssuranceScope] = &[
    AssuranceScope::Cycle {
        cycle_id: String::new(),
    },
    AssuranceScope::Module {
        module: String::new(),
    },
    AssuranceScope::Global,
];

#[allow(unused)]
const EVIDENCE_TAG_VARIANT_LIST: &[EvidenceTag] = &[
    EvidenceTag::EngineTest {
        count: 0,
        passed: 0,
    },
    EvidenceTag::ClippyClean,
    EvidenceTag::FmtClean,
    EvidenceTag::ReplayProof {
        cyc_id: String::new(),
        attempts: 0,
    },
    EvidenceTag::Receipt {
        sha256: String::new(),
    },
    EvidenceTag::SchemaStable,
    EvidenceTag::DeterminismByteEqual,
    EvidenceTag::BridgeClosed,
    EvidenceTag::ManualOverride {
        actor: String::new(),
        reason: String::new(),
    },
];

#[allow(unused)]
const ASSURANCE_STATUS_VARIANT_LIST: &[AssuranceStatus] = &[
    AssuranceStatus::Pass,
    AssuranceStatus::Degraded {
        reason: String::new(),
    },
    AssuranceStatus::Fail {
        reason: String::new(),
    },
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(tag: EvidenceTag) -> AssuranceEvidence {
        AssuranceEvidence {
            tag,
            collected_at: "t".to_string(),
            source: "src".to_string(),
        }
    }

    // ── S-1: Promotion passes when match rate ≥ threshold ───────────────

    #[test]
    fn s1_promotion_passes_at_threshold() {
        let engine = AssuranceEngine::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let evidence = vec![
            ev(EvidenceTag::EngineTest {
                count: 100,
                passed: 97,
            }),
            ev(EvidenceTag::ClippyClean),
            ev(EvidenceTag::Receipt {
                sha256: "abc".to_string(),
            }),
        ];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        assert_eq!(verdict.status, AssuranceStatus::Pass);
        assert_eq!(verdict.failing_rules, 0);
    }

    // ── S-2: Promotion fails when match rate < threshold ────────────────

    #[test]
    fn s2_promotion_fails_below_threshold() {
        let engine = AssuranceEngine::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let evidence = vec![ev(EvidenceTag::EngineTest {
            count: 100,
            passed: 90,
        })];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        assert!(matches!(verdict.status, AssuranceStatus::Fail { .. }));
        assert!(verdict.failing_rules >= 1);
    }

    // ── S-3: Smoke is permissive ────────────────────────────────────────

    #[test]
    fn s3_smoke_is_permissive() {
        let engine = AssuranceEngine::new(AssuranceProfile::Smoke);
        let verdict = engine.evaluate(&[], "t".to_string());
        assert_eq!(verdict.status, AssuranceStatus::Pass);
    }

    // ── S-4: Full fails without clippy evidence ─────────────────────────

    #[test]
    fn s4_full_fails_without_clippy() {
        let engine = AssuranceEngine::new(AssuranceProfile::Full);
        let verdict = engine.evaluate(&[], "t".to_string());
        assert!(matches!(verdict.status, AssuranceStatus::Fail { .. }));
    }

    #[test]
    fn s4_full_passes_with_all_evidence() {
        let engine = AssuranceEngine::new(AssuranceProfile::Full);
        let evidence = vec![
            ev(EvidenceTag::EngineTest {
                count: 100,
                passed: 99,
            }),
            ev(EvidenceTag::ClippyClean),
            ev(EvidenceTag::FmtClean),
            ev(EvidenceTag::DeterminismByteEqual),
            ev(EvidenceTag::Receipt {
                sha256: "abc".to_string(),
            }),
        ];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        assert_eq!(verdict.status, AssuranceStatus::Pass);
    }

    // ── S-5: Should rules degrade, do not fail ──────────────────────────

    #[test]
    fn s5_should_degrades_not_fails() {
        let rules = vec![AssuranceRule {
            rule_id: "custom.should".to_string(),
            surface: "x".to_string(),
            kind: AssuranceRuleKind::InvariantHolds {
                invariant: "fmt_clean".to_string(),
            },
            severity: AssuranceSeverity::Should,
            scope: AssuranceScope::Global,
        }];
        let engine = AssuranceEngine::new(AssuranceProfile::Custom { rules });
        let verdict = engine.evaluate(&[], "t".to_string());
        assert!(matches!(verdict.status, AssuranceStatus::Degraded { .. }));
        assert_eq!(verdict.failing_rules, 0);
        assert_eq!(verdict.should_rules, 1);
    }

    // ── S-6: Multiple evidence tags combine ─────────────────────────────

    #[test]
    fn s6_multiple_evidence_tags_combine() {
        let engine = AssuranceEngine::new(AssuranceProfile::Promotion {
            min_match_rate_bps: 9500,
        });
        let evidence = vec![
            ev(EvidenceTag::EngineTest {
                count: 100,
                passed: 97,
            }),
            ev(EvidenceTag::Receipt {
                sha256: "abc".to_string(),
            }),
            ev(EvidenceTag::ClippyClean),
        ];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        // Should have at least two rule results (engine + receipt +
        // clippy).
        assert!(verdict.rule_results.len() >= 2);
        assert_eq!(verdict.status, AssuranceStatus::Pass);
    }

    // ── S-7: Determinism ────────────────────────────────────────────────

    #[test]
    fn s7_determinism_byte_equal() {
        let engine = AssuranceEngine::new(AssuranceProfile::Full);
        let evidence = vec![
            ev(EvidenceTag::EngineTest {
                count: 100,
                passed: 99,
            }),
            ev(EvidenceTag::ClippyClean),
            ev(EvidenceTag::FmtClean),
            ev(EvidenceTag::DeterminismByteEqual),
            ev(EvidenceTag::Receipt {
                sha256: "abc".to_string(),
            }),
        ];
        let a = engine.evaluate(&evidence, "t".to_string());
        let b = engine.evaluate(&evidence, "t".to_string());
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── S-8: Rule results sorted by rule_id ─────────────────────────────

    #[test]
    fn s8_rule_results_sorted_by_rule_id() {
        let rules = vec![
            AssuranceRule {
                rule_id: "r3".to_string(),
                surface: "x".to_string(),
                kind: AssuranceRuleKind::DeterminismByteEqual,
                severity: AssuranceSeverity::Info,
                scope: AssuranceScope::Global,
            },
            AssuranceRule {
                rule_id: "r1".to_string(),
                surface: "x".to_string(),
                kind: AssuranceRuleKind::DeterminismByteEqual,
                severity: AssuranceSeverity::Info,
                scope: AssuranceScope::Global,
            },
            AssuranceRule {
                rule_id: "r2".to_string(),
                surface: "x".to_string(),
                kind: AssuranceRuleKind::DeterminismByteEqual,
                severity: AssuranceSeverity::Info,
                scope: AssuranceScope::Global,
            },
        ];
        let engine = AssuranceEngine::new(AssuranceProfile::Custom { rules });
        let evidence = vec![ev(EvidenceTag::DeterminismByteEqual)];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        assert_eq!(verdict.rule_results[0].rule_id, "r1");
        assert_eq!(verdict.rule_results[1].rule_id, "r2");
        assert_eq!(verdict.rule_results[2].rule_id, "r3");
    }

    // ── Extra: profile ids ──────────────────────────────────────────────

    #[test]
    fn profile_ids() {
        assert_eq!(AssuranceProfile::Smoke.id(), "smoke".to_string());
        assert_eq!(AssuranceProfile::Full.id(), "full".to_string());
        assert_eq!(
            AssuranceProfile::Promotion {
                min_match_rate_bps: 9500
            }
            .id(),
            "promotion(min_match_rate_bps=9500)".to_string()
        );
        assert_eq!(
            AssuranceProfile::Custom { rules: vec![] }.id(),
            "custom(rules=0)".to_string()
        );
    }

    // ── Extra: ReceiptHashMatches positive and negative ─────────────────

    #[test]
    fn receipt_hash_matches_positive() {
        let rules = vec![AssuranceRule {
            rule_id: "rh".to_string(),
            surface: "x".to_string(),
            kind: AssuranceRuleKind::ReceiptHashMatches {
                expected_sha256: "abc".to_string(),
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        }];
        let engine = AssuranceEngine::new(AssuranceProfile::Custom { rules });
        let evidence = vec![ev(EvidenceTag::Receipt {
            sha256: "abc".to_string(),
        })];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        assert_eq!(verdict.status, AssuranceStatus::Pass);
    }

    #[test]
    fn receipt_hash_matches_negative() {
        let rules = vec![AssuranceRule {
            rule_id: "rh".to_string(),
            surface: "x".to_string(),
            kind: AssuranceRuleKind::ReceiptHashMatches {
                expected_sha256: "abc".to_string(),
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Global,
        }];
        let engine = AssuranceEngine::new(AssuranceProfile::Custom { rules });
        let evidence = vec![ev(EvidenceTag::Receipt {
            sha256: "def".to_string(),
        })];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        assert!(matches!(verdict.status, AssuranceStatus::Fail { .. }));
    }

    // ── Extra: ReplayExistsFor positive ─────────────────────────────────

    #[test]
    fn replay_exists_for() {
        let rules = vec![AssuranceRule {
            rule_id: "re".to_string(),
            surface: "x".to_string(),
            kind: AssuranceRuleKind::ReplayExistsFor {
                cycle: "C1".to_string(),
            },
            severity: AssuranceSeverity::Must,
            scope: AssuranceScope::Cycle {
                cycle_id: "C1".to_string(),
            },
        }];
        let engine = AssuranceEngine::new(AssuranceProfile::Custom { rules });
        let evidence = vec![ev(EvidenceTag::ReplayProof {
            cyc_id: "C1".to_string(),
            attempts: 7,
        })];
        let verdict = engine.evaluate(&evidence, "t".to_string());
        assert_eq!(verdict.status, AssuranceStatus::Pass);
    }

    // ── Extra: EvidenceTag id stability ─────────────────────────────────

    #[test]
    fn evidence_tag_id() {
        assert_eq!(EvidenceTag::ClippyClean.id(), "clippy_clean".to_string());
        assert_eq!(EvidenceTag::FmtClean.id(), "fmt_clean".to_string());
        assert_eq!(
            EvidenceTag::Receipt {
                sha256: "h".to_string()
            }
            .id(),
            "receipt(sha256=h)".to_string()
        );
    }
}
