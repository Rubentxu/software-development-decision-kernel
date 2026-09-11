//! DAG validation and topological ordering for the Target/Task substrate.
//!
//! Per SPEC-009 §Rules: target resolution is deterministic under effective
//! config/pack versions, and DAG cycles fail closed.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashSet, VecDeque};

use super::{Target, TargetTaskError, Task, TaskDag};

/// Validate a target's task graph and return a topologically-ordered DAG.
///
/// # Errors
///
/// - `DuplicateTaskId` when two tasks share an id within the same target.
/// - `UnknownDependency` when a `depends_on` references a task id that
///   is not declared on the target.
/// - `CycleDetected` when the dependency graph has a cycle (fail-closed
///   per SPEC-009).
pub fn resolve_dag(target: &Target) -> Result<TaskDag, TargetTaskError> {
    let mut by_id: BTreeMap<String, Task> = BTreeMap::new();
    for task in &target.tasks {
        if by_id.insert(task.id.clone(), task.clone()).is_some() {
            return Err(TargetTaskError::DuplicateTaskId(
                task.id.clone(),
                target.name.clone(),
            ));
        }
    }

    // Pass 1: every `depends_on` must resolve to a known task.
    for task in &target.tasks {
        for dep in &task.depends_on {
            if !by_id.contains_key(dep) {
                return Err(TargetTaskError::UnknownDependency {
                    target: target.name.clone(),
                    task: task.id.clone(),
                    dep: dep.clone(),
                });
            }
        }
    }

    // Pass 2: Kahn's algorithm for topological order. Detects cycles
    // when the walk leaves tasks unvisited.
    let mut indeg: BTreeMap<&str, usize> = BTreeMap::new();
    for (id, task) in &by_id {
        indeg.insert(id.as_str(), task.depends_on.len());
    }

    let mut queue: VecDeque<String> = indeg
        .iter()
        .filter_map(|(id, d)| (*d == 0).then_some((*id).to_string()))
        .collect();
    let mut order: Vec<String> = Vec::with_capacity(by_id.len());

    while let Some(id) = queue.pop_front() {
        order.push(id.clone());
        // Find tasks that depend on `id` and decrement.
        let dependents: Vec<String> = by_id
            .values()
            .filter(|t| t.depends_on.iter().any(|d| d == &id))
            .map(|t| t.id.clone())
            .collect();
        for dep_id in dependents {
            if let Some(entry) = indeg.get_mut(dep_id.as_str()) {
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    queue.push_back(dep_id);
                }
            }
        }
    }

    if order.len() != by_id.len() {
        // Find one task still with indeg > 0 to name in the error.
        let task = by_id
            .values()
            .find(|t| indeg.get(t.id.as_str()).copied().unwrap_or(0) > 0)
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "<unknown>".into());
        return Err(TargetTaskError::CycleDetected {
            target: target.name.clone(),
            task,
        });
    }

    // Sanity check: no duplicate visits.
    let mut seen: HashSet<&str> = HashSet::with_capacity(order.len());
    for id in &order {
        debug_assert!(seen.insert(id.as_str()), "topo sort revisited `{id}`");
    }

    Ok(TaskDag {
        target: target.name.clone(),
        order,
        by_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target_task::{
        AuthorityRequirement, Cacheability, Determinism, EvidenceContract, MemoryEffect,
        RetryPolicy, SideEffectClass,
    };

    fn task(id: &str, deps: &[&str]) -> Task {
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
            has_body: false,
        }
    }

    fn target(name: &str, tasks: Vec<Task>) -> Target {
        Target {
            name: name.into(),
            about: format!("target {name}"),
            tasks,
        }
    }

    #[test]
    fn single_task_no_deps_orders_itself() {
        let dag = resolve_dag(&target("t", vec![task("a", &[])])).unwrap();
        assert_eq!(dag.order, vec!["a"]);
    }

    #[test]
    fn linear_chain_orders_deps_first() {
        let dag = resolve_dag(&target(
            "t",
            vec![task("c", &["b"]), task("a", &[]), task("b", &["a"])],
        ))
        .unwrap();
        assert_eq!(dag.order, vec!["a", "b", "c"]);
    }

    #[test]
    fn diamond_orders_diamond_correctly() {
        // a -> b, a -> c, b -> d, c -> d
        let dag = resolve_dag(&target(
            "t",
            vec![
                task("a", &[]),
                task("b", &["a"]),
                task("c", &["a"]),
                task("d", &["b", "c"]),
            ],
        ))
        .unwrap();
        // `a` first, `d` last, `b` and `c` in between (any order).
        assert_eq!(dag.order.first().unwrap(), "a");
        assert_eq!(dag.order.last().unwrap(), "d");
        let b_pos = dag.order.iter().position(|x| x == "b").unwrap();
        let c_pos = dag.order.iter().position(|x| x == "c").unwrap();
        let d_pos = dag.order.iter().position(|x| x == "d").unwrap();
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
    }

    #[test]
    fn duplicate_task_id_errors() {
        let err = resolve_dag(&target("t", vec![task("a", &[]), task("a", &[])])).unwrap_err();
        assert!(matches!(err, TargetTaskError::DuplicateTaskId(_, _)));
    }

    #[test]
    fn unknown_dependency_errors() {
        let err = resolve_dag(&target("t", vec![task("a", &["missing"])])).unwrap_err();
        assert!(matches!(err, TargetTaskError::UnknownDependency { .. }));
    }

    #[test]
    fn cycle_errors_fail_closed() {
        // a -> b -> a
        let err =
            resolve_dag(&target("t", vec![task("a", &["b"]), task("b", &["a"])])).unwrap_err();
        match err {
            TargetTaskError::CycleDetected { target, task: _ } => {
                assert_eq!(target, "t");
            }
            other => panic!("expected CycleDetected, got {other:?}"),
        }
    }

    #[test]
    fn self_loop_is_a_cycle() {
        let err = resolve_dag(&target("t", vec![task("a", &["a"])])).unwrap_err();
        assert!(matches!(err, TargetTaskError::CycleDetected { .. }));
    }
}
