// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// target_task.rs — SPEC-M6.2: 8 integration scenarios covering the
// Target/Task DAG substrate.

use sddk_engine::target_task::builtin::builtin_targets;
use sddk_engine::target_task::dag::resolve_dag;
use sddk_engine::target_task::registry::TargetRegistry;
use sddk_engine::target_task::{
    AuthorityRequirement, Cacheability, Determinism, EvidenceContract, MemoryEffect, RetryPolicy,
    SideEffectClass, Target, TargetTaskError, Task,
};

fn minimal_task(id: &str) -> Task {
    Task {
        id: id.into(),
        inputs: vec![],
        outputs: vec![],
        depends_on: vec![],
        side_effect_class: SideEffectClass::Pure,
        authority_requirement: AuthorityRequirement::None,
        determinism: Determinism::Deterministic,
        cacheability: Cacheability::Cacheable,
        retry_policy: RetryPolicy::NoRetry,
        evidence_contract: EvidenceContract::None,
        memory_effects: MemoryEffect::None,
    }
}

fn minimal_target(name: &str, task_ids: &[&str]) -> Target {
    Target {
        name: name.into(),
        about: format!("target {name}"),
        tasks: task_ids.iter().map(|i| minimal_task(i)).collect(),
    }
}

// SC-M6.2-1: `TargetRegistry::resolve("status")` returns a 1-task resolution
// (status is 3-task in the shipped built-in; we use a minimal target here
// for a deterministic single-task assertion).
#[test]
fn sc_m6_2_1_resolve_minimal_target_returns_one_task() {
    let mut r = TargetRegistry::new();
    r.register(minimal_target("one", &["a"]));
    let res = r.resolve("one");
    assert_eq!(res.target, "one");
    let dag = res.dag.expect("dag");
    assert_eq!(dag.order, vec!["a"]);
    assert_eq!(res.validation, "ok");
}

// SC-M6.2-2: `TargetRegistry::resolve("run")` returns a DAG with 5 tasks
// from the built-in targets.
#[test]
fn sc_m6_2_2_resolve_run_target_returns_five_tasks() {
    let r = TargetRegistry::with_builtins();
    let res = r.resolve("run");
    let dag = res.dag.expect("dag");
    assert_eq!(dag.order.len(), 5);
}

// SC-M6.2-3: Topological sort of a diamond DAG puts `a` first and `d` last.
#[test]
fn sc_m6_2_3_topological_sort_diamond() {
    let mut r = TargetRegistry::new();
    r.register(Target {
        name: "diamond".into(),
        about: "diamond DAG".into(),
        tasks: vec![
            minimal_task("a"),
            minimal_task_with_deps("b", &["a"]),
            minimal_task_with_deps("c", &["a"]),
            minimal_task_with_deps("d", &["b", "c"]),
        ],
    });
    let res = r.resolve("diamond");
    let dag = res.dag.expect("dag");
    assert_eq!(dag.order.first().unwrap(), "a");
    assert_eq!(dag.order.last().unwrap(), "d");
}

// SC-M6.2-4: A cycle in the DAG fails closed with `CycleDetected`.
#[test]
fn sc_m6_2_4_cycle_fails_closed() {
    let mut r = TargetRegistry::new();
    r.register(Target {
        name: "loopy".into(),
        about: "intentionally cyclic".into(),
        tasks: vec![
            minimal_task_with_deps("a", &["b"]),
            minimal_task_with_deps("b", &["a"]),
        ],
    });
    let res = r.resolve("loopy");
    assert!(res.dag.is_none());
    assert!(
        res.validation.contains("cycle"),
        "validation should mention cycle: {}",
        res.validation
    );
}

#[test]
fn resolve_dag_helper_detects_cycle() {
    let target = Target {
        name: "t".into(),
        about: "t".into(),
        tasks: vec![
            minimal_task_with_deps("a", &["b"]),
            minimal_task_with_deps("b", &["a"]),
        ],
    };
    let err = resolve_dag(&target).unwrap_err();
    assert!(matches!(err, TargetTaskError::CycleDetected { .. }));
}

// SC-M6.2-5: Unknown target name produces a `TargetResolution` with
// `dag = None` and `validation` describing the error.
#[test]
fn sc_m6_2_5_unknown_target_returns_resolution_with_no_dag() {
    let r = TargetRegistry::new();
    let res = r.resolve("ghost");
    assert_eq!(res.target, "ghost");
    assert!(res.dag.is_none());
    assert!(res.validation.contains("unknown target"));
}

// SC-M6.2-6: All built-in targets resolve without cycles and the registry
// carries exactly the four canonical names.
#[test]
fn sc_m6_2_6_builtins_resolve_cleanly() {
    let r = TargetRegistry::with_builtins();
    assert_eq!(r.len(), 4);
    for name in ["status", "run", "ship", "recover"] {
        let res = r.resolve(name);
        let dag = res.dag.expect("built-in target should resolve");
        // Ordering invariant: deps appear before dependents.
        let positions: std::collections::HashMap<&str, usize> = dag
            .order
            .iter()
            .enumerate()
            .map(|(i, t)| (t.as_str(), i))
            .collect();
        for task in dag.by_id.values() {
            for dep in &task.depends_on {
                let dep_pos = positions[dep.as_str()];
                let task_pos = positions[task.id.as_str()];
                assert!(
                    dep_pos < task_pos,
                    "task `{}` appeared before its dep `{}`",
                    task.id,
                    dep
                );
            }
        }
    }
}

// SC-M6.2-7: `builtin_targets()` returns exactly 4 targets.
#[test]
fn sc_m6_2_7_builtin_count_is_four() {
    let targets = builtin_targets();
    assert_eq!(targets.len(), 4);
    let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["status", "run", "ship", "recover"]);
}

// SC-M6.2-8: Duplicate task id within a target fails with `DuplicateTaskId`.
#[test]
fn sc_m6_2_8_duplicate_task_id_fails() {
    let target = Target {
        name: "dup".into(),
        about: "dup".into(),
        tasks: vec![minimal_task("a"), minimal_task("a")],
    };
    let err = resolve_dag(&target).unwrap_err();
    assert!(matches!(err, TargetTaskError::DuplicateTaskId(_, _)));
}

fn minimal_task_with_deps(id: &str, deps: &[&str]) -> Task {
    Task {
        id: id.into(),
        inputs: vec![],
        outputs: vec![],
        depends_on: deps.iter().map(|s| s.to_string()).collect(),
        side_effect_class: SideEffectClass::Pure,
        authority_requirement: AuthorityRequirement::None,
        determinism: Determinism::Deterministic,
        cacheability: Cacheability::Cacheable,
        retry_policy: RetryPolicy::NoRetry,
        evidence_contract: EvidenceContract::None,
        memory_effects: MemoryEffect::None,
    }
}
