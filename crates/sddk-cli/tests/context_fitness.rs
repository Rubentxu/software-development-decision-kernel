//! A2-S1 — Context/dependency fitness suite.
//!
//! Executable ratchets for the context-first boundary rules. They fail closed:
//! a forbidden edge (or a new root-level context module without an ADR) fails
//! the test. Provider/host SDK names are checked textually because those SDKs
//! are not yet dependencies; the guard prevents them from being introduced.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out
}

/// Provider/RPC/host SDK names that must never appear in domain or knowledge code.
const FORBIDDEN_SDK_TOKENS: &[&str] = &[
    "prost",
    "tonic",
    "grpc",
    "cognicode",
    "chronos",
    "jcode",
    "reqwest",
];

/// Detect *identifier* use of a forbidden crate (not string literals such as
/// pack ids): `use <t>`, `<t>::`, or `extern crate <t>`.
fn offending(path: &Path, tokens: &[&str]) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let lower = line.to_lowercase();
        for t in tokens {
            let used = lower.contains(&format!("use {t}"))
                || lower.contains(&format!("{t}::"))
                || lower.contains(&format!("extern crate {t}"));
            if used {
                hits.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
            }
        }
    }
    hits
}

/// Forbidden *crate dependencies* on the domain manifest.
#[test]
fn no_domain_manifest_provider_or_rpc_crate_dependency() {
    let manifest = std::fs::read_to_string(repo_root().join("crates/sddk-domain/Cargo.toml"))
        .expect("domain manifest");
    let mut violations = Vec::new();
    for line in manifest.lines() {
        let l = line.trim().to_lowercase();
        for t in ["prost", "tonic", "cognicode", "chronos", "jcode", "reqwest"] {
            if l.starts_with(&format!("{t} "))
                || l.starts_with(&format!("{t}="))
                || l.starts_with(&format!("{t} ="))
            {
                violations.push(line.to_string());
            }
        }
    }
    assert!(
        violations.is_empty(),
        "sddk-domain must not depend on provider/RPC/host SDK crates: {violations:?}"
    );
}

#[test]
fn no_domain_to_rpc_or_provider_or_host_sdk() {
    let root = repo_root().join("crates/sddk-domain/src");
    let mut violations = Vec::new();
    for f in rust_sources(&root) {
        violations.extend(offending(&f, FORBIDDEN_SDK_TOKENS));
    }
    assert!(
        violations.is_empty(),
        "sddk-domain must not reference RPC/provider/host SDK types:\n{}",
        violations.join("\n")
    );
}

#[test]
fn no_knowledge_to_provider_sdk() {
    let root = repo_root().join("crates/sddk-engine/src");
    let knowledge_modules = [
        "semantic_graph.rs",
        "semantic_node.rs",
        "semantic_kind.rs",
        "vault_boundary.rs",
        "why_queries.rs",
    ];
    let provider_tokens = ["cognicode", "chronos", "prost", "tonic"];
    let mut violations = Vec::new();
    for m in knowledge_modules {
        let f = root.join(m);
        if f.exists() {
            violations.extend(offending(&f, &provider_tokens));
        }
    }
    assert!(
        violations.is_empty(),
        "knowledge must not depend on provider SDKs:\n{}",
        violations.join("\n")
    );
}

/// The alignment context (when it exists) must not import governance impl,
/// the AuthorityEngine, or the instruction compiler.
#[test]
fn no_alignment_to_governance_authority_or_instruction_compiler() {
    let tokens = ["authority_engine", "instruction_compiler", "authority::"];
    let mut violations = Vec::new();
    for base in ["crates/sddk-engine/src", "crates/sddk-domain/src"] {
        let root = repo_root().join(base);
        for f in rust_sources(&root) {
            let p = f.to_string_lossy().to_lowercase();
            if !p.contains("alignment") {
                continue;
            }
            violations.extend(offending(&f, &tokens));
        }
    }
    assert!(
        violations.is_empty(),
        "alignment must stay independent of governance/instruction compiler:\n{}",
        violations.join("\n")
    );
}

/// Workbook/projection code must never write canonical facts.
#[test]
fn no_workbook_canonical_write() {
    let tokens = ["INSERT INTO", "UPDATE ", "DELETE FROM"];
    let mut violations = Vec::new();
    for base in ["crates/sddk-engine/src", "crates/sddk-domain/src"] {
        let root = repo_root().join(base);
        for f in rust_sources(&root) {
            let name = f.file_name().unwrap().to_string_lossy().to_lowercase();
            if !(name.contains("workbook") || name.contains("cockpit")) {
                continue;
            }
            violations.extend(offending(&f, &tokens));
        }
    }
    assert!(
        violations.is_empty(),
        "workbooks/projections must not write canonical facts:\n{}",
        violations.join("\n")
    );
}

/// Generic agentic contracts must not contain JCode-specific types.
#[test]
fn no_jcode_type_in_generic_agentic_contract() {
    let tokens = ["jcode", "j_code"];
    let mut violations = Vec::new();
    for base in ["crates/sddk-domain/src", "crates/sddk-engine/src"] {
        let root = repo_root().join(base);
        for f in rust_sources(&root) {
            let p = f.to_string_lossy().to_lowercase();
            // Agentic-workspace contracts, if/when they exist, are generic.
            if !(p.contains("agentic") || p.contains("session_binding")) {
                continue;
            }
            violations.extend(offending(&f, &tokens));
        }
    }
    assert!(
        violations.is_empty(),
        "generic agentic contracts must not reference JCode types:\n{}",
        violations.join("\n")
    );
}

/// Root-level context modules may not be added after R0 without an ADR.
/// A new module is accepted iff its name is mentioned in some ADR document.
#[test]
fn no_new_root_level_context_module_without_adr() {
    let baseline: BTreeSet<&str> = BASELINE_ROOT_MODULES.iter().copied().collect();
    let adr_dir = repo_root().join("docs/architecture/adrs");
    let mut adr_text = String::new();
    if let Ok(entries) = std::fs::read_dir(&adr_dir) {
        for e in entries.flatten() {
            if let Ok(t) = std::fs::read_to_string(e.path()) {
                adr_text.push_str(&t.to_lowercase());
                adr_text.push('\n');
            }
        }
    }

    let mut violations = Vec::new();
    for base in ["crates/sddk-domain/src", "crates/sddk-engine/src"] {
        let root = repo_root().join(base);
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let stem = name.trim_end_matches(".rs").to_string();
            if name.starts_with('.') || stem == "lib" {
                continue;
            }
            if baseline.contains(stem.as_str()) {
                continue;
            }
            if !adr_text.contains(&stem.to_lowercase()) {
                violations.push(format!("{base}/{name}"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "new root-level context modules require an ADR (none found): {violations:?}"
    );
}

/// Baseline root modules at the certified R0 starting point.
const BASELINE_ROOT_MODULES: &[&str] = &[
    // sddk-domain
    "backlog",
    "channel",
    "compiler",
    "context_read",
    "cycle",
    "delivery_kind",
    "error",
    "event_envelope",
    "event_registry",
    "evidence",
    "execution_graph_compiler",
    "execution_scope",
    "fork",
    "format",
    "goal",
    "graph",
    "identity",
    "legacy",
    "macros",
    "metrics",
    "models",
    "operator_contract",
    "pack",
    "plan_revision",
    "planning",
    "ports",
    "projections",
    "proposal",
    "replay",
    "rules",
    "schema",
    "spine",
    "staleness",
    "test_adapters",
    "test_apply",
    "test_evidence",
    "test_model",
    "test_ports",
    "test_select",
    "transition_ast",
    "uat",
    "validator",
    "view",
    "workflow",
    "workflow_ir",
    "workflow_run",
    // sddk-engine
    "active_graph",
    "active_graph_digest",
    "active_graph_drift",
    "active_graph_view",
    "adoption",
    "agent_contribution_envelope",
    "agent_host",
    "agent_role_contract",
    "authority",
    "authority_engine",
    "build_work_graph",
    "canonical_event_log",
    "cas_object_store",
    "change_contract",
    "circuit_breaker",
    "cockpit_observability",
    "cockpit_views",
    "cold_start",
    "context_capsule",
    "context_compiler",
    "continuation_candidate",
    "converge_verification",
    "cycle_narrative",
    "cycle_pause",
    "cycle_replan",
    "cycle_summary",
    "cycle_supersede",
    "decision_lab_baseline",
    "decision_lab_experimental",
    "decision_memory",
    "decision_plane_gate",
    "durable_map_fanout",
    "engineering_assurance",
    "engineering_assurance_resolvers",
    "event_bus",
    "evidence_backed_promotion",
    "evidence_ref",
    "evidence_relation_mapping",
    "execution_controller",
    "experience_episodes",
    "fingerprint",
    "ga_publish",
    "gate_error",
    "gate_evaluator",
    "gate_signing",
    "generic_pack_contracts",
    "human_decision",
    "human_resume_view",
    "inc_generator",
    "incident_pack",
    "integrate_parity",
    "join_guard",
    "lab_promotion",
    "operator",
    "orchestration_synthesis",
    "pack_agnosticity",
    "pack_registry",
    "paths",
    "production_hardening",
    "projector_registry",
    "completion_provider_router",
    "receipt_writers",
    "release_readiness",
    "replay_proof",
    "retry",
    "revision_substrate",
    "risk_approval_policy",
    "rules",
    "run_view",
    "secretary_closed_set",
    "secretary_l0",
    "secretary_l1",
    "secretary_l2_replan",
    "security_upgrade_rollback",
    "semantic_graph",
    "semantic_kind",
    "semantic_node",
    "signed_gates",
    "spike_sp06",
    "state_class_lint",
    "strategy_comparison",
    "strategy_experiments",
    "supply_chain_artifacts",
    "target_task",
    "task_executor",
    "tasks",
    "telemetry",
    "typed_child_output",
    "typed_reduce_aggregator",
    "uat_lifecycle",
    "uat_pack",
    "up_to_date",
    "vault_boundary",
    "version",
    "why_queries",
    "workflow_metrics",
    "workflow_runtime",
];
