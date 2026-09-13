# SDDK Context-First Architecture Consolidation

**Fecha:** 2026-09-10  
**Estado:** propuesta normativa consolidada y refinada.  
**Target observado:** `Rubentxu/software-development-decision-kernel`, workspace 1.162.0 en la revisión usada para este paquete.

Este paquete **sustituye como propuesta de diseño** a:

- `SDDK-Semantic-Core-Consolidation-2026-09-09`;
- `SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09`;
- `SDDK-Progressive-Software-Knowledge-Verify-DebVerify-Evolution-2026-09-10`;
- `SDDK-Semantic-Core-Agent-Experience-Software-Alignment-Consolidation-2026-09-10`.

La diferencia principal es estructural: **los bounded contexts y sus ownerships se fijan antes de implementar el siguiente evolutivo**, para no construir primero módulos genéricos y reubicarlos después.

## Contextos propuestos

```text
Shared Kernel / Platform primitives
        |
        +--> Planning
        +--> Execution
        +--> Decision
        +--> Knowledge
        +--> Software Alignment
        +--> Verification
        +--> Governance
        +--> Agent Experience
        +--> Extension Platform
```

Los crates actuales siguen siendo inicialmente **capas de packaging Rust** (`sddk-domain`, `sddk-engine`, `sddk-storage`, `sddk-cli`, ...). Dentro de ellos se reorganizan módulos por bounded context. **No se crean nueve crates nuevos de golpe.** Un contexto sólo se separará físicamente en crate cuando métricas de connascence/change-coupling/build boundaries demuestren que aporta valor.

## Principio rector de Software Alignment

> Software Alignment compara el software observado con su intención, decisiones y lentes seleccionadas; produce conocimiento consultivo, tensiones y oportunidades. **No gobierna, no ordena, no autoriza y no impone un paradigma.**

Las salidas de Alignment se entregan al agente bajo `advisory_context`; **nunca se compilan en `EffectiveInstructions`**.

## Verify vs DebVerify

- `verify`: **delta reciente + impacto**; mantiene sincronizado el conocimiento afectado.
- `deb-verify`: **proyecto completo**; reconcilia el baseline y cuestiona conocimiento antiguo, deuda y desalineamientos globales.

## Modo potenciado

SDDK funciona en `BASE`. Proveedores opcionales habilitan:

- `STATIC_ENHANCED`: CogniCode u otro provider estático.
- `RUNTIME_ENHANCED`: Chronos u otro provider runtime.
- `FULLY_ENHANCED`: ambos tipos.

Los providers son servicios independientes, on-demand y negociados por capabilities. La ausencia de un provider produce `UNKNOWN/NOT_EVALUATED/EvidenceGap` según contrato, nunca un falso PASS.

## Ruta de lectura

1. `00-SYSTEM/BOUNDED-CONTEXT-MAP.md`
2. `00-SYSTEM/TARGET-SOURCE-LAYOUT.md`
3. `00-SYSTEM/CROSS-CONTEXT-DEPENDENCY-RULES.md`
4. `02-BOUNDED-CONTEXTS/software-alignment/README.md`
5. `02-BOUNDED-CONTEXTS/knowledge/README.md`
6. `02-BOUNDED-CONTEXTS/verification/README.md`
7. `03-ROADMAP/ROADMAP.md`
8. `04-MIGRATION/CURRENT-TO-TARGET-FILE-MAP.md`
9. `05-UAT/UAT-MASTER.md`

## Baseline 09/09 conformance prerequisite

Before starting R0 implementation, close the previous `Semantic Core + Agent Experience` baseline using `08-BASELINE-CONFORMANCE-09-09/`.

This is a finite conformance programme, not a new bounded context. Its C0→C7 roadmap requires explicit SPEC-001..018 traceability, UAT-01..22 execution and completion of M9 removal semantics. R0 starts only after a green `09-09-CONFORMANCE-RECEIPT.md`.
