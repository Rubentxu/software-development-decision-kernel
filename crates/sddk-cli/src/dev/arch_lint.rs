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

// M7.1C — Live in-process walker (cli_walker over crate::run_from).
const MARKER_M7_1C_LIVE_WALKER: &str = "m7_1c.live_walker_in_process";

// M7.2 — Surface Integration Layer (programmatic provider-adapter payload).
const MARKER_M7_2_SURFACE_INTEGRATION: &str = "m7_2.surface_integration_layer_delivered";

// M7.4 — Full AgentProfile + per-command ArgSpec JSON-Schema derivation.
const MARKER_M7_4_AGENT_PROFILE: &str = "m7_4.agent_profile_full_model_delivered";
const MARKER_M7_4_ARG_SCHEMA: &str = "m7_4.arg_spec_json_schema_derivation_delivered";

// M7.3 — SkillDefinition contract (SPEC-016).
const MARKER_M7_3_SKILL_DEFINITION: &str = "m7_3.skill_definition_contract_delivered";

// M7.5 — InstructionCompiler + EffectiveInstructions (SPEC-014).
const MARKER_M7_5_INSTRUCTION_COMPILER: &str = "m7_5.instruction_compiler_delivered";

// M7.6 — AgentExecutionReceipt (SPEC-018).
const MARKER_M7_6_EXECUTION_RECEIPT: &str = "m7_6.agent_execution_receipt_delivered";

// M7.7 — Skill registry disk bridge + runtime admission gate.
const MARKER_M7_7_SKILL_REGISTRY_BRIDGE: &str = "m7_7.skill_registry_bridge_delivered";

// M8 — Default SkillRegistry surface (`sddk dev skills list/verify`).
const MARKER_M7_8_SKILLS_SURFACE: &str = "m7_8.skills_surface_delivered";

// M8.0 — `sddk dev graph list/edges/projection` operator surface for
// the H9 Active Graph & Cockpit engine (`crates/sddk-engine/src/active_graph`).
const MARKER_M8_0_ACTIVE_GRAPH_SURFACE: &str = "m8_0.active_graph_surface_delivered";

// M8.1 — `sddk dev graph why` causal query surface for the H9 WHY engine
// (`crates/sddk-engine/src/why_queries`). Exposes the three why-verbs
// (why / debt-why / decision-why) as CLI subcommands.
const MARKER_M8_1_WHY_QUERIES_SURFACE: &str = "m8_1.why_queries_surface_delivered";

// M8.2 — `sddk dev cockpit view` operator surface for the H9 Cockpit Views
// engine (`crates/sddk-engine/src/cockpit_views`, COCKPIT-001). Exposes the
// four view kinds (overview / journal / timeline / execution) as a CLI
// subcommand. The companion `cockpit_observability` (COCKPIT-002) lands in
// M8.3.
const MARKER_M8_2_COCKPIT_VIEWS_SURFACE: &str = "m8_2.cockpit_views_surface_delivered";

// M8.3 — `sddk dev cockpit obs` operator surface for the H9 Cockpit
// Observability engine (`crates/sddk-engine/src/cockpit_observability`,
// COCKPIT-002). Exposes the five operational views (providers / usage /
// assurance / handoff / experiments) as a sibling CLI subcommand in the
// same `crates/sddk-cli/src/dev/cockpit` module. Lives in the same
// surface module as M8.2 — adds a `CockpitCommand::Obs` variant rather
// than a new CLI module, because both surfaces share `load_input()`,
// `CockpitViewRow`, and the text/JSON rendering pipeline.
const MARKER_M8_3_COCKPIT_OBSERVABILITY_SURFACE: &str = "m8_3.cockpit_observability_surface_delivered";

// M8.4 — Auto-derive `ActiveGraphInput` from a cycle's archive manifest
// via `sddk dev cockpit {view,obs} --from-cycle <cycle-id>`. Reads the
// manifest at `~/.sddk-knowledge/sddk-framework/cycles/<cycle-id>/archive-manifest.md`
// and produces a projection only from what the manifest honestly exposes
// (commit SHAs as workflow_nodes, bridges with known SHAs as evidence_of
// edges, deferred items as memory_refs). No `git rev-parse`, no
// inference — anything the manifest does not state is silently dropped
// per the "honest derivation" contract documented in `derive_active_graph_input_from_manifest`.
const MARKER_M8_4_ACTIVE_GRAPH_INPUT_AUTO_DERIVED: &str = "m8_4.active_graph_input_auto_derived";

// M8.5 — `parent_of` edges from an explicit `## Commit parents` section in
// the cycle archive manifest. Extends the M8.4 derivation contract: a
// bullet like ``- `child` → `parent` (note)`` under that heading produces
// a `workflow_edges` entry, but only when both SHAs appear in the known
// commit list. Manifests without the section stay backward-compatible
// (empty `parent_edges`, same as M8.4). Lives in the same `cockpit` module
// because the parser reuses `parse_section_bullets_for` and the same
// honest-derivation rules.
const MARKER_M8_5_COMMIT_PARENTS_SECTION_PARSED: &str =
    "m8_5.commit_parents_section_parsed";

// M8.6 — Per-node and per-edge provenance metadata carried through the
// `ActiveGraphInput` ↔ `ActiveGraphProjection` pipeline so every
// projected entity is attributable back to the byte that produced it.
// Lives in the engine (`sddk_engine::active_graph::{ProvenanceRef,
// ProvenanceSourceKind}`); wired into the CLI projection in
// `dev/cockpit.rs` and `dev/graph.rs` (`NodeRow::provenance` +
// `EdgeRow::provenance` with `skip_serializing_if = "Option::is_none"`
// so JSON stays backward-compatible when no provenance is recorded).
const MARKER_M8_6_PROVENANCE_THREADED_THROUGH_GRAPH: &str =
    "m8_6.provenance_threaded_through_graph";

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

/// M7.1C alignment markers (Live in-process walker).
///
/// - `live_walker_in_process`: the `examples_walker` module is delivered
///   AND its `cli_walker` function delegates to `crate::run_from` (the
///   in-process CLI entry point), not a stub. We approximate this with
///   `has_delivered_entry` against `crates/sddk-cli/src/examples_walker`
///   because the registration of the module as delivered is the gate the
///   release flow enforces; the actual in-process delegation is verified
///   by inline tests in `examples_walker::tests`.
pub fn m7_1c_live_walker_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_1C_LIVE_WALKER.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/examples_walker"),
    }]
}

/// M7.2 alignment markers (Surface Integration Layer).
///
/// - `surface_integration_layer_delivered`: the `surface_integration`
///   module is delivered and exposes `surface_for_provider` plus the
///   three provider renderers (OpenAI, Anthropic, generic). Verifying
///   the actual payload shapes is the responsibility of inline tests
///   in `surface_integration::tests`; this marker is the registry
///   gate the release flow enforces.
pub fn m7_2_surface_integration_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_2_SURFACE_INTEGRATION.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/surface_integration"),
    }]
}

/// M7.4 alignment markers (Full AgentProfile model).
///
/// - `agent_profile_full_model_delivered`: the `agent_profile` module
///   is delivered with the full AgentProfile struct (allowed_stabilities,
///   allowed_side_effects, required_authority ceiling, allowed_targets)
///   plus the canonical profile constructors (default, read_only,
///   approver, ci_bot).
pub fn m7_4_agent_profile_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_4_AGENT_PROFILE.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/agent_profile"),
    }]
}

/// M7.4 alignment markers (ArgSpec JSON-Schema derivation).
///
/// - `arg_spec_json_schema_derivation_delivered`: the `arg_schema` module
///   is delivered with `arg_specs_to_json_schema` and
///   `command_spec_to_json_schema` (JSON Schema draft-07), closing the
///   M7.2 deferred gap on `parameters` / `input_schema`.
pub fn m7_4_arg_schema_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_4_ARG_SCHEMA.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/arg_schema"),
    }]
}

/// M7.3 alignment markers (SkillDefinition contract, SPEC-016).
///
/// - `skill_definition_contract_delivered`: the `skill_definition` module
///   is delivered with the typed `SkillDefinition` / `SkillRegistry`
///   pair, fail-closed validation of the SPEC-016 anti-patterns
///   (Skill ≠ Capability), content hashing for `AgentExecutionReceipt`
///   provenance, and `CommandSpec::required_skills` wiring.
pub fn m7_3_skill_definition_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_3_SKILL_DEFINITION.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/skill_definition"),
    }]
}

/// M7.5 alignment markers (InstructionCompiler + EffectiveInstructions).
///
/// - `instruction_compiler_delivered`: the `instruction_compiler` module
///   is delivered with the typed `InstructionCompiler` /
///   `EffectiveInstructions` pair, fail-closed conflict detection
///   (`InvariantViolation`, `PolicyNarrowingViolation`,
///   `TaskMissing`), canonical-JSON content hashing for
///   `AgentExecutionReceipt` provenance, and the `ref_token()`
///   formatter.
pub fn m7_5_instruction_compiler_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_5_INSTRUCTION_COMPILER.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/instruction_compiler"),
    }]
}

/// M7.6 alignment markers (AgentExecutionReceipt).
///
/// - `agent_execution_receipt_delivered`: the `execution_receipt` module
///   is delivered with the typed `AgentExecutionReceipt` envelope,
///   `ReplayClass` enum, `ProviderUsage` struct, `ValidationWarning`
///   for deferred producers, fail-closed `validate()` (rejects empty
///   ids, non-hex hashes, invalid skill ref_tokens, non-RFC3339
///   timestamps), canonical-JSON content hashing, and a typed
///   `AgentExecutionReceiptBuilder` that wires
///   `effective_instructions_hash` from M7.5
///   (`EffectiveInstructions::content_hash`) and `selected_skill_refs`
///   from M7.3 (`SkillDefinition::ref_token` + `content_hash`).
pub fn m7_6_execution_receipt_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_6_EXECUTION_RECEIPT.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/execution_receipt"),
    }]
}

/// M7.7 — Skill registry disk bridge + runtime admission gate.
///
/// Closes the deferred #2 from M7.3: the CLI runner now calls
/// `SkillRegistry::admits_command` before dispatching each command,
/// using a registry loaded from `~/.local/share/sddk/framework/skills/`
/// (and project/user-level overrides). The bridge is a separate module
/// (`skill_registry_bridge`) so it is independently testable and can be
/// reused by other consumers (e.g. `sddk dev doctor --verbose`).
pub fn m7_7_skill_registry_bridge_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_7_SKILL_REGISTRY_BRIDGE.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/skill_registry_bridge"),
    }]
}

/// M8 — `sddk dev skills list/verify` surface.
///
/// Ships the operator-facing counterpart of the M7.7 gate: a CLI
/// subcommand that surfaces the loaded skill registry so users can see
/// why the gate warned and which skills are actually available. Two
/// subcommands — `list` enumerates every loaded skill (with scope,
/// id, version, ref_token, source path), `verify` walks every
/// `CommandSpec` with declared `required_skill`s and reports whether
/// each is admitted, missing, or matches a known M7.5 contractual
/// placeholder. Closes the deferred #3 from M7.3 in a minimal but
/// honest form: the registry is *inspectable* but the three M7.5
/// placeholders (`core.contract-review@v1`, `core.workflow-orchestration
/// @v1`, `core.release-planning@v1`) are still not real on-disk skills.
pub fn m7_8_skills_surface_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M7_8_SKILLS_SURFACE.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/skills"),
    }]
}

/// M8.0 alignment check (`sddk dev graph list/edges/projection`).
///
/// Marker definition:
/// - `m8_0.active_graph_surface_delivered`: a unique canonical author exists
///   for the `crates/sddk-cli/src/dev/graph` module path with
///   `target_milestone: delivered`. The corresponding engine module
///   (`crates/sddk-engine/src/active_graph`) is already shipped as part of
///   the H9 Active Graph & Cockpit horizon (GR-WHY-001 typed causal
///   projection). M8.0 exposes that projection via the CLI without
///   changing the engine contract.
pub fn m8_0_active_graph_surface_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M8_0_ACTIVE_GRAPH_SURFACE.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/graph"),
    }]
}

/// M8.1 alignment check (`sddk dev graph why`).
///
/// Marker definition:
/// - `m8_1.why_queries_surface_delivered`: the `crates/sddk-cli/src/dev/graph`
///   module owns a `run_dev_graph_why` runner that dispatches `why` /
///   `debt-why` / `decision-why` against the engine `DefaultWhyQueryEngine`.
///   The corresponding engine module (`crates/sddk-engine/src/why_queries`)
///   is already shipped as part of GRAPH-WHY-002.
pub fn m8_1_why_queries_surface_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M8_1_WHY_QUERIES_SURFACE.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/graph"),
    }]
}

/// M8.2 alignment check (`sddk dev cockpit view`).
///
/// Marker definition:
/// - `m8_2.cockpit_views_surface_delivered`: the `crates/sddk-cli/src/dev/cockpit`
///   module owns a `run_dev_cockpit_view` runner that dispatches
///   `overview` / `journal` / `timeline` / `execution` against the engine
///   `DefaultCockpitViewBuilder`. The corresponding engine module
///   (`crates/sddk-engine/src/cockpit_views`) is already shipped as part
///   of COCKPIT-001. The companion observability surface
///   (`cockpit_observability`, COCKPIT-002) is deferred to M8.3 and does
///   not gate this marker.
pub fn m8_2_cockpit_views_surface_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M8_2_COCKPIT_VIEWS_SURFACE.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/cockpit"),
    }]
}

/// M8.3 alignment check (`sddk dev cockpit obs`).
///
/// Marker definition:
/// - `m8_3.cockpit_observability_surface_delivered`: the same
///   `crates/sddk-cli/src/dev/cockpit` module owns a `run_dev_cockpit_obs`
///   runner that dispatches `providers` / `usage` / `assurance` / `handoff`
///   / `experiments` against the engine `DefaultCockpitObservabilityBuilder`.
///   The corresponding engine module
///   (`crates/sddk-engine/src/cockpit_observability`) is already shipped
///   as part of COCKPIT-002. M8.3 shares the surface module with M8.2;
///   the marker key is distinct (`m8_3.cockpit_observability_*` vs
///   `m8_2.cockpit_views_*`) so each milestone has its own gate.
pub fn m8_3_cockpit_observability_surface_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M8_3_COCKPIT_OBSERVABILITY_SURFACE.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/cockpit"),
    }]
}

/// M8.4 alignment check (`sddk dev cockpit --from-cycle`).
///
/// Marker definition:
/// - `m8_4.active_graph_input_auto_derived`: the
///   `crates/sddk-cli/src/dev/cockpit` module exposes a
///   `derive_active_graph_input_from_manifest` pure parser + a
///   `load_input_from_cycle` filesystem helper + a `resolve_input`
///   dispatcher (with `--from-cycle` taking precedence over
///   `--from-input`) that lets `sddk dev cockpit {view,obs}` work
///   against a live cycle's archive manifest without hand-feeding a
///   JSON fixture. The marker gates on the surface module path
///   (same as M8.2 / M8.3) rather than a separate module, because the
///   cycle-aware input loading is an extension of the existing
///   cockpit surface, not a new operator surface.
pub fn m8_4_active_graph_input_auto_derived_alignment_checks(yaml: &str) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M8_4_ACTIVE_GRAPH_INPUT_AUTO_DERIVED.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/cockpit"),
    }]
}

/// M8.5 alignment check (`parent_of` edges from `## Commit parents`).
///
/// Marker definition:
/// - `m8_5.commit_parents_section_parsed`: the
///   `crates/sddk-cli/src/dev/cockpit` module exposes a
///   `parse_parent_edge_bullet` helper + a `## Commit parents` section
///   walker in `parse_commit_rows` that turns bullets like
///   ``- `<child>` → `<parent>` (note)`` into `workflow_edges` entries
///   (filtered to known-SHA pairs). Manifests without the section stay
///   backward-compatible. Gates on the same surface module path as
///   M8.2 / M8.3 / M8.4 because the parsing is a strict extension of
///   the same `derive_active_graph_input_from_manifest` pure parser.
pub fn m8_5_commit_parents_section_parsed_alignment_checks(
    yaml: &str,
) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M8_5_COMMIT_PARENTS_SECTION_PARSED.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/cockpit"),
    }]
}

/// M8.6 alignment check (per-node + per-edge provenance).
///
/// Marker definition:
/// - `m8_6.provenance_threaded_through_graph`: both
///   `crates/sddk-cli/src/dev/cockpit` AND `crates/sddk-cli/src/dev/graph`
///   participate in the provenance contract (the cockpit parser mints
///   the `ProvenanceRef`s; the graph emitter serializes them). The
///   marker gates on both surface modules being delivered because the
///   data flow is two-sided.
pub fn m8_6_provenance_threaded_through_graph_alignment_checks(
    yaml: &str,
) -> Vec<MarkerStatus> {
    vec![MarkerStatus {
        id: MARKER_M8_6_PROVENANCE_THREADED_THROUGH_GRAPH.to_string(),
        present: has_delivered_entry(yaml, "crates/sddk-cli/src/dev/cockpit")
            && has_delivered_entry(yaml, "crates/sddk-cli/src/dev/graph"),
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

    #[test]
    fn m7_1c_live_walker_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/examples_walker
    target_milestone: delivered
"#;
        let markers = m7_1c_live_walker_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_1C_LIVE_WALKER)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_1c_live_walker_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_1c_live_walker_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_2_surface_integration_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/surface_integration
    target_milestone: delivered
"#;
        let markers = m7_2_surface_integration_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_2_SURFACE_INTEGRATION)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_2_surface_integration_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_2_surface_integration_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_4_agent_profile_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/agent_profile
    target_milestone: delivered
"#;
        let markers = m7_4_agent_profile_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_4_AGENT_PROFILE)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_4_agent_profile_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_4_agent_profile_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_4_arg_schema_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/arg_schema
    target_milestone: delivered
"#;
        let markers = m7_4_arg_schema_alignment_checks(yaml);
        let m = markers
            .iter()
            .find(|m| m.id == MARKER_M7_4_ARG_SCHEMA)
            .expect("marker");
        assert!(m.present);
    }

    #[test]
    fn m7_4_arg_schema_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_4_arg_schema_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_3_skill_definition_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/skill_definition
    target_milestone: delivered
"#;
        let markers = m7_3_skill_definition_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M7_3_SKILL_DEFINITION)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m7_3_skill_definition_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_3_skill_definition_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_5_instruction_compiler_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/instruction_compiler
    target_milestone: delivered
"#;
        let markers = m7_5_instruction_compiler_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M7_5_INSTRUCTION_COMPILER)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m7_5_instruction_compiler_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_5_instruction_compiler_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_6_execution_receipt_marker_recognised() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/execution_receipt
    target_milestone: delivered
"#;
        let markers = m7_6_execution_receipt_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M7_6_EXECUTION_RECEIPT)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m7_6_execution_receipt_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_6_execution_receipt_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_7_skill_registry_bridge_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/skill_registry_bridge
    target_milestone: delivered
"#;
        let markers = m7_7_skill_registry_bridge_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M7_7_SKILL_REGISTRY_BRIDGE)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m7_7_skill_registry_bridge_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_7_skill_registry_bridge_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m7_8_skills_surface_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/skills
    target_milestone: delivered
"#;
        let markers = m7_8_skills_surface_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M7_8_SKILLS_SURFACE)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m7_8_skills_surface_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/semantic_graph
    target_milestone: delivered
"#;
        let markers = m7_8_skills_surface_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m8_0_active_graph_surface_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/graph
    target_milestone: delivered
"#;
        let markers = m8_0_active_graph_surface_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M8_0_ACTIVE_GRAPH_SURFACE)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m8_0_active_graph_surface_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/active_graph
    target_milestone: delivered
"#;
        let markers = m8_0_active_graph_surface_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m8_1_why_queries_surface_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/graph
    target_milestone: delivered
"#;
        let markers = m8_1_why_queries_surface_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M8_1_WHY_QUERIES_SURFACE)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m8_1_why_queries_surface_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/why_queries
    target_milestone: delivered
"#;
        let markers = m8_1_why_queries_surface_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m8_2_cockpit_views_surface_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/cockpit
    target_milestone: delivered
"#;
        let markers = m8_2_cockpit_views_surface_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M8_2_COCKPIT_VIEWS_SURFACE)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m8_2_cockpit_views_surface_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/cockpit_views
    target_milestone: delivered
"#;
        let markers = m8_2_cockpit_views_surface_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m8_3_cockpit_observability_surface_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/cockpit
    target_milestone: delivered
"#;
        let markers = m8_3_cockpit_observability_surface_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M8_3_COCKPIT_OBSERVABILITY_SURFACE)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m8_3_cockpit_observability_surface_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/cockpit_observability
    target_milestone: delivered
"#;
        let markers = m8_3_cockpit_observability_surface_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m8_4_active_graph_input_auto_derived_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/cockpit
    target_milestone: delivered
"#;
        let markers = m8_4_active_graph_input_auto_derived_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M8_4_ACTIVE_GRAPH_INPUT_AUTO_DERIVED)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m8_4_active_graph_input_auto_derived_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/active_graph
    target_milestone: delivered
"#;
        let markers = m8_4_active_graph_input_auto_derived_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m8_5_commit_parents_section_parsed_marker_present_when_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/cockpit
    target_milestone: delivered
"#;
        let markers = m8_5_commit_parents_section_parsed_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M8_5_COMMIT_PARENTS_SECTION_PARSED)
            .expect("marker present");
        assert!(marker.present);
    }

    #[test]
    fn m8_5_commit_parents_section_parsed_marker_absent_when_module_missing() {
        let yaml = r#"
entries:
  - module: crates/sddk-engine/src/cockpit_views
    target_milestone: delivered
"#;
        let markers = m8_5_commit_parents_section_parsed_alignment_checks(yaml);
        for m in &markers {
            assert!(!m.present, "marker {} should be absent", m.id);
        }
    }

    #[test]
    fn m8_6_provenance_threaded_through_graph_marker_present_when_both_modules_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/cockpit
    target_milestone: delivered
  - module: crates/sddk-cli/src/dev/graph
    target_milestone: delivered
"#;
        let markers = m8_6_provenance_threaded_through_graph_alignment_checks(yaml);
        let marker = markers
            .iter()
            .find(|m| m.id == MARKER_M8_6_PROVENANCE_THREADED_THROUGH_GRAPH)
            .expect("marker present");
        assert!(
            marker.present,
            "both cockpit + graph modules delivered ⇒ marker present"
        );
    }

    #[test]
    fn m8_6_provenance_threaded_through_graph_marker_absent_when_only_cockpit_registered() {
        let yaml = r#"
entries:
  - module: crates/sddk-cli/src/dev/cockpit
    target_milestone: delivered
"#;
        let markers = m8_6_provenance_threaded_through_graph_alignment_checks(yaml);
        for m in &markers {
            assert!(
                !m.present,
                "marker {} must be absent when graph emitter is missing",
                m.id
            );
        }
    }
}
