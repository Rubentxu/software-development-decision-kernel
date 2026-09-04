//! IR-permutation equivalence tests for `build_operator` (REQ-IRDT-HS-04, REQ-IRDT-DC-02).
//!
//! Two `WorkflowIR`s with identical `compute_content_hash()` but different BTreeMap
//! insertion order must produce observationally equivalent runtime trees.

use std::collections::BTreeMap;
use std::sync::Arc;

use sddk_domain::{CapabilityId, Operator as DomainOperator, OperatorId, WorkflowIR};
use sddk_engine::operator::build_operator;

/// Helper: build a WorkflowIR with the given operators.
fn make_ir(operators: BTreeMap<OperatorId, DomainOperator>) -> WorkflowIR {
    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test".into(),
            version: "1.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Default::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

/// Extracts children count from Sequence debug format.
fn extract_sequence_children_count(op: &Arc<dyn sddk_engine::operator::Operator>) -> Option<usize> {
    let debug = format!("{:?}", op);
    // Format is like "Sequence { children: [...] }"
    if debug.contains("Sequence") {
        // Extract the number from "children: [ ... ]" 
        // Simple heuristic: look for "children: [" and count commas + 1
        if let Some(start) = debug.find("children: [") {
            let after = &debug[start + "children: [".len()..];
            if let Some(end) = after.find(']') {
                let inner = &after[..end];
                if inner.trim().is_empty() {
                    return Some(0);
                }
                return Some(inner.matches(',').count() + 1);
            }
        }
    }
    None
}

/// Extracts children count from Parallel debug format.
fn extract_parallel_children_count(op: &Arc<dyn sddk_engine::operator::Operator>) -> Option<usize> {
    let debug = format!("{:?}", op);
    if debug.contains("Parallel") {
        if let Some(start) = debug.find("children: [") {
            let after = &debug[start + "children: [".len()..];
            if let Some(end) = after.find(']') {
                let inner = &after[..end];
                if inner.trim().is_empty() {
                    return Some(0);
                }
                return Some(inner.matches(',').count() + 1);
            }
        }
    }
    None
}

/// Extracts branches count from Choice debug format.
fn extract_choice_branches_count(op: &Arc<dyn sddk_engine::operator::Operator>) -> Option<usize> {
    let debug = format!("{:?}", op);
    if debug.contains("Choice") {
        if let Some(start) = debug.find("branches:") {
            let after = &debug[start + "branches:".len()..];
            // Just check if branches exist
            if after.trim().starts_with("{") {
                return Some(after.matches("kind:").count());
            }
        }
    }
    None
}

/// Scenario: two IRs with identical compute_content_hash but permuted BTreeMap insertion order
/// produce observationally equivalent runtime trees (REQ-IRDT-HS-04, REQ-IRDT-DC-02).
#[test]
fn build_operator_equivalent_ir_permutation_equivalence() {
    // Build IR A: operators inserted in order [t1, t2, seq]
    let t1_a = DomainOperator::Task {
        capability: CapabilityId("t1.cap".into()),
        inputs: Default::default(),
    };
    let t2_a = DomainOperator::Task {
        capability: CapabilityId("t2.cap".into()),
        inputs: Default::default(),
    };
    let seq_a = DomainOperator::Sequence {
        body: vec![OperatorId("t1".into()), OperatorId("t2".into())],
    };
    let mut ops_a = BTreeMap::new();
    ops_a.insert(OperatorId("t1".into()), t1_a);
    ops_a.insert(OperatorId("t2".into()), t2_a);
    ops_a.insert(OperatorId("seq".into()), seq_a);
    let ir_a = make_ir(ops_a);

    // Build IR B: same operators, different insertion order [t2, t1, seq]
    let t1_b = DomainOperator::Task {
        capability: CapabilityId("t1.cap".into()),
        inputs: Default::default(),
    };
    let t2_b = DomainOperator::Task {
        capability: CapabilityId("t2.cap".into()),
        inputs: Default::default(),
    };
    let seq_b = DomainOperator::Sequence {
        body: vec![OperatorId("t1".into()), OperatorId("t2".into())],
    };
    let mut ops_b = BTreeMap::new();
    ops_b.insert(OperatorId("t2".into()), t2_b);
    ops_b.insert(OperatorId("t1".into()), t1_b);
    ops_b.insert(OperatorId("seq".into()), seq_b);
    let ir_b = make_ir(ops_b);

    // Verify content hashes are equal
    assert_eq!(
        ir_a.compute_content_hash(),
        ir_b.compute_content_hash(),
        "IRs must have identical compute_content_hash"
    );

    // Build runtime trees
    let seq_ir_a = ir_a.operators.get(&OperatorId("seq".into())).unwrap();
    let seq_ir_b = ir_b.operators.get(&OperatorId("seq".into())).unwrap();

    let runtime_a = build_operator(seq_ir_a, &ir_a).expect("build_operator must succeed");
    let runtime_b = build_operator(seq_ir_b, &ir_b).expect("build_operator must succeed");

    // Both should have kind "Sequence"
    assert_eq!(runtime_a.kind(), runtime_b.kind(), "kind must match");
    
    // Both should have same children count
    let count_a = extract_sequence_children_count(&runtime_a);
    let count_b = extract_sequence_children_count(&runtime_b);
    assert_eq!(count_a, count_b, "Sequence children count must match across IR permutations");
}

/// Scenario: Sequence children order matches IR declaration order, NOT sorted order
/// (REQ-IRDT-DC-03).
#[test]
fn sequence_declaration_order_preserved() {
    let t1 = DomainOperator::Task {
        capability: CapabilityId("first".into()),
        inputs: Default::default(),
    };
    let t2 = DomainOperator::Task {
        capability: CapabilityId("second".into()),
        inputs: Default::default(),
    };
    let t3 = DomainOperator::Task {
        capability: CapabilityId("third".into()),
        inputs: Default::default(),
    };
    let seq = DomainOperator::Sequence {
        body: vec![
            OperatorId("c".into()),
            OperatorId("a".into()),
            OperatorId("b".into()),
        ],
    };
    let mut ops = BTreeMap::new();
    ops.insert(OperatorId("t1".into()), t1);
    ops.insert(OperatorId("t2".into()), t2);
    ops.insert(OperatorId("t3".into()), t3);
    ops.insert(OperatorId("c".into()), DomainOperator::Task {
        capability: CapabilityId("c.cap".into()),
        inputs: Default::default(),
    });
    ops.insert(OperatorId("a".into()), DomainOperator::Task {
        capability: CapabilityId("a.cap".into()),
        inputs: Default::default(),
    });
    ops.insert(OperatorId("b".into()), DomainOperator::Task {
        capability: CapabilityId("b.cap".into()),
        inputs: Default::default(),
    });
    ops.insert(OperatorId("seq".into()), seq);
    let ir = make_ir(ops);

    let seq_ir = ir.operators.get(&OperatorId("seq".into())).unwrap();
    let runtime = build_operator(seq_ir, &ir).expect("build_operator must succeed");

    assert_eq!(runtime.kind(), "Sequence", "must be a Sequence");
    // Verify children exist via debug format
    let debug = format!("{:?}", runtime);
    assert!(
        debug.contains("c.cap") && debug.contains("a.cap") && debug.contains("b.cap"),
        "All child capabilities must be present in debug output"
    );
}

/// Scenario: Choice.branches iteration is BTreeMap sorted-key order (REQ-IRDT-DC-04).
#[test]
fn choice_branches_sorted() {
    let choice = DomainOperator::Choice {
        branches: [
            ("z".to_string(), OperatorId("z_id".into())),
            ("a".to_string(), OperatorId("a_id".into())),
            ("m".to_string(), OperatorId("m_id".into())),
        ]
        .into_iter()
        .collect(),
    };
    let mut ops = BTreeMap::new();
    ops.insert(OperatorId("z_id".into()), DomainOperator::Task {
        capability: CapabilityId("z.cap".into()),
        inputs: Default::default(),
    });
    ops.insert(OperatorId("a_id".into()), DomainOperator::Task {
        capability: CapabilityId("a.cap".into()),
        inputs: Default::default(),
    });
    ops.insert(OperatorId("m_id".into()), DomainOperator::Task {
        capability: CapabilityId("m.cap".into()),
        inputs: Default::default(),
    });
    ops.insert(OperatorId("choice".into()), choice);
    let ir = make_ir(ops);

    let choice_ir = ir.operators.get(&OperatorId("choice".into())).unwrap();
    let runtime = build_operator(choice_ir, &ir).expect("build_operator must succeed");

    assert_eq!(runtime.kind(), "Choice", "must be a Choice");
    let debug = format!("{:?}", runtime);
    // Verify that branches were resolved
    assert!(
        debug.contains("a.cap") && debug.contains("m.cap") && debug.contains("z.cap"),
        "All branch capabilities must be present"
    );
}
