# Roadmap actualizado — SDDK post-1.50.0

**Baseline técnica:** `1.50.0` / `643180a21ab1c9e7a63758ad221d97ec1640ae5a`  
**Fecha de propuesta:** 2026-08-28

## 0. Roadmap heredado

Mantener como entregado/cerrado:
- Stabilization v3.6 / PR1–PR9.
- E2E-2026-08.
- CP-2026-08 control plane.
- RS-2026-08 separation/zero-intrusion.

Mantener como línea funcional:
- UAT-2026-08-v3 Human-Governed AI Quality Control Plane.

Reconocer como baseline ya aterrizada:
- Instruction Contract Matrix.
- facade first-class: `status`, `plan`, `run`, `ship`, `recover`.
- behavioral/shadow parity tests.
- review sizing advisory separado de safety brakes.

## 1. Nueva línea HX

### HX0 — Canonical State & Reconciliation — P0
**Objetivo:** eliminar ambigüedad de autoridad antes de presentar estado al humano.

Entregables:
- corregir `status-query.md`;
- Source of Truth Matrix canónica;
- `CurrentRunView` schema;
- contract tests de reconstrucción;
- mapa de prompts que no pueden volver a declarar authority.

**Exit:** 0 contradicciones conocidas entre orchestrator/MCW/status-query/CLI contract.

### HX1 — Human Interaction Domain — P0
**Objetivo:** contratos estructurados independientes del renderer.

Entregables:
- `InteractionEvent`;
- `StageReport`;
- Decision/Reframe/Assumption;
- attention enum;
- domain tests;
- versioning de schemas.

**Exit:** cada transición relevante puede proyectarse sin texto libre.

### HX2 — Never Lost — P0/P1
**Objetivo:** orientación continua y resume durable.

Entregables:
- breadcrumb;
- phase start/completion summaries;
- noteworthy finding;
- blocked summary;
- Resume Summary;
- semantic commands `where/status/plan/risks/artifacts`;
- novice renderer default.

**Exit:** >=90% UAT identifica estado/next/action requerida.

### HX3 — Human Authority & Reframes — P1
**Objetivo:** HITL basado en riesgo y decisiones explicables.

Entregables:
- `DecisionRequired`;
- policy guided/balanced/autopilot;
- `/why`;
- decision/reframe/assumption projections;
- receipts para decisiones humanas;
- UAT integration port.

**Exit:** 100% high-risk decisions tienen receipt; <=1 approval innecesaria/ciclo.

### HX4 — Personality & Audience — P1
**Objetivo:** hacer la interacción configurable sin contaminar semántica.

Entregables:
- novice/standard/expert/audit;
- `wisecracking_robot`;
- safety tone suppression;
- locale;
- golden semantic invariance.

**Exit:** 100% parity de facts/verdict/next_action entre renderers.

### HX5 — Preference Memory — P2
**Objetivo:** relación durable y editable.

Entregables:
- profile XDG;
- candidate/learned/pinned;
- confidence/provenance;
- `/memory`;
- forget/edit/pin/export;
- optional Engram adapter spike.

**Exit:** 0 preferencias críticas aprendidas sin provenance; correction rate <10% en dogfood.

### HX6 — Friction Telemetry + F3 — P2
**Objetivo:** aprender dónde la colaboración falla.

Entregables:
- friction event taxonomy;
- control-plane metrics;
- dashboard slice;
- F3 recommendations;
- no automatic safety-policy mutation.

**Exit:** métricas por ciclo y cross-project; recomendaciones reproducibles.

### HX7 — Dogfood, UAT & Stabilization — P1/P2
**Objetivo:** cerrar el evolutivo con evidencia humana.

Entregables:
- UAT suite;
- resume tests;
- persona tests;
- failure/recovery tests;
- docs;
- migration;
- release.

**Exit:** todos los criterios de `MILESTONES-AND-EXIT-CRITERIA.md`.

## 2. Relación con UAT F0–F14

```text
UAT F0─F11  ───────────────────────────────► puede continuar
                     │
HX0 → HX1 → HX2 → HX3
                     │
                     └────────► prerequisito recomendado para UAT F12─F14
                                  Guided Runner / UX / human sign-off
```

Motivo: F12–F14 y Companion necesitan el mismo lenguaje de atención, decisión, explicación y sign-off.

## 3. Orden recomendado de ejecución

1. HX0.
2. HX1.
3. HX2.
4. HX3.
5. Integración UAT HumanDecisionPort.
6. HX4.
7. HX5.
8. HX6.
9. HX7.

No adelantar memoria/personality a HX0–HX2. Un agente con personalidad que no puede reconstruir correctamente el estado empeora la confianza.
