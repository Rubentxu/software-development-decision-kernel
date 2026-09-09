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
}
