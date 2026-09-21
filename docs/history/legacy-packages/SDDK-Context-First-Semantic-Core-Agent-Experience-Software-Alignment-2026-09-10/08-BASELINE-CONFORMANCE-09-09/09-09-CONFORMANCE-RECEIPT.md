# 09-09-CONFORMANCE-RECEIPT

> Produced by the production-readiness convergence (A1). This is a receipt over
> executable evidence, not a roadmap status. A row is `PASS` only where a real
> test/fixture/command exercises the authority/boundary of the scenario.

## Identity

- Repository commit: `0c2ca56` (`main`)
- SDDK workspace version: `1.169.19`
- Baseline package: `SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09`
- Date: 2026-09-14
- Runner/environment: local `cargo` (dev profile), Linux; full workspace profile

## Verdict

`PASS` — 100% conformance. Every SPEC-001..018 row is `PASS`/`PASS_WITH_COMPAT`,
UAT-01..22 are green with direct evidence, all M9 removals are proven, and the
recovery/rebuild/migration fixtures pass at the certified commit. No unresolved
MUST finding remains.

## SPEC-001..018

| SPEC | Status | Code evidence | Test/UAT evidence | Compatibility note |
|---|---|---|---|---|
| SPEC-001 Canonical authority/facts/evidence | PASS | `sddk-storage/src/event_store.rs` (`events_v1` sole append authority); `crates/sddk-storage/src/migrations.rs` MIGRATION_20 drops `ledger_events` | `crates/sddk-storage/tests/event_store.rs`, `chain_verification.rs`, `canonical_parity.rs` | none (C1.5 removal) |
| SPEC-002 Lifecycle model | PASS | `sddk-engine/src/workflow_runtime.rs`; Cycle summary derived from Run facts | `runtime_cycle_status_cutover.rs`, `cycle_summary_derived_from_run_facts` | decode-only runtime Cycle variants |
| SPEC-003 Revision substrate | PASS | `sddk-engine/src/revision_substrate.rs` | `revision_substrate` unit tests + `ref_store_cas_race_has_single_winner` | ADR-0097 additive substrate |
| SPEC-004 Decision Memory | PASS | `sddk-engine/src/decision_memory.rs` | decision_memory tests; `13-A0-REVISION-SUBSTRATE-CLASSIFICATION.md` | decision-semantic, distinct from substrate |
| SPEC-005 SemanticGraph + WHY | PASS | `sddk-storage/src/graph_store.rs` (rebuildable projection); `why_queries.rs` | `graph_store_roundtrip.rs`, `graph_rebuild_then_query_then_why` | projection only |
| SPEC-006 Knowledge/Vault/Context | PASS | `sddk-vault/src/*`; `sddk-domain/src/context_read.rs` | `cli_vault_index_validate_search_and_export`, `cli_vault_validate_closed_set_guard` | Vault = source, not authority |
| SPEC-007 Agent protocol/handoff | PASS | `sddk-engine/src/agent_contribution_envelope.rs` | synthesis/dissent tests (`check_dissent`, `check_hidden_conflict`) | |
| SPEC-008 Authority/side effects | PASS | `sddk-engine/src/authority_engine.rs`; `sddk-cli/src/admission.rs` | approval matrix, grant→re-admit, `zero_bypass_every_surface_blocks_require_approval`, `cli_approval_loop_e2e` | `EnforcementStage::LowMedium` rollback affordance |
| SPEC-009 Target/Task workflow | PASS | `sddk-engine/src/target_task/*`; command registry | `target_task.rs`, `cli_walks_cycle_with_fencing_and_rebuilds_state` | |
| SPEC-010 Pack SDK | PASS | `sddk-domain/src/pack.rs`; `sddk-pack-uat` | `pack_conformance_fixtures`, `cli_pack_validate_and_lint_enforce_manifest` | |
| SPEC-011 Observability views | PASS | `sddk-engine/src/cockpit_views.rs`, `active_graph.rs` (derived) | cockpit/view tests; `active_graph_view_derives_from_canonical_projection` | derived view, not authority |
| SPEC-012 Configuration | PASS | `sddk-cli/src/config_cmd.rs` (5-layer precedence resolver + `config explain <key>` source chain) | `config_cmd::tests::scoped_overrides_project_but_inherits_undeclared`, `env_layer_beats_files`, `unknown_key_fails_explicitly`, `precedence_lists_five_bands` | env-only keys unchanged |
| SPEC-013 Agent Experience contract | PASS | `sddk-cli/src/agent_profile.rs`, `command_surface.rs`; typed `CommandSpec` | `16-A1-AGENT-ASSET-INVENTORY.md`; asset lints (`deny`, 0 hits) + negative fixtures | `ADR-0106` (prompt text ≠ architecture) |
| SPEC-014 Instruction compiler | PASS | `sddk-cli/src/instruction_compiler.rs` | `lint_instruction_contract`, compiler tests | |
| SPEC-015 Command registry/agent surface | PASS_WITH_COMPAT | `sddk-cli/src/command_spec.rs` | `clap_surface_and_command_specs_are_in_sync`, `agent_surface_golden` | clap parser compat (owner/trigger in `15-A0-...crosswalk.md`) |
| SPEC-016 Skill contract | PASS | `sddk-cli/src/skill_definition.rs` | `admits_command_*`, `axs2_skill_cannot_satisfy_mandatory_task_requirement` | |
| SPEC-017 Agent profiles/provider adapters | PASS | `sddk-cli/src/agent_profile.rs`; adapter tests | `agent_profile_carries_no_provider_transport_data`, `adapter_converts_v1_without_data_loss` | |
| SPEC-018 Agent execution provenance | PASS | `sddk-engine/src/agent_execution_receipt.rs` (provenance chain) | `build_provenance_chain*`, `context_capsule_provenance_is_ordered_deterministically` | |

## UAT-01..22

| UAT | Result | Command/test | Evidence artifact |
|---|---|---|---|
| UAT-01 fresh adoption | PASS | `adopt_apply_is_non_intrusive_and_does_not_plant_workflow`, `adoption_registration_is_transactional_idempotent_and_conflict_safe` | cli adopt tests |
| UAT-02 deterministic planning reconciliation | PASS | `axs2_conflict_resolution_is_deterministic_across_orderings`, `compile_deterministic` | engine tests |
| UAT-03 multi-run Cycle summary | PASS | `cycle_summary_derived_from_run_facts`, `derive_cycle_summary` | engine tests |
| UAT-04 governed side effect approval | PASS | `approval_matrix_is_action_driven_not_band_driven`, `cli_approval_loop_e2e`, `zero_bypass_every_surface_blocks_require_approval` | admission + e2e |
| UAT-05 conflicting Contributions | PASS | `check_dissent`, `check_hidden_conflict` | synthesis tests |
| UAT-06 Decision Memory recovery | PASS | `build_provenance_chain_over_persisted_records`, `e2e_stop_midflight_restart_replays_to_equivalent_terminal_state` | engine tests |
| UAT-07 graph rebuild + WHY equivalence | PASS | `graph_rebuild_then_query_then_why`, `rebuild_integration` | storage tests |
| UAT-08 Vault cannot mutate canonical | PASS | `cli_vault_validate_closed_set_guard` | vault tests |
| UAT-09 Pack isolation | PASS | `pack_conformance_fixtures` | pack-uat |
| UAT-10 legacy command compat/deprecation | PASS | `command_surface::tests::uat10_deprecated_command_is_gated_with_guidance_and_opt_in`, `default_surface_excludes_experimental_and_deprecated` | cli tests |
| UAT-11 drift guard second authority | PASS | `graph_rebuild_detects_content_hash_drift_and_chain_tamper`, cross-storage drift tests, arch lints | storage + dev lint |
| UAT-12 long-session recovery | PASS | `restart_survival`, `runtime_restart_survival`, `workflow_run_restart_survival` | engine tests |
| UAT-13 agent command surface | PASS | `agent_surface_golden`, `surface_with_filter` | cli tests |
| UAT-14 cheat-sheet examples parse/schema | PASS | `m7_1_examples_published_marker_recognised`, `m7_1b_examples_walker_marker_recognised` | cli tests |
| UAT-15 CLI change without CommandSpec breaks test | PASS | `clap_surface_and_command_specs_are_in_sync` | command_spec_tests |
| UAT-16 skill missing capability | PASS | `axs2_skill_cannot_satisfy_mandatory_task_requirement`, `admission_fails_closed_for_missing_real_skill` | cli/engine tests |
| UAT-17 conflicting instruction fails closed | PASS | `lint_instruction_contract` | compiler tests |
| UAT-18 same profile two adapters | PASS | `agent_profile_carries_no_provider_transport_data` | cli tests |
| UAT-19 execution provenance hashes | PASS | `build_provenance_chain_v2`, `context_capsule_provenance_is_ordered_deterministically` | engine tests |
| UAT-20 deprecated asset mutation rejected | PASS | `dev_lint_e2e::uat20_deprecated_asset_is_rejected_by_enforce_without_mutation`, `asset_lint_detects_injected_deprecated_asset` | cli e2e |
| UAT-21 contextual command surface | PASS | `related_at_depth`, `default_surface_excludes_experimental_and_deprecated` | cli tests |
| UAT-22 command knowledge ≠ shipping authority | PASS | `sc_m6_3_3_ship_target_halts_at_publish_under_system_actor`, `sddk023_ship_side_effects_not_empty` | engine tests |

## M9 removals

- duplicate event write paths: **removed** — `events_v1` sole append authority; MIGRATION_20 drops `ledger_events` (`migration_16.rs`, `canonical_parity.rs`).
- planning evidence duplicates: **closed for Base** — `evidence_kind_v1` lint is `default = deny` with 0 hits after bounded exclude_paths; `sddk dev lint deprecated-patterns` enforces it; E1 authors evidence via `resolve_planning_evidence_relation` / `CoreRelationKind`; legacy `PlanningEvidenceKind` is read-only decode with a removal trigger (PR-GAP-002 → PASS_WITH_COMPAT).
- ActiveGraph authority: **projection-only** — `active_graph_view_derives_from_canonical_projection`.
- AgentResult production writes: **guarded** — `agent_result_used` lint at zero hits.
- runtime Cycle states: **decode-only** — `runtime_cycle_status_cutover.rs`.
- handwritten command/cheat-sheet authority: **bounded compat** — `clap_surface_and_command_specs_are_in_sync` (PR-GAP-007 `PASS_WITH_COMPAT`).
- obsolete monolithic prompt paths: **removed / none active** — `obsolete_monolithic_prompt_paths_are_absent` (`prompts/` is modular; no root monolithic prompt); inventory in `16-A1-AGENT-ASSET-INVENTORY.md`.
- blocking architecture/agent drift rules: **present** — `dev check-architecture`, `arch_lint`, `mutation` tests.
- single normative architecture entry point: **reconciled** — `docs/architecture/README.md` + `docs/architecture/specs/README.md` crosswalk (PR-GAP-010 `PASS`).

## Recovery / rebuild

- clean repo: PASS — workspace suite + `cli_*` e2e green.
- migrated repo: PASS — `legacy_ledger_migration` (3), `canonical_parity` (3), `sqlite_storage` (35).
- projection rebuild: PASS — `rebuild_integration` (5), `cli_projection_rebuild` (5).
- fresh-process recovery: PASS — `restart_survival` (4), `runtime_restart_survival` (1), `workflow_run_restart_survival`.

## Compatibility allowlist

| Path | Reason | Read-only | Removal trigger | Parity test |
|---|---|---|---|---|
| `sddk-domain::legacy` decode | historical agent-output decode | yes | no supported historical payload requires it | `legacy.rs::tests` |
| `EnforcementStage::LowMedium` | single-commit rollback affordance | n/a (unconstructed) | M4 proven across a release cycle | `require_approval_high_blocks_at_stage_all` |
| clap `Command` enum | runtime argv parser | n/a | spec table generated from clap | `clap_surface_and_command_specs_are_in_sync` |
| `PlanningEvidenceKind` decode | legacy evidence taxonomy | yes | no supported repository/schema/persisted fixture requires legacy `PlanningEvidenceKind` decode | `evidence_kind_v1` lint |

## Unresolved MUST findings

`NONE`.
