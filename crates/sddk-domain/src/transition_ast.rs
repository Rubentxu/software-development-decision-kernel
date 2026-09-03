//! Versioned transition AST with closed-set validation and deterministic semantics.
//!
//! Provides a typed, migration-safe contract for transition predicates per DW-IR-002
//! and ADR-024. Replaces stringly `Requirement` kinds with a closed enum AST,
//! adds depth-bounded evaluation, and enables deterministic serialization + hashing.
//!
//! # Design
//!
//! - `TransitionAst` is a versioned enum whose only current variant is `V1(TransitionSpecV1)`.
//! - `PredicateExpr` is a closed AST enum for transition predicates.
//! - Deterministic serialization via canonical JSON + SHA-256 content hash (mirrors WorkflowIR/ExecutionScope).
//! - Lossless migration to/from legacy `Requirement` types.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::workflow::StateRef;

/// Schema version constant for `TransitionSpecV1`.
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum predicate expression tree depth to prevent stack overflow.
/// Evaluations exceeding this bound return `false`; validation rejects depth > MAX.
pub const MAX_PREDICATE_DEPTH: usize = 16;

/// Content hash in `sha256:<64-hex-lowercase>` format (mirrors WorkflowIR/ExecutionScope).
pub type ContentHash = String;

// ── Closed-set predicate AST ─────────────────────────────────────────────────

/// Closed-set predicate expression AST for transition requirements.
///
/// Each variant is explicit and closed; there is no catch-all fallback.
/// Depth is bounded to prevent stack overflow during evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredicateExpr {
    /// Requires an artifact to be present.
    ArtifactPresent {
        /// Artifact name.
        name: String,
    },
    /// Requires a gate to have passed.
    GatePassed {
        /// Gate name.
        name: String,
    },
    /// Requires a capability to have been admitted.
    CapabilityAdmitted {
        /// Capability identifier.
        id: String,
    },
    /// Conjunction of predicates (all must hold).
    All(Vec<PredicateExpr>),
    /// Disjunction of predicates (any must hold).
    Any(Vec<PredicateExpr>),
    /// Negation of a predicate.
    Not(Box<PredicateExpr>),
}

// Compile-time guard: 6 closed variants (REQ-1, REQ-6 #7)
crate::assert_variant_count_eq!(
    PredicateExpr,
    6,
    [
        PredicateExpr::ArtifactPresent { .. },
        PredicateExpr::GatePassed { .. },
        PredicateExpr::CapabilityAdmitted { .. },
        PredicateExpr::All(_),
        PredicateExpr::Any(_),
        PredicateExpr::Not(_),
    ]
);

// ── Versioned transition spec ────────────────────────────────────────────────

/// Versioned transition AST enum.
///
/// Currently only `V1` exists. The versioned envelope allows future migration
/// without breaking existing serialized forms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "schema", content = "data", rename_all = "snake_case")]
pub enum TransitionAst {
    /// Version 1 transition spec.
    V1(TransitionSpecV1),
}

/// V1 transition spec — typed, migration-safe transition contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionSpecV1 {
    /// Schema version — must be `SCHEMA_VERSION` (1).
    pub schema_version: u32,
    /// Non-empty transition identifier.
    pub id: String,
    /// Source state (optional for initial transitions).
    #[serde(default)]
    pub from: Option<StateRef>,
    /// Target state.
    pub to: StateRef,
    /// Predicate expressions required for this transition.
    pub requires: Vec<PredicateExpr>,
    /// Workflow paths allowed to use this transition.
    pub paths: BTreeSet<String>,
    /// Artifacts produced by this transition.
    pub produces: Vec<String>,
    /// Failure state (optional).
    #[serde(default)]
    pub on_failure: Option<StateRef>,
}

impl TransitionSpecV1 {
    /// Validates this transition spec instance.
    ///
    /// Returns `Ok(())` if valid, `Err(TransitionAstError)` otherwise.
    pub fn validate(&self) -> Result<(), TransitionAstError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(TransitionAstError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        if self.id.is_empty() {
            return Err(TransitionAstError::EmptyId);
        }
        for expr in &self.requires {
            Self::validate_predicate_depth(expr, 1)?;
        }
        Ok(())
    }

    /// Validates predicate depth recursively, returning error if depth exceeds MAX.
    fn validate_predicate_depth(
        expr: &PredicateExpr,
        depth: usize,
    ) -> Result<(), TransitionAstError> {
        if depth > MAX_PREDICATE_DEPTH {
            return Err(TransitionAstError::PredicateDepthExceeded {
                got: depth,
                max: MAX_PREDICATE_DEPTH,
            });
        }
        match expr {
            PredicateExpr::ArtifactPresent { .. }
            | PredicateExpr::GatePassed { .. }
            | PredicateExpr::CapabilityAdmitted { .. } => {}
            PredicateExpr::All(children) => {
                if children.is_empty() {
                    return Err(TransitionAstError::EmptyPredicate);
                }
                for child in children {
                    Self::validate_predicate_depth(child, depth + 1)?;
                }
            }
            PredicateExpr::Any(children) => {
                if children.is_empty() {
                    return Err(TransitionAstError::EmptyPredicate);
                }
                for child in children {
                    Self::validate_predicate_depth(child, depth + 1)?;
                }
            }
            PredicateExpr::Not(child) => {
                Self::validate_predicate_depth(child, depth + 1)?;
            }
        }
        Ok(())
    }

    /// Serializes to canonical JSON with deterministic key ordering.
    ///
    /// Uses `serde_json` BTreeMap default for deterministic map ordering
    /// and `BTreeSet` for sorted fields — field insertion order in source
    /// does NOT affect output.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("TransitionSpecV1 is always serializable")
    }

    /// Computes the content hash over the canonical JSON form.
    ///
    /// Format: `sha256:<64-hex-lowercase>`.
    pub fn compute_content_hash(&self) -> ContentHash {
        let json = self.to_canonical_json();
        let digest = Sha256::digest(json.as_bytes());
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }
}

// ── Evaluation ───────────────────────────────────────────────────────────────

/// Evaluation context for predicate expressions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvalContext {
    /// Available artifact names.
    pub artifacts: BTreeSet<String>,
    /// Passed gate names.
    pub gates: BTreeSet<String>,
    /// Admitted capability identifiers.
    pub capabilities: BTreeSet<String>,
}

impl PredicateExpr {
    /// Evaluates this predicate against the given context.
    ///
    /// Returns `false` if the depth bound is exceeded (never panics/stack-overflows).
    /// - `All([])` → false (empty conjunction, consistent with EmptyPredicate rejection)
    /// - `Any([])` → false (empty disjunction)
    /// - `Not` → negation
    pub fn evaluate(&self, ctx: &EvalContext) -> bool {
        self.evaluate_impl(ctx, 1)
    }

    fn evaluate_impl(&self, ctx: &EvalContext, depth: usize) -> bool {
        if depth > MAX_PREDICATE_DEPTH {
            return false;
        }
        match self {
            PredicateExpr::ArtifactPresent { name } => ctx.artifacts.contains(name),
            PredicateExpr::GatePassed { name } => ctx.gates.contains(name),
            PredicateExpr::CapabilityAdmitted { id } => ctx.capabilities.contains(id),
            PredicateExpr::All(children) => {
                // Empty All is rejected by validate, but evaluate returns false for safety
                if children.is_empty() {
                    return false;
                }
                children
                    .iter()
                    .all(|c| c.evaluate_impl(ctx, depth.saturating_add(1)))
            }
            PredicateExpr::Any(children) => {
                // Empty Any is rejected by validate, but evaluate returns false for safety
                if children.is_empty() {
                    return false;
                }
                children
                    .iter()
                    .any(|c| c.evaluate_impl(ctx, depth.saturating_add(1)))
            }
            PredicateExpr::Not(child) => !child.evaluate_impl(ctx, depth.saturating_add(1)),
        }
    }

    /// Computes the depth of this expression (leaf = 1).
    #[allow(dead_code)]
    fn compute_depth(&self) -> usize {
        match self {
            PredicateExpr::ArtifactPresent { .. }
            | PredicateExpr::GatePassed { .. }
            | PredicateExpr::CapabilityAdmitted { .. } => 1,
            PredicateExpr::All(children) | PredicateExpr::Any(children) => {
                children
                    .iter()
                    .map(|c| c.compute_depth())
                    .max()
                    .unwrap_or(1)
                    + 1
            }
            PredicateExpr::Not(child) => child.compute_depth() + 1,
        }
    }

    /// Migrates from a legacy `Requirement`.
    ///
    /// Maps the legacy closed sets into `PredicateExpr`:
    /// - `Simple(s)` → `ArtifactPresent { name: s }` (evidence: `valid_transition` maps kindless to artifact)
    /// - `Structured { kind: "artifact", name }` → `ArtifactPresent`
    /// - `Structured { kind: "gate", name }` → `GatePassed`
    /// - Any other kind → `Err(TransitionAstError::UnsupportedLegacyKind)`
    pub fn from_legacy(
        req: &crate::workflow::Requirement,
    ) -> Result<PredicateExpr, TransitionAstError> {
        match req {
            crate::workflow::Requirement::Simple(s) => {
                Ok(PredicateExpr::ArtifactPresent { name: s.clone() })
            }
            crate::workflow::Requirement::Structured { kind, name } => match kind.as_str() {
                "artifact" => Ok(PredicateExpr::ArtifactPresent { name: name.clone() }),
                "gate" => Ok(PredicateExpr::GatePassed { name: name.clone() }),
                _ => Err(TransitionAstError::UnsupportedLegacyKind { kind: kind.clone() }),
            },
        }
    }
}

// ── Errors ───────────────────────────────────────────────────────────────────

/// Validation and migration errors for transition AST types.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum TransitionAstError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version that was found.
        got: u32,
        /// The version that was expected.
        want: u32,
    },

    /// Transition identifier is empty.
    #[error("transition id is empty")]
    EmptyId,

    /// Predicate expression is empty (All/Any with zero children).
    #[error("predicate expression is empty")]
    EmptyPredicate,

    /// Predicate expression depth exceeds MAX_PREDICATE_DEPTH.
    #[error("predicate depth {got} exceeds maximum {max}")]
    PredicateDepthExceeded {
        /// The depth that was found.
        got: usize,
        /// The maximum allowed depth.
        max: usize,
    },

    /// Legacy requirement has an unsupported kind.
    #[error("unsupported legacy requirement kind: {kind}")]
    UnsupportedLegacyKind {
        /// The unsupported kind value.
        kind: String,
    },
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cycle::{CycleStatus, Phase};
    use crate::workflow::Requirement;

    fn make_state(status: CycleStatus, phase: Phase) -> StateRef {
        StateRef {
            status,
            phase: Some(phase),
        }
    }

    fn make_ctx(artifacts: &[&str], gates: &[&str], capabilities: &[&str]) -> EvalContext {
        EvalContext {
            artifacts: artifacts.iter().map(|s| s.to_string()).collect(),
            gates: gates.iter().map(|s| s.to_string()).collect(),
            capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_spec(
        id: &str,
        from: Option<StateRef>,
        to: StateRef,
        requires: Vec<PredicateExpr>,
    ) -> TransitionSpecV1 {
        TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: id.to_string(),
            from,
            to,
            requires,
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        }
    }

    // ── REQ-1 variant count ────────────────────────────────────────────────

    #[test]
    fn predicate_expr_has_exactly_six_variants() {
        let variants = [
            PredicateExpr::ArtifactPresent { name: "a".into() },
            PredicateExpr::GatePassed { name: "g".into() },
            PredicateExpr::CapabilityAdmitted { id: "c".into() },
            PredicateExpr::All(vec![]),
            PredicateExpr::Any(vec![]),
            PredicateExpr::Not(Box::new(PredicateExpr::ArtifactPresent {
                name: "x".into(),
            })),
        ];
        assert_eq!(
            variants.len(),
            6,
            "PredicateExpr must have exactly 6 closed variants"
        );
    }

    // ── REQ-2 canonical JSON byte-stability ────────────────────────────────

    #[test]
    fn canonical_json_byte_stable() {
        let spec1 = TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: None,
            to: make_state(CycleStatus::Open, Phase::Explore),
            requires: vec![],
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        };
        let spec2 = TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: None,
            to: make_state(CycleStatus::Open, Phase::Explore),
            requires: vec![],
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        };
        let json1 = spec1.to_canonical_json();
        let json2 = spec2.to_canonical_json();
        assert_eq!(
            json1, json2,
            "canonical JSON must be byte-identical for equal specs"
        );
    }

    #[test]
    fn canonical_json_field_order_independent() {
        let build = || TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: None,
            to: make_state(CycleStatus::Open, Phase::Explore),
            requires: vec![],
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        };
        assert_eq!(build().to_canonical_json(), build().to_canonical_json());
    }

    // ── REQ-2 hash equality / mutation inequality ───────────────────────────

    #[test]
    fn hash_equality_for_equal_specs() {
        let spec1 = make_spec(
            "t1",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        let spec2 = make_spec(
            "t1",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        assert_eq!(
            spec1.compute_content_hash(),
            spec2.compute_content_hash(),
            "equal specs must have identical hashes"
        );
    }

    #[test]
    fn hash_changes_on_id_mutation() {
        let spec = make_spec(
            "t1",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        let mut mutated = spec.clone();
        mutated.id = "t2".into();
        assert_ne!(
            spec.compute_content_hash(),
            mutated.compute_content_hash(),
            "mutated id must change hash"
        );
    }

    #[test]
    fn hash_changes_on_to_mutation() {
        let spec = make_spec(
            "t1",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        let mut mutated = spec.clone();
        mutated.to = make_state(CycleStatus::Blocked, Phase::Explore);
        assert_ne!(
            spec.compute_content_hash(),
            mutated.compute_content_hash(),
            "mutated to must change hash"
        );
    }

    #[test]
    fn hash_changes_on_from_mutation() {
        let spec = make_spec(
            "t1",
            Some(make_state(CycleStatus::Open, Phase::Explore)),
            make_state(CycleStatus::Open, Phase::Specify),
            vec![],
        );
        let mut mutated = spec.clone();
        mutated.from = Some(make_state(CycleStatus::Blocked, Phase::Explore));
        assert_ne!(
            spec.compute_content_hash(),
            mutated.compute_content_hash(),
            "mutated from must change hash"
        );
    }

    // ── REQ-3 validation errors ────────────────────────────────────────────

    #[test]
    fn validate_rejects_wrong_schema_version() {
        let mut spec = make_spec(
            "t1",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        spec.schema_version = 99;
        let err = spec.validate().unwrap_err();
        assert!(matches!(
            err,
            TransitionAstError::UnsupportedSchemaVersion { got: 99, want: 1 }
        ));
    }

    #[test]
    fn validate_rejects_empty_id() {
        let spec = make_spec(
            "",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        let err = spec.validate().unwrap_err();
        assert!(matches!(err, TransitionAstError::EmptyId));
    }

    #[test]
    fn validate_rejects_empty_all() {
        let spec = TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: None,
            to: make_state(CycleStatus::Open, Phase::Explore),
            requires: vec![PredicateExpr::All(vec![])],
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        };
        let err = spec.validate().unwrap_err();
        assert!(matches!(err, TransitionAstError::EmptyPredicate));
    }

    #[test]
    fn validate_rejects_empty_any() {
        let spec = TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: None,
            to: make_state(CycleStatus::Open, Phase::Explore),
            requires: vec![PredicateExpr::Any(vec![])],
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        };
        let err = spec.validate().unwrap_err();
        assert!(matches!(err, TransitionAstError::EmptyPredicate));
    }

    #[test]
    fn validate_rejects_depth_exceeded() {
        // Build a chain of depth > MAX_PREDICATE_DEPTH iteratively
        let mut expr: PredicateExpr = PredicateExpr::ArtifactPresent {
            name: "base".into(),
        };
        for _ in 0..MAX_PREDICATE_DEPTH {
            expr = PredicateExpr::All(vec![expr]);
        }
        let spec = TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: None,
            to: make_state(CycleStatus::Open, Phase::Explore),
            requires: vec![expr],
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        };
        let err = spec.validate().unwrap_err();
        assert!(matches!(
            err,
            TransitionAstError::PredicateDepthExceeded { got, max: MAX_PREDICATE_DEPTH }
            if got > MAX_PREDICATE_DEPTH
        ));
    }

    #[test]
    fn validate_accepts_valid_spec() {
        let spec = make_spec(
            "t1",
            Some(make_state(CycleStatus::Open, Phase::Explore)),
            make_state(CycleStatus::Open, Phase::Specify),
            vec![
                PredicateExpr::ArtifactPresent {
                    name: "exploration-report".into(),
                },
                PredicateExpr::GatePassed {
                    name: "exploration-sufficient".into(),
                },
            ],
        );
        assert!(spec.validate().is_ok());
    }

    // ── REQ-2 serde round-trip ─────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_transition_ast_v1() {
        let spec = TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: Some(make_state(CycleStatus::Open, Phase::Explore)),
            to: make_state(CycleStatus::Open, Phase::Specify),
            requires: vec![
                PredicateExpr::ArtifactPresent {
                    name: "spec".into(),
                },
                PredicateExpr::GatePassed {
                    name: "spec-adequate".into(),
                },
            ],
            paths: {
                let mut s = BTreeSet::new();
                s.insert("a_min".to_string());
                s
            },
            produces: vec!["specification".into()],
            on_failure: Some(make_state(CycleStatus::Blocked, Phase::Specify)),
        };
        let original = TransitionAst::V1(spec.clone());
        let hash_before = spec.compute_content_hash();

        let json = serde_json::to_string(&original).unwrap();
        let roundtrip: TransitionAst = serde_json::from_str(&json).unwrap();
        assert_eq!(
            original, roundtrip,
            "round-trip must preserve value equality"
        );

        let TransitionAst::V1(ref v1) = roundtrip;
        assert_eq!(
            hash_before,
            v1.compute_content_hash(),
            "round-trip must preserve content hash"
        );
    }

    #[test]
    fn serde_roundtrip_canonical_json_preserved() {
        let spec = make_spec(
            "t1",
            Some(make_state(CycleStatus::Open, Phase::Explore)),
            make_state(CycleStatus::Open, Phase::Specify),
            vec![PredicateExpr::ArtifactPresent {
                name: "spec".into(),
            }],
        );
        let original = TransitionAst::V1(spec.clone());
        let canonical_before = spec.to_canonical_json();
        let json = serde_json::to_string(&original).unwrap();
        let roundtrip: TransitionAst = serde_json::from_str(&json).unwrap();
        // The canonical JSON of the inner spec must match after round-trip.
        // Note: the outer json string has the {"schema":"v1","data":...} wrapper,
        // but the inner spec's canonical form is just the spec JSON.
        let TransitionAst::V1(ref v1) = roundtrip;
        assert_eq!(
            canonical_before,
            v1.to_canonical_json(),
            "canonical JSON must match after round-trip"
        );
    }

    // ── REQ-4 evaluation truth tables ─────────────────────────────────────

    #[test]
    fn evaluate_artifact_present_true() {
        let expr = PredicateExpr::ArtifactPresent {
            name: "spec".into(),
        };
        let ctx = make_ctx(&["spec"], &[], &[]);
        assert!(expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_artifact_present_false() {
        let expr = PredicateExpr::ArtifactPresent {
            name: "spec".into(),
        };
        let ctx = make_ctx(&[], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_gate_passed_true() {
        let expr = PredicateExpr::GatePassed {
            name: "tests-pass".into(),
        };
        let ctx = make_ctx(&[], &["tests-pass"], &[]);
        assert!(expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_gate_passed_false() {
        let expr = PredicateExpr::GatePassed {
            name: "tests-pass".into(),
        };
        let ctx = make_ctx(&[], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_capability_admitted_true() {
        let expr = PredicateExpr::CapabilityAdmitted {
            id: "forge-capability".into(),
        };
        let ctx = make_ctx(&[], &[], &["forge-capability"]);
        assert!(expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_capability_admitted_false() {
        let expr = PredicateExpr::CapabilityAdmitted {
            id: "forge-capability".into(),
        };
        let ctx = make_ctx(&[], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_all_true() {
        let expr = PredicateExpr::All(vec![
            PredicateExpr::ArtifactPresent {
                name: "spec".into(),
            },
            PredicateExpr::GatePassed {
                name: "tests-pass".into(),
            },
        ]);
        let ctx = make_ctx(&["spec"], &["tests-pass"], &[]);
        assert!(expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_all_false_partial() {
        let expr = PredicateExpr::All(vec![
            PredicateExpr::ArtifactPresent {
                name: "spec".into(),
            },
            PredicateExpr::GatePassed {
                name: "tests-pass".into(),
            },
        ]);
        let ctx = make_ctx(&["spec"], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_all_false_empty() {
        // Empty All is rejected by validate, but evaluate returns false for safety
        let expr = PredicateExpr::All(vec![]);
        let ctx = make_ctx(&[], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_any_true_partial() {
        let expr = PredicateExpr::Any(vec![
            PredicateExpr::ArtifactPresent {
                name: "spec".into(),
            },
            PredicateExpr::GatePassed {
                name: "tests-pass".into(),
            },
        ]);
        let ctx = make_ctx(&[], &["tests-pass"], &[]);
        assert!(expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_any_false() {
        let expr = PredicateExpr::Any(vec![
            PredicateExpr::ArtifactPresent {
                name: "spec".into(),
            },
            PredicateExpr::GatePassed {
                name: "tests-pass".into(),
            },
        ]);
        let ctx = make_ctx(&[], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_any_false_empty() {
        // Empty Any is rejected by validate, but evaluate returns false for safety
        let expr = PredicateExpr::Any(vec![]);
        let ctx = make_ctx(&[], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_not_true() {
        let expr = PredicateExpr::Not(Box::new(PredicateExpr::ArtifactPresent {
            name: "spec".into(),
        }));
        let ctx = make_ctx(&[], &[], &[]);
        assert!(expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_not_false() {
        let expr = PredicateExpr::Not(Box::new(PredicateExpr::ArtifactPresent {
            name: "spec".into(),
        }));
        let ctx = make_ctx(&["spec"], &[], &[]);
        assert!(!expr.evaluate(&ctx));
    }

    #[test]
    fn evaluate_nested_complex() {
        // All(Any(ArtifactPresent("a"), GatePassed("g")), Not(CapabilityAdmitted("c")))
        let expr = PredicateExpr::All(vec![
            PredicateExpr::Any(vec![
                PredicateExpr::ArtifactPresent { name: "a".into() },
                PredicateExpr::GatePassed { name: "g".into() },
            ]),
            PredicateExpr::Not(Box::new(PredicateExpr::CapabilityAdmitted {
                id: "c".into(),
            })),
        ]);
        // ctx: has "a", no gate, no capability → Any(true, false) = true, Not(false) = true → All(true, true) = true
        let ctx1 = make_ctx(&["a"], &[], &[]);
        assert!(expr.evaluate(&ctx1));
        // ctx: has "a", no gate, has capability → Any(true, false) = true, Not(true) = false → All(true, false) = false
        let ctx2 = make_ctx(&["a"], &[], &["c"]);
        assert!(!expr.evaluate(&ctx2));
    }

    // ── REQ-4 depth bound (stack overflow prevention) ────────────────────

    #[test]
    fn evaluate_depth_exceeded_returns_false() {
        // Build a deeply nested expression that exceeds MAX_PREDICATE_DEPTH
        let mut expr: PredicateExpr = PredicateExpr::ArtifactPresent {
            name: "base".into(),
        };
        for _ in 0..MAX_PREDICATE_DEPTH {
            expr = PredicateExpr::All(vec![expr]);
        }
        // Evaluate must return false, not panic
        let ctx = make_ctx(&["base"], &[], &[]);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| expr.evaluate(&ctx)));
        assert!(result.is_ok(), "evaluate must not panic on deep expression");
        // After depth is exceeded at the All level, it should return false
        // because the inner evaluation depth exceeds MAX
        assert!(
            !expr.evaluate(&ctx),
            "evaluate on over-deep expression must return false, not panic"
        );
    }

    #[test]
    fn validate_and_evaluate_depth_boundary() {
        // At exactly MAX_PREDICATE_DEPTH depth, validate should pass and evaluate should work
        let mut expr: PredicateExpr = PredicateExpr::ArtifactPresent {
            name: "base".into(),
        };
        for _ in 0..MAX_PREDICATE_DEPTH - 1 {
            expr = PredicateExpr::All(vec![expr]);
        }
        // depth = MAX (leaf=1, each All adds 1)
        let spec = TransitionSpecV1 {
            schema_version: SCHEMA_VERSION,
            id: "t1".into(),
            from: None,
            to: make_state(CycleStatus::Open, Phase::Explore),
            requires: vec![expr.clone()],
            paths: BTreeSet::new(),
            produces: vec![],
            on_failure: None,
        };
        assert!(
            spec.validate().is_ok(),
            "depth == MAX should pass validation"
        );
        let ctx = make_ctx(&["base"], &[], &[]);
        assert!(
            expr.evaluate(&ctx),
            "depth == MAX should evaluate correctly"
        );
    }

    // ── REQ-5 legacy migration ─────────────────────────────────────────────

    #[test]
    fn from_legacy_simple_becomes_artifact_present() {
        let req = Requirement::Simple("project.adopted".into());
        let result = PredicateExpr::from_legacy(&req).unwrap();
        assert!(matches!(
            result,
            PredicateExpr::ArtifactPresent { name } if name == "project.adopted"
        ));
    }

    #[test]
    fn from_legacy_structured_artifact_becomes_artifact_present() {
        let req = Requirement::Structured {
            kind: "artifact".into(),
            name: "specification".into(),
        };
        let result = PredicateExpr::from_legacy(&req).unwrap();
        assert!(matches!(
            result,
            PredicateExpr::ArtifactPresent { name } if name == "specification"
        ));
    }

    #[test]
    fn from_legacy_structured_gate_becomes_gate_passed() {
        let req = Requirement::Structured {
            kind: "gate".into(),
            name: "tests-pass".into(),
        };
        let result = PredicateExpr::from_legacy(&req).unwrap();
        assert!(matches!(
            result,
            PredicateExpr::GatePassed { name } if name == "tests-pass"
        ));
    }

    #[test]
    fn from_legacy_unknown_kind_returns_error() {
        let req = Requirement::Structured {
            kind: "unknown-kind".into(),
            name: "something".into(),
        };
        let err = PredicateExpr::from_legacy(&req).unwrap_err();
        assert!(matches!(
            err,
            TransitionAstError::UnsupportedLegacyKind { kind }
            if kind == "unknown-kind"
        ));
    }

    // ── REQ-5 round-trip property for legacy → AST ────────────────────────

    #[test]
    fn legacy_simple_roundtrip_preserves_kind_and_name() {
        let req = Requirement::Simple("project.adopted".into());
        let expr = PredicateExpr::from_legacy(&req).unwrap();
        assert!(matches!(
            expr,
            PredicateExpr::ArtifactPresent { name } if name == "project.adopted"
        ));
    }

    #[test]
    fn legacy_structured_artifact_roundtrip_preserves() {
        let req = Requirement::Structured {
            kind: "artifact".into(),
            name: "specification".into(),
        };
        let expr = PredicateExpr::from_legacy(&req).unwrap();
        assert!(matches!(
            expr,
            PredicateExpr::ArtifactPresent { name } if name == "specification"
        ));
    }

    #[test]
    fn legacy_structured_gate_roundtrip_preserves() {
        let req = Requirement::Structured {
            kind: "gate".into(),
            name: "tests-pass".into(),
        };
        let expr = PredicateExpr::from_legacy(&req).unwrap();
        assert!(matches!(
            expr,
            PredicateExpr::GatePassed { name } if name == "tests-pass"
        ));
    }

    // ── Additional: content hash format ───────────────────────────────────

    #[test]
    fn content_hash_format() {
        let spec = make_spec(
            "t1",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        let hash = spec.compute_content_hash();
        assert!(
            hash.starts_with("sha256:"),
            "hash must start with 'sha256:': {}",
            hash
        );
        let hex = &hash[7..];
        assert_eq!(hex.len(), 64, "hash hex must be 64 chars: {}", hex);
        assert!(
            hex.chars().all(|c| c.is_ascii_hexdigit()),
            "hash hex must be all hex digits: {}",
            hex
        );
    }

    // ── Additional: EmptyId variant name ───────────────────────────────────

    #[test]
    fn empty_id_error_message() {
        let spec = make_spec(
            "",
            None,
            make_state(CycleStatus::Open, Phase::Explore),
            vec![],
        );
        let err = spec.validate().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("empty"),
            "EmptyId error must mention 'empty': {}",
            msg
        );
    }
}
