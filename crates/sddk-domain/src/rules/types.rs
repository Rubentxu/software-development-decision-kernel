//! Domain types for the architecture-rule registry.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Greppable enforcement level for an architecture rule.
///
/// - `Error` violations cause CI to fail immediately.
/// - `Warning` violations are reported but never block CI.
/// - `WarningThenRatchet` violations start as warnings but become errors once the
///   ratchet-condition is met (e.g. "new code added after this rule may not violate it").
///   The intent is to freeze violations in place while allowing existing violations to
///   be grandfathered in during a transition period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
    /// Rule violation causes a hard CI failure; no exceptions allowed in automated runs.
    Error,
    /// Rule violation is logged and reported but never blocks CI.
    Warning,
    /// Currently a warning; becomes an error when the ratchet condition activates
    /// (e.g. all new code must comply while existing violations are grandfathered).
    WarningThenRatchet,
}

crate::assert_variant_count_eq!(
    RuleSeverity,
    3,
    [
        RuleSeverity::Error,
        RuleSeverity::Warning,
        RuleSeverity::WarningThenRatchet,
    ]
);

/// Surface analysed by the rule evaluator.
///
/// Each variant identifies the artifact and analysis method used to detect violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleTarget {
    /// Evaluates the directed dependency graph between crates/packages.
    /// Violations indicate unwanted architectural coupling (e.g. engine → storage).
    DependencyGraph,
    /// Evaluates source-level imports and function/call-graph edges.
    /// Violations indicate runtime-level architectural breaches (e.g. CLI importing persistence).
    SourceImportsAndCalls,
    /// Evaluates `pack.toml` / `pack.yaml` declarations.
    /// Violations indicate incomplete or incorrect pack dependency metadata.
    PackManifest,
    /// Evaluates imports of `capabilities/` namespace from other layers.
    /// Violations indicate cross-cutting concerns leaking into layers that must not own them.
    CapabilityImports,
}

/// Outcome of a single rule evaluation against a baseline.
///
/// - `Pass`: rule was checked and no violation was found.
/// - `Fail`: rule was checked and a violation was found.
/// - `Waived`: the rule applies but a human has explicitly granted an exception
///   (recorded in the `Waiver` registry with a `granted_until_sha`).
/// - `NotApplicable`: the rule does not apply to this context
///   (e.g. the target surface is absent, or a waiver has expired).
///   Unlike `Waived`, no human exception was granted — the rule simply doesn't
///   apply at the evaluated baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    /// Rule was evaluated and no violation was detected at the baseline.
    Pass,
    /// Rule was evaluated and a violation was detected at the baseline.
    Fail,
    /// A waiver is active for this rule at the evaluated baseline;
    /// the violation exists but is explicitly excepted until `granted_until_sha`.
    Waived,
    /// The rule does not apply to this context (target absent, wrong phase,
    /// or an expired waiver that no longer shields the violation).
    NotApplicable,
}

/// Mechanism used by the rule evaluator to determine violations.
///
/// - `Heuristic`: pattern-matching or approximate inference over source text or graph shape.
///   Lower precision; acceptable for early-phase or cross-language checks.
/// - `Ast`: precise structural analysis over a parsed Abstract Syntax Tree.
///   High precision for language-specific rules (e.g. no `Rc<...>` in certain modules).
/// - `Schema`: validated against a formal JSON Schema or equivalent declarative contract.
///   Used when the rule is expressed as a schema assertion rather than imperative code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluatorKind {
    /// Approximate pattern-match or heuristic; fast but may produce false positives.
    Heuristic,
    /// Precise AST-level structural analysis for a specific language.
    Ast,
    /// Evaluation against a formal JSON Schema or equivalent schema document.
    Schema,
}

/// A named, enforceable architectural constraint parsed from `architecture-rules.yaml`.
///
/// `id` is the stable identifier used in waiver references and audit trails.
/// `rule` is the evaluator-specific expression that is checked (e.g. a graph-pattern,
/// a regex, a schema clause). `desired_state` optionally records the agreed architectural
/// intent and is used in human-readable diffs when the rule is violated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureRule {
    /// Stable identifier for this rule (e.g. `"ARCH001"`). Used in waivers,
    /// evaluation records, and as the canonical reference in audit trails.
    pub id: String,
    /// Enforcement level — controls whether violations fail CI, warn, or ratchet.
    pub severity: RuleSeverity,
    /// Evaluator-specific rule expression (graph pattern, regex, schema, etc.).
    /// Interpretation depends on `target` and `evaluator_kind`.
    pub rule: String,
    /// Which codebase surface this rule evaluates.
    pub target: RuleTarget,
    /// Optional human-readable statement of the desired architectural state.
    /// Used in diff/review output to explain *why* this rule exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_state: Option<String>,
    /// File globs this rule applies to (used by source-scanning evaluators like ARCH008).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
}

/// A time-limited, human-granted exception to an architecture rule.
///
/// Waivers are recorded in `architecture-rules.yaml` and evaluated against the
/// `head_anchor` of the baseline. A waiver is active only when the baseline's
/// head anchor is less than or equal to `granted_until_sha`; once the baseline
/// advances past that SHA the waiver expires and the rule resumes its normal status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiver {
    /// Unique identifier for this waiver (e.g. `"WV-0001"`).
    pub id: String,
    /// ID of the `ArchitectureRule` this waiver excepts.
    pub rule_id: String,
    /// Human-authored justification for granting this waiver
    /// (visible in audit reports and CI output).
    pub reason: String,
    /// Git SHA at which this waiver expires. When a baseline's head anchor
    /// is later than this SHA the waiver is ignored and the rule applies normally.
    pub granted_until_sha: String,
    /// Username or identifier of the person who granted this waiver.
    pub granted_by: String,
    /// ISO-8601 timestamp recording when the waiver was issued.
    pub granted_at: String,
    /// Optional per-rule file scope for this waiver.
    #[serde(default)]
    pub scope: BTreeMap<String, Vec<String>>,
}

/// Immutable reference to a captured baseline state used for rule evaluation.
///
/// The combination of `schema_version`, `head_anchor`, and `sha256` fully
/// characterises the evaluated codebase snapshot, enabling deterministic replay
/// and reproducible comparison between evaluation runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineRef {
    /// Schema version of the baseline capture format.
    pub schema_version: String,
    /// Human-readable git ref pointing to the evaluated commit
    /// (branch name, tag, or short SHA — used in waiver expiry checks).
    pub head_anchor: String,
    /// SHA-256 digest of the complete source tree at `head_anchor`.
    /// Used to detect whether the baseline has been altered since capture.
    pub sha256: String,
    /// Optional SDDK cycle identifier that produced this baseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<String>,
    /// ISO-8601 wall-clock timestamp at which the baseline was captured.
    pub captured_at: String,
}

/// Result of evaluating a single `ArchitectureRule` against a `BaselineRef`.
///
/// Carries everything needed to reproduce, audit, or render the evaluation result:
/// the rule checked, the outcome, the evaluator metadata, and a reference to an
/// active waiver if the rule was waived at the evaluated baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEvaluation {
    /// ID of the evaluated rule (matches `ArchitectureRule.id`).
    pub rule_id: String,
    /// Outcome of the evaluation (Pass, Fail, Waived, or NotApplicable).
    pub status: RuleStatus,
    /// Arbitrary JSON payload with evaluator-specific observation details
    /// (e.g. list of violating edges, detected pattern, schema validation errors).
    pub observed: serde_json::Value,
    /// SHA-256 of the baseline at which this evaluation was performed.
    pub baseline_sha256: String,
    /// ISO-8601 wall-clock timestamp when the evaluation was run.
    pub evaluated_at: String,
    /// Identifier for the evaluator binary that ran this check
    /// (e.g. `"sddk-rules-cli@0.1.0"`).
    pub evaluated_by: String,
    /// If non-None, references the active `Waiver.id` that caused the `Waived` status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver_id: Option<String>,
    /// Technique used by the evaluator to determine the result.
    pub evaluator_kind: EvaluatorKind,
    /// Version string of the evaluator binary (allows replay with the same tool).
    pub evaluator_version: String,
    /// Optional provenance note, typically used when an evaluator defers detailed
    /// analysis to a future work item (e.g. `"deferred to WI-4"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
}
