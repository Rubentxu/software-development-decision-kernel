---
title: "Parche de roadmap — Cycle supersede, replan-in-place y recover-forward"
author: deep-research-orchestrator
date: 2026-08-31
status: draft
related: research/cycle-supersede-replan/cycle-supersede-replan-research-report.md
applies_to: docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md
---

# Parche de roadmap

## Resumen

Este parche prioriza 6 items para los próximos ciclos (cycle-50 en
adelante), derivados de la investigación profunda sobre el documento
`docs/evolutivo-correcciones-flexibilidad.md`. NO modifica las Waves
1–4 del plan actual (cycle-44–49); propone trabajo **después** de
cycle-49 (Wave 4 facade completion).

**Estado actual preservado**:
- Wave 1–4 sin cambios (cycle-44–49 según ROADMAP §Wave plan).
- VAULT003 / RepairReceipt queue (v1.65.6) sin cambios.
- release.sh end-to-end pipeline (v1.65.0) sin cambios.

---

## Propuesta de faseo

### Bloque 0 — Housekeeping (cycle-50, ≤ 1 día)

| # | Item | ADR | Tamaño | Bloqueador |
|---|---|---|---|---|
| 3.6 | Publicar ADR-0078 retroactivo (VAULT003 scope policy) | DRAFT-ADR-0078 | S | ninguno |

**Por qué primero**: cierra el dangling reference sin tocar código. Es
**el único item que no toca crates/** — solo mueve un archivo de
`research/.../adr-drafts/` a `docs/adr/ADR-0078-vault003-scope-policy.md`.

### Bloque 1 — Foundation (cycle-50, mismo ciclo)

| # | Item | ADR | Tamaño | Bloqueador |
|---|---|---|---|---|
| 3.4 | WriterXdgFailClosed trait + validación `vault export --output` | DRAFT-ADR-D | S | ninguno |

**Por qué encaja en cycle-50**: A-min, sin dependencias, scope
controlado. Cierra insight #5.

### Bloque 2 — Pre-flight GAP-6 (cycle-50 bis o PR pre-existente)

| # | Item | ADR | Tamaño | Notas |
|---|---|---|---|---|
| (GAP-6) | Investigar y fixear `cycle lock acquire` (FOREIGN KEY constraint) | (out of scope) | M | AGENTS.md §8 ya lo documenta; requiere reproducción |

**Estado**: este es el **único bloqueador real** para los items 3.1 y
3.3 (cycle supersede y replan-in-place). Sin lock acquire funcional,
no hay lease, no hay supersede, no hay replan.

**Recomendación**: pre-flight antes de cycle-51. Si no se cierra,
cycle-51 se sustituye por un ciclo A-min dedicado a GAP-6.

### Bloque 3 — Cycle supersede + replan (cycle-51 + cycle-53)

| # | Item | ADR | Tamaño | Bloqueador | Depende |
|---|---|---|---|---|---|
| 3.1 | `cycle supersede` como primitiva | DRAFT-ADR-A | M | GAP-6 cerrado | — |
| 3.3 | `cycle replan-in-place` | DRAFT-ADR-C | M | ADR-A | ADR-A |

**Faseo interno de cycle-51**:
- T1: `CycleCommand::Supersede` + `CycleSupersedeArgs` (estructura).
- T2: `cycle.supersede.requested` + `cycle.supersede.applied` eventos.
- T3: `supersede-receipt.json` writer (atomic temp + rename).
- T4: lint rule: no supersede without prior cycle lease.
- T5: RED test (anti-tautology per cycle-36 discipline).
- T6: GREEN test (cargo test --workspace --locked).
- T7: fmt + clippy + docs.

**Faseo interno de cycle-53**:
- T1: `CycleCommand::Replan` + `CycleReplanArgs`.
- T2: `cycle.replan.*` eventos (cadena con supersede).
- T3: successor cycle binding (per Wave plan §Wave 1.4).
- T4: replan counter limit (max 5).
- T5: RED test (counter=6 fails).
- T6: GREEN test.
- T7: fmt + clippy + docs.

### Bloque 4 — Gate classification + recovery contract (cycle-52)

| # | Item | ADR | Tamaño | Bloqueador | Depende |
|---|---|---|---|---|---|
| 3.2 | Gate classification (security/process/mixed) | DRAFT-ADR-B | M | ninguno | — |
| 3.8 | Recovery-action contract (RFC 9457 problem details) | DRAFT-ADR-G | M | ADR-B | ADR-B |

**Faseo interno de cycle-52**:
- T1: gate descriptor registry (`docs/gates/*.yml`).
- T2: orchestrator reads `class` BEFORE applying gate.
- T3: process gates emit `recover-forward <command>`.
- T4: lint rule: every gate has `class`.
- T5: `crates/sddk-cli/src/recovery.rs` (closed-set registry).
- T6: error sites adopt RFC 9457 JSON shape.
- T7: RED test (anti-tautology).
- T8: GREEN test.
- T9: fmt + clippy + docs.

### Bloque 5 — Cycle vs hypothesis + complexity budget (cycle-54)

| # | Item | ADR | Tamaño | Bloqueador | Depende |
|---|---|---|---|---|---|
| 3.5 | DesignDecision primitiva (cycle vs hypothesis) | DRAFT-ADR-E | L | ADR-A | ADR-A |
| 3.7 | Complexity budget (trend metric, no rule) | DRAFT-ADR-F | S | ADR-B | ADR-B |

**Faseo interno de cycle-54**:
- T1: `crates/sddk-domain/src/decision.rs` (DesignDecision type).
- T2: decision events (`decision.created`, `decision.failed`,
  `decision.superseded`, `decision.activated`).
- T3: cycle manifest gains `current_decision_id`.
- T4: depth limit (≤ 10).
- T5: `GateComplexityBudget` metric.
- T6: trend detector (3 consecutive cycles).
- T7: RED test (depth=11 fails).
- T8: GREEN test.
- T9: fmt + clippy + docs.

---

## Resumen cronológico propuesto

```
Hoy         v1.65.7 (HEAD 405a3f0)
            │ último HANDOFF: cycle-43 (v1.48.10)
            ▼
cycle-49    Wave 4 facade completion (status actual según ROADMAP)
            │
            ▼
cycle-50    ADR-0078 retroactivo (S)            ─┐
            + WriterXdgFailClosed trait (S)      │ Housekeeping +
            [1 ciclo A-min; 2 items, ~1.5 días] ─┘ foundation
            │
            ▼
GAP-6 pre-flight (1 ciclo A-min si no resuelto)
            │
            ▼
cycle-51    cycle supersede (M) [PREREQUISITO: GAP-6]
            [1 ciclo A-min; 1 item, ~3-4 días]
            │
            ▼
cycle-52    gate classification (M)
            + recovery-action contract (M)
            [1 ciclo A-min; 2 items, ~5-6 días]
            │
            ▼
cycle-53    replan-in-place (M) [depende de cycle-51]
            [1 ciclo A-min; 1 item, ~3-4 días]
            │
            ▼
cycle-54    cycle vs hypothesis (L)
            + complexity budget (S)
            [1 ciclo A-full; 2 items, ~5-7 días]
            │
            ▼
Post-cycle-54 — evaluar estado; decidir si más rounds
```

**Total estimado**: 5 ciclos (50–54) + 1 pre-flight = ~6 ciclos sobre
~3–4 meses calendario.

---

## Items no incluidos en este parche

### Lateral-thinking L2 (self-recovering process gates)

**Cuándo**: cycle-55+, después de validar ADR-B en producción.

**Razón**: combina ADR-B (classification) + ADR-G (recovery contract).
Requiere evidencia de que el mecanismo funciona antes de automatizarlo.

### Lateral-thinking L3 (goal-scoped writer sin path input)

**Cuándo**: cycle-55+, refactor mayor.

**Razón**: requiere romper `vault export --output` para usuarios
externos. Necesita deprecation cycle + comunicación.

### Lateral-thinking L4 (supersede-receipt como nodo vault)

**Cuándo**: cycle-52 o cycle-53, aditivo.

**Razón**: opcional; no bloquea recovery-forward.

### Lateral-thinking L5 (phase regression audit)

**Cuándo**: cycle-55+.

**Razón**: depende de métricas estables (cycle-54 introduce complexity
budget).

### Lateral-thinking L6 (repair-receipt waiver)

**Cuándo**: cycle-52 o cycle-53, aditivo.

**Razón**: depende de ADR-A (supersede); pequeño pero trazable.

### Lateral-thinking L7 (complexity budget como trend metric)

**Cuándo**: cycle-54 (incluido en este parche como parte de ADR-F).

---

## Compatibilidad con Wave plan actual

| Wave / Cycle | Estado | Compatibilidad con este parche |
|---|---|---|
| Wave 1 (cycle-46) | planificado | sin cambios; cycle-50+ comienza después |
| Wave 2 (cycle-47) | planificado | sin cambios |
| Wave 3 (cycle-48) | planificado | sin cambios |
| Wave 4 (cycle-49) | planificado | sin cambios; este parche comienza en cycle-50 |
| Phase 4 dynamic graph | fase futura | sin cambios; independiente |
| Workflow Runtime v2 (ADR-041) | fase futura | cycle-54+ puede coordinarse con Goal primitive |
| Phase 14 hardening | fase futura | sin cambios |

---

## Riesgos del faseo propuesto

| Riesgo | Likelihood | Impact | Mitigación |
|---|---|---|---|
| GAP-6 no se cierra antes de cycle-51 | medium | high | Pre-flight GAP-6 antes de cycle-51; si no, sustituir cycle-51 por fix |
| Cycle-50 se alarga (ADR-0078 + ADR-D en 1 ciclo) | low | low | A-min; 2 items pequeños caben |
| Cycle-52 (gate classification + recovery contract) se alarga | medium | medium | Si se alarga, separar en cycle-52a (B) y cycle-52b (G) |
| Cycle-54 (A-full) revela más trabajo | medium | low | A-full permite extensiones; budget de 5–7 días |
| Decisión humana cambia el orden | medium | medium | Documentar este parche; usuario decide |

---

## Próximo paso

**Hoy (2026-08-31)**: este parche es **propuesta**, no mandato. Está
disponible para revisión.

**Próximo movimiento humano**: decidir si cycle-50 abre con ADR-0078 +
ADR-D (recomendación) o si se prioriza otra cosa.

**Próximo movimiento del orchestrator**: si el humano aprueba,
`sddk cycle start --name cycle-50-housekeeping-and-xdg` carga el
launch plan con items 3.6 + 3.4.

---

## Patch instructions (para el humano)

Si decides aplicar este parche, las modificaciones al
`docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` son:

1. Después de §Wave 5 — Hardening (deferred), añadir:

   ```markdown
   ## Post-Wave 4 — Recover-forward cycle series (cycle-50+)

   > **Authority**: research/cycle-supersede-replan/cycle-supersede-replan-research-report.md
   > **Principle**: Fail closed para seguridad; recover forward para proceso.

   ### cycle-50 — Housekeeping + XDG writer foundation
   - ADR-0078 retroactivo (VAULT003 scope policy)
   - ADR-D (WriterXdgFailClosed trait + vault_cmd.rs:443 validation)
   - Tamaño: A-min; ~1.5 días

   ### GAP-6 pre-flight (cycle-50 bis if needed)
   - `cycle lock acquire` (FOREIGN KEY constraint, AGENTS.md §8)
   - Tamaño: A-min; ~2-3 días
   - Hard dependency for cycle-51

   ### cycle-51 — cycle supersede (PREREQUISITO: GAP-6)
   - ADR-A (cycle supersede as first-class operation)
   - SPEC-SUPERSEDE-001
   - Tamaño: A-min; ~3-4 días

   ### cycle-52 — Gate classification + recovery-action contract
   - ADR-B (security/process/mixed classification)
   - ADR-G (RFC 9457 problem details)
   - Tamaño: A-min; ~5-6 días

   ### cycle-53 — replan-in-place (depende de cycle-51)
   - ADR-C (cycle.replan operation)
   - SPEC-REPLAN-001
   - Tamaño: A-min; ~3-4 días

   ### cycle-54 — Cycle vs hypothesis + complexity budget
   - ADR-E (DesignDecision primitive)
   - ADR-F (trend metric, not rule)
   - Tamaño: A-full; ~5-7 días
   ```

2. Añadir al §Cycle binding (sugerido):

   ```markdown
   | `recover_forward_series` | cycle-50..54 | cycle-49 Wave 4 facade shipped; GAP-6 fixed | cycle-50–5, cycle-52+2, cycle-53+1, cycle-54+2; spec/blueprint deliverables match the cycles |
   ```

3. Añadir al §Cross-phase slice — durable technical-debt remediation:

   ```markdown
   | Post-Wave 4 recover-forward | Failure paths that block learning | research/cycle-supersede-replan/ (10 evidence cards + 4 blueprints + 2 specs + 5 ADRs) | A-min × 4 + A-full × 1 between cycle-50 and cycle-54 |
   ```

---

## Conclusión

Este parche es **inminente y de bajo riesgo** si se ejecuta después de
cycle-49. Los items propuestos:

- Son **aditivos** (no modifican código publicado).
- Son **compatibles** (Wave plan intacto).
- Son **verificables** (10 evidence cards + RED tests anti-tautology).
- Son **recuperables** (cada ADR es independiente; si uno falla, los
  otros pueden seguir).

**Recomendación**: aplicar el parche completo. Si se decide aplicar
parcialmente, priorizar ADR-0078 + ADR-D + ADR-A (cycle-50–51) como
fundación; dejar cycle-52–54 como candidatos posteriores.