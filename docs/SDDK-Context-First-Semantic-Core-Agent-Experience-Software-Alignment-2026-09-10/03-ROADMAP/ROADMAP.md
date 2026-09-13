# Canonical roadmap — context-first implementation

## Strategy

## Prerequisite — 09/09 baseline conformance closeout

Before R0, execute `08-BASELINE-CONFORMANCE-09-09/roadmap/CONFORMANCE-CLOSEOUT-ROADMAP.md` C0→C7. The previous Semantic Core + Agent Experience baseline must have a green `09-09-CONFORMANCE-RECEIPT.md`: SPEC-001..018 traced, UAT-01..22 green, and M9 deprecated production paths removed or strictly read-only/time-bounded compatibility.

**Dependency:** `C0 -> C1 -> C2 -> C3 -> C4 -> C5 -> C6 -> C7 -> R0`.

Primero fijar ownership y mover módulos; después implementar conocimiento/alignment. Cada milestone tiene una hipótesis y un exit observable. Architecture split posterior depende de métricas, no del diagrama deseado.

## R0 — Context boundary cut (behaviour preserving)

**Objetivo:** no implementar el evolutivo nuevo sobre carpetas que sabemos que cambiarán.

- crear módulos `shared/planning/execution/decision/knowledge/alignment/verification/governance/agent_experience/extension` dentro de domain/engine;
- mover archivos actuales según `04-MIGRATION/CURRENT-TO-TARGET-FILE-MAP.md`;
- mantener `pub use` compatibility shims;
- dependency fitness tests;
- cero cambio de persistencia/semántica.

**Exit:** workspace + installed CLI UAT verdes y ninguna ruta nueva se crea fuera del mapa sin ADR.

## R1 — Semantic Core final consolidation

Completar Fact/Object/Projection/Ephemeral, CanonicalEventLog, universal Evidence, Goal→WorkItem→Workflow→Plan→Run y AuthorityEngine en sus contextos definitivos.

## R2 — Decision + Knowledge substrate

Revision substrate/Decision Memory estabilizados; `KnowledgeAssertion`, `KnowledgeBasis`, KMT v1 y SemanticGraph overlay, aún sin Alignment LLM.

## R3 — Agent Experience advisory split

Formalizar `ContextCapsule.advisory_context`; fitness test que Alignment nunca entra en EffectiveInstructions. CommandRegistry/skills/profiles quedan con ownership definitivo.

## R4 — Software Alignment core (BASE mode)

- UniversalConcern;
- AlignmentLens;
- ArchitecturalIntentSnapshot;
- Assessment/Tension/Opportunity/ContractViolation;
- lenses `core`, `hexagonal`, `ddd`, `oo`, `functional`, `adt`, `smells`, `connascence` inicialmente basadas en evidencia disponible;
- Workbooks MVP.

**Exit:** puede evaluar fixtures sin CogniCode/Chronos y nunca bloquear por sí mismo.

## R5 — Verify delta knowledge synchronization

KMT dimension diff, impact base, staleness propagation, cards, AssessDelta, LLM incremental evaluator, VerifyReceipt.

**Exit:** cambios localizados no causan full scan; comment-only no invalida dependency alignment.

## R6 — DebVerify global reconciliation

Full/stratified baseline, debt taxonomy, risk×uncertainty, global workbook reconciliation, revisit triggers.

**Exit:** encuentra deuda fixture fuera del Git delta.

## R7 — Static Enhanced provider

Provider SPI + CogniCode RPC adapter + capabilities + Evidence normalization. SDDK Base sigue siendo primera clase.

## R8 — Runtime Enhanced provider

Chronos RPC adapter, ScenarioEvidence, BehaviorFingerprint, Static/Runtime workbook, golden architectural scenarios.

## R9 — Control tower + lens ecosystem

Grids/charts/DSM/heatmaps, Knowledge Health, pack-contributed lenses, attention/watchlists. Sin universal quality score.

## R10 — Governance ratchets and stable contracts

Policies pueden consumir explicit contracts/evidence; waivers expirables; new-violation ratchets; WHY/WHY-NOT integra knowledge/alignment provenance. El advisory boundary sigue intacto.

## R11 — Evaluate crate splits

Usar métricas reales para decidir si `knowledge`, `alignment` o `verification` merecen crate propio. **No mover por estética.** Un split aprobado conserva module paths mediante facade durante la transición.

## Dependency

```text
R0 -> R1 -> R2 -> R3 -> R4 -> R5 -> R6 -> R7 -> R8 -> R9 -> R10 -> R11
```

Spikes de providers pueden comenzar antes; producción no depende de ellos hasta R7/R8.
