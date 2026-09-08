//! Context Capsule Compiler — RED tests from REQ-ContextCapsuleCompiler §Scenarios.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ContextCapsuleCompiler.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-081-CONTEXT-CAPSULE-COMPILER.md

use std::sync::Arc;

use sddk_engine::retry::MockClock;
use sddk_engine::{
    Assumption, CapsuleError, CapsuleInputs, CapsuleTarget, ChangeKind, CompilerPolicy,
    ContextCapsule, ContextCompiler, InMemoryCapsuleInputs, NegativeKnowledge, NegativeStatus,
    Scope, StaleReason,
};

fn clock() -> Arc<MockClock> {
    Arc::new(MockClock::new(1_000))
}

fn target() -> CapsuleTarget {
    CapsuleTarget {
        workflow_run: "wf-1".to_string(),
        node_run: "nr-1".to_string(),
        attempt: "at-1".to_string(),
    }
}

fn fixture_inputs() -> InMemoryCapsuleInputs {
    InMemoryCapsuleInputs::new()
        .with_objective("Review target architecture")
        .with_dod(vec!["risks identified".to_string()])
        .with_must_read(vec![
            "ref:arch-overview".to_string(),
            "ref:runtime-callgraph".to_string(),
            "ref:decisions-d42".to_string(),
            "ref:ledger-100".to_string(),
            "ref:artifact-x".to_string(),
        ])
        .with_relevant(vec!["ref:notes".to_string()])
        .with_negative_knowledge(vec![NegativeKnowledge {
            claim: "Graph is source of truth".to_string(),
            status: NegativeStatus::RuledOut,
            reason: "Event Ledger is authority per ADR-021".to_string(),
            evidence_ref: Some("ADR-021".to_string()),
        }])
        .with_assumptions(vec![Assumption {
            text: "Event Ledger remains authority".to_string(),
            confidence: 1.0,
        }])
        .with_constraints(vec!["no repository mutation".to_string()])
}

fn compiler_with(inputs: InMemoryCapsuleInputs, clock: Arc<MockClock>) -> ContextCompiler {
    ContextCompiler::new(Arc::new(inputs) as Arc<dyn CapsuleInputs>, clock)
}

// ── Scenario 1 — Fresh compile produces bounded capsule ─────────────────────

#[test]
fn scenario_1_fresh_compile_produces_bounded_capsule() {
    let c = compiler_with(fixture_inputs(), clock());
    let cap = c.compile(target()).expect("compile ok");
    assert_eq!(cap.capsule_id, "wf-1:nr-1:at-1");
    assert_eq!(cap.artifacts.must_read.len(), 5);
    assert_eq!(cap.objective, "Review target architecture");
    assert!(cap.budget.actual_tokens <= cap.budget.max_tokens);
}

// ── Scenario 2 — Stale ref is annotated, not removed ─────────────────────────

#[test]
fn scenario_2_stale_ref_is_annotated_not_removed() {
    let mut inp = fixture_inputs();
    inp = inp.with_content_changed("ref:runtime-callgraph", 500);
    let c = compiler_with(inp, clock());
    let cap = c.compile(target()).expect("compile ok");
    assert!(
        cap.staleness
            .stale_refs
            .iter()
            .any(|s| s.ref_id == "ref:runtime-callgraph"
                && s.reason == StaleReason::ContentChangedSinceCompile)
    );
    assert!(
        cap.artifacts
            .must_read
            .contains(&"ref:runtime-callgraph".to_string())
    );
}

// ── Scenario 3 — Delta compile preserves negative knowledge ────────────────

#[test]
fn scenario_3_delta_preserves_negative_knowledge() {
    let parent_inp = fixture_inputs();
    let parent_clock = clock();
    let parent_cap = compiler_with(parent_inp, parent_clock.clone())
        .compile(target())
        .expect("parent");

    let mut delta_target = target();
    delta_target.attempt = "at-2".to_string();
    let delta_inp = fixture_inputs().with_previous_capsule(parent_cap.clone());
    let cap = compiler_with(delta_inp, parent_clock)
        .compile_delta(delta_target, &parent_cap)
        .expect("delta ok");

    assert_eq!(cap.capsule_id, "wf-1:nr-1:at-1:delta:2");
    assert_eq!(cap.recovery.previous_attempt, Some("at-1".to_string()));
    assert!(
        cap.negative_knowledge
            .iter()
            .any(|n| n.claim == "Graph is source of truth")
    );
    assert!(
        cap.changes_since_parent
            .iter()
            .any(|c| c.kind == ChangeKind::UnchangedReusable)
    );
}

// ── Scenario 4 — Budget exceeded fails closed ──────────────────────────────

#[test]
fn scenario_4_budget_exceeded_fails_closed() {
    let mut inp = fixture_inputs();
    // Pad must_read with a very long string to blow past budget.
    inp = inp.with_must_read(vec!["x".repeat(2_000_000); 1]);
    let policy = CompilerPolicy {
        max_tokens: 100,
        ..CompilerPolicy::default()
    };
    let c = compiler_with(inp, clock()).with_policy(policy);
    let err = c.compile(target()).expect_err("must fail");
    assert!(matches!(err, CapsuleError::BudgetExceeded { .. }));
}

// ── Scenario 5 — Decision overridden marks stale ────────────────────────────

#[test]
fn scenario_5_decision_overridden_marks_stale() {
    let inp = fixture_inputs().with_decision_overridden("ref:decisions-d42", 2_000);
    let c = compiler_with(inp, clock());
    let cap = c.compile(target()).expect("ok");
    assert!(cap
        .staleness
        .stale_refs
        .iter()
        .any(|s| s.ref_id == "ref:decisions-d42"
            && s.reason == StaleReason::DecisionOverridden));
}

// ── Scenario 6 — Determinism: byte-stable across runs ────────────────────────

#[test]
fn scenario_6_determinism_byte_stable() {
    let inp = fixture_inputs();
    let clock_a = Arc::new(MockClock::new(2_000));
    let clock_b = Arc::new(MockClock::new(2_000));
    let cap_a = compiler_with(inp.clone(), clock_a)
        .compile(target())
        .expect("a");
    let cap_b = compiler_with(inp, clock_b).compile(target()).expect("b");
    let a = serde_json::to_vec(&cap_a).expect("serialize a");
    let b = serde_json::to_vec(&cap_b).expect("serialize b");
    assert_eq!(a, b, "capsules must be byte-stable across runs");
}

// ── Scenario 7 — Privacy: excluded scope does not leak ──────────────────────

#[test]
fn scenario_7_excluded_scope_does_not_leak() {
    let inp = fixture_inputs()
        .with_scope(Scope {
            include: vec!["arch".to_string()],
            exclude: vec!["ui-impl".to_string()],
        })
        .with_ref_tags("ref:artifact-x", vec!["ui-impl".to_string()])
        .with_ref_tags("ref:arch-overview", vec!["arch".to_string()]);
    let c = compiler_with(inp, clock());
    let cap = c.compile(target()).expect("ok");
    assert!(
        !cap.artifacts
            .must_read
            .contains(&"ref:artifact-x".to_string())
    );
    assert!(
        cap.artifacts
            .must_read
            .contains(&"ref:arch-overview".to_string())
    );
}

// ── Scenario 8 — Recovery preserves attempt chain ───────────────────────────

#[test]
fn scenario_8_recovery_preserves_attempt_chain() {
    let parent_cap = compiler_with(fixture_inputs(), clock())
        .compile(target())
        .expect("parent");
    let mut delta_target = target();
    delta_target.attempt = "at-3".to_string();
    let inp = fixture_inputs().with_previous_capsule(parent_cap.clone());
    let c = compiler_with(inp, clock());
    let cap = c.compile_delta(delta_target, &parent_cap).expect("delta");
    assert_eq!(cap.recovery.previous_attempt, Some("at-1".to_string()));
    assert_eq!(
        cap.recovery.previous_capsule,
        Some(parent_cap.capsule_id.clone())
    );
}

// ── Scenario 9 — TTL-based staleness ────────────────────────────────────────

#[test]
fn scenario_9_ttl_based_staleness() {
    let mut inp = fixture_inputs();
    inp = inp.with_last_seen("ref:arch-overview", 0);
    let policy = CompilerPolicy {
        staleness_ttl_ms: 1_000,
        ..CompilerPolicy::default()
    };
    let c = compiler_with(inp, clock()).with_policy(policy);
    // clock is at 1_000; last_seen 0; ttl 1_000; (1000 - 0) > 1000 -> stale
    let cap = c.compile(target()).expect("ok");
    assert!(
        cap.staleness
            .stale_refs
            .iter()
            .any(|s| s.ref_id == "ref:arch-overview" && s.reason == StaleReason::BeyondTtl)
    );
}

// ── Scenario 10 — Empty objective is rejected ────────────────────────────────

#[test]
fn scenario_10_empty_objective_is_rejected() {
    let inp = InMemoryCapsuleInputs::new().with_must_read(vec!["ref:x".to_string()]);
    let c = compiler_with(inp, clock());
    let err = c.compile(target()).expect_err("must fail");
    assert!(matches!(err, CapsuleError::MissingRequired { ref field } if field == "objective"));
}

#[allow(dead_code)]
fn _capsule_marker(_: ContextCapsule) {}
