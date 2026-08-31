---
status: accepted
date: 2026-08-16
deciders: [orchestrator, sddk-apply]
linked-cycles: [p-52b95ef55999f9de/arch-rules-phase1-evaluators]
---

# ADR-0014 — Phase 1 Architecture Evaluators: Live Capture + Real Rules

## Contexto

Phase 1 entry para el roadmap de arquitectura SDDK 2.0 (M1 "Hexagonal Kernel").

La Phase 0 estableció los tipos fundamentales (`RuleRegistry`, `Baseline`, `RuleEvaluation`) y un stub `evaluate_all` que devolvía `NotApplicable` para todas las reglas. Esto permitió iterar sobre el schema sin bloquear, pero hacía que las exit criteria de Phase 1 ("ARCH001 passes without waiver") fueran imposibles de verificar.

El estado real del repo muestra que `sddk-engine` tiene una dependencia directa a `sddk-storage` en su `Cargo.toml`, lo cual constituye una violación de ARCH001. Sin evaluadores reales, esta violación era invisible.

## Decisión

Implementar evaluadores reales para ARCH001-005 y una CLI `sddk dev check-architecture` que:

1. Captura un baseline vivo del workspace en cada ejecución (`BaselineConsumer::capture_live`), parseando el `Cargo.toml` workspace y escaneando `crates/*/src/**/*.rs` para `use sddk_X::...`.

2. Evalúa cada regla contra el baseline capturado:
   - **ARCH001** (`engine_must_not_depend_on_storage`): Fail si existe cualquier edge de `sddk-engine` → `sddk-storage` (Cargo dep o use statement).
   - **ARCH002** (`domain_must_not_depend_on_adapters`): Fail si existe edge de `sddk-domain` → `{sddk-storage, sddk-gateway, sddk-cli}`.
   - **ARCH003** (`cli_must_not_own_persistence_logic`): Fail si existe edge de `sddk-cli` → `sddk-storage` (proxy a nivel de imports para "no SQL en CLI").
   - **ARCH004** (`packs_must_declare_dependencies`): NotApplicable — el kernel repo no es un pack host (Phase 4 no shipped).
   - **ARCH005** (`reactive_behaviors_must_not_execute_governed_effects_directly`): NotApplicable — Phase 5 reactive runtime no shipped.

3. Waivers preservados del stub: `baseline.ref_.head_anchor <= w.granted_until_sha` → Waived.

4. CLI `sddk dev check-architecture [--root] [--rules <path>] [--out <json>]` que imprime tabla tabular y sale 1 si alguna rule con severity=Error tiene status=Fail.

### Arquitectura de `capture_live`

- Parser hand-rolled para `Cargo.toml` workspace (sin nuevas deps): extracción de `members` y `workspace.dependencies`.
- Parser para cada `Cargo.toml` de crate: resolución de `version.workspace = true` desde `workspace.dependencies`.
- `normalize_crate_name`: `path:../sddk-X` → `sddk-X`; `version string` → `sddk-version_string`.
- Glob de `src/**/*.rs` + scan lineal de `use sddk_X::` lines.
- SHA-256: workspace `Cargo.toml` + todos los `src/**/*.rs` en orden lexicográfico (determinista entre re-runs sin edits locales).
- `head_anchor`: `git rev-parse --short HEAD` (best-effort).

### Nuevo campo `CrossCrateImport.kind`

`CrossCrateImport` gained a `kind: CrossCrateImportKind` field (`CargoDep` | `Use`) para distinguir edges de Cargo de edges de use statements. La deserialización de baselines viejos (sin `kind`) defaulta a `Use` para backward compat.

## Consecuencias

Positivas:
- El ratchet ARCH001 se activa: `sddk dev check-architecture` reporta FAIL para el estado actual del repo.
- El ciclo de arquitectura ahora es verificable: se puede distinguir Pass (sin violaciones) de NotApplicable (sin evaluador) de Waived (con excepción activa).
- La CLI funciona contra cualquier workspace con crates, sin gate de managed-knowledge.

Negativas / riesgos:
- ARCH003 es un proxy (import-level) para "no SQL en CLI". El enforcement real requeriría call-graph analysis, out of scope para Phase 1.
- La violación ARCH001 (engine→storage) es real: la eliminación requiere definir ports en `sddk-app` o mover la lógica de orquestación al CLI gateway, fuera del scope de esta sesión.
- El parser hand-rolled de Cargo.toml es menos robusto que un parser TOML real. Si el formato de `Cargo.toml` cambia significativamente, puede romper.

## Alternativas consideradas

- **TOML parser (toml crate)**: más robusto pero añade una dep a sddk-engine. Descartado para mantener minimalismo.
- **Call-graph analysis para ARCH003**: demasiado costoso para Phase 1; el proxy de imports es suficiente para detectar violaciones obvias.
- **Baseline JSON pre-capturado**: la Phase 0 usaba un baseline JSON fijo. La captura viva elimina la necesidad de mantener un baseline outdated.

## Decisiones relacionadas

- ADR-0001: sandbox de validación E2E — la arquitectura de reglas es ortogonal.
- ADR-0002: atomic seq allocation — la persistencia del ledger es ortogonal.
- Phase 0 stub: establishes the types (RuleRegistry, Baseline, RuleEvaluation) used here.
- Phase 4 (Pack Runtime): cuando ships, ARCH004 cambiará de N/A a evaluación real.
- Phase 5 (Reactive Graph): cuando ships, ARCH005 cambiará de N/A a evaluación real.

## Notas de implementacion

- `BaselineConsumer::capture_live` vive en `crates/sddk-engine/src/rules/baseline.rs`.
- Evaluadores en `crates/sddk-engine/src/rules/evaluators.rs`.
- CLI en `crates/sddk-cli/src/dev/check_arch.rs`.
- Tests en `crates/sddk-engine/tests/rules_evaluator.rs` y `crates/sddk-cli/src/dev/tests/check_architecture_tests.rs`.
