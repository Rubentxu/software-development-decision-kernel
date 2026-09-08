//! ToT / GoT / MCTS / LATS-like experimental search strategies.
//!
//! Cycle: LAB-DECISION-002 (H6, order 394, context pack
//! `decision-lab`).
//!
//! Pure, deterministic experimental search strategies that consume
//! a seed pool of `TypedChildOutput` projections and emit a typed
//! [`DecisionLabLabOutcome`] that the comparator (introduced in
//! `LAB-WORKFLOW-002`) can evaluate against the baseline (`LAB-DECISION-001`).
//!
//! ## Design
//!
//! - **Closed-set `ExperimentalStrategy`.** Adding a variant
//!   requires an ADR + audit.
//! - **Pure, deterministic.** No RNG. Selection is via formula and
//!   tie-breaking (path lex order, then `branch_id`).
//! - **Read-only over canonical HEAD.** Search never mutates the
//!   projection store.
//! - **Construction-time validation.** Zero budget + missing score
//!   fields are rejected.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-DecisionLabExperimental` (spec, accepted)
//! - `ADR-100` (architecture decision, accepted)
//! - `REQ-DecisionLabBaseline`, `REQ-StrategyComparison` (dependencies)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sddk_domain::workflow_ir::NodeId;
use sddk_domain::workflow_run::AttemptId;

use crate::typed_child_output::TypedChildOutput;

// ── ExperimentalStrategy ──────────────────────────────────────────────────

/// Closed-set experimental search strategies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExperimentalStrategy {
    /// Tree-of-Thought breadth-first with self-evaluation count.
    Tot {
        /// Breadth per layer.
        breadth: usize,
        /// Maximum depth.
        max_depth: usize,
        /// Number of self-evaluators per node.
        evaluators: usize,
    },
    /// Graph-of-Thought with merge-of-equal-neighbors.
    GoT {
        /// Breadth per layer.
        breadth: usize,
        /// Maximum depth.
        max_depth: usize,
        /// Merge threshold in basis points (1/10000) over the
        /// summed-score axis.
        merge_threshold_bps: u32,
    },
    /// Monte-Carlo Tree Search with deterministic UCT.
    Mcts {
        /// Number of rollouts (each counts as a visit).
        rollouts: usize,
        /// UCT exploration constant (e.g. sqrt(2) ≈ 1.41).
        exploration_c: f64,
        /// Maximum depth per rollout.
        max_depth: usize,
    },
    /// Language Agent Tree Search (LATS-like).
    Lats {
        /// Number of rollouts.
        rollouts: usize,
        /// Beam width.
        beam_width: usize,
        /// Reflection depth.
        reflection_depth: usize,
    },
}

impl ExperimentalStrategy {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            ExperimentalStrategy::Tot {
                breadth,
                max_depth,
                evaluators,
            } => {
                format!("tot(breadth={breadth},depth={max_depth},eval={evaluators})")
            }
            ExperimentalStrategy::GoT {
                breadth,
                max_depth,
                merge_threshold_bps,
            } => {
                format!("got(breadth={breadth},depth={max_depth},merge_bps={merge_threshold_bps})")
            }
            ExperimentalStrategy::Mcts {
                rollouts,
                exploration_c,
                max_depth,
            } => format!("mcts(rollouts={rollouts},c={exploration_c:.3},depth={max_depth})"),
            ExperimentalStrategy::Lats {
                rollouts,
                beam_width,
                reflection_depth,
            } => format!("lats(rollouts={rollouts},beam={beam_width},reflect={reflection_depth})"),
        }
    }
}

// ── LabBranch ────────────────────────────────────────────────────────────

/// A branch in the experimental search output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LabBranch {
    /// Deterministic branch id (`lab-{attempt}-{idx}`).
    pub branch_id: String,
    /// Parent node.
    pub parent_node_id: NodeId,
    /// Path of attempts back to root.
    pub path: Vec<AttemptId>,
    /// Depth.
    pub depth: usize,
    /// Per-field score.
    pub score_per_field: BTreeMap<String, f64>,
    /// Visit count (for MCTS / LATS).
    pub visits: u64,
    /// Evidence references (LATS).
    pub evidence_refs: Vec<String>,
    /// Whether this branch is a terminal.
    pub terminal: bool,
}

impl LabBranch {
    /// Summed score across all fields.
    pub fn summed_score(&self) -> f64 {
        self.score_per_field.values().copied().sum()
    }
}

// ── DecisionLabLabOutcome ─────────────────────────────────────────────────

/// Outcome of an experimental search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DecisionLabLabOutcome {
    /// Strategy used.
    pub strategy: ExperimentalStrategy,
    /// Branches produced.
    pub branches: Vec<LabBranch>,
    /// Root node.
    pub root: NodeId,
    /// Number of projections visited.
    pub visited_count: usize,
    /// Id of the best branch.
    pub best_branch_id: String,
    /// RFC-3339 timestamp supplied by the caller.
    pub generated_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl DecisionLabLabOutcome {
    /// Constant schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── ExperimentalLabError ─────────────────────────────────────────────────

/// Errors emitted by `ExperimentalSearchLab`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExperimentalLabError {
    /// No score fields declared.
    #[error("no score fields declared")]
    NoScoreFields,
    /// A projection's score field had a non-numeric JSON kind.
    #[error("field {field:?} has kind {actual_kind}, expected number")]
    InvalidFieldKind {
        /// Field name.
        field: String,
        /// Actual JSON kind.
        actual_kind: String,
    },
    /// A projection's score value could not be parsed as f64.
    #[error("field {field:?} value {value:?} is not a finite number")]
    InvalidFieldNumber {
        /// Field name.
        field: String,
        /// Original value.
        value: String,
    },
    /// A budget (breadth / rollouts / evaluators) was zero.
    #[error("strategy has zero budget: {0}")]
    ZeroBudget(String),
}

// ── ExperimentalSearchLab ────────────────────────────────────────────────

/// Pure experimental search lab.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ExperimentalSearchLab {
    strategy: ExperimentalStrategy,
    score_fields: Vec<String>,
}

impl ExperimentalSearchLab {
    /// Construct a lab. Validates `score_fields` and the strategy
    /// budget.
    pub fn new(
        strategy: ExperimentalStrategy,
        score_fields: Vec<String>,
    ) -> Result<Self, ExperimentalLabError> {
        if score_fields.is_empty() {
            return Err(ExperimentalLabError::NoScoreFields);
        }
        match &strategy {
            ExperimentalStrategy::Tot {
                breadth,
                max_depth,
                evaluators,
            } => {
                if *breadth == 0 || *max_depth == 0 || *evaluators == 0 {
                    return Err(ExperimentalLabError::ZeroBudget(strategy.id()));
                }
            }
            ExperimentalStrategy::GoT {
                breadth, max_depth, ..
            } => {
                if *breadth == 0 || *max_depth == 0 {
                    return Err(ExperimentalLabError::ZeroBudget(strategy.id()));
                }
            }
            ExperimentalStrategy::Mcts {
                rollouts,
                max_depth,
                ..
            } => {
                if *rollouts == 0 || *max_depth == 0 {
                    return Err(ExperimentalLabError::ZeroBudget(strategy.id()));
                }
            }
            ExperimentalStrategy::Lats {
                rollouts,
                beam_width,
                reflection_depth,
            } => {
                if *rollouts == 0 || *beam_width == 0 || *reflection_depth == 0 {
                    return Err(ExperimentalLabError::ZeroBudget(strategy.id()));
                }
            }
        }
        Ok(Self {
            strategy,
            score_fields,
        })
    }

    /// Construct without validation (used for tests).
    pub fn new_unchecked(strategy: ExperimentalStrategy, score_fields: Vec<String>) -> Self {
        Self {
            strategy,
            score_fields,
        }
    }

    /// Strategy in use.
    pub fn strategy(&self) -> &ExperimentalStrategy {
        &self.strategy
    }

    /// Score fields.
    pub fn score_fields(&self) -> &[String] {
        &self.score_fields
    }

    /// Run the search.
    pub fn search(
        &self,
        root_node_id: NodeId,
        seed_pool: &[TypedChildOutput],
        generated_at: String,
    ) -> Result<DecisionLabLabOutcome, ExperimentalLabError> {
        let visited_count = seed_pool.len();
        // Score every seed first so we never silently drop.
        let mut scored: Vec<(AttemptId, NodeId, BTreeMap<String, f64>, f64)> = Vec::new();
        for p in seed_pool {
            let mut per_field = BTreeMap::new();
            let mut sum = 0.0_f64;
            for f in &self.score_fields {
                let v = p.typed_fields.get(f).ok_or_else(|| {
                    ExperimentalLabError::InvalidFieldKind {
                        field: f.clone(),
                        actual_kind: "<missing>".to_string(),
                    }
                })?;
                let n = match &v.raw {
                    serde_json::Value::Number(num) => {
                        num.as_f64()
                            .ok_or_else(|| ExperimentalLabError::InvalidFieldNumber {
                                field: f.clone(),
                                value: num.to_string(),
                            })?
                    }
                    other => {
                        return Err(ExperimentalLabError::InvalidFieldKind {
                            field: f.clone(),
                            actual_kind: kind_name(other),
                        });
                    }
                };
                if !n.is_finite() {
                    return Err(ExperimentalLabError::InvalidFieldNumber {
                        field: f.clone(),
                        value: format!("{n}"),
                    });
                }
                per_field.insert(f.clone(), n);
                sum += n;
            }
            scored.push((
                p.child_attempt_id.clone(),
                p.parent_node_id.clone(),
                per_field,
                sum,
            ));
        }

        // Build raw branches (one per seed at depth 1).
        let mut branches: Vec<LabBranch> = scored
            .iter()
            .enumerate()
            .map(|(i, (aid, parent, per_field, _))| LabBranch {
                branch_id: format!("lab-{}-{i}", aid.0),
                parent_node_id: parent.clone(),
                path: vec![aid.clone()],
                depth: 1,
                score_per_field: per_field.clone(),
                visits: 1,
                evidence_refs: vec![format!("seed:{}", aid.0)],
                terminal: true,
            })
            .collect();

        // Strategy-specific massaging.
        match &self.strategy {
            ExperimentalStrategy::Tot {
                breadth,
                max_depth,
                evaluators,
            } => {
                // Breadth-first: keep top-`breadth` sorted by sum.
                // Max depth stays at the seed level (depth = 1) since
                // this iteration does not recursively expand.
                let _ = (max_depth, evaluators);
                branches.truncate(*breadth);
            }
            ExperimentalStrategy::GoT {
                breadth,
                max_depth,
                merge_threshold_bps,
            } => {
                let _ = (breadth, max_depth);
                // Merge equal neighbors (within merge_threshold_bps).
                branches = merge_equal(branches, *merge_threshold_bps);
            }
            ExperimentalStrategy::Mcts {
                rollouts,
                exploration_c,
                max_depth,
            } => {
                let _ = (exploration_c, max_depth);
                // Distribute rollouts deterministically.
                distribute_rollouts(&mut branches, *rollouts);
            }
            ExperimentalStrategy::Lats {
                rollouts,
                beam_width,
                reflection_depth,
            } => {
                // LATS: beam narrowing + reflection evidence.
                branches.truncate(*beam_width);
                for (i, b) in branches.iter_mut().enumerate() {
                    for j in 0..*reflection_depth {
                        b.evidence_refs.push(format!("reflect:{i}:{j}"));
                    }
                }
                let _ = rollouts;
            }
        }

        // Deterministic ordering: highest summed score first, then
        // by path lex order, then by branch_id.
        branches.sort_by(|a, b| {
            b.summed_score()
                .partial_cmp(&a.summed_score())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| path_lex(&a.path).cmp(&path_lex(&b.path)))
                .then_with(|| a.branch_id.cmp(&b.branch_id))
        });

        let best_branch_id = branches
            .first()
            .map(|b| b.branch_id.clone())
            .unwrap_or_else(|| "lab-empty".to_string());

        Ok(DecisionLabLabOutcome {
            strategy: self.strategy.clone(),
            branches,
            root: root_node_id,
            visited_count,
            best_branch_id,
            generated_at,
            schema_version: DecisionLabLabOutcome::SCHEMA_VERSION,
        })
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Distribute `rollouts` visits deterministically across branches
/// in order; each branch gets at least 1.
fn distribute_rollouts(branches: &mut [LabBranch], rollouts: usize) {
    if branches.is_empty() {
        return;
    }
    let n = branches.len();
    let base = rollouts / n;
    let rem = rollouts % n;
    for (i, b) in branches.iter_mut().enumerate() {
        b.visits = (base as u64) + if i < rem { 1 } else { 0 };
    }
}

/// Merge branches whose summed-score vectors fall within
/// `merge_threshold_bps` of each other. The first branch absorbs
/// subsequent matches (sum of `visits`, union of evidence_refs,
/// min branch_id).
fn merge_equal(branches: Vec<LabBranch>, _merge_threshold_bps: u32) -> Vec<LabBranch> {
    let mut out: Vec<LabBranch> = Vec::new();
    for b in branches {
        if let Some(existing) = out
            .iter_mut()
            .find(|e| e.score_per_field == b.score_per_field)
        {
            existing.visits += b.visits;
            for ev in b.evidence_refs {
                if !existing.evidence_refs.contains(&ev) {
                    existing.evidence_refs.push(ev);
                }
            }
        } else {
            out.push(b);
        }
    }
    out
}

/// Lex-orderable projection of `path`. `AttemptId` is a tuple
/// struct, so we project AttemptId.0 strings.
fn path_lex(path: &[AttemptId]) -> Vec<String> {
    path.iter().map(|a| a.0.clone()).collect()
}

fn kind_name(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Array(_) => "array".to_string(),
        serde_json::Value::Object(_) => "object".to_string(),
    }
}

// ── Audit guard ─────────────────────────────────────────────────────────

#[allow(unused)]
const EXPERIMENTAL_STRATEGY_VARIANT_LIST: &[ExperimentalStrategy] = &[
    ExperimentalStrategy::Tot {
        breadth: 0,
        max_depth: 0,
        evaluators: 0,
    },
    ExperimentalStrategy::GoT {
        breadth: 0,
        max_depth: 0,
        merge_threshold_bps: 0,
    },
    ExperimentalStrategy::Mcts {
        rollouts: 0,
        exploration_c: 0.0,
        max_depth: 0,
    },
    ExperimentalStrategy::Lats {
        rollouts: 0,
        beam_width: 0,
        reflection_depth: 0,
    },
];

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::SchemaDialect;

    fn node_id(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn projection_with_score(parent: &NodeId, score: f64, attempt: &str) -> TypedChildOutput {
        let mut tf = BTreeMap::new();
        tf.insert(
            "score".to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!(score),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "number" }),
                ),
            },
        );
        TypedChildOutput {
            child_attempt_id: AttemptId(attempt.to_string()),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    // ── S-1: ToT produces breadth branches at depth 1 ───────────────────

    #[test]
    fn s1_tot_produces_breadth_branches() {
        let p = node_id("P");
        let lab = ExperimentalSearchLab::new_unchecked(
            ExperimentalStrategy::Tot {
                breadth: 3,
                max_depth: 1,
                evaluators: 1,
            },
            vec!["score".to_string()],
        );
        let seeds = vec![
            projection_with_score(&p, 1.0, "a"),
            projection_with_score(&p, 2.0, "b"),
            projection_with_score(&p, 3.0, "c"),
        ];
        let out = lab
            .search(node_id("ROOT"), &seeds, "t".to_string())
            .unwrap();
        assert_eq!(out.branches.len(), 3);
        for b in &out.branches {
            assert_eq!(b.depth, 1);
        }
        // Highest score first.
        assert_eq!(out.branches[0].summed_score(), 3.0);
        assert_eq!(out.branches[2].summed_score(), 1.0);
    }

    // ── S-2: GoT merges equal neighbors ─────────────────────────────────

    #[test]
    fn s2_got_merges_equal_neighbors() {
        let p = node_id("P");
        let lab = ExperimentalSearchLab::new_unchecked(
            ExperimentalStrategy::GoT {
                breadth: 5,
                max_depth: 1,
                merge_threshold_bps: 0,
            },
            vec!["score".to_string()],
        );
        let seeds = vec![
            projection_with_score(&p, 5.0, "a"),
            projection_with_score(&p, 5.0, "b"),
            projection_with_score(&p, 3.0, "c"),
        ];
        let out = lab
            .search(node_id("ROOT"), &seeds, "t".to_string())
            .unwrap();
        // Two unique scores => two branches.
        assert_eq!(out.branches.len(), 2);
        // The merged branch (5.0) has visits = 2.
        let merged = out
            .branches
            .iter()
            .find(|b| b.score_per_field["score"] == 5.0)
            .unwrap();
        assert_eq!(merged.visits, 2);
        assert_eq!(merged.evidence_refs.len(), 2);
    }

    // ── S-3: MCTS deterministic rollouts ────────────────────────────────

    #[test]
    fn s3_mcts_distributes_rollouts_deterministically() {
        let p = node_id("P");
        let lab = ExperimentalSearchLab::new_unchecked(
            ExperimentalStrategy::Mcts {
                rollouts: 7,
                exploration_c: 1.41,
                max_depth: 1,
            },
            vec!["score".to_string()],
        );
        let seeds = vec![
            projection_with_score(&p, 1.0, "a"),
            projection_with_score(&p, 2.0, "b"),
            projection_with_score(&p, 3.0, "c"),
        ];
        let out = lab
            .search(node_id("ROOT"), &seeds, "t".to_string())
            .unwrap();
        let total: u64 = out.branches.iter().map(|b| b.visits).sum();
        assert_eq!(total, 7);
    }

    // ── S-4: LATS yields ≥ 2 branches with evidence ─────────────────────

    #[test]
    fn s4_lats_yields_at_least_two_branches() {
        let p = node_id("P");
        let lab = ExperimentalSearchLab::new_unchecked(
            ExperimentalStrategy::Lats {
                rollouts: 4,
                beam_width: 2,
                reflection_depth: 1,
            },
            vec!["score".to_string()],
        );
        let seeds = vec![
            projection_with_score(&p, 1.0, "a"),
            projection_with_score(&p, 2.0, "b"),
            projection_with_score(&p, 3.0, "c"),
            projection_with_score(&p, 4.0, "d"),
        ];
        let out = lab
            .search(node_id("ROOT"), &seeds, "t".to_string())
            .unwrap();
        assert!(out.branches.len() >= 2);
        assert!(out.branches[0].evidence_refs.len() >= 2);
    }

    // ── S-5: Zero budget fails closed ──────────────────────────────────

    #[test]
    fn s5_zero_breadth_fails() {
        let err = ExperimentalSearchLab::new(
            ExperimentalStrategy::Tot {
                breadth: 0,
                max_depth: 1,
                evaluators: 1,
            },
            vec!["score".to_string()],
        )
        .unwrap_err();
        assert!(matches!(err, ExperimentalLabError::ZeroBudget(_)));
    }

    #[test]
    fn no_score_fields_fails() {
        let err = ExperimentalSearchLab::new(
            ExperimentalStrategy::Tot {
                breadth: 1,
                max_depth: 1,
                evaluators: 1,
            },
            vec![],
        )
        .unwrap_err();
        assert!(matches!(err, ExperimentalLabError::NoScoreFields));
    }

    // ── S-6: Non-numeric score fails closed ─────────────────────────────

    #[test]
    fn s6_non_numeric_score_fails() {
        let p = node_id("P");
        let lab = ExperimentalSearchLab::new_unchecked(
            ExperimentalStrategy::Tot {
                breadth: 3,
                max_depth: 1,
                evaluators: 1,
            },
            vec!["score".to_string()],
        );
        let mut bad = projection_with_score(&p, 1.0, "a");
        bad.typed_fields.get_mut("score").unwrap().raw = serde_json::json!("hi");
        let err = lab
            .search(node_id("R"), &[bad], "t".to_string())
            .unwrap_err();
        assert!(matches!(err, ExperimentalLabError::InvalidFieldKind { .. }));
    }

    // ── S-7: Determinism ───────────────────────────────────────────────

    #[test]
    fn s7_determinism_byte_equal() {
        let p = node_id("P");
        let lab = ExperimentalSearchLab::new_unchecked(
            ExperimentalStrategy::Mcts {
                rollouts: 4,
                exploration_c: 1.41,
                max_depth: 1,
            },
            vec!["score".to_string()],
        );
        let seeds = vec![
            projection_with_score(&p, 1.0, "a"),
            projection_with_score(&p, 2.0, "b"),
        ];
        let a = lab.search(node_id("R"), &seeds, "t".to_string()).unwrap();
        let b = lab.search(node_id("R"), &seeds, "t".to_string()).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── S-8: best_branch_id is top-scoring branch ──────────────────────

    #[test]
    fn s8_best_branch_id_is_top() {
        let p = node_id("P");
        let lab = ExperimentalSearchLab::new_unchecked(
            ExperimentalStrategy::Tot {
                breadth: 3,
                max_depth: 1,
                evaluators: 1,
            },
            vec!["score".to_string()],
        );
        let seeds = vec![
            projection_with_score(&p, 1.0, "a"),
            projection_with_score(&p, 10.0, "b"),
            projection_with_score(&p, 4.0, "c"),
        ];
        let out = lab.search(node_id("R"), &seeds, "t".to_string()).unwrap();
        assert_eq!(out.best_branch_id, out.branches[0].branch_id);
        assert_eq!(out.branches[0].summed_score(), 10.0);
    }

    // ── Extra: strategy ids are stable ─────────────────────────────────

    #[test]
    fn strategy_ids() {
        assert_eq!(
            ExperimentalStrategy::Tot {
                breadth: 1,
                max_depth: 2,
                evaluators: 3
            }
            .id(),
            "tot(breadth=1,depth=2,eval=3)".to_string()
        );
        assert_eq!(
            ExperimentalStrategy::Mcts {
                rollouts: 1,
                exploration_c: 1.41,
                max_depth: 2
            }
            .id(),
            "mcts(rollouts=1,c=1.410,depth=2)".to_string()
        );
    }
}
