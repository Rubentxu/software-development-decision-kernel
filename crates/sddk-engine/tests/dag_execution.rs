// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// dag_execution.rs — SPEC-M6.3: 6 integration scenarios covering the
// DAG executor with per-task authority gating.

use sddk_engine::target_task::executor::{DagExecutor, ExecutorConfig};
use sddk_engine::target_task::outcome::{ExecutionStatus, TaskStatus};
use sddk_engine::target_task::registry::TargetRegistry;

fn registry() -> TargetRegistry {
    TargetRegistry::with_builtins()
}

fn executor(dry_run: bool, allow_high_band: bool) -> DagExecutor {
    DagExecutor::new(ExecutorConfig {
        dry_run,
        allow_high_band,
    })
}

// SC-M6.3-1: Dry-run on `status` admits all 3 tasks without bodies.
#[test]
fn sc_m6_3_1_status_dry_run_admits_all_tasks() {
    let r = registry();
    let report = executor(true, false).run(&r, "status", "system");
    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.tasks.len(), 3);
    assert!(report.dry_run);
    for t in &report.tasks {
        assert_eq!(t.status, TaskStatus::Skipped);
        assert!(t.note.contains("admitted") || t.note.contains("read access"));
    }
}

// SC-M6.3-2: `run` target with system actor executes all 5 tasks in topo order.
#[test]
fn sc_m6_3_2_run_target_executes_in_topo_order() {
    let r = registry();
    let report = executor(false, false).run(&r, "run", "system");
    assert_eq!(report.status, ExecutionStatus::Succeeded);
    let names: Vec<&str> = report.tasks.iter().map(|t| t.task_id.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "context.resolve",
            "plan.validate",
            "assurance",
            "evidence.collect",
            "apply",
        ]
    );
    for t in &report.tasks {
        assert_eq!(t.status, TaskStatus::Executed);
    }
}

// SC-M6.3-3: Ship target halts at `gh.release.publish` (Approval gate).
#[test]
fn sc_m6_3_3_ship_target_halts_at_publish_under_system_actor() {
    let r = registry();
    // System actor admits Write tasks; Approval still halts.
    let report = executor(false, false).run(&r, "ship", "system");
    assert_eq!(report.status, ExecutionStatus::Denied);
    let publish = report
        .tasks
        .iter()
        .find(|t| t.task_id == "gh.release.publish")
        .expect("publish task");
    assert_eq!(publish.status, TaskStatus::Skipped);
    assert!(publish.note.contains("awaiting approval"));
    // Earlier tasks executed.
    let manifest = &report.tasks[0];
    assert_eq!(manifest.task_id, "manifest.verify");
    assert_eq!(manifest.status, TaskStatus::Executed);
}

#[test]
fn sc_m6_3_3b_allow_high_band_lets_ship_target_succeed() {
    let r = registry();
    let report = executor(false, true).run(&r, "ship", "system");
    assert_eq!(report.status, ExecutionStatus::Succeeded);
    assert_eq!(report.executed_count(), 4);
}

// Regression: allow_high_band must bypass RequireApproval for user actors
// (system actors are already Allow, so this exercises the Write → RequireApproval
// branch under --allow-high-band).
#[test]
fn sc_m6_3_3c_allow_high_band_bypasses_user_write_approval() {
    let r = registry();
    let report = executor(false, true).run(&r, "ship", "user:alice");
    assert_eq!(report.status, ExecutionStatus::Succeeded);
    assert_eq!(report.executed_count(), 4);
    for t in &report.tasks {
        assert_eq!(t.status, TaskStatus::Executed);
    }
}

// SC-M6.3-4: Unknown target name returns a Failed report with empty tasks.
#[test]
fn sc_m6_3_4_unknown_target_returns_failed_report() {
    let r = registry();
    let report = executor(false, false).run(&r, "ghost", "system");
    assert_eq!(report.status, ExecutionStatus::Failed);
    assert_eq!(report.tasks.len(), 0);
    assert!(report.validation.contains("unknown target"));
}

// SC-M6.3-5: Dry-run on `ship` emits all Skipped outcomes with admit notes.
#[test]
fn sc_m6_3_5_ship_dry_run_has_no_denial() {
    let r = registry();
    let report = executor(true, false).run(&r, "ship", "system");
    assert_eq!(report.status, ExecutionStatus::DryRun);
    assert_eq!(report.tasks.len(), 4);
    for t in &report.tasks {
        // Dry-run reports Skipped even for Approval tasks (no halt).
        assert_eq!(t.status, TaskStatus::Skipped);
        assert!(!t.note.contains("upstream"));
    }
}

// SC-M6.3-6: Counter methods on `ExecutionReport` are accurate.
#[test]
fn sc_m6_3_6_execution_counters_are_accurate() {
    let r = registry();
    let report = executor(false, false).run(&r, "ship", "system");
    // 3 executed (manifest, build, bundle), 1 skipped (publish).
    assert_eq!(report.executed_count(), 3);
    assert_eq!(report.skipped_count(), 1);
    assert!(report.had_halt());
}

// SC-M6.3 supplementary: change/verify/audit typed DAGs execute.
#[test]
fn change_verify_audit_targets_execute_with_system_actor() {
    let r = registry();
    for target in ["change", "verify", "audit"] {
        let report = executor(false, false).run(&r, target, "system");
        // All tasks use Read or Write; system actor admits all.
        assert_eq!(
            report.status,
            ExecutionStatus::Succeeded,
            "target {target} should succeed: {:?}",
            report
        );
        assert!(report.executed_count() >= 1);
    }
}
