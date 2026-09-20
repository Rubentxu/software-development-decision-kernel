# SCOPE-CONTRACT — a6-cc-s4-release-pipeline-contract (CC-S4 / AIW-S6)

## Goal

Definir y demostrar un `CoverageContract` de aceptación para un **segundo
consumer real** distinto de `verify-kernel::static_evidence`:
`release-pipeline::static_evidence` (v1.1.0). Valida que el versionado por
consumer funciona end-to-end: dos consumers sobre el mismo scope con
contratos distintos, sin interferencia de gaps, con identidad de veredicto
cualificada por `contract_id + contract_version` (N5/N6 de CC-S1 / ADR-0139).

## In scope

- Test de aceptación `a6_cc_s4_release_pipeline_contract.rs`:
  - AR-1: dos consumers, mismas bases y provider union → ambos semantics Demonstrated.
  - AR-2: provider parcial → gaps `MissingSemanticClass` solo para el consumer
    que los requiere (no leak entre consumers).
  - AR-3: bump de versión del contrato (v1.1.0 → v1.2.0) no altera veredicto
    con mismas observaciones; identidad de contrato incluye id+versión+consumer.
  - AR-4: la basis lleva `contract_revision` propia; sin constantes globales.

## Out of scope

- Declarar `STATIC_ENHANCED` global del workspace.
- Construir infraestructura de release-pipeline real.
- Cambios en `code_intelligence_port.rs` (el contrato existente es suficiente).

## Evidence

- `cargo test -p sddk-engine --test a6_cc_s4_release_pipeline_contract` → 4 PASS / 0 FAIL.
- Regresión: `a6_cc_s1_static_graph_completeness` 12 PASS + 1 ignored;
  `a6_s1_uat_coverage_fake` 12 PASS.
- Nota AR-5: la evaluación default penaliza `provider_strategy=lightweight`
  si el snapshot no declara `available_strategies=["lightweight",...]` —
  el test lo declara explícitamente (hallazgo documentado, no bug).
