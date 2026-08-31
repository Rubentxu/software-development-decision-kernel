# ADR-0043 — Compiler determinista sin LLM

**Status:** Accepted

## Context

El ciclo 2 introduce `WorkflowCompiler` (en `crates/sddk-domain/src/compiler.rs`) que traduce
`WorkflowManifest` legacy (601 líneas, SDD-fase-acoplado) a `WorkflowIR` nuevo (genérico, SDD-agnóstico).
La pregunta es si el compilador puede delegar a un LLM para resolver mapeos ambiguos.

## Decision

El compilador es **completamente determinista** y **no invoca LLM**. Las reglas de mapeo
(`Phase → capability`, `CyclePath → Operator`, `Transition → Sequence/Choice/Parallel`) son
funciones puras sobre tipos Rust, parametrizadas por `CapabilityRegistry` pero sin sampling
probabilístico.

El compilador emite un `WorkflowIR` con:
- `provenance.generated_by = "compiler:1.30.0"`
- `provenance.compiled_at = <ISO8601 determinista del host>`
- `compute_content_hash()` produce el mismo `sha256:<64-hex>` para entradas idénticas

## Consequences

- Mismo manifest → mismo IR → mismo hash, en cualquier máquina y hora.
- Replay tests pueden comparar hashes golden canónicos.
- Sin dependencia de claves de API ni cuotas de modelo.
- Validación determinista: el validador (ADR-0044) opera sobre el IR sin re-evaluar el manifest.
- Cambios en capacidades requieren migración explícita; no hay "mejora automática" del compilador.

## Rejected

- LLM-as-compiler: imposible reproducir hashes golden entre ejecuciones; introduce coste + latencia
  no deterministas en CI; rompe replay determinista.
- Compilación híbrida (LLM para casos ambiguos + determinista para el resto): mezcla los dos modos,
  hace el compilador imposible de validar formalmente.

## Verification

- `crates/sddk-domain/tests/compiler_determinism.rs` proptest 1000 iteraciones con seeds distintos.
- Golden hashes en `compiler.rs::tests` para fixtures `a_min`, `a_lite`, `a_full`.
- Cycle 2 spec §3: 8 escenarios Given/When/Then verifican la propiedad.