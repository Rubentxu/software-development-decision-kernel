//! BUILD WorkGraph / WorkUnit Mapping.
//!
//! Cycle: SDD-ADAPTIVE-002 (H8, order 450, context pack
//! `adaptive-sdd`).
//!
//! Pure, deterministic converter that turns a `ShapeDecision` into
//! a typed `WorkGraph` of `WorkUnit`s forming a DAG. The DAG is
//! the executable plan the workflow engine consumes.
//!
//! ## Design
//!
//! - **Closed-set `WorkUnitStage`.** Six stages cover the pipeline:
//!   Compile / Apply / Verify / Promote / Integrate / Finalize.
//! - **Pure `WorkGraphBuilder`.** Deterministic ordering + unit_ids.
//! - **Topology validated.** No duplicates, no orphans, no cycles.
//! - **`Promote` is conditional.** Included only for `LabAgent`.
//! - **Read-only** over the input decision.
//! - **`#[non_exhaustive]`** everywhere.
//!
//! See:
//!
//! - `REQ-BuildWorkGraph` (spec, accepted)
//! - `ADR-106` (architecture decision, accepted)
//! - `REQ-ChangeContractShape` (dependency).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::change_contract::{ShapeDecision, SpecialistKind};

// ── WorkUnitStage ─────────────────────────────────────────────────────────

/// Closed-set stages of a WorkUnit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkUnitStage {
    /// Compile stage.
    Compile,
    /// Apply stage.
    Apply,
    /// Verify stage.
    Verify,
    /// Promote stage.
    Promote,
    /// Integrate stage.
    Integrate,
    /// Finalize stage.
    Finalize,
}

impl WorkUnitStage {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            WorkUnitStage::Compile => "compile".to_string(),
            WorkUnitStage::Apply => "apply".to_string(),
            WorkUnitStage::Verify => "verify".to_string(),
            WorkUnitStage::Promote => "promote".to_string(),
            WorkUnitStage::Integrate => "integrate".to_string(),
            WorkUnitStage::Finalize => "finalize".to_string(),
        }
    }
}

// ── WorkUnit ──────────────────────────────────────────────────────────────

/// A single WorkUnit in the WorkGraph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WorkUnit {
    /// Deterministic unit id.
    pub unit_id: String,
    /// Stage.
    pub stage: WorkUnitStage,
    /// Surface (mirrors contract surface).
    pub surface: String,
    /// Preconditions (unit_ids of predecessors).
    pub preconditions: Vec<String>,
    /// Payload kind (assertion).
    pub payload_kind: String,
    /// RFC-3339 timestamp supplied by the caller.
    pub generated_at: String,
}

// ── WorkGraph ────────────────────────────────────────────────────────────

/// A typed DAG of WorkUnits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WorkGraph {
    /// Graph id.
    pub graph_id: String,
    /// Contract id (mirrors source decision).
    pub contract_id: String,
    /// Specialist id (None if no chosen specialist).
    pub specialist_id: Option<String>,
    /// Units (topologically ordered, lex tie-breakers).
    pub units: Vec<WorkUnit>,
    /// RFC-3339 timestamp supplied by the caller.
    pub generated_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl WorkGraph {
    /// Schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── WorkUnitTopologyError ────────────────────────────────────────────────

/// Errors emitted during topology validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkUnitTopologyError {
    /// Predecessor id doesn't exist.
    #[error("unit {unit_id} has unknown predecessor {predecessor}")]
    UnknownPredecessor {
        /// Unit id.
        unit_id: String,
        /// Predecessor id.
        predecessor: String,
    },
    /// Cycle in the precedence graph.
    #[error("cycle detected: {cycle_units:?}")]
    Cycle {
        /// Units in the cycle.
        cycle_units: Vec<String>,
    },
    /// Duplicate unit id.
    #[error("duplicate unit id: {unit_id}")]
    DuplicateUnitId {
        /// Unit id.
        unit_id: String,
    },
}

// ── WorkGraphBuildError ──────────────────────────────────────────────────

/// Errors emitted by `WorkGraphBuilder`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkGraphBuildError {
    /// Topology validation failed.
    #[error("topology: {0}")]
    Topology(WorkUnitTopologyError),
    /// Internal error (should not occur in normal operation).
    #[error("internal: {0}")]
    Internal(String),
}

// ── WorkGraphBuilder ─────────────────────────────────────────────────────

/// Pure builder.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WorkGraphBuilder;

impl WorkGraphBuilder {
    /// Build a WorkGraph from a `ShapeDecision`.
    pub fn build(
        &self,
        decision: &ShapeDecision,
        generated_at: String,
    ) -> Result<WorkGraph, WorkGraphBuildError> {
        let surface = self.infer_surface(decision);
        let specialist_id = decision.chosen.as_ref().map(|s| s.specialist_id.clone());

        // Case 1: no chosen specialist → single Finalize noop.
        let stages: Vec<WorkUnitStage> = if decision.chosen.is_none() {
            vec![WorkUnitStage::Finalize]
        } else {
            // Build the canonical pipeline.
            let mut ss = vec![
                WorkUnitStage::Compile,
                WorkUnitStage::Apply,
                WorkUnitStage::Verify,
                WorkUnitStage::Integrate,
                WorkUnitStage::Finalize,
            ];
            // Include Promote iff LabAgent.
            if let Some(s) = &decision.chosen
                && s.kind == SpecialistKind::LabAgent
            {
                // Insert before Integrate.
                let pos = ss
                    .iter()
                    .position(|x| matches!(x, WorkUnitStage::Integrate))
                    .unwrap_or(ss.len());
                ss.insert(pos, WorkUnitStage::Promote);
            }
            ss
        };

        let mut units: Vec<WorkUnit> = Vec::new();
        let mut prev: Option<String> = None;
        for (i, stage) in stages.iter().enumerate() {
            let unit_id = format!("{surface}-{}-{i:02}", stage.id());
            let preconditions: Vec<String> = if let Some(p) = &prev {
                vec![p.clone()]
            } else {
                vec![]
            };
            let payload_kind = match stage {
                WorkUnitStage::Compile => "compile".to_string(),
                WorkUnitStage::Apply => "apply".to_string(),
                WorkUnitStage::Verify => "verify".to_string(),
                WorkUnitStage::Promote => "promote_lab".to_string(),
                WorkUnitStage::Integrate => "integrate".to_string(),
                WorkUnitStage::Finalize => "finalize".to_string(),
            };
            units.push(WorkUnit {
                unit_id,
                stage: *stage,
                surface: surface.clone(),
                preconditions,
                payload_kind,
                generated_at: generated_at.clone(),
            });
            prev = Some(units.last().unwrap().unit_id.clone());
        }

        // Validate topology.
        Self::validate_topology(&units)?;

        Ok(WorkGraph {
            graph_id: format!("wg-{}", decision.contract_id),
            contract_id: decision.contract_id.clone(),
            specialist_id,
            units,
            generated_at,
            schema_version: WorkGraph::SCHEMA_VERSION,
        })
    }

    /// Validate topology of a list of WorkUnits.
    pub fn validate_topology(units: &[WorkUnit]) -> Result<(), WorkGraphBuildError> {
        // 1. duplicate unit_id.
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for u in units {
            if !seen.insert(u.unit_id.as_str()) {
                return Err(WorkGraphBuildError::Topology(
                    WorkUnitTopologyError::DuplicateUnitId {
                        unit_id: u.unit_id.clone(),
                    },
                ));
            }
        }

        // 2. unknown predecessor.
        let ids: BTreeSet<&str> = units.iter().map(|u| u.unit_id.as_str()).collect();
        for u in units {
            for p in &u.preconditions {
                if !ids.contains(p.as_str()) {
                    return Err(WorkGraphBuildError::Topology(
                        WorkUnitTopologyError::UnknownPredecessor {
                            unit_id: u.unit_id.clone(),
                            predecessor: p.clone(),
                        },
                    ));
                }
            }
        }

        // 3. cycle detection (DFS).
        let mut state: BTreeMap<String, u8> = BTreeMap::new(); // 0=white, 1=gray, 2=black
        for u in units {
            state.insert(u.unit_id.clone(), 0);
        }
        let mut path: Vec<String> = Vec::new();
        for u in units {
            if state.get(&u.unit_id).copied().unwrap_or(0) == 0
                && let Some(cycle) = Self::dfs(&u.unit_id, units, &mut state, &mut path)
            {
                return Err(WorkGraphBuildError::Topology(
                    WorkUnitTopologyError::Cycle { cycle_units: cycle },
                ));
            }
        }
        Ok(())
    }

    fn dfs(
        start: &str,
        units: &[WorkUnit],
        state: &mut BTreeMap<String, u8>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        let mut stack: Vec<String> = vec![start.to_string()];
        path.push(start.to_string());
        state.insert(start.to_string(), 1);
        while let Some(node) = stack.last().cloned() {
            let successors: Vec<String> = units
                .iter()
                .find(|u| u.unit_id == node)
                .map(|u| u.preconditions.clone())
                .unwrap_or_default();
            let mut advanced = false;
            for s in &successors {
                match state.get(s).copied().unwrap_or(0) {
                    0 => {
                        state.insert(s.clone(), 1);
                        stack.push(s.clone());
                        path.push(s.clone());
                        advanced = true;
                        break;
                    }
                    1 => {
                        let idx = path.iter().position(|p| p == s).unwrap_or(0);
                        let mut cycle: Vec<String> = path[idx..].to_vec();
                        cycle.push(s.clone());
                        return Some(cycle);
                    }
                    _ => continue,
                }
            }
            if !advanced {
                state.insert(node, 2);
                stack.pop();
                path.pop();
            }
        }
        None
    }

    /// Infer the surface for the graph from the decision.
    fn infer_surface(&self, decision: &ShapeDecision) -> String {
        // Use the rationale or, if a specialist was chosen, the
        // first capability; otherwise the contract_id.
        if let Some(s) = &decision.chosen
            && let Some(c) = s.capabilities.first()
        {
            return c.clone();
        }
        decision.contract_id.clone()
    }
}

// ── Audit guards ──────────────────────────────────────────────────────────

#[allow(unused)]
const WORK_UNIT_STAGE_VARIANT_LIST: &[WorkUnitStage] = &[
    WorkUnitStage::Compile,
    WorkUnitStage::Apply,
    WorkUnitStage::Verify,
    WorkUnitStage::Promote,
    WorkUnitStage::Integrate,
    WorkUnitStage::Finalize,
];

#[allow(unused)]
const WORK_UNIT_TOPOLOGY_ERROR_VARIANT_LIST: &[WorkUnitTopologyError] = &[
    WorkUnitTopologyError::UnknownPredecessor {
        unit_id: String::new(),
        predecessor: String::new(),
    },
    WorkUnitTopologyError::Cycle {
        cycle_units: vec![],
    },
    WorkUnitTopologyError::DuplicateUnitId {
        unit_id: String::new(),
    },
];

#[allow(unused)]
const WORK_GRAPH_BUILD_ERROR_VARIANT_LIST: &[WorkGraphBuildError] = &[
    WorkGraphBuildError::Topology(WorkUnitTopologyError::DuplicateUnitId {
        unit_id: String::new(),
    }),
    WorkGraphBuildError::Internal(String::new()),
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_contract::Specialist;

    fn decision_with(chosen: Option<Specialist>, contract_id: &str) -> ShapeDecision {
        ShapeDecision {
            contract_id: contract_id.to_string(),
            chosen,
            rejected: vec![],
            rationale: "r".to_string(),
            decided_at: "t".to_string(),
            schema_version: crate::change_contract::ShapeDecision::SCHEMA_VERSION,
        }
    }

    fn spec(kind: SpecialistKind) -> Specialist {
        Specialist {
            specialist_id: "alice".to_string(),
            kind,
            capabilities: vec!["sddk-engine/sc".to_string()],
            admission_criteria: vec![],
            max_concurrent: 1,
        }
    }

    // ── S-1: No chosen specialist ⇒ single Finalize unit ───────────────

    #[test]
    fn s1_no_chosen_single_finalize() {
        let builder = WorkGraphBuilder;
        let d = decision_with(None, "C1");
        let g = builder.build(&d, "t".to_string()).unwrap();
        assert_eq!(g.units.len(), 1);
        assert_eq!(g.units[0].stage, WorkUnitStage::Finalize);
        assert!(g.specialist_id.is_none());
    }

    // ── S-2: Human/LLM/Rule pipeline ─────────────────────────────────────

    #[test]
    fn s2_llm_pipeline() {
        let builder = WorkGraphBuilder;
        let d = decision_with(Some(spec(SpecialistKind::LlmAgent)), "C1");
        let g = builder.build(&d, "t".to_string()).unwrap();
        assert_eq!(g.units.len(), 5);
        let stages: Vec<WorkUnitStage> = g.units.iter().map(|u| u.stage).collect();
        assert_eq!(
            stages,
            vec![
                WorkUnitStage::Compile,
                WorkUnitStage::Apply,
                WorkUnitStage::Verify,
                WorkUnitStage::Integrate,
                WorkUnitStage::Finalize,
            ]
        );
        // No Promote.
        assert!(!stages.contains(&WorkUnitStage::Promote));
    }

    // ── S-3: Lab pipeline includes Promote ─────────────────────────────

    #[test]
    fn s3_lab_includes_promote() {
        let builder = WorkGraphBuilder;
        let d = decision_with(Some(spec(SpecialistKind::LabAgent)), "C1");
        let g = builder.build(&d, "t".to_string()).unwrap();
        assert_eq!(g.units.len(), 6);
        let stages: Vec<WorkUnitStage> = g.units.iter().map(|u| u.stage).collect();
        assert_eq!(
            stages,
            vec![
                WorkUnitStage::Compile,
                WorkUnitStage::Apply,
                WorkUnitStage::Verify,
                WorkUnitStage::Promote,
                WorkUnitStage::Integrate,
                WorkUnitStage::Finalize,
            ]
        );
    }

    // ── S-4: unit_ids are deterministic ─────────────────────────────────

    #[test]
    fn s4_unit_ids_deterministic() {
        let builder = WorkGraphBuilder;
        let d = decision_with(Some(spec(SpecialistKind::LlmAgent)), "C1");
        let g1 = builder.build(&d, "t".to_string()).unwrap();
        let g2 = builder.build(&d, "t".to_string()).unwrap();
        let ids1: Vec<&String> = g1.units.iter().map(|u| &u.unit_id).collect();
        let ids2: Vec<&String> = g2.units.iter().map(|u| &u.unit_id).collect();
        assert_eq!(ids1, ids2);
    }

    // ── S-5: Determinism ───────────────────────────────────────────────

    #[test]
    fn s5_determinism_byte_equal() {
        let builder = WorkGraphBuilder;
        let d = decision_with(Some(spec(SpecialistKind::Human)), "C1");
        let a = builder.build(&d, "t".to_string()).unwrap();
        let b = builder.build(&d, "t".to_string()).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── S-6: Topology validates clean ──────────────────────────────────

    #[test]
    fn s6_topology_validates_clean() {
        let builder = WorkGraphBuilder;
        let d = decision_with(Some(spec(SpecialistKind::LlmAgent)), "C1");
        let g = builder.build(&d, "t".to_string()).unwrap();
        assert!(WorkGraphBuilder::validate_topology(&g.units).is_ok());
    }

    // ── S-7: surface mirrored ──────────────────────────────────────────

    #[test]
    fn s7_surface_mirrored() {
        let builder = WorkGraphBuilder;
        let mut s = spec(SpecialistKind::LlmAgent);
        s.capabilities = vec!["sddk-engine/sc".to_string()];
        let d = decision_with(Some(s), "C1");
        let g = builder.build(&d, "t".to_string()).unwrap();
        for u in &g.units {
            assert_eq!(u.surface, "sddk-engine/sc");
        }
    }

    // ── S-8: specialist_id propagated ──────────────────────────────────

    #[test]
    fn s8_specialist_id_propagated() {
        let builder = WorkGraphBuilder;
        let d = decision_with(Some(spec(SpecialistKind::Human)), "C1");
        let g = builder.build(&d, "t".to_string()).unwrap();
        assert_eq!(g.specialist_id, Some("alice".to_string()));
    }

    // ── Extra: stage ids ────────────────────────────────────────────────

    #[test]
    fn stage_ids() {
        assert_eq!(WorkUnitStage::Compile.id(), "compile".to_string());
        assert_eq!(WorkUnitStage::Promote.id(), "promote".to_string());
        assert_eq!(WorkUnitStage::Finalize.id(), "finalize".to_string());
    }

    // ── Extra: topological validation catches cycles ───────────────────

    #[test]
    fn topology_catches_cycle() {
        let mut units: Vec<WorkUnit> = vec![
            WorkUnit {
                unit_id: "a".to_string(),
                stage: WorkUnitStage::Compile,
                surface: "s".to_string(),
                preconditions: vec![],
                payload_kind: "p".to_string(),
                generated_at: "t".to_string(),
            },
            WorkUnit {
                unit_id: "b".to_string(),
                stage: WorkUnitStage::Apply,
                surface: "s".to_string(),
                preconditions: vec!["a".to_string()],
                payload_kind: "p".to_string(),
                generated_at: "t".to_string(),
            },
        ];
        // Fake a cycle.
        units[0].preconditions = vec!["b".to_string()];
        let err = WorkGraphBuilder::validate_topology(&units).unwrap_err();
        assert!(matches!(err, WorkGraphBuildError::Topology(_)));
    }
}
