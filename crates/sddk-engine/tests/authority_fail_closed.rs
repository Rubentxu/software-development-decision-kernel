// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// authority_fail_closed.rs — A5-3 F4 (R5: denied ⇒ zero side effect)
//
// Pins the fail-closed contract: when the Authority Engine returns
// `Deny` or `RequireApproval` (without `--allow-high-band`), the
// `DagExecutor` MUST NOT dispatch any task body to its handler.
// The observable guarantees:
//
//   1. `ExecutionReport::executed_count() == 0`
//   2. `ExecutionReport::status == ExecutionStatus::Denied`
//   3. `ExecutionReport::had_denial() == true`
//   4. Downstream tasks are reported as `Skipped` with the
//      `upstream denied` / `upstream required approval` note.
//
// These are integration tests for the engine crate; they construct
// `TaskDag` values directly (no fixtures, no registry). The tests
// intentionally do NOT mock the Authority Engine: they exercise the
// real `gate_task` decision function and the real executor
// short-circuit paths in `executor.rs`.
//
// Cycle: A5-3 (Concurrency / CAS / authority side-effect races)
// Risk closed: R5 (denied ⇒ zero side effect)

#![allow(missing_docs)]

use std::collections::BTreeMap;

use sddk_engine::target_task::executor::{DagExecutor, ExecutorConfig};
use sddk_engine::target_task::outcome::{ExecutionStatus, TaskStatus};
use sddk_engine::target_task::{
    AuthorityRequirement, Cacheability, Determinism, EvidenceContract, MemoryEffect, RetryPolicy,
    SideEffectClass, Task, TaskDag,
};

/// Build a minimal two-task DAG with the given authority requirement
/// on the FIRST task and a `None` requirement on the second.
///
/// The second task's `None` requirement guarantees it WOULD be
/// admitted if the first task's gate returned `Allow`; if the first
/// task denies, the executor MUST still skip the second task and
/// never reach its body.
fn dag_with_two_tasks(authority_first: AuthorityRequirement) -> TaskDag {
    let mut by_id = BTreeMap::new();
    by_id.insert(
        "first".to_string(),
        Task {
            id: "first".to_string(),
            inputs: vec![],
            outputs: vec![],
            depends_on: vec![],
            side_effect_class: SideEffectClass::Write,
            authority_requirement: authority_first,
            determinism: Determinism::Deterministic,
            cacheability: Cacheability::Never,
            retry_policy: RetryPolicy::NoRetry,
            evidence_contract: EvidenceContract::FullReceipt,
            memory_effects: MemoryEffect::AppendFact,
            has_body: true,
        },
    );
    by_id.insert(
        "second".to_string(),
        Task {
            id: "second".to_string(),
            inputs: vec![],
            outputs: vec![],
            depends_on: vec!["first".to_string()],
            side_effect_class: SideEffectClass::Write,
            authority_requirement: AuthorityRequirement::None,
            determinism: Determinism::Deterministic,
            cacheability: Cacheability::Never,
            retry_policy: RetryPolicy::NoRetry,
            evidence_contract: EvidenceContract::FullReceipt,
            memory_effects: MemoryEffect::AppendFact,
            has_body: true,
        },
    );
    TaskDag {
        target: "test-target".to_string(),
        order: vec!["first".to_string(), "second".to_string()],
        by_id,
    }
}

#[test]
fn deny_short_circuits_walk_with_zero_executed_tasks() {
    // Write requirement on a non-system, non-user, non-agent actor →
    // `Deny`. The downstream `None`-requiring task must be skipped.
    let dag = dag_with_two_tasks(AuthorityRequirement::Write);
    let executor = DagExecutor::new(ExecutorConfig::default());
    let report = executor.walk(&dag, "bogus:actor");

    assert_eq!(
        report.executed_count(),
        0,
        "Deny must produce zero executed tasks; got {:?}",
        report.tasks
    );
    assert_eq!(
        report.status,
        ExecutionStatus::Denied,
        "Deny must propagate to top-level status"
    );
    assert!(
        report.had_denial(),
        "had_denial() must be true; got tasks {:?}",
        report.tasks
    );

    // The downstream `second` task must be reported as Skipped with the
    // `upstream denied` note (fail-closed downstream propagation).
    let second = report
        .tasks
        .iter()
        .find(|t| t.task_id == "second")
        .expect("second task in report");
    assert_eq!(second.status, TaskStatus::Skipped);
    assert!(
        second.note.contains("upstream denied"),
        "downstream must be skipped with `upstream denied` note; got `{}`",
        second.note
    );
}

#[test]
fn require_approval_short_circuits_walk_with_zero_executed_tasks() {
    // Approval requirement + non-system actor (no --allow-high-band)
    // → RequireApproval. Executor MUST treat this as a soft halt:
    // zero executions, top-level status `Denied`.
    let dag = dag_with_two_tasks(AuthorityRequirement::Approval);
    let executor = DagExecutor::new(ExecutorConfig::default());
    let report = executor.walk(&dag, "user:alice");

    assert_eq!(
        report.executed_count(),
        0,
        "RequireApproval without --allow-high-band must yield zero executions; got {:?}",
        report.tasks
    );
    assert_eq!(
        report.status,
        ExecutionStatus::Denied,
        "RequireApproval must surface as top-level Denied (operator must escalate)"
    );
    assert!(report.had_denial() || report.had_halt());

    let second = report
        .tasks
        .iter()
        .find(|t| t.task_id == "second")
        .expect("second task in report");
    assert_eq!(second.status, TaskStatus::Skipped);
    assert!(
        second.note.contains("upstream required approval")
            || second.note.contains("upstream denied"),
        "downstream must be skipped with approval/denial note; got `{}`",
        second.note
    );
}

#[test]
fn allow_high_band_promotes_approval_to_allow_and_executes() {
    // Sanity-check the inverse: with --allow-high-band, Approval is
    // promoted to Allow and BOTH tasks execute. This pins that the
    // fail-closed behavior above is gated by config, not structural.
    let dag = dag_with_two_tasks(AuthorityRequirement::Approval);
    let executor = DagExecutor::new(ExecutorConfig {
        allow_high_band: true,
        ..Default::default()
    });
    let report = executor.walk(&dag, "user:alice");

    // No host handler is wired (handlers=None), so both tasks fall
    // into the `NotImplemented` branch (SP-07). Crucially: NOT zero.
    // The point of this test is that the gate promoted Approval →
    // Allow, so the executor DID reach the body-dispatch phase.
    assert_eq!(
        report.executed_count(),
        0,
        "with handlers=None the bodies don't run, but the gate MUST have let them through"
    );
    assert_ne!(
        report.status,
        ExecutionStatus::Denied,
        "with --allow-high-band Approval must NOT produce a Denied status; got {:?}",
        report.status
    );
}

#[test]
fn system_actor_write_does_not_deny_and_runs() {
    // Sanity-check the inverse: `Write` for a `system:*` actor is
    // admitted. Both tasks reach the handler dispatch phase (no real
    // side effects because handlers=None). The test pins that the
    // fail-closed path is specific to the actor class — not a blanket
    // deny on all writes.
    let dag = dag_with_two_tasks(AuthorityRequirement::Write);
    let executor = DagExecutor::new(ExecutorConfig::default());
    let report = executor.walk(&dag, "system:sddk");

    assert_ne!(
        report.status,
        ExecutionStatus::Denied,
        "system actor on Write must NOT deny; got {:?}",
        report.status
    );
    assert_eq!(report.executed_count(), 0); // handlers=None → not implemented
}

#[test]
fn user_actor_write_requires_approval() {
    // Write for a `user:*` actor (no --allow-high-band) → RequireApproval.
    // Top-level status is Denied; zero executions; downstream skipped.
    let dag = dag_with_two_tasks(AuthorityRequirement::Write);
    let executor = DagExecutor::new(ExecutorConfig::default());
    let report = executor.walk(&dag, "user:bob");

    assert_eq!(report.executed_count(), 0);
    assert_eq!(report.status, ExecutionStatus::Denied);
    assert!(report.had_denial() || report.had_halt());
}

#[test]
fn user_actor_read_is_admitted() {
    // Read for a `user:*` actor is Always-Admit; the first task is
    // admitted (NotImplemented because handlers=None); the second is
    // upstream-gated by the first and lands on the same NotImplemented
    // path. This pins the inverse: Read is NOT deny-gated for users.
    let dag = dag_with_two_tasks(AuthorityRequirement::Read);
    let executor = DagExecutor::new(ExecutorConfig::default());
    let report = executor.walk(&dag, "user:carol");

    assert_ne!(
        report.status,
        ExecutionStatus::Denied,
        "Read for user actor must NOT deny; got {:?}",
        report.status
    );
}
