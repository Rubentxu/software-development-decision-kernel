# Conformance Fitness Ratchets

These rules become blocking during the closeout. Names are illustrative; implementation may use Rust tests, cargo metadata scans, Semgrep/tree-sitter rules or dedicated architecture tests.

Estado verificado contra main el 2026-09-13. Búsqueda realizada: `grep -rn "conf09" crates/ docs/` — **cero hits** fuera de `08-BASELINE-CONFORMANCE-09-09/`. Los equivalentes funcionales existentes se citan como evidencia parcial; NINGÚN ratchet lleva el nombre `conf09_*` ni tiene mutation test dedicado.

| Ratchet | Estado | Equivalente funcional existente (si lo hay) | Mutation test |
|---|---|---|---|
| conf09_one_event_append_authority | MISSING (parcial) | `crates/sddk-cli/src/dev/arch_lint.rs` marker `canonical_event_log_owner_recognised` (solo reconoce presencia, no inyecta segunda autoridad); `crates/sddk-storage/tests/cross_ledger_consistency.rs` detecta huérfanos entre tablas | No |
| conf09_no_legacy_event_writes | MISSING | `crates/sddk-engine/tests/phase_events_integration.rs:153` `pe04_ledger_coexistence_events_v1_only` documenta la coexistencia, no la prohíbe | No |
| conf09_universal_evidence_only | MISSING | `crates/sddk-domain/tests/gate_receipt_pass_evidence.rs::passed_with_all_three_fields_is_accepted` valida evidencia de gates, no universalidad | No |
| conf09_no_planning_evidence_new_writes | MISSING (parcial) | lint advisory `evidence_kind_v1` (`docs/architecture/lints/deprecated_patterns.toml:72`; 57 hits, advisory, migración diferida a evidence-migration-v2) | No (advisory, no deny) |
| conf09_no_runtime_cycle_truth | MISSING (parcial) | `crates/sddk-domain/tests/sp04_cycle_status_slimming.rs:137` `derived_variants_have_documented_migration_target` (partición documentada, sin guard de escritura) | No |
| conf09_one_generic_revision_primitive_set | MISSING | Tests de interop `crates/sddk-engine/tests/decision_memory_tests.rs:664` `dmt_22_cross_substrate_interop_with_envelope`, pero sin prohibición de substrates paralelos | No |
| conf09_all_governed_effects_use_authority_engine | MISSING (parcial) | `crates/sddk-cli/src/dev/arch_lint.rs` `m5_authority_engine_single_path_recognised` (:1000); `crates/sddk-engine/tests/authority_engine_runner.rs:99` `sc_m5_6_runner_coexists_with_legacy_validate` (coexistencia explícita, no cutover) | No |
| conf09_no_legacy_authority_new_consumers | MISSING | Sin equivalente. `WritableSurface` aparece en 14 ficheros (`crates/sddk-engine/src/authority.rs`, `event_bus/emit.rs`, `cycle_pause.rs`, `cycle_supersede.rs`, `authority_engine/runner.rs`, tests...) sin lint de nuevos consumidores | No |
| conf09_command_contract_single_source | MISSING (parcial) | `crates/sddk-cli/tests/command_spec_tests.rs:187` `clap_surface_and_command_specs_are_in_sync` + `crates/sddk-cli/tests/cli_golden.rs:71` `cli_golden_surface_matches_blessed_snapshot` (contención, pero el CommandSpec sigue hand-curated, `crates/sddk-cli/src/command_spec.rs:6`) | No |
| conf09_agent_assets_no_deprecated_semantics | EXISTS (equivalente, deny) | lints deny `asset_deprecated_namespace` (:349), `asset_raw_store_reference` (:372), `asset_authority_language` (:398) en `docs/architecture/lints/deprecated_patterns.toml`; `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs::tests::live_registry_asset_lints_are_promoted_or_advisory_and_clean` | Parcial: `dev::lint::deprecated_patterns::tests::enforce_flag_exits_nonzero_on_deny_lint` demuestra fallo del flag, no de un fixture de agente |
| conf09_semantic_graph_only_authoritative_graph | MISSING (parcial) | `crates/sddk-cli/src/dev/arch_lint.rs` `m3_semantic_graph_singleton_recognised` (marker de presencia, no inyección de segunda grafo) | No |
| conf09_active_graph_projection_only | MISSING | Sin guard. `crates/sddk-engine/src/active_graph*.rs` derivado por diseño, sin ratchet que lo impida mutar | No |
| conf09_every_projection_has_rebuild_contract | MISSING (parcial) | `crates/sddk-domain/tests/projections_determinism.rs`, `crates/sddk-storage/tests/rebuild_integration.rs`, `crates/sddk-cli/tests/cli_projection_rebuild.rs` cubren rebuilds concretos; no hay verificación declarativa por proyección (CA-003) | No |
| conf09_every_storage_model_has_state_class | MISSING (parcial) | `crates/sddk-domain/tests/hashmap_audit.rs`, `variant_counts.rs` auditan formas, no clases Fact/Object/Projection/Ephemeral (CA-004) | No |
| conf09_no_deprecated_production_path | EXISTS (equivalente, deny) | lint deny `agent_result_used` (`docs/architecture/lints/deprecated_patterns.toml:30`); `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs::tests::live_registry_agent_result_used_is_deny_and_clean` | Parcial: el test valida el registry vivo, no un fixture de regresión inyectada |
| conf09_docs_status_matches_delivery | MISSING | Sin equivalente. Los 18 arch-specs llevan `status: proposed` (`docs/architecture/specs/arch-spec-001..018.md:5`) pese a estar implementados; sin test que falle por este drift | No |
| conf09_uat_01_22_traceable | PARTIAL (este paquete) | `uat/UAT-09-09-CONFORMANCE-MASTER.md` (este directorio): 17/22 MAPPED, 5 PARTIAL, 0 UNMAPPED; trazabilidad manual, sin guard CI que falle si un UAT pierde su test | No |

## Allowlist policy

Any temporary allowlist entry MUST include:

```text
symbol/path
reason
canonical_replacement
read_or_write
owner
removal_trigger
expiry/version
parity_test
```

Write-capable legacy entries are forbidden at C7.

## Mutation tests

At least these intentional regressions SHOULD be injected in CI or a dedicated architecture test suite:

- add second EventStore authority -> fail;
- construct deprecated planning evidence in production module -> fail;
- add `CycleStatus::ApprovalPending` write -> fail;
- bypass AuthorityEngine from effect adapter -> fail;
- change CLI option without command registry update -> fail;
- add deprecated command to active agent prompt -> fail.

Estado 2026-09-13: **ninguna de las seis mutaciones anteriores tiene un test que la inyecte y espere fallo**. Lo más cercano:

- `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs::tests::enforce_flag_exits_nonzero_on_deny_lint` (mutación 6, aproximada: valida el flag, no un asset real);
- `crates/sddk-cli/tests/command_spec_tests.rs:187` `clap_surface_and_command_specs_are_in_sync` (mutación 5, real: falla si el enum clap y la spec table divergen).

A fitness rule is not trusted until at least one fixture proves it fails for the prohibited change.
