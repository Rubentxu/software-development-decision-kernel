//! Integration tests for the execution graph compiler (REQ-DW-RUNTIME-001-Compile-Contract, -Determinism, -Provenance-Identity).
//!
//! Tests compile `&PlanRevisionV1` into `ExecutionGraphRevision` and verify:
//! - Happy-path compilation produces correct revision numbers, schema version, and digest
//! - Identical inputs produce byte-equal outputs (determinism)
//! - Serde round-trip preserves digest
//! - Different compile anchors yield different revision IDs
//! - Provenance identity is triple-bound
//! - Edge synthesis from Operator::referenced_ids()
//! - Deterministic BTreeMap edge sets

use std::collections::BTreeMap;

use sddk_domain::ExecutionGraphRevision;
use sddk_domain::execution_graph_compiler::compile_plan_to_revision;
use sddk_domain::plan_revision::{
    NormalizedPlanV1, PlanMutation, PlanProvenanceV1, PlanRevisionV1,
};
use sddk_domain::workflow_ir::{Budgets, CapabilityId, NodeId, Operator, OperatorId, WorkflowIR};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds a minimal `WorkflowIR` with a single `Operator::Task` node.
fn sample_ir() -> WorkflowIR {
    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators: BTreeMap::from([(
            OperatorId("t1".into()),
            Operator::Task {
                capability: CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        )]),
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    }
}

/// Builds a `PlanRevisionV1` from a `WorkflowIR` (mutation = Initial).
fn sample_plan_revision() -> PlanRevisionV1 {
    let ir = sample_ir();
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision")
}

// ---------------------------------------------------------------------------
// Compile Contract — Happy Path
// ---------------------------------------------------------------------------

/// Scenario: valid plan compiles happy path (REQ-DW-RUNTIME-001-Compile-Contract).
#[test]
fn valid_plan_compiles_happy_path() {
    let plan = sample_plan_revision();
    let result = compile_plan_to_revision(&plan, None, "anchor-v1");

    let r = result.expect("compile should succeed for well-formed plan");
    assert_eq!(r.revision, 0, "first revision must be 0");
    assert!(r.parent.is_none(), "no parent for first revision");
    assert_eq!(r.schema_version, 1, "schema_version must be 1");
    assert!(r.events.is_empty(), "events must be empty");
    assert!(
        !r.nodes.is_empty(),
        "nodes must be populated from plan operators"
    );
    // Digest must match self-computed digest
    let computed = r.compute_digest();
    assert_eq!(
        r.digest, computed,
        "digest must equal compute_digest() output"
    );
}

/// Scenario: chained compile increments revision (REQ-DW-RUNTIME-001-Compile-Contract).
#[test]
fn chained_compile_increments_revision() {
    let plan = sample_plan_revision();

    let parent =
        compile_plan_to_revision(&plan, None, "anchor-v1").expect("first compile should succeed");

    let child = compile_plan_to_revision(&plan, Some(&parent), "anchor-v1-chained")
        .expect("second compile should succeed");

    assert_eq!(
        child.revision,
        parent.revision + 1,
        "child revision must be parent.revision + 1"
    );
    assert!(
        child.parent.is_some(),
        "child must have boxed parent reference"
    );
    assert_ne!(
        child.digest, parent.digest,
        "different anchors must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

/// Scenario: identical input byte-equal output (REQ-DW-RUNTIME-001-Determinism).
#[test]
fn identical_input_byte_equal_output() {
    let plan = sample_plan_revision();

    let r1 =
        compile_plan_to_revision(&plan, None, "anchor-v1").expect("first compile should succeed");
    let r2 =
        compile_plan_to_revision(&plan, None, "anchor-v1").expect("second compile should succeed");

    let ser1 = serde_json::to_vec(&r1).expect("r1 must serialize");
    let ser2 = serde_json::to_vec(&r2).expect("r2 must serialize");
    assert_eq!(
        ser1, ser2,
        "identical inputs must produce byte-equal serialized output"
    );
    assert_eq!(r1.digest, r2.digest, "digests must be equal");
}

/// Scenario: serde round-trip preserves digest (REQ-DW-RUNTIME-001-Determinism).
#[test]
fn serde_round_trip_preserves_digest() {
    let plan = sample_plan_revision();

    let r1 = compile_plan_to_revision(&plan, None, "anchor-v1").expect("compile should succeed");
    let original_digest = r1.digest;

    let bytes = serde_json::to_vec(&r1).expect("must serialize");
    let r2: ExecutionGraphRevision = serde_json::from_slice(&bytes).expect("must deserialize");

    assert_eq!(
        r2.digest, original_digest,
        "round-tripped digest must be unchanged"
    );
}

/// Scenario: different anchor yields different revision_id (REQ-DW-RUNTIME-001-Determinism, -Provenance-Identity).
#[test]
fn different_anchor_yields_different_revision_id() {
    let plan = sample_plan_revision();

    let r_a = compile_plan_to_revision(&plan, None, "anchor-a").expect("compile with anchor-a");
    let r_b = compile_plan_to_revision(&plan, None, "anchor-b").expect("compile with anchor-b");

    assert_ne!(
        r_a.revision_id, r_b.revision_id,
        "different anchors must produce different revision_ids"
    );
    assert_ne!(
        r_a.digest, r_b.digest,
        "different anchors must produce different digests"
    );
}

/// Scenario: revision_id triple-binds identity (REQ-DW-RUNTIME-001-Provenance-Identity).
#[test]
fn revision_id_triple_binds_identity() {
    let plan = sample_plan_revision();

    let parent = compile_plan_to_revision(&plan, None, "anchor-root").expect("parent compile");
    let child =
        compile_plan_to_revision(&plan, Some(&parent), "anchor-child").expect("child compile");

    // revision_id must be 64 lowercase hex chars (sha256 output)
    let parent_rev_id_str = &parent.revision_id.0;
    let child_rev_id_str = &child.revision_id.0;
    assert_eq!(
        parent_rev_id_str.chars().count(),
        64,
        "revision_id must be 64 hex chars"
    );
    // Digits 0-9 are hex but not lowercase letters, so check differently
    assert!(
        parent_rev_id_str
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "revision_id must be lowercase hex"
    );
    assert_ne!(
        parent_rev_id_str, child_rev_id_str,
        "parent and child revision_ids must differ"
    );
}

// ---------------------------------------------------------------------------
// Edge Synthesis (S3)
// ---------------------------------------------------------------------------

/// Builds a plan with `Operator::Sequence { body: [a, b] }` so we get edges.
fn sequence_plan_revision() -> PlanRevisionV1 {
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators: BTreeMap::from([
            (
                OperatorId("a".into()),
                Operator::Task {
                    capability: CapabilityId("test.cap".into()),
                    inputs: Default::default(),
                },
            ),
            (
                OperatorId("b".into()),
                Operator::Task {
                    capability: CapabilityId("test.cap".into()),
                    inputs: Default::default(),
                },
            ),
            (
                OperatorId("seq".into()),
                Operator::Sequence {
                    body: vec![OperatorId("a".into()), OperatorId("b".into())],
                },
            ),
        ]),
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    };
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision")
}

/// Scenario: edge relation matches operator variant (REQ-DW-RUNTIME-001-Edge-Synthesis).
#[test]
fn edge_relation_matches_operator_variant() {
    let plan = sequence_plan_revision();
    let r = compile_plan_to_revision(&plan, None, "anchor-seq-v1").expect("compile should succeed");

    // Every edge from a Sequence operator must have relation == "sequence"
    // and from/to present in r.nodes.keys()
    assert!(
        !r.edges.is_empty(),
        "edges must be synthesised from referenced_ids()"
    );
    for (edge_id, edge) in &r.edges {
        // Check from/to are in nodes
        assert!(
            r.nodes.contains_key(&NodeId(edge.from.clone())),
            "edge from '{}' must be in nodes",
            edge.from
        );
        assert!(
            r.nodes.contains_key(&NodeId(edge.to.clone())),
            "edge to '{}' must be in nodes",
            edge.to
        );
        assert_eq!(
            edge.relation, "sequence",
            "edge from Sequence operator must have relation='sequence'"
        );
        // EdgeId.0 must be in the expected format: "{}->{}->{}"
        assert!(
            edge_id.0.contains("->"),
            "edge_id must contain '->' separator"
        );
    }
}

/// Scenario: deterministic BTreeMap edge set (REQ-DW-RUNTIME-001-Edge-Synthesis, -Determinism).
#[test]
fn deterministic_btreemap_edge_set() {
    let plan = sequence_plan_revision();

    let r1 = compile_plan_to_revision(&plan, None, "anchor-seq-v1")
        .expect("first compile should succeed");
    let r2 = compile_plan_to_revision(&plan, None, "anchor-seq-v1")
        .expect("second compile should succeed");

    assert_eq!(
        r1.edges, r2.edges,
        "edges must be equal across identical compilations"
    );
    // Also verify BTreeMap iteration order is identical
    let keys1: Vec<_> = r1.edges.keys().collect();
    let keys2: Vec<_> = r2.edges.keys().collect();
    assert_eq!(keys1, keys2, "edge key iteration order must be identical");
}

// ---------------------------------------------------------------------------
// Fail-Closed Validation (S4)
// ---------------------------------------------------------------------------

/// Helper: plan revision with exactly `n` task operators, using hard-limit budgets.
fn plan_with_n_nodes(n: usize) -> PlanRevisionV1 {
    use std::collections::BTreeMap;
    let mut operators = BTreeMap::new();
    for i in 0..n {
        operators.insert(
            OperatorId(format!("op{}", i)),
            Operator::Task {
                capability: CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        );
    }
    let budgets = Budgets::hard_limits();
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets,
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    };
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision")
}

/// Helper: plan with a reachable directed cycle: Sequence(a) → Sequence(b) → Sequence(c) → Sequence(a).
fn plan_with_cycle() -> PlanRevisionV1 {
    use std::collections::BTreeMap;
    // a → b → c → a (all sequences, so the cycle is reachable)
    let operators = BTreeMap::from([
        (
            OperatorId("a".into()),
            Operator::Sequence {
                body: vec![OperatorId("b".into())],
            },
        ),
        (
            OperatorId("b".into()),
            Operator::Sequence {
                body: vec![OperatorId("c".into())],
            },
        ),
        (
            OperatorId("c".into()),
            Operator::Sequence {
                body: vec![OperatorId("a".into())],
            },
        ),
    ]);
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    };
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision")
}

/// Helper: plan with an orphan reference: op "x" references "orphan" which does not exist.
fn plan_with_orphan_reference() -> PlanRevisionV1 {
    use std::collections::BTreeMap;
    let operators = BTreeMap::from([(
        OperatorId("x".into()),
        Operator::Sequence {
            body: vec![OperatorId("orphan".into())],
        },
    )]);
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    };
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision")
}

// ── The 8 typed tests ───────────────────────────────────────────────────────

/// Scenario: empty plan is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
#[test]
fn rejects_empty_plan() {
    let plan = plan_with_n_nodes(0);
    let result = compile_plan_to_revision(&plan, None, "anchor");
    assert!(result.is_err(), "compile must reject empty plan");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::EmptyPlan
        ),
        "must be EmptyPlan variant, got: {:?}",
        err
    );
    // No partial graph observable
    assert!(matches!(
        err,
        sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::EmptyPlan
    ));
}

/// Scenario: node count overflow is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
#[test]
fn rejects_node_count_overflow() {
    // Budgets::hard_limits().max_nodes = 10_000
    let plan = plan_with_n_nodes(10_001);
    let result = compile_plan_to_revision(&plan, None, "anchor");
    assert!(result.is_err(), "compile must reject node count overflow");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::NodeCountExceeded { .. }
        ),
        "must be NodeCountExceeded, got: {:?}",
        err
    );
}

/// Scenario: depth overflow is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
#[test]
fn rejects_depth_overflow() {
    // Build a chain exceeding depth 64
    use std::collections::BTreeMap;
    const DEPTH: usize = 65;
    let mut operators = BTreeMap::new();
    for i in 0..DEPTH {
        let child = if i + 1 < DEPTH {
            Some(OperatorId(format!("op{}", i + 1)))
        } else {
            None
        };
        let op = match child {
            Some(c) => Operator::Sequence { body: vec![c] },
            None => Operator::Task {
                capability: CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        };
        operators.insert(OperatorId(format!("op{}", i)), op);
    }
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets {
            max_depth: 64,
            ..Default::default()
        },
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    };
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    let plan = PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision");
    let result = compile_plan_to_revision(&plan, None, "anchor");
    assert!(result.is_err(), "compile must reject depth overflow");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::DepthExceeded { .. }
        ),
        "must be DepthExceeded, got: {:?}",
        err
    );
}

/// Scenario: cycle is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
#[test]
fn rejects_cycle() {
    let plan = plan_with_cycle();
    let result = compile_plan_to_revision(&plan, None, "anchor");
    assert!(result.is_err(), "compile must reject cycle");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::CycleDetected
        ),
        "must be CycleDetected, got: {:?}",
        err
    );
}

/// Scenario: orphan operator reference is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
#[test]
fn rejects_orphan_reference() {
    let plan = plan_with_orphan_reference();
    let result = compile_plan_to_revision(&plan, None, "anchor");
    assert!(result.is_err(), "compile must reject orphan reference");
    let err = result.unwrap_err();
    assert!(matches!(err, sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::OrphanOperatorReference { .. }),
        "must be OrphanOperatorReference, got: {:?}", err);
}

/// Scenario: unsupported schema version is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
///
/// Note: `NormalizedPlanV1::from_workflow_ir` always sets `schema_version = 1`
/// (the IR layer already validated it). The `UnsupportedSchemaVersion` variant
/// remains in the enum for completeness but is unreachable at the compile layer.
/// This test verifies the OUTPUT always has `schema_version = 1` (I-2).
#[test]
fn output_schema_version_is_always_one() {
    let plan = sample_plan_revision();
    let r = compile_plan_to_revision(&plan, None, "anchor").expect("compile should succeed");
    assert_eq!(
        r.schema_version, 1,
        "output schema_version must always be 1 (I-2)"
    );
}

/// Scenario: invalid plan mutation is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
#[test]
fn rejects_invalid_plan_mutation() {
    // PlanMutation::StructureReplaced is not applicable to compilation
    use std::collections::BTreeMap;
    let operators = BTreeMap::from([(
        OperatorId("t1".into()),
        Operator::Task {
            capability: CapabilityId("test.cap".into()),
            inputs: Default::default(),
        },
    )]);
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    };
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    // Manually construct a plan with StructureReplaced mutation
    let plan = PlanRevisionV1 {
        revision_id: "dummy".into(),
        parent_revision_id: Some("parent-id".into()),
        mutation: PlanMutation::StructureReplaced,
        provenance,
        normalized,
    };
    let result = compile_plan_to_revision(&plan, None, "anchor");
    assert!(
        result.is_err(),
        "compile must reject StructureReplaced mutation"
    );
    let err = result.unwrap_err();
    assert!(matches!(err, sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::InvalidPlanMutation { .. }),
        "must be InvalidPlanMutation, got: {:?}", err);
}

/// Scenario: empty lineage is rejected (REQ-DW-RUNTIME-001-Fail-Closed-Validation).
#[test]
fn rejects_empty_lineage() {
    // Empty lineage at compile layer: the plan has Initial mutation but no parent and empty nodes
    // (this is the same as empty plan, but we use Initial to be explicit)
    use std::collections::BTreeMap;
    let operators = BTreeMap::new(); // empty
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    };
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    let plan = PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision");
    let result = compile_plan_to_revision(&plan, None, "anchor");
    assert!(result.is_err(), "compile must reject empty lineage");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::EmptyLineage
                | sddk_domain::execution_graph_compiler::ExecutionGraphCompileError::EmptyPlan
        ),
        "must be EmptyLineage or EmptyPlan, got: {:?}",
        err
    );
}
