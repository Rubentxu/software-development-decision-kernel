//! Built-in targets shipped with the engine (M6.2).
//!
//! Per SPEC-009 §UX, the canonical targets are `change`, `verify`,
//! `ship`, `recover`, `audit`. This cycle ships the four that already
//! have shadow routers (`status`, `run`, `ship`, `recover`); the
//! remaining three (`change`, `verify`, `audit`) belong to M6.3
//! when their task bodies are wired.
//!
//! All tasks declared here are *stubs* (no-op task bodies). The
//! substrate only requires that the DAG be declared and validated;
//! execution is a later cycle.

#![allow(missing_docs)]

use super::{
    AuthorityRequirement, Cacheability, Determinism, EvidenceContract, MemoryEffect, Target, Task,
};

fn stub_task(
    id: &str,
    deps: &[&str],
    side_effect: super::SideEffectClass,
    authority: AuthorityRequirement,
    determinism: Determinism,
    evidence: EvidenceContract,
    memory: MemoryEffect,
) -> Task {
    Task {
        id: id.into(),
        inputs: vec![],
        outputs: vec![],
        depends_on: deps.iter().map(|s| s.to_string()).collect(),
        side_effect_class: side_effect,
        authority_requirement: authority,
        determinism,
        cacheability: match determinism {
            Determinism::Deterministic => Cacheability::Cacheable,
            Determinism::ExternallyInfluenced => Cacheability::CacheableWithCheck,
            Determinism::LlmInfluenced => Cacheability::Never,
        },
        retry_policy: super::RetryPolicy::Bounded { max_attempts: 3 },
        evidence_contract: evidence,
        memory_effects: memory,
        // SP-07: built-in targets declare intent, not wired bodies. The
        // executor must report `not_implemented` / `degraded` until M6.3
        // wires real task bodies — never a fabricated `executed`.
        has_body: false,
    }
}

/// Target: `status` — report workspace status (read-only).
pub fn status_target() -> Target {
    Target {
        name: "status".into(),
        about: "Report workspace status (deterministic, read-only)".into(),
        tasks: vec![
            stub_task(
                "context.resolve",
                &[],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::Deterministic,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "ledger.snapshot",
                &["context.resolve"],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::ExternallyInfluenced,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "render",
                &["ledger.snapshot"],
                super::SideEffectClass::Pure,
                AuthorityRequirement::None,
                Determinism::Deterministic,
                EvidenceContract::None,
                MemoryEffect::None,
            ),
        ],
    }
}

/// Target: `run` — execute the active cycle (full workflow).
pub fn run_target() -> Target {
    Target {
        name: "run".into(),
        about: "Execute the active cycle end-to-end".into(),
        tasks: vec![
            stub_task(
                "context.resolve",
                &[],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::Deterministic,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "plan.validate",
                &["context.resolve"],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::Deterministic,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "assurance",
                &["plan.validate"],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::ExternallyInfluenced,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "evidence.collect",
                &["assurance"],
                super::SideEffectClass::Write,
                AuthorityRequirement::Write,
                Determinism::Deterministic,
                EvidenceContract::FullReceipt,
                MemoryEffect::AppendFact,
            ),
            stub_task(
                "apply",
                &["evidence.collect"],
                super::SideEffectClass::Write,
                AuthorityRequirement::Write,
                Determinism::Deterministic,
                EvidenceContract::FullReceipt,
                MemoryEffect::AppendFact,
            ),
        ],
    }
}

/// Target: `ship` — publish a release (git + receipt).
pub fn ship_target() -> Target {
    Target {
        name: "ship".into(),
        about: "Publish a release via local Git + GH release".into(),
        tasks: vec![
            stub_task(
                "manifest.verify",
                &[],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::Deterministic,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "build.binary",
                &["manifest.verify"],
                super::SideEffectClass::Write,
                AuthorityRequirement::Write,
                Determinism::Deterministic,
                EvidenceContract::FullReceipt,
                MemoryEffect::None,
            ),
            stub_task(
                "bundle.tarball",
                &["build.binary"],
                super::SideEffectClass::Write,
                AuthorityRequirement::Write,
                Determinism::Deterministic,
                EvidenceContract::FullReceipt,
                MemoryEffect::None,
            ),
            stub_task(
                "gh.release.publish",
                &["bundle.tarball"],
                super::SideEffectClass::Network,
                AuthorityRequirement::Approval,
                Determinism::ExternallyInfluenced,
                EvidenceContract::FullReceipt,
                MemoryEffect::AppendFact,
            ),
        ],
    }
}

/// Target: `recover` — restore a corrupted cycle from receipts.
pub fn recover_target() -> Target {
    Target {
        name: "recover".into(),
        about: "Restore a corrupted cycle from durable receipts".into(),
        tasks: vec![
            stub_task(
                "ledger.scan",
                &[],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::ExternallyInfluenced,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "receipt.replay",
                &["ledger.scan"],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::Deterministic,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "graph.rebuild",
                &["receipt.replay"],
                super::SideEffectClass::Write,
                AuthorityRequirement::Write,
                Determinism::Deterministic,
                EvidenceContract::FullReceipt,
                MemoryEffect::Replace,
            ),
        ],
    }
}

/// Target: `change` — create a planning work item (M6.3 typed DAG).
///
/// Wraps the M6.1 shadow router `change` with a typed DAG so the
/// engine and CLI converge on the same shape.
pub fn change_target() -> Target {
    Target {
        name: "change".into(),
        about: "Create a planning work item in a cycle".into(),
        tasks: vec![
            stub_task(
                "context.resolve",
                &[],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::Deterministic,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "plan.workitem.create",
                &["context.resolve"],
                super::SideEffectClass::Write,
                AuthorityRequirement::Write,
                Determinism::Deterministic,
                EvidenceContract::FullReceipt,
                MemoryEffect::AppendFact,
            ),
        ],
    }
}

/// Target: `verify` — cross-cutting verification (M6.3 typed DAG).
pub fn verify_target() -> Target {
    Target {
        name: "verify".into(),
        about: "Verify ledger continuity and capability policy snapshot".into(),
        tasks: vec![
            stub_task(
                "ledger.verify",
                &[],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::Deterministic,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "capability.status",
                &["ledger.verify"],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::ExternallyInfluenced,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
        ],
    }
}

/// Target: `audit` — cross-cutting audit (M6.3 typed DAG).
pub fn audit_target() -> Target {
    Target {
        name: "audit".into(),
        about: "Audit memory reflog + ledger events".into(),
        tasks: vec![
            stub_task(
                "memory.reflog",
                &[],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::ExternallyInfluenced,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
            stub_task(
                "ledger.events",
                &["memory.reflog"],
                super::SideEffectClass::Read,
                AuthorityRequirement::Read,
                Determinism::ExternallyInfluenced,
                EvidenceContract::OneFact,
                MemoryEffect::None,
            ),
        ],
    }
}

/// All built-in targets, in declaration order.
pub fn builtin_targets() -> Vec<Target> {
    vec![
        status_target(),
        run_target(),
        ship_target(),
        recover_target(),
        change_target(),
        verify_target(),
        audit_target(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target_task::dag::resolve_dag;

    #[test]
    fn status_target_resolves_to_three_tasks() {
        let dag = resolve_dag(&status_target()).unwrap();
        assert_eq!(dag.order.len(), 3);
        assert_eq!(dag.order.first().unwrap(), "context.resolve");
        assert_eq!(dag.order.last().unwrap(), "render");
    }

    #[test]
    fn run_target_resolves_to_five_tasks() {
        let dag = resolve_dag(&run_target()).unwrap();
        assert_eq!(dag.order.len(), 5);
        assert_eq!(dag.order.last().unwrap(), "apply");
    }

    #[test]
    fn ship_target_publishes_after_bundle() {
        let dag = resolve_dag(&ship_target()).unwrap();
        let bundle = dag
            .order
            .iter()
            .position(|x| x == "bundle.tarball")
            .unwrap();
        let publish = dag
            .order
            .iter()
            .position(|x| x == "gh.release.publish")
            .unwrap();
        assert!(bundle < publish);
    }

    #[test]
    fn recover_target_rebuilds_after_replay() {
        let dag = resolve_dag(&recover_target()).unwrap();
        let replay = dag
            .order
            .iter()
            .position(|x| x == "receipt.replay")
            .unwrap();
        let rebuild = dag.order.iter().position(|x| x == "graph.rebuild").unwrap();
        assert!(replay < rebuild);
    }

    #[test]
    fn builtin_targets_all_resolve_without_cycle() {
        for t in builtin_targets() {
            resolve_dag(&t)
                .unwrap_or_else(|e| panic!("built-in target `{}` failed to resolve: {e}", t.name));
        }
    }
}
