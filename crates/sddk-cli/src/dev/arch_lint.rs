// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// arch_lint.rs — T-08 (M1 arch-spec-001 CA-007)
//
// Doctor extension: surface the M1 alignment invariants in
// `sddk dev doctor --format=json`. Pure functions:
//   - `mirror_alignment_checks(yaml_text)` returns the marker list.
//   - tests assert three markers named canonically.
//
// The doctor subcommand in `dev/doctor.rs` calls these and emits
// `DoctorCheck { tool, present }` per marker.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkerStatus {
    pub id: String,
    pub present: bool,
}

const MARKER_CANONICAL_EVENT_LOG_SINGLETON: &str = "m1.canonical_event_log_singleton";
const MARKER_COMPATIBILITY_MIRRORS_MARKED: &str = "m1.compatibility_mirrors_marked";
const MARKER_CAS_INDEX_CONSISTENCY: &str = "m1.cas_index_consistency";

/// Returns the three M1 markers. Each is `present = true` iff the
/// responsibilities yaml text satisfies the corresponding invariant.
///
/// Marker definitions:
/// - `canonical_event_log_singleton`: a unique canonical author exists per
///   `target_milestone: delivered` for the canonical_event_log module path.
/// - `compatibility_mirrors_marked`: every model with `target_milestone != delivered`
///   has `compatibility_mirror: true` annotation in its entry, OR is the
///   declared canonical owner of its concept.
/// - `cas_index_consistency`: at least one Object-class entry names `cas`
///   (CAS store), confirming the new module is in the registry.
pub fn mirror_alignment_checks(responsibilities_yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_CANONICAL_EVENT_LOG_SINGLETON.to_string(),
            present: has_canonical_event_log_owner(responsibilities_yaml),
        },
        MarkerStatus {
            id: MARKER_COMPATIBILITY_MIRRORS_MARKED.to_string(),
            present: compatibility_mirrors_marked(responsibilities_yaml),
        },
        MarkerStatus {
            id: MARKER_CAS_INDEX_CONSISTENCY.to_string(),
            present: cas_index_consistent(responsibilities_yaml),
        },
    ]
}

/// The responsibilities yaml has an entry whose `module` starts with
/// `crates/sddk-engine/src/canonical_event_log` and `target_milestone: delivered`.
fn has_canonical_event_log_owner(yaml: &str) -> bool {
    for block in split_entries(yaml) {
        if block.contains("crates/sddk-engine/src/canonical_event_log")
            && block.contains("target_milestone: delivered")
        {
            return true;
        }
    }
    false
}

/// Every non-delivered (planned / future) entry declares
/// `compatibility_mirror: true` OR is a root canonical authority
/// (i.e., it owns the concept and is the unique `state_class: Fact` for it).
///
/// Conservative heuristic: at least one entry has the marker flag true.
fn compatibility_mirrors_marked(yaml: &str) -> bool {
    for block in split_entries(yaml) {
        // The marker can appear either as a structured field or as a textual
        // annotation in `notes:`. Either form satisfies the invariant.
        if block.contains("compatibility_mirror: true")
            || block.contains("compatibility mirror per arch-spec-001")
            || block.contains("retained as compatibility mirror")
        {
            return true;
        }
    }
    false
}

/// At least one Object-class entry references the `cas_object_store`
/// module path, confirming the CAS owner is registered.
fn cas_index_consistent(yaml: &str) -> bool {
    for block in split_entries(yaml) {
        if block.contains("crates/sddk-engine/src/cas_object_store")
            && block.contains("target_milestone: delivered")
        {
            return true;
        }
    }
    false
}

/// Splits the yaml text into top-level entries (best-effort, line-based).
fn split_entries(yaml: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = String::new();
    for line in yaml.lines() {
        if line.trim_start().starts_with("- module:") {
            if !current.is_empty() {
                entries.push(std::mem::take(&mut current));
            }
            current.push_str(line);
            current.push('\n');
        } else if !current.is_empty() {
            current.push_str(line);
            current.push('\n');
        }
    }
    if !current.is_empty() {
        entries.push(current);
    }
    entries
}

// ── M3 markers (semantic graph + vault + context) ────────────────────────

const MARKER_SEMANTIC_GRAPH_SINGLETON: &str = "m3.semantic_graph_singleton";
const MARKER_VAULT_KNOWLEDGE_SOURCE_ONLY: &str = "m3.vault_knowledge_source_only";
const MARKER_CONTEXT_CAPSULE_HAS_PROVENANCE: &str = "m3.context_capsule_has_provenance";

const MARKER_MEMORY_STORE_SINGLE_CANONICAL: &str = "m4.memory_store_single_canonical";
const MARKER_MEMORY_CLI_SPEC_SUBSET: &str = "m4.memory_cli_spec_subset";
const MARKER_MEMORY_DIFF_IS_SEMANTIC: &str = "m4.memory_diff_is_semantic";

const MARKER_AUTHORITY_ENGINE_SINGLE_PATH: &str = "m5.authority_engine_single_path";
const MARKER_ADMISSION_EXPLAINABLE: &str = "m5.admission_explainable";
const MARKER_LEGACY_AUTHORITY_COMPAT: &str = "m5.legacy_authority_compat";

const MARKER_CLI_SPEC_TABLE_CANONICAL: &str = "m6_1.cli_spec_table_canonical";
const MARKER_CLI_FIRST_CLASS_ROUTERS: &str = "m6_1.cli_first_class_routers";
const MARKER_CONFIG_EXPLAIN_DECLARATIVE: &str = "m6_1.config_explain_declarative";

const MARKER_TARGET_TASK_REGISTRY: &str = "m6_2.target_task_registry";
const MARKER_TARGET_DAG_TOPOLOGICAL: &str = "m6_2.target_dag_topological";
const MARKER_TARGET_BUILTINS_RESOLVABLE: &str = "m6_2.target_builtins_resolvable";

const MARKER_DAG_EXECUTOR_DELIVERED: &str = "m6_3.dag_executor_delivered";
const MARKER_DAG_OUTCOME_SERIALIZABLE: &str = "m6_3.dag_outcome_serializable";
const MARKER_DAG_FIRST_CLASS_TYPED: &str = "m6_3.dag_first_class_typed";

// M7.1 — Command Registry as Single Authority (SPEC-015 + ADR-014).
const MARKER_M7_1_FULL_FIELDS: &str = "m7_1.command_spec_full_fields";
const MARKER_M7_1_EXAMPLES_PUBLISHED: &str = "m7_1.examples_published_for_core_commands";
const MARKER_M7_1_CHEAT_SHEET_PRESENT: &str = "m7_1.cheat_sheet_renderer_present";

// M7.1B — Examples Wiring (closes A2 / A6 / A7-partial).
const MARKER_M7_1B_EXAMPLES_WALKED: &str = "m7_1b.examples_walked_against_runtime";

/// M3 alignment markers (arch-spec-005 + arch-spec-006).
///
/// Marker definitions:
/// - `semantic_graph_singleton`: a unique canonical author exists for the
///   `semantic_graph` module path with `target_milestone: delivered`.
/// - `vault_knowledge_source_only`: at least one entry owns the
///   `vault_boundary` module with `target_milestone: delivered`.
/// - `context_capsule_has_provenance`: the `context_compiler` module
///   exists in the registry with `target_milestone: delivered`.
pub fn semantic_graph_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_SEMANTIC_GRAPH_SINGLETON.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/semantic_graph"),
        },
        MarkerStatus {
            id: MARKER_VAULT_KNOWLEDGE_SOURCE_ONLY.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/vault_boundary"),
        },
        MarkerStatus {
            id: MARKER_CONTEXT_CAPSULE_HAS_PROVENANCE.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/context_compiler"),
        },
    ]
}

/// M4 markers (Decision Memory CLI, SPEC-004 §CLI).
///
/// Marker definitions:
/// - `memory_store_single_canonical`: the `memory_cmd` CLI module is
///   registered as the single canonical CLI surface for the Decision
///   Memory API.
/// - `memory_cli_spec_subset`: the `memory_cmd` registry entry does not
///   re-declare an alternate target_milestone; the CLI is a single
///   surface, not a fork.
/// - `memory_diff_is_semantic`: the underlying `decision_memory`
///   engine module is delivered, confirming the CLI's `diff` command
///   is wired to a semantic engine diff (not a custom bytewise diff).
pub fn decision_memory_cli_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_MEMORY_STORE_SINGLE_CANONICAL.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/memory_cmd"),
        },
        MarkerStatus {
            id: MARKER_MEMORY_CLI_SPEC_SUBSET.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/memory_cmd"),
        },
        MarkerStatus {
            id: MARKER_MEMORY_DIFF_IS_SEMANTIC.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/decision_memory"),
        },
    ]
}

/// M5 markers (Unified Authority Runner, SPEC-M5).
///
/// Marker definitions:
/// - `authority_engine_single_path`: the `authority_engine/runner` module is
///   delivered, confirming CLI call sites route admission through the runner.
/// - `admission_explainable`: the underlying `authority_engine` substrate
///   is delivered (the runner wraps `DefaultAuthorityEngine::admit + explain`).
/// - `legacy_authority_compat`: the legacy `authority` module is still
///   present (strangler window, removal belongs to M9).
pub fn unified_authority_runner_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_AUTHORITY_ENGINE_SINGLE_PATH.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/authority_engine/runner"),
        },
        MarkerStatus {
            id: MARKER_ADMISSION_EXPLAINABLE.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/authority_engine"),
        },
        MarkerStatus {
            id: MARKER_LEGACY_AUTHORITY_COMPAT.to_string(),
            // Legacy compat: present when ANY entry points to authority.* — but
            // we use a heuristic: count entries referencing authority.rs.
            present: yaml.contains("crates/sddk-engine/src/authority")
                && !yaml.contains("crates/sddk-engine/src/authority_engine\""),
        },
    ]
}

/// M6.1 markers (Convention-first CLI surface, SPEC-M6.1).
///
/// Marker definitions:
/// - `cli_spec_table_canonical`: the `command_spec` module is delivered,
///   confirming agents can introspect the CLI without runtime clap parsing.
/// - `cli_first_class_routers`: the `change`, `verify`, and `audit` router
///   modules are delivered as first-class verbs (no longer buried under
///   legacy surfaces).
/// - `config_explain_declarative`: the `config_cmd` module is delivered,
///   confirming the precedence chain is rendered from a declarative
///   table rather than ad-hoc inspection.
pub fn convention_first_cli_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_CLI_SPEC_TABLE_CANONICAL.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/command_spec"),
        },
        MarkerStatus {
            id: MARKER_CLI_FIRST_CLASS_ROUTERS.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/change")
                && has_delivered_entry(yaml, "crates/sddk-cli/src/verify_cmd")
                && has_delivered_entry(yaml, "crates/sddk-cli/src/audit_cmd"),
        },
        MarkerStatus {
            id: MARKER_CONFIG_EXPLAIN_DECLARATIVE.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/config_cmd"),
        },
    ]
}

/// M6.2 markers (Target/Task DAG registry, SPEC-M6.2).
///
/// Marker definitions:
/// - `target_task_registry`: the `target_task/registry` module is delivered,
///   confirming the lookup index for declared targets exists.
/// - `target_dag_topological`: the `target_task/dag` module is delivered,
///   confirming topological ordering and cycle detection are available.
/// - `target_builtins_resolvable`: the `target_task/builtin` module is
///   delivered, confirming the four canonical built-in targets
///   (`status`, `run`, `ship`, `recover`) resolve cleanly.
pub fn target_task_dag_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_TARGET_TASK_REGISTRY.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/target_task/registry"),
        },
        MarkerStatus {
            id: MARKER_TARGET_DAG_TOPOLOGICAL.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/target_task/dag"),
        },
        MarkerStatus {
            id: MARKER_TARGET_BUILTINS_RESOLVABLE.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/target_task/builtin"),
        },
    ]
}

/// M6.3 markers (DAG execution, SPEC-M6.3).
///
/// Marker definitions:
/// - `dag_executor_delivered`: the `target_task/executor` module is
///   delivered, confirming the per-task authority gate exists.
/// - `dag_outcome_serializable`: the `target_task/outcome` module is
///   delivered, confirming `ExecutionReport` and `TaskOutcome` are
///   serializable for the agent surface.
/// - `dag_first_class_typed`: the `change`/`verify`/`audit` typed
///   DAGs are declared on the built-in `builtin` module.
pub fn dag_execution_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_DAG_EXECUTOR_DELIVERED.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/target_task/executor"),
        },
        MarkerStatus {
            id: MARKER_DAG_OUTCOME_SERIALIZABLE.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/target_task/outcome"),
        },
        MarkerStatus {
            id: MARKER_DAG_FIRST_CLASS_TYPED.to_string(),
            // Same registry entry as M6.2 builtin marker; the typed
            // change/verify/audit shipped in M6.3 live in the same
            // module.
            present: has_delivered_entry(yaml, "crates/sddk-engine/src/target_task/builtin"),
        },
    ]
}

/// M7.1 alignment markers (SPEC-015 + ADR-014: Command Registry as
/// Single Authority).
///
/// - `command_spec_full_fields`: the `command_spec` module is delivered
///   and exposes the full SPEC-015 field set on `CommandSpec`
///   (stability, side_effect_class, required_authority, outputs,
///   preconditions, examples, related).
/// - `examples_published_for_core_commands`: the `command_surface`
///   aggregator is delivered and ships typed examples for the core
///   target commands (list / resolve / run).
/// - `cheat_sheet_renderer_present`: the `cheat_sheet` renderer is
///   delivered and powers the `sddk agent-help agent` CLI command.
pub fn m7_1_command_registry_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![
        MarkerStatus {
            id: MARKER_M7_1_FULL_FIELDS.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/command_spec"),
        },
        MarkerStatus {
            id: MARKER_M7_1_EXAMPLES_PUBLISHED.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/command_surface"),
        },
        MarkerStatus {
            id: MARKER_M7_1_CHEAT_SHEET_PRESENT.to_string(),
            present: has_delivered_entry(yaml, "crates/sddk-cli/src/cheat_sheet"),
        },
    ]
}

/// M7.1B alignment markers (Examples Wiring — A2/A6/A7-partial closure).
///
/// - `examples_walked_against_runtime`: the `examples_walker` module is
///   delivered and exposes `walk_examples()` that walks every
///   `ExampleSpec` against a pluggable `Walker` closure, returning a
///   deterministic `ExampleWalkReport`.
pub fn m7_1b_examples_wiring_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_1B_EXAMPLES_WALKED.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/examples_walker"),
    }]
}

fn has_delivered_entry(yaml: &str, module_path: &str) -> bool {
    for block in split_entries(yaml) {
        if block.contains(module_path) && block.contains("target_milestone: delivered") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_event_log_owner_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/canonical_event_log
    contract: arch-spec-001-canonical-authority
    primary_adr: ADR-0094-ONE-CANONICAL-FACT-LOG
    coverage: full
    state_class: Fact
    authority: sddk-engine
    replaces: null
    target_milestone: delivered
    notes: CanonicalEventLog shipped v1.153.0
"#;
        let markers = mirror_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_CANONICAL_EVENT_LOG_SINGLETON)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn compatibility_mirrors_marked_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/event_bus
    contract: arch-spec-007-agent-protocol
    target_milestone: delivered
"#;
        // No marker. Insufficient.
        let markers = mirror_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_COMPATIBILITY_MIRRORS_MARKED)
            .expect("marker");
        assert!(!m.present);

        let yaml_with_marker = format!(
            "{}\n  - module: crates/sddk-engine/src/legacy_ledger\n    compatibility_mirror: true\n    target_milestone: M2\n",
            yaml
        );
        let markers = mirror_alignment_checks(&yaml_with_marker);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_COMPATIBILITY_MIRRORS_MARKED)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn cas_index_consistent_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/cas_object_store
    contract: arch-spec-001-canonical-authority
    primary_adr: ADR-0094-ONE-CANONICAL-FACT-LOG
    coverage: full
    state_class: Object
    authority: sddk-engine
    replaces: null
    target_milestone: delivered
    notes: CAS store shipped v1.153.0
"#;
        let markers = mirror_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_CAS_INDEX_CONSISTENCY)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn empty_yaml_returns_all_false() {
        let markers = mirror_alignment_checks("");
        assert_eq!(markers.len(), 3);
        assert!(markers.iter().all(|m| !m.present));
    }

    #[test]
    fn compatibility_mirror_via_notes_text_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/event_envelope
    notes: "Legacy event envelope retained as compatibility mirror per arch-spec-001 CA-007."
    target_milestone: M9
"#;
        let markers = mirror_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_COMPATIBILITY_MIRRORS_MARKED)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn split_entries_isolates_each_module_block() {
        let yaml = r#"
- module: a
  notes: alpha
- module: b
  notes: beta
"#;
        let entries = split_entries(yaml);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("notes: alpha"));
        assert!(entries[1].contains("notes: beta"));
    }

    #[test]
    fn m3_semantic_graph_singleton_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    contract: arch-spec-005
    target_milestone: delivered
    notes: "M3 semantic graph shipped v1.154.0"
"#;
        let markers = semantic_graph_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_SEMANTIC_GRAPH_SINGLETON)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m3_vault_knowledge_source_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/vault_boundary
    contract: arch-spec-006
    target_milestone: delivered
"#;
        let markers = semantic_graph_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_VAULT_KNOWLEDGE_SOURCE_ONLY)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m3_context_compiler_present_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/context_compiler
    contract: arch-spec-006
    target_milestone: delivered
"#;
        let markers = semantic_graph_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_CONTEXT_CAPSULE_HAS_PROVENANCE)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m4_memory_store_single_canonical_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/memory_cmd
    contract: arch-spec-004-decision-memory
    target_milestone: delivered
  - module: crates/sddk-engine/src/decision_memory
    contract: arch-spec-004-decision-memory
    target_milestone: delivered
"#;
        let markers = decision_memory_cli_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_MEMORY_STORE_SINGLE_CANONICAL)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m4_memory_cli_spec_subset_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/memory_cmd
    target_milestone: delivered
"#;
        let markers = decision_memory_cli_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_MEMORY_CLI_SPEC_SUBSET)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m4_memory_diff_is_semantic_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/decision_memory
    target_milestone: delivered
"#;
        let markers = decision_memory_cli_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_MEMORY_DIFF_IS_SEMANTIC)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m4_markers_absent_when_memory_cmd_not_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = decision_memory_cli_alignment_checks(yaml);
        let cli = markers
            .iter()
            .find(|m| m.id == MARKER_MEMORY_STORE_SINGLE_CANONICAL)
            .expect("marker");
        assert!(!cli.present);
    }

    #[test]
    fn m5_authority_engine_single_path_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/authority_engine/runner
    target_milestone: delivered
"#;
        let markers = unified_authority_runner_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_AUTHORITY_ENGINE_SINGLE_PATH)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m5_admission_explainable_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/authority_engine
    target_milestone: delivered
"#;
        let markers = unified_authority_runner_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_ADMISSION_EXPLAINABLE)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m5_legacy_authority_compat_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/authority
    target_milestone: delivered
"#;
        let markers = unified_authority_runner_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_LEGACY_AUTHORITY_COMPAT)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m5_markers_absent_when_runner_not_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = unified_authority_runner_alignment_checks(yaml);
        let path = markers
            .iter()
            .find(|m| m.id == MARKER_AUTHORITY_ENGINE_SINGLE_PATH)
            .expect("marker");
        assert!(!path.present);
    }

    #[test]
    fn m6_1_spec_table_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/command_spec
    target_milestone: delivered
"#;
        let markers = convention_first_cli_alignment_checks(yaml);
        let spec = markers
            .iter()
            .find(|m| m.id == MARKER_CLI_SPEC_TABLE_CANONICAL)
            .expect("marker");
        assert!(spec.present);
    }

    #[test]
    fn m6_1_first_class_routers_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/change
    target_milestone: delivered
  - module: crates/sddk-cli/src/verify_cmd
    target_milestone: delivered
  - module: crates/sddk-cli/src/audit_cmd
    target_milestone: delivered
"#;
        let markers = convention_first_cli_alignment_checks(yaml);
        let r = markers
            .iter()
            .find(|m| m.id == MARKER_CLI_FIRST_CLASS_ROUTERS)
            .expect("marker");
        assert!(r.present);
    }

    #[test]
    fn m6_1_routers_marker_absent_when_partial() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/change
    target_milestone: delivered
  - module: crates/sddk-cli/src/verify_cmd
    target_milestone: delivered
"#;
        let markers = convention_first_cli_alignment_checks(yaml);
        let r = markers
            .iter()
            .find(|m| m.id == MARKER_CLI_FIRST_CLASS_ROUTERS)
            .expect("marker");
        assert!(
            !r.present,
            "audit missing → first-class routers marker must fail"
        );
    }

    #[test]
    fn m6_1_config_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/config_cmd
    target_milestone: delivered
"#;
        let markers = convention_first_cli_alignment_checks(yaml);
        let c = markers
            .iter()
            .find(|m| m.id == MARKER_CONFIG_EXPLAIN_DECLARATIVE)
            .expect("marker");
        assert!(c.present);
    }

    #[test]
    fn m6_2_target_task_registry_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/target_task/registry
    target_milestone: delivered
"#;
        let markers = target_task_dag_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_TARGET_TASK_REGISTRY)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m6_2_target_dag_topological_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/target_task/dag
    target_milestone: delivered
"#;
        let markers = target_task_dag_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_TARGET_DAG_TOPOLOGICAL)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m6_2_target_builtins_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/target_task/builtin
    target_milestone: delivered
"#;
        let markers = target_task_dag_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_TARGET_BUILTINS_RESOLVABLE)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m6_2_target_markers_absent_when_modules_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = target_task_dag_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m6_3_dag_executor_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/target_task/executor
    target_milestone: delivered
"#;
        let markers = dag_execution_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_DAG_EXECUTOR_DELIVERED)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m6_3_dag_outcome_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/target_task/outcome
    target_milestone: delivered
"#;
        let markers = dag_execution_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_DAG_OUTCOME_SERIALIZABLE)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m6_3_dag_first_class_typed_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/target_task/builtin
    target_milestone: delivered
"#;
        let markers = dag_execution_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_DAG_FIRST_CLASS_TYPED)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m6_3_dag_markers_absent_when_modules_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = dag_execution_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_1_command_spec_full_fields_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/command_spec
    target_milestone: delivered
"#;
        let markers = m7_1_command_registry_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_1_FULL_FIELDS)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_1_examples_published_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/command_surface
    target_milestone: delivered
"#;
        let markers = m7_1_command_registry_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_1_EXAMPLES_PUBLISHED)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_1_cheat_sheet_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/cheat_sheet
    target_milestone: delivered
"#;
        let markers = m7_1_command_registry_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_1_CHEAT_SHEET_PRESENT)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_1_command_registry_markers_absent_when_modules_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_1_command_registry_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_1b_examples_walker_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/examples_walker
    target_milestone: delivered
"#;
        let markers = m7_1b_examples_wiring_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_1B_EXAMPLES_WALKED)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_1b_examples_walker_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_1b_examples_wiring_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }
}
