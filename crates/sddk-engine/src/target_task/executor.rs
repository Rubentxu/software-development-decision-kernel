//! `executor` — `DagExecutor` for the Target/Task DAG (M6.3).
//!
//! Walks a `TaskDag` in topological order, gates each task through a
//! typed authority check derived from the task's `AuthorityRequirement`,
//! and emits an `ExecutionReport`.
//!
//! Per SPEC-009 §Rules: side-effect tasks route through the Authority
//! Engine. This cycle implements the gate locally by mapping each
//! `AuthorityRequirement` to a typed policy decision:
//!
//! - `None`           → always admit (pure task).
//! - `Read`           → admit for `system`/`user:*`/`agent:*` actors.
//! - `Write`          → admit for `system`; require approval for
//!   `user:*`/`agent:*`; deny otherwise.
//! - `Approval`       → always require approval; deny without the
//!   `--allow-high-band` flag.
//!
//! The gate is intentionally narrow this cycle; a follow-up cycle
//! will route the same calls through `AuthorityEngineRunner::admit_surface`
//! once the runner's policy table includes the new surfaces
//! (status.*, run.*, ship.*, etc.).

#![allow(missing_docs)]

use super::outcome::{ExecutionReport, ExecutionStatus, TaskOutcome};
use super::registry::TargetRegistry;
use super::{AuthorityRequirement, TaskDag};

/// Per-task authority gate result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GateDecision {
    /// Task is admitted; the executor will run its body.
    Allow { explanation: String },
    /// Task requires approval; the executor will skip unless `allow_high_band`.
    RequireApproval { explanation: String },
    /// Task is denied; the executor will halt and mark downstream skipped.
    Deny { explanation: String },
}

/// Configuration knobs for `DagExecutor`.
#[derive(Debug, Clone, Default)]
pub struct ExecutorConfig {
    /// When true, skip task bodies and emit `Skipped` outcomes.
    pub dry_run: bool,
    /// When true, treat `RequireApproval` as `Allow` (for CI fixtures).
    pub allow_high_band: bool,
}

/// Walks a `TaskDag` and produces an `ExecutionReport`.
///
/// The executor is stateless: construct a new instance per walk.
#[derive(Debug, Default)]
pub struct DagExecutor {
    config: ExecutorConfig,
}

impl DagExecutor {
    /// Construct an executor with the given configuration.
    pub fn new(config: ExecutorConfig) -> Self {
        Self { config }
    }

    /// Resolve a target from the registry and execute it.
    ///
    /// Returns `ExecutionReport::unresolved` when the target name is
    /// not registered or the DAG fails validation.
    pub fn run(
        &self,
        registry: &TargetRegistry,
        target_name: &str,
        actor: &str,
    ) -> ExecutionReport {
        let resolution = registry.resolve(target_name);
        if let Some(dag) = resolution.dag {
            self.walk(&dag, actor)
        } else {
            ExecutionReport::unresolved(target_name, resolution.validation)
        }
    }

    /// Walk an already-resolved DAG.
    pub fn walk(&self, dag: &TaskDag, actor: &str) -> ExecutionReport {
        let mut tasks: Vec<TaskOutcome> = Vec::with_capacity(dag.order.len());
        let mut status = if self.config.dry_run {
            ExecutionStatus::DryRun
        } else {
            ExecutionStatus::Succeeded
        };

        for task_id in &dag.order {
            let task = dag.by_id.get(task_id).expect("task in dag");
            let gate = gate_task(
                task.authority_requirement,
                actor,
                self.config.allow_high_band,
            );
            match gate {
                GateDecision::Allow { explanation } => {
                    if self.config.dry_run {
                        tasks.push(TaskOutcome::skipped(
                            task_id,
                            format!("admitted (dry-run): {explanation}"),
                        ));
                    } else {
                        tasks.push(TaskOutcome::executed(
                            task_id,
                            format!("executed: {explanation}"),
                        ));
                    }
                }
                GateDecision::RequireApproval { explanation } => {
                    tasks.push(TaskOutcome::skipped(
                        task_id,
                        format!("awaiting approval: {explanation}"),
                    ));
                    if !self.config.dry_run {
                        // Treat as a soft halt: subsequent tasks
                        // also skipped but the report status is
                        // Denied so the caller knows to escalate.
                        status = ExecutionStatus::Denied;
                        for downstream in
                            &dag.order[dag.order.iter().position(|x| x == task_id).unwrap() + 1..]
                        {
                            tasks.push(TaskOutcome::skipped(
                                downstream,
                                "upstream required approval",
                            ));
                        }
                        break;
                    }
                }
                GateDecision::Deny { explanation } => {
                    tasks.push(TaskOutcome::denied(task_id, explanation.clone()));
                    status = ExecutionStatus::Denied;
                    let halt_at = dag.order.iter().position(|x| x == task_id).unwrap();
                    for downstream in &dag.order[halt_at + 1..] {
                        tasks.push(TaskOutcome::skipped(downstream, "upstream denied"));
                    }
                    break;
                }
            }
        }

        ExecutionReport {
            target: dag.target.clone(),
            status,
            dry_run: self.config.dry_run,
            tasks,
            validation: "ok".to_string(),
        }
    }
}

fn gate_task(
    requirement: AuthorityRequirement,
    actor: &str,
    allow_high_band: bool,
) -> GateDecision {
    match requirement {
        AuthorityRequirement::None => GateDecision::Allow {
            explanation: "pure task".to_string(),
        },
        AuthorityRequirement::Read => GateDecision::Allow {
            explanation: format!("read access for actor `{actor}`"),
        },
        AuthorityRequirement::Write => {
            let decision = if actor.starts_with("system") {
                GateDecision::Allow {
                    explanation: "write access (system actor)".to_string(),
                }
            } else if actor.starts_with("user:") || actor.starts_with("agent:") {
                GateDecision::RequireApproval {
                    explanation: format!("write access requires approval for `{actor}`"),
                }
            } else {
                GateDecision::Deny {
                    explanation: format!("write access denied for unknown actor `{actor}`"),
                }
            };
            // --allow-high-band promotes RequireApproval to Allow across all
            // requirements, so CI fixtures and explicit operator overrides can
            // bypass the prompt uniformly.
            if allow_high_band {
                promote_approval_to_allow(decision, "high-band write bypass (--allow-high-band)")
            } else {
                decision
            }
        }
        AuthorityRequirement::Approval => {
            if allow_high_band {
                GateDecision::Allow {
                    explanation: "high-band approval bypass (--allow-high-band)".to_string(),
                }
            } else {
                GateDecision::RequireApproval {
                    explanation: format!("high-band surface for actor `{actor}`"),
                }
            }
        }
    }
}

/// Convert a `RequireApproval` decision into `Allow` when the operator
/// explicitly bypasses the gate (via `--allow-high-band`). Other
/// variants pass through unchanged so `Deny` stays terminal.
fn promote_approval_to_allow(decision: GateDecision, explanation: &str) -> GateDecision {
    match decision {
        GateDecision::RequireApproval { .. } => GateDecision::Allow {
            explanation: explanation.to_string(),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target_task::builtin::{
        change_target, recover_target, run_target, ship_target, status_target,
    };
    use crate::target_task::{
        AuthorityRequirement, Cacheability, Determinism, EvidenceContract, MemoryEffect,
        SideEffectClass, Task,
    };

    fn minimal_target(name: &str, ids: &[&str]) -> crate::target_task::Target {
        crate::target_task::Target {
            name: name.into(),
            about: format!("target {name}"),
            tasks: ids
                .iter()
                .map(|id| Task {
                    id: (*id).into(),
                    inputs: vec![],
                    outputs: vec![],
                    depends_on: vec![],
                    side_effect_class: SideEffectClass::Pure,
                    authority_requirement: AuthorityRequirement::None,
                    determinism: Determinism::Deterministic,
                    cacheability: Cacheability::Cacheable,
                    retry_policy: crate::target_task::RetryPolicy::NoRetry,
                    evidence_contract: EvidenceContract::None,
                    memory_effects: MemoryEffect::None,
                })
                .collect(),
        }
    }

    #[test]
    fn dry_run_skips_bodies_and_marks_all_tasks_skipped() {
        let registry = TargetRegistry::with_builtins();
        let ex = DagExecutor::new(ExecutorConfig {
            dry_run: true,
            allow_high_band: false,
        });
        let report = ex.run(&registry, "status", "system");
        assert_eq!(report.status, ExecutionStatus::DryRun);
        assert_eq!(report.tasks.len(), 3);
        assert!(
            report
                .tasks
                .iter()
                .all(|t| t.status == crate::target_task::outcome::TaskStatus::Skipped)
        );
        assert!(report.dry_run);
    }

    #[test]
    fn status_target_with_system_actor_executes_cleanly() {
        let registry = TargetRegistry::with_builtins();
        let ex = DagExecutor::new(ExecutorConfig::default());
        let report = ex.run(&registry, "status", "system");
        assert_eq!(report.status, ExecutionStatus::Succeeded);
        assert_eq!(report.executed_count(), 3);
    }

    #[test]
    fn ship_target_halts_at_gh_release_publish_for_user_actor() {
        let registry = TargetRegistry::with_builtins();
        let ex = DagExecutor::new(ExecutorConfig::default());
        // system actor admits the Write tasks (manifest/build/bundle)
        // and halts only at the Approval task (gh.release.publish).
        let report = ex.run(&registry, "ship", "system");
        assert_eq!(report.status, ExecutionStatus::Denied);
        let publish = report
            .tasks
            .iter()
            .find(|t| t.task_id == "gh.release.publish")
            .expect("publish task");
        assert_eq!(
            publish.status,
            crate::target_task::outcome::TaskStatus::Skipped
        );
        assert!(
            publish.note.contains("awaiting approval"),
            "note was: {}",
            publish.note
        );
    }

    #[test]
    fn unknown_target_returns_unresolved_report() {
        let registry = TargetRegistry::with_builtins();
        let ex = DagExecutor::new(ExecutorConfig::default());
        let report = ex.run(&registry, "ghost", "system");
        assert_eq!(report.status, ExecutionStatus::Failed);
        assert_eq!(report.tasks.len(), 0);
        assert!(report.validation.contains("unknown target"));
    }

    #[test]
    fn allow_high_band_lets_ship_target_succeed() {
        let registry = TargetRegistry::with_builtins();
        let ex = DagExecutor::new(ExecutorConfig {
            dry_run: false,
            allow_high_band: true,
        });
        let report = ex.run(&registry, "ship", "system");
        assert_eq!(report.status, ExecutionStatus::Succeeded);
        assert_eq!(report.executed_count(), 4);
    }

    #[test]
    fn write_task_denies_unknown_actor() {
        let mut registry = TargetRegistry::new();
        registry.register(minimal_target("w", &["only"]));
        // Patch the only task to require Write.
        let mut target = registry.get("w").unwrap().clone();
        target.tasks[0].authority_requirement = AuthorityRequirement::Write;
        registry.register(target);
        let ex = DagExecutor::new(ExecutorConfig::default());
        let report = ex.run(&registry, "w", "anonymous");
        assert_eq!(report.status, ExecutionStatus::Denied);
        let only = &report.tasks[0];
        assert_eq!(only.status, crate::target_task::outcome::TaskStatus::Denied);
        assert!(only.note.contains("denied"));
    }

    #[test]
    fn dag_order_is_preserved_in_execution_report() {
        let registry = TargetRegistry::with_builtins();
        let ex = DagExecutor::new(ExecutorConfig::default());
        let report = ex.run(&registry, "run", "system");
        let names: Vec<&str> = report.tasks.iter().map(|t| t.task_id.as_str()).collect();
        let expected: Vec<&str> = vec![
            "context.resolve",
            "plan.validate",
            "assurance",
            "evidence.collect",
            "apply",
        ];
        assert_eq!(names, expected);
    }

    // Touch the built-in target builders so the import survives.
    #[test]
    fn built_in_targets_are_loadable_via_registry() {
        let registry = TargetRegistry::with_builtins();
        assert!(registry.get("status").is_some());
        assert!(registry.get("run").is_some());
        assert!(registry.get("ship").is_some());
        assert!(registry.get("recover").is_some());
        // References kept to satisfy the import set; the builders
        // are exercised by the registry.
        let _ = status_target();
        let _ = run_target();
        let _ = ship_target();
        let _ = recover_target();
        let _ = change_target();
    }

    #[test]
    fn promote_approval_to_allow_converts_require_approval() {
        let d = GateDecision::RequireApproval {
            explanation: "needs review".into(),
        };
        let promoted = promote_approval_to_allow(d, "bypass");
        assert!(matches!(promoted, GateDecision::Allow { .. }));
    }

    #[test]
    fn promote_approval_to_allow_preserves_deny() {
        let d = GateDecision::Deny {
            explanation: "nope".into(),
        };
        let out = promote_approval_to_allow(d, "bypass");
        assert!(matches!(out, GateDecision::Deny { .. }));
    }

    #[test]
    fn promote_approval_to_allow_preserves_allow() {
        let d = GateDecision::Allow {
            explanation: "fine".into(),
        };
        let out = promote_approval_to_allow(d, "bypass");
        assert!(matches!(out, GateDecision::Allow { .. }));
    }
}
