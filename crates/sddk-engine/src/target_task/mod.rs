//! `target_task` — Target/Task workflow substrate (M6.2).
//!
//! Per [[SPEC-009]] and [[ADR-010]], a **Target** is a named user intent
//! (`change`, `verify`, `ship`, `recover`, `audit`) — distinct from a
//! domain `Goal`. A Target resolves to a deterministic Task DAG that
//! declares dependencies, side-effect class, authority requirement,
//! determinism, cacheability, retry policy, evidence contract, and
//! memory effects.
//!
//! This module is the typed substrate. It is *not* the executor —
//! execution lands in a later cycle. The substrate provides:
//!
//! - [`Target`]: a named user intent with task declarations.
//! - [`Task`]: the 11-field task contract from SPEC-009.
//! - [`TaskDag`]: a resolved DAG with topological ordering and cycle
//!   detection.
//! - [`TargetRegistry`]: lookup `target_name -> Target`.
//!
//! All types are pure values (no I/O).

#![allow(missing_docs)]

pub mod builtin;
pub mod dag;
pub mod executor;
pub mod handlers;
pub mod outcome;
pub mod registry;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Side-effect classification per SPEC-009.
///
/// Drives whether a task routes through the Authority Engine. Pure
/// tasks (no I/O, no state) bypass admission; side-effect tasks
/// require a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    /// Pure computation; no I/O, no state.
    Pure,
    /// Reads from a stable source (filesystem, ledger). Read-only.
    Read,
    /// Writes to a durable location. Requires capability.
    Write,
    /// Network egress. Requires capability + approval at high band.
    Network,
}

/// Authority requirement of a task.
///
/// The task declares the minimum authority band needed for execution.
/// The Authority Engine (M5) maps this to a `Capability` at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityRequirement {
    /// No authority needed (pure task).
    None,
    /// Read-only capability suffices.
    Read,
    /// Standard write capability needed.
    Write,
    /// High-band approval gate.
    Approval,
}

/// Determinism classification (SPEC-009 §Cache rule).
///
/// `Deterministic` tasks may be cached; `LLMInfluenced` outputs are
/// never silently reused as cache truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Determinism {
    /// Pure function of declared inputs; safe to cache.
    Deterministic,
    /// Output depends on external state; cache requires explicit check.
    ExternallyInfluenced,
    /// LLM-shaped output; never silently reused.
    LlmInfluenced,
}

/// Cacheability classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cacheability {
    /// Eligible for cache (inputs hash → outputs hash).
    Cacheable,
    /// Cacheable only with explicit freshness check.
    CacheableWithCheck,
    /// Never cache (e.g. side-effect on remote system).
    Never,
}

/// Retry policy for failed tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryPolicy {
    /// No retry; failure is fatal.
    NoRetry,
    /// Retry up to N times with backoff.
    Bounded { max_attempts: u32 },
    /// Unlimited retry until success or operator kill.
    Forever,
}

/// Memory effect declaration.
///
/// Tasks may write to the Decision Memory (M4) as a side effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEffect {
    /// No memory writes.
    None,
    /// Append-only memory fact.
    AppendFact,
    /// Replace existing memory entry.
    Replace,
    /// Snapshot (full rewrite of a subtree).
    Snapshot,
}

/// Evidence contract — the canonical-event-log requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceContract {
    /// No evidence required.
    None,
    /// Single fact log entry per execution.
    OneFact,
    /// Fact + receipt + cas-anchored artifacts.
    FullReceipt,
}

/// A typed task per SPEC-009.
///
/// The 11-field contract is enforced by the type system: every field is
/// required at construction. There is no "partial task" state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Stable identifier within the parent target.
    pub id: String,
    /// Declared inputs (path or logical id).
    pub inputs: Vec<String>,
    /// Declared outputs (path or logical id).
    pub outputs: Vec<String>,
    /// Tasks this one waits on (same target).
    pub depends_on: Vec<String>,
    /// Side-effect class.
    pub side_effect_class: SideEffectClass,
    /// Authority requirement.
    pub authority_requirement: AuthorityRequirement,
    /// Determinism classification.
    pub determinism: Determinism,
    /// Cacheability classification.
    pub cacheability: Cacheability,
    /// Retry policy.
    pub retry_policy: RetryPolicy,
    /// Evidence contract.
    pub evidence_contract: EvidenceContract,
    /// Memory effect declaration.
    pub memory_effects: MemoryEffect,
    /// Whether a real task body is wired for this task. `false` marks a
    /// declaration stub: the executor reports `not_implemented` instead of
    /// a fabricated `executed` receipt (SP-07 false-success fix).
    pub has_body: bool,
}

/// A named user intent per SPEC-009 / ADR-010.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// Stable target name (e.g. "change", "verify", "ship").
    pub name: String,
    /// Human-readable description.
    pub about: String,
    /// Owned task declarations, in declaration order.
    pub tasks: Vec<Task>,
}

/// Topologically-ordered task DAG for a target.
///
/// `order` is the execution sequence (deps first). The original task
/// ids are preserved in `by_id` for O(1) lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDag {
    /// Target name (parent).
    pub target: String,
    /// Topologically ordered task ids.
    pub order: Vec<String>,
    /// Index for fast lookup.
    pub by_id: BTreeMap<String, Task>,
}

/// Resolution result for a target lookup.
///
/// Distinct from the DAG itself so the engine and CLI can render
/// different projections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetResolution {
    /// Target name.
    pub target: String,
    /// Resolved DAG (if validation succeeded).
    pub dag: Option<TaskDag>,
    /// Validation message when `dag` is `None`.
    pub validation: String,
}

/// Errors raised by the target_task substrate.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TargetTaskError {
    /// Target name not in registry.
    #[error("unknown target `{0}`")]
    UnknownTarget(String),
    /// DAG has a cycle (fail-closed per SPEC-009).
    #[error("cycle detected in target `{target}` involving task `{task}`")]
    CycleDetected { target: String, task: String },
    /// DAG references an unknown task id in `depends_on`.
    #[error("task `{task}` in target `{target}` depends on unknown task `{dep}`")]
    UnknownDependency {
        target: String,
        task: String,
        dep: String,
    },
    /// Two tasks share the same id.
    #[error("duplicate task id `{0}` in target `{1}`")]
    DuplicateTaskId(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
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
            has_body: false,
        }
    }

    #[test]
    fn task_construction_preserves_all_eleven_fields() {
        let t = Task {
            id: "build".into(),
            inputs: vec!["src/".into()],
            outputs: vec!["target/".into()],
            depends_on: vec!["resolve".into()],
            side_effect_class: SideEffectClass::Write,
            authority_requirement: AuthorityRequirement::Write,
            determinism: Determinism::Deterministic,
            cacheability: Cacheability::Cacheable,
            retry_policy: RetryPolicy::Bounded { max_attempts: 3 },
            evidence_contract: EvidenceContract::FullReceipt,
            memory_effects: MemoryEffect::AppendFact,
            has_body: false,
        };
        assert_eq!(t.id, "build");
        assert_eq!(t.depends_on, vec!["resolve"]);
        assert_eq!(t.retry_policy, RetryPolicy::Bounded { max_attempts: 3 });
    }

    #[test]
    fn target_groups_tasks_under_a_named_intent() {
        let target = Target {
            name: "status".into(),
            about: "Report workspace status".into(),
            tasks: vec![minimal_task("resolve"), minimal_task("render")],
        };
        assert_eq!(target.tasks.len(), 2);
        assert_eq!(target.tasks[0].id, "resolve");
    }

    #[test]
    fn unknown_target_error_carries_the_name() {
        let e = TargetTaskError::UnknownTarget("nope".into());
        assert_eq!(e.to_string(), "unknown target `nope`");
    }

    #[test]
    fn cycle_detected_error_names_target_and_task() {
        let e = TargetTaskError::CycleDetected {
            target: "t1".into(),
            task: "build".into(),
        };
        assert!(e.to_string().contains("t1"));
        assert!(e.to_string().contains("build"));
    }
}
