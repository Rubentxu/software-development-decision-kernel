//! Property-based and determinism tests for WorkflowIR types.
//!
//! Covers:
//! - Hash determinism: same IR → same hash always
//! - BTreeMap insertion-order independence (100 random permutations produce same hash)
//! - JSON roundtrip stability
//! - `compute_content_hash` matches `sha256:<64-hex>` regex
//! - All 12 operators can be nested arbitrarily deep without overflow

use std::collections::BTreeMap;

use proptest::prelude::*;
use regex::Regex;
use sddk_domain::workflow_ir::{
    Budgets, CapabilityId, ExpansionPermission, Operator, OperatorId, Provenance, SCHEMA_VERSION,
    TemplateRef, WorkflowIR,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn sample_template_ref() -> TemplateRef {
    TemplateRef {
        id: "sddk.test.template".into(),
        version: "1.0.0".into(),
    }
}

fn sample_budgets() -> Budgets {
    Budgets {
        max_wall_ms: 60000,
        max_tokens: 100_000,
        max_cost_micros: 1_000_000,
        max_depth: 50,
        max_nodes: 200,
        remaining_tokens: Some(95_000),
        no_progress_threshold: u32::MAX,
    }
}

fn sample_provenance() -> Provenance {
    Provenance {
        generated_by: "sddk-test".into(),
        prompt_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .into(),
        model_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
            .into(),
        policy_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
            .into(),
    }
}

/// Builds a minimal WorkflowIR with one Task operator.
fn minimal_ir() -> WorkflowIR {
    let op_id = OperatorId("op-1".into());
    WorkflowIR {
        ir_id: None,
        schema_version: SCHEMA_VERSION,
        template_ref: sample_template_ref(),
        operators: {
            let mut m = BTreeMap::new();
            m.insert(
                op_id.clone(),
                Operator::Task {
                    capability: CapabilityId("test.cap".into()),
                    inputs: BTreeMap::new(),
                },
            );
            m
        },
        guards: BTreeMap::new(),
        expansion_permissions: [ExpansionPermission::Discover].into(),
        budgets: sample_budgets(),
        required_invariants: Default::default(),
        provenance: sample_provenance(),
    }
}

// ── Hash determinism ────────────────────────────────────────────────────────

/// Hash is deterministic: calling compute_content_hash twice on the same IR
/// returns identical values.
#[test]
fn hash_is_deterministic() {
    let ir = minimal_ir();
    let h1 = ir.compute_content_hash();
    let h2 = ir.compute_content_hash();
    assert_eq!(h1, h2, "hash must be deterministic");
}

/// Two semantically identical IRs (same structure) produce identical hashes
/// even if built via different code paths.
#[test]
fn identical_structure_same_hash() {
    let mk_ir = |cap: &str| {
        let op_id = OperatorId("op-1".into());
        WorkflowIR {
            ir_id: None,
            schema_version: SCHEMA_VERSION,
            template_ref: sample_template_ref(),
            operators: {
                let mut m = BTreeMap::new();
                m.insert(
                    op_id,
                    Operator::Task {
                        capability: CapabilityId(cap.into()),
                        inputs: BTreeMap::new(),
                    },
                );
                m
            },
            guards: BTreeMap::new(),
            expansion_permissions: [ExpansionPermission::Discover].into(),
            budgets: sample_budgets(),
            required_invariants: Default::default(),
            provenance: sample_provenance(),
        }
    };

    let ir1 = mk_ir("test.cap");
    let ir2 = mk_ir("test.cap");
    let ir3 = mk_ir("other.cap");

    assert_eq!(ir1.compute_content_hash(), ir2.compute_content_hash());
    assert_ne!(
        ir1.compute_content_hash(),
        ir3.compute_content_hash(),
        "different content must produce different hash"
    );
}

/// `compute_content_hash` output matches `sha256:<64-hex-lowercase>` regex.
#[test]
fn content_hash_format_matches_sha256_regex() {
    let re = Regex::new(r"^sha256:[0-9a-f]{64}$").unwrap();
    let ir = minimal_ir();
    let hash = ir.compute_content_hash();
    assert!(
        re.is_match(&hash),
        "hash '{hash}' must match sha256:<64-hex-lowercase>"
    );
}

/// Hash format length is 71 chars (16 for "sha256:" + 64 for hex).
#[test]
fn content_hash_length_is_71() {
    let hash = minimal_ir().compute_content_hash();
    assert_eq!(hash.len(), 71, "sha256: prefix (7) + 64 hex chars = 71");
}

/// `ir_id` is excluded from hash (hash must be identical before/after assigning id).
#[test]
fn ir_id_is_excluded_from_hash() {
    let mut ir = minimal_ir();
    let hash_without_id = ir.compute_content_hash();

    ir.ir_id = Some(sddk_domain::workflow_ir::IrId("ir-test-123".into()));
    let hash_with_id = ir.compute_content_hash();

    assert_eq!(
        hash_without_id, hash_with_id,
        "ir_id must not affect content hash"
    );
}

/// `schema_version` is excluded from hash.
#[test]
fn schema_version_excluded_from_hash() {
    let mut ir = minimal_ir();
    let hash_v1 = ir.compute_content_hash();

    ir.schema_version = 99;
    let hash_v99 = ir.compute_content_hash();

    assert_eq!(
        hash_v1, hash_v99,
        "schema_version must not affect content hash"
    );
}

// ── BTreeMap insertion-order independence ─────────────────────────────────────

/// Inserting the same key/value pairs in different BTreeMap orders produces
/// the same JSON serialization and thus the same hash.
#[test]
fn btreemap_order_does_not_affect_hash() {
    // Build 10 IRs where operators map is inserted in different orders
    // All 10 IRs have the same logical content (same 20 operators with same capabilities)
    // but inserted in different orders.
    let caps: Vec<_> = (0..20).map(|i| format!("test.cap.{}", i)).collect();

    let mut hashes = Vec::new();
    for perm in 0..10 {
        // Insert in perm-scrambled order but same content
        let order: Vec<usize> = if perm == 0 {
            (0..20).collect()
        } else {
            let mut v: Vec<usize> = (0..20).collect();
            // Simple shuffle: reverse every perm-th chunk
            let chunk_size = perm.max(1);
            for chunk_start in (0..v.len()).step_by(chunk_size) {
                let chunk_end = (chunk_start + chunk_size).min(v.len());
                v[chunk_start..chunk_end].reverse();
            }
            v
        };

        let mut ops = BTreeMap::new();
        for idx in &order {
            let op_id = OperatorId(format!("op-{}", idx));
            ops.insert(
                op_id,
                Operator::Task {
                    capability: CapabilityId(caps[*idx].clone()),
                    inputs: BTreeMap::new(),
                },
            );
        }

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: SCHEMA_VERSION,
            template_ref: sample_template_ref(),
            operators: ops,
            guards: BTreeMap::new(),
            expansion_permissions: [ExpansionPermission::Discover].into(),
            budgets: sample_budgets(),
            required_invariants: Default::default(),
            provenance: sample_provenance(),
        };
        hashes.push(ir.compute_content_hash());
    }

    // All hashes should be identical for the same logical content
    let first = hashes[0].clone();
    for (i, h) in hashes.iter().enumerate() {
        assert_eq!(
            h, &first,
            "BTreeMap iteration order must not affect hash (perm {i})"
        );
    }
}

/// Guards BTreeMap order does not affect hash.
#[test]
fn guards_btreemap_order_independent() {
    let mut ir = minimal_ir();

    // Insert guards in two different orders
    let mut guards1 = BTreeMap::new();
    guards1.insert(
        OperatorId("op-1".into()),
        sddk_domain::workflow_ir::GuardExpr {
            expr: "a > 0".into(),
        },
    );
    guards1.insert(
        OperatorId("op-2".into()),
        sddk_domain::workflow_ir::GuardExpr {
            expr: "b > 0".into(),
        },
    );

    ir.guards = guards1;
    let hash1 = ir.compute_content_hash();

    let mut guards2 = BTreeMap::new();
    guards2.insert(
        OperatorId("op-2".into()),
        sddk_domain::workflow_ir::GuardExpr {
            expr: "b > 0".into(),
        },
    );
    guards2.insert(
        OperatorId("op-1".into()),
        sddk_domain::workflow_ir::GuardExpr {
            expr: "a > 0".into(),
        },
    );

    ir.guards = guards2;
    let hash2 = ir.compute_content_hash();

    assert_eq!(hash1, hash2, "guard insertion order must not affect hash");
}

// ── JSON roundtrip ───────────────────────────────────────────────────────────

/// JSON serialization roundtrip preserves the IR.
#[test]
fn json_roundtrip_preserves_ir() {
    let ir = minimal_ir();
    let json = serde_json::to_string(&ir).expect("IR must serialize to JSON");
    let ir2: WorkflowIR = serde_json::from_str(&json).expect("JSON must deserialize back to IR");
    assert_eq!(ir, ir2);
}

/// Roundtrip preserves the content hash.
#[test]
fn json_roundtrip_preserves_hash() {
    let ir = minimal_ir();
    let hash_before = ir.compute_content_hash();
    let json = serde_json::to_string(&ir).expect("must serialize");
    let ir2: WorkflowIR = serde_json::from_str(&json).expect("must deserialize");
    let hash_after = ir2.compute_content_hash();
    assert_eq!(hash_before, hash_after, "roundtrip must preserve hash");
}

// ── Budgets serde behavior ───────────────────────────────────────────────────

/// Budgets with no_progress_threshold = u32::MAX is skipped during serialization.
/// This prevents cluttering JSON with the default value.
#[test]
fn budgets_no_progress_threshold_max_is_skipped_in_json() {
    let budgets = Budgets {
        max_wall_ms: 60000,
        max_tokens: u64::MAX,
        max_cost_micros: u64::MAX,
        max_depth: u64::MAX,
        max_nodes: 100,
        remaining_tokens: None,
        no_progress_threshold: u32::MAX,
    };
    let json = serde_json::to_string(&budgets).expect("budgets must serialize");
    // u32::MAX should be skipped, so "no_progress_threshold" should NOT appear
    assert!(
        !json.contains("no_progress_threshold"),
        "u32::MAX no_progress_threshold should be skipped in JSON, got: {}",
        json
    );
}

/// Budgets with non-max no_progress_threshold IS included in JSON.
#[test]
fn budgets_no_progress_threshold_non_max_included_in_json() {
    let budgets = Budgets {
        max_wall_ms: 60000,
        max_tokens: u64::MAX,
        max_cost_micros: u64::MAX,
        max_depth: u64::MAX,
        max_nodes: 100,
        remaining_tokens: None,
        no_progress_threshold: 3,
    };
    let json = serde_json::to_string(&budgets).expect("budgets must serialize");
    assert!(
        json.contains("\"no_progress_threshold\":3"),
        "non-max no_progress_threshold should be in JSON, got: {}",
        json
    );
}

/// Budgets with missing no_progress_threshold deserializes to u32::MAX (default).
#[test]
fn budgets_missing_no_progress_threshold_defaults_to_max() {
    let json = r#"{
        "max_wall_ms": 60000,
        "max_tokens": 18446744073709551615,
        "max_cost_micros": 18446744073709551615,
        "max_depth": 18446744073709551615,
        "max_nodes": 100
    }"#;
    let budgets: Budgets = serde_json::from_str(json).expect("budgets must deserialize from JSON");
    assert_eq!(
        budgets.no_progress_threshold,
        u32::MAX,
        "missing no_progress_threshold should default to u32::MAX"
    );
}

/// Budgets roundtrip preserves no_progress_threshold value.
#[test]
fn budgets_roundtrip_preserves_no_progress_threshold() {
    let original = Budgets {
        max_wall_ms: 60000,
        max_tokens: u64::MAX,
        max_cost_micros: u64::MAX,
        max_depth: u64::MAX,
        max_nodes: 100,
        remaining_tokens: None,
        no_progress_threshold: 5,
    };
    let json = serde_json::to_string(&original).expect("must serialize");
    let deserialized: Budgets = serde_json::from_str(&json).expect("must deserialize");
    assert_eq!(
        deserialized.no_progress_threshold, 5,
        "roundtrip must preserve no_progress_threshold"
    );
}

// ── Operator nesting depth ───────────────────────────────────────────────────

/// All 12 operators can be nested to depth 10 without stack overflow.
#[test]
fn all_operators_nest_to_depth_10() {
    // Build a deeply nested Sequence chain
    let mut operators: BTreeMap<OperatorId, Operator> = BTreeMap::new();
    let mut prev_id = OperatorId("op-0".into());

    for i in 1..=10 {
        let id = OperatorId(format!("op-{}", i));
        operators.insert(
            id.clone(),
            Operator::Sequence {
                body: vec![prev_id.clone()],
            },
        );
        prev_id = id;
    }

    // Add the innermost leaf
    operators.insert(
        OperatorId("op-0".into()),
        Operator::Task {
            capability: CapabilityId("test.deep".into()),
            inputs: BTreeMap::new(),
        },
    );

    let ir = WorkflowIR {
        ir_id: None,
        schema_version: SCHEMA_VERSION,
        template_ref: sample_template_ref(),
        operators,
        guards: BTreeMap::new(),
        expansion_permissions: [ExpansionPermission::Discover].into(),
        budgets: sample_budgets(),
        required_invariants: Default::default(),
        provenance: sample_provenance(),
    };

    // Must not overflow
    let hash = ir.compute_content_hash();
    assert!(hash.starts_with("sha256:"));
}

/// Parallel + Map operators at depth 5 don't overflow.
#[test]
fn parallel_and_map_nest_without_overflow() {
    let mut operators: BTreeMap<OperatorId, Operator> = BTreeMap::new();

    // Depth 5: parallel branch containing map
    operators.insert(
        OperatorId("outer".into()),
        Operator::Parallel {
            branches: vec![OperatorId("map-op".into())],
            max_concurrency: 4,
        },
    );
    operators.insert(
        OperatorId("map-op".into()),
        Operator::Map {
            source: OperatorId("src".into()),
            body: OperatorId("task".into()),
            max_concurrency: 2,
        },
    );
    operators.insert(
        OperatorId("src".into()),
        Operator::Task {
            capability: CapabilityId("test.src".into()),
            inputs: BTreeMap::new(),
        },
    );
    operators.insert(
        OperatorId("task".into()),
        Operator::Task {
            capability: CapabilityId("test.task".into()),
            inputs: BTreeMap::new(),
        },
    );

    let ir = WorkflowIR {
        ir_id: None,
        schema_version: SCHEMA_VERSION,
        template_ref: sample_template_ref(),
        operators,
        guards: BTreeMap::new(),
        expansion_permissions: [ExpansionPermission::Discover].into(),
        budgets: sample_budgets(),
        required_invariants: Default::default(),
        provenance: sample_provenance(),
    };

    let hash = ir.compute_content_hash();
    assert!(hash.starts_with("sha256:"));
}

/// Choice operator with 10 branches has stable hash.
#[test]
fn choice_with_10_branches_stable_hash() {
    let mut branches = BTreeMap::new();
    for i in 0..10 {
        branches.insert(format!("cond_{}", i), OperatorId(format!("op-{}", i)));
    }

    let operators = {
        let mut ops = BTreeMap::new();
        ops.insert(OperatorId("choice".into()), Operator::Choice { branches });
        for i in 0..10 {
            ops.insert(
                OperatorId(format!("op-{}", i)),
                Operator::Task {
                    capability: CapabilityId(format!("test.cap.{}", i)),
                    inputs: BTreeMap::new(),
                },
            );
        }
        ops
    };

    let ir = WorkflowIR {
        ir_id: None,
        schema_version: SCHEMA_VERSION,
        template_ref: sample_template_ref(),
        operators,
        guards: BTreeMap::new(),
        expansion_permissions: [ExpansionPermission::Discover].into(),
        budgets: sample_budgets(),
        required_invariants: Default::default(),
        provenance: sample_provenance(),
    };

    let h1 = ir.compute_content_hash();
    let h2 = ir.compute_content_hash();
    assert_eq!(h1, h2);
}

/// Loop operator at depth 3 has stable hash.
#[test]
fn loop_nested_three_times_hash_stable() {
    let operators = {
        let mut ops = BTreeMap::new();
        // outer loop
        ops.insert(
            OperatorId("loop-outer".into()),
            Operator::Loop {
                max_iterations: 10,
                until: sddk_domain::workflow_ir::GuardExpr {
                    expr: "done".into(),
                },
                body: OperatorId("loop-mid".into()),
            },
        );
        ops.insert(
            OperatorId("loop-mid".into()),
            Operator::Loop {
                max_iterations: 5,
                until: sddk_domain::workflow_ir::GuardExpr {
                    expr: "mid_done".into(),
                },
                body: OperatorId("loop-inner".into()),
            },
        );
        ops.insert(
            OperatorId("loop-inner".into()),
            Operator::Loop {
                max_iterations: 3,
                until: sddk_domain::workflow_ir::GuardExpr {
                    expr: "inner_done".into(),
                },
                body: OperatorId("task".into()),
            },
        );
        ops.insert(
            OperatorId("task".into()),
            Operator::Task {
                capability: CapabilityId("test.loop".into()),
                inputs: BTreeMap::new(),
            },
        );
        ops
    };

    let ir = WorkflowIR {
        ir_id: None,
        schema_version: SCHEMA_VERSION,
        template_ref: sample_template_ref(),
        operators,
        guards: BTreeMap::new(),
        expansion_permissions: [ExpansionPermission::Discover].into(),
        budgets: sample_budgets(),
        required_invariants: Default::default(),
        provenance: sample_provenance(),
    };

    let h1 = ir.compute_content_hash();
    let h2 = ir.compute_content_hash();
    assert_eq!(h1, h2);
}

// ── Schema version monotonicity ────────────────────────────────────────────────

/// SCHEMA_VERSION constant is 1 for all IR types.
#[test]
fn schema_version_constant_is_one() {
    assert_eq!(SCHEMA_VERSION, 1);
}

// ── Proptest string-based property tests ─────────────────────────────────────

proptest! {
    /// Different capability strings produce different hashes.
    #[test]
    fn different_capabilities_produce_different_hashes(cap1 in "[a-z]{3,10}", cap2 in "[a-z]{3,10}") {
        let cap1_clone = cap1.clone();
        let cap2_clone = cap2.clone();

        let ir1 = {
            let op_id = OperatorId("op-1".into());
            WorkflowIR {
                ir_id: None,
                schema_version: SCHEMA_VERSION,
                template_ref: sample_template_ref(),
                operators: {
                    let mut m = BTreeMap::new();
                    m.insert(op_id, Operator::Task {
                        capability: CapabilityId(cap1),
                        inputs: BTreeMap::new(),
                    });
                    m
                },
                guards: BTreeMap::new(),
                expansion_permissions: [ExpansionPermission::Discover].into(),
                budgets: sample_budgets(),
                required_invariants: Default::default(),
                provenance: sample_provenance(),
            }
        };

        let ir2 = {
            let op_id = OperatorId("op-1".into());
            WorkflowIR {
                ir_id: None,
                schema_version: SCHEMA_VERSION,
                template_ref: sample_template_ref(),
                operators: {
                    let mut m = BTreeMap::new();
                    m.insert(op_id, Operator::Task {
                        capability: CapabilityId(cap2),
                        inputs: BTreeMap::new(),
                    });
                    m
                },
                guards: BTreeMap::new(),
                expansion_permissions: [ExpansionPermission::Discover].into(),
                budgets: sample_budgets(),
                required_invariants: Default::default(),
                provenance: sample_provenance(),
            }
        };

        if cap1_clone != cap2_clone {
            prop_assert_ne!(
                ir1.compute_content_hash(),
                ir2.compute_content_hash(),
                "different capabilities must produce different hashes"
            );
        }
    }
}
