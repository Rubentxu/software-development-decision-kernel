# PLAN — UAT v3: Human-Governed AI Quality Control Plane

**Status:** PROPOSED
**Date:** 2026-08-10
**Author:** orchestrator (with user research input)
**Scope:** Reorientar el UAT del framework desde "framework UAT con automatizaciones" a una plataforma de **Human-Governed AI Quality / TestOps**.
**Baseline:** v1.6.1 (auto-runner `uat run` recién integrado)

---

## 0. Principio rector

> **Agents propose and execute. Machines measure and assess. Humans retain acceptance authority.**

`TEST PASSED` ≠ `PRODUCT ACCEPTED`. El humano no necesita juzgar todo: los asserts, Playwright, axe y los VLM pueden medir y evaluar preliminarmente; **ninguno de ellos se confunde con la aceptación de negocio**.

---

## 1. Diagnóstico del encaje actual (v1.6.1)

### 1.1 Qué existe hoy

| Pieza | Ubicación | Estado |
|---|---|---|
| Schema UAT v2 (plan/session/report) | `crates/sddk-domain/src/uat.rs` (~2300 líneas) | Estable, versionado (`schema_version: 2`) |
| `automation.{status, ref, ci_job, when}` | `UatAutomation` (uat.rs:233) | Metadata-only hasta hoy; `uat run` lo hace ejecutable |
| `UatExecutor {Human, Fara, Mixed, Automated}` | uat.rs:322 | Añadido `Automated` en v1.6.1 |
| `UatEvidenceKind {file, screenshot, command_output, assertion, metric, note}` | uat.rs:94 | Tipado, sha256-pinned |
| CLI `uat` (17 subcomandos) | `crates/sddk-cli/src/uat.rs` (~2500 líneas) | plan/validate/dashboard/open/ingest/report/status/failures/config/gate/migrate/verify-integrity/storage-path/build-manifest/scenario-context/history/run |
| Runner tipado sin shell | `crates/sddk-gateway/src/runner.rs` | `RunSpec`/`RunOutcome` — sólido, reutilizable |
| Agentes UAT | `agents/uat-{planner,guide,runner,reporter}.md` | 4 agentes, contratos claros, `executor: fara` para pre-flight |
| Skills UAT | `skills/uat-{dashboard,evidence,guided-mode,traceability}/` | 4 skills delegate-only |
| Dashboard kit | `assets/uat-dashboard/{kit,themes,views}/` | tokens/components/storage.js + guided/interactive/report |
| Fara 1.5 (CUA) | llama.cpp local `:8082` (skill `cua-test-orchestrator`) | Server DOWN hoy; integrable vía HTTP |
| Playwright | `~/.local/bin/playwright` global | Disponible; `ui-audit-protocol` + `playwright-cli` skills |
| Control plane | `sddk telemetry ingest` + `uat_results` | Sesiones agregadas, no grafos |
| Workflow | `workflow/workflow.yaml` | Fase `uat`, gates `uat-activated`/`uat-verdict`/`release-uat-approved` |
| ADRs | `ADR-012` (HITL), `ADR-013` (dashboard kit) | Aceptados, vigentes |
| REQs | `RF-019` (HITL), `RF-020` (data-driven), `RNF-010` (gate configurable) | Activos |

### 1.2 Qué está bien (NO tocar)

- **Evidence sha256-pinned** (content-addressable) — ya correcto.
- **Runner tipado sin shell** (`sddk_gateway::run`) — base sólida para todos los executors.
- **Data-driven YAML canónico** — separación contenido/presentación.
- **Agentes que producen YAML, nunca HTML.**
- **Integridad fail-closed** (`verify-integrity`, build fingerprint).
- **Local-first** (ADR-010) y **cero intrusión** (ADR-0011).
- **Ledger hash-chain** como trazabilidad base.

### 1.3 El problema fundamental

`automation.status: manual|scripted|fara` **mezcla cuatro dimensiones independientes**:

1. **Quién ejecuta** (executor)
2. **Qué evidencia se captura** (evidence)
3. **Quién juzga si se cumple el criterio** (oracle)
4. **Cuándo hace falta el humano** (review policy)

Un escenario puede ser `executor: playwright, oracle: dom+visual_ai, review: only_if_uncertain`. Otro puede ser `executor: fara, oracle: human, review: required`. El modelo actual no puede expresarlo.

---

## 2. Modelo objetivo — schema v3

### 2.1 Los 4 ejes del Scenario (v3)

```text
SCENARIO
├── EXECUTOR        (quién hace cosas — nunca decide PASS/FAIL global)
├── EVIDENCE        (qué se captura — bundle content-addressable)
├── ORACLES         (quién mide la evidencia contra el criterio)
└── REVIEW POLICY   (cuándo interviene el humano)
        └── ACCEPTANCE (estado final: ACCEPTED ≠ PASSED)
```

### 2.2 Executor (Capa 1)

```yaml
executor:
  kind: cli | api | script | playwright | computer_use | human
  # cli/api/script → RunSpec (gateway runner actual)
  # playwright      → PlaywrightExecutor (sensor + actuador, NUNCA juez)
  # computer_use    → Fara (observe→think→act→observe), trajectory completo
  # human           → solo el wizard guiado/matriz (regla actual: agentes nunca escriben executor: human)
```

**Regla dura:** el executor produce evidencia, **nunca emite el veredicto global**.

### 2.3 Evidence Bundle (Capa 2)

```yaml
evidence:
  screenshots: true
  playwright_trace: true      # default: trace > video (Playwright docs)
  console: true
  network: failures_only
  accessibility: true          # axe
  geometry: true               # boundingBox
  video: false                 # opcional, solo review humana / exploratory
  trajectory: true             # para computer_use (Fara)
```

Cada artefacto **content-addressable** (`sha256:...`) con `environment: {git_sha, app_version, browser, viewport, os}` y `execution: {executor, model, model_hash, prompt_hash}`.

**Playwright = sensor + actuador, no juez.** Proporciona trace, screenshots, network, console, DOM/ARIA snapshots, bounding boxes, visual diff (`toHaveScreenshot`), axe.

### 2.4 Oracles (Capa 3)

```yaml
oracles:
  - kind: exit_code | http | text | json_schema | dom | geometry | accessibility | visual_diff | visual_ai | llm_rubric | human
```

- **Deterministas** (exit_code, http, text, json_schema, dom, geometry, accessibility, visual_diff) — deciden sin IA.
- **Semánticos** (visual_ai con rúbrica, llm_rubric) — evalúan preliminarmente, con `confidence`.
- **Human** — único que puede dar aceptación de negocio.

**Regla de juez independiente:** el que ejecuta no es el único que juzga. Fara ejecuta → oracles deterministas + visual_ai (¿mismo modelo? opcional pero señalado) + M3 judge + humano.

### 2.5 Review Policy (Capa 4)

```yaml
review:
  policy: risk_based           # en vez de revisar el 20% fijo
  require_human_when:
    - business_criticality >= high
    - ai_confidence < 0.85
    - oracle_conflict == true      # dos oracles discrepan
    - first_execution == true
    - visual_change == significant
    - historical_failure_rate > 0.2
  sampling: 0.02               # 1-5% aleatorio incluso en PASS confiados
```

### 2.6 Estados separados (el cambio de dominio más importante)

```text
Execution result   : PASSED | FAILED | BLOCKED | ERROR | SKIPPED
Machine assessment : SUPPORTED_PASS | SUPPORTED_FAIL | UNCERTAIN | CONFLICTING
Human decision     : PENDING | APPROVED | REJECTED | WAIVED
Acceptance status  : ACCEPTED | REJECTED | CONDITIONAL | PENDING
```

**`PASSED != ACCEPTED`** — propiedad de dominio central.

### 2.7 Nuevos artefactos de dominio

| Artefacto | Descripción |
|---|---|
| `UatScenarioV3` | 4 ejes: executor / evidence / oracles / review |
| `UatExecutorSpec` | kind + params (command, url, goal, model, budget) |
| `UatEvidenceBundle` | conjunto content-addressable + env + execution metadata |
| `UatOracleSpec` + `UatOracleAssessment` | criterio + resultado (pass/fail/uncertain/conflicting + confidence) |
| `UatReviewPolicy` | reglas `require_human_when` + sampling |
| `UatTestabilityReport` | readiness: deterministic/browser_automatable/agentic_automatable/requires_human + recommended_executor/oracles |
| `UatValidationSession` | Launch unificado (release, commit, env, n ejecuciones, n reviews) |
| `UatMission` | exploratory testing (goal, budget actions/time, findings) |
| `UatDisagreement` | machine PASS / human REJECT → dataset de aprendizaje |
| `UatAcceptanceRecord` | quién aceptó, cuándo, contra qué evidencia |
| `UatEvent` | log inmutable: PlanProposed→…→AcceptanceGranted/Revoked→ReleaseGateEvaluated |

---

## 3. Flujo agentic objetivo

```text
REQUIREMENTS
    │
    ▼
Test Designer AI ──► Coverage Agent ──► HUMAN PLAN APPROVAL
    │                    │
    └────────────────────┴──► Testability Agent (recomienda executor/oracles)
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
         SCRIPT/CLI          PLAYWRIGHT             FARA (CUA)
              │                   │                   │
              └───────────────────┼───────────────────┘
                                  ▼
                            EVIDENCE STORE
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
      deterministic          visual_ai            telemetry
         oracles              oracle               oracle
              │                   │                   │
              └───────────────────┼───────────────────┘
                                  ▼
                            QUALITY JUDGE
                                  │
                          confidence / risk
                                  │
                    ┌─────────────┴─────────────┐
                    ▼                           ▼
              AUTO VERIFIED               HUMAN REVIEW
                    │                           │  (queue + sampling +
                    └─────────────┬─────────────┘   disagreement capture)
                                  ▼
                            ACCEPTANCE
                                  │
                                  ▼
                            RELEASE GATE
```

**Separation of Duties:** Planner ≠ Executor ≠ Judge ≠ Approver. El que diseña no aprueba sus tests; el que ejecuta no emite aceptación; el judge no modifica evidencia; solo el humano aprueba.

---

## 4. Cambios necesarios por capa

### 4.1 `sddk-domain` — schema v3 (+migrador)

| Cambio | Detalle |
|---|---|
| `UatScenarioV3` | Reemplaza `automation.status` por executor/evidence/oracles/review |
| Nuevos tipos | §2.7 (13 tipos nuevos) |
| `UatStatus` → 3 estados | Mantener `UatStatus` como *execution result*; añadir `UatAssessment`, `UatHumanDecision`, `UatAcceptanceStatus` |
| Migración v2→v3 | `migrate_plan_v2_to_v3` (patrón de `migrate_plan_v1_to_v2`): `automation.status` → executor + review heurística |
| Event log | `UatEvent` enum + helpers |
| `LATEST_PLAN_SCHEMA_VERSION = 3` | Acepta v1/v2/v3 en validación; renderer soporta las 3 |

### 4.2 `sddk-gateway` — nuevos executors

| Cambio | Detalle |
|---|---|
| `PlaywrightExecutor` | Wrapper sobre CLI `playwright` (disponible en `~/.local/bin`): `sddk_gateway::playwright::run(spec) -> EvidenceBundle` — navega, actúa, captura trace/screenshots/console/network/geometry/axe |
| `ComputerUseExecutor` | Adaptador HTTP a Fara (llama.cpp `:8082`, patrón `cua-test-orchestrator`): goal → trajectory → screenshots; estado del server comprobado (hoy DOWN) |
| `EvidenceCollector` | Normaliza salidas de cualquier executor a `UatEvidenceBundle` content-addressable |
| Oracles deterministas | `oracles.rs`: exit_code, http, text, json_schema, dom, geometry, accessibility, visual_diff — sin IA, testables |

### 4.3 `sddk-cli` — nuevos subcomandos

```text
uat testability --plan FILE        # Testability Agent: recomienda executor/oracles por scenario
uat run --executor playwright ...  # ejecuta con cualquier executor, genera bundle
uat assess --session FILE          # corre oracles sobre la evidencia → machine assessment
uat review --queue [--risk-filter] # Human Review Queue: pendientes, evidencia, Approve/Reject/Waive
uat accept --scenario S --decision approved|rejected|conditional
uat mission --plan FILE --goal "..."  # exploratory con Fara: budget, findings
uat session --release X            # ValidationSession (Launch) — contexto unificado
uat event log --release X          # evento inmutable reconstruible
```

`uat run` actual (scripted) se convierte en `executor: cli|script` — sin perder nada.

### 4.4 Agentes

| Agente | Cambio |
|---|---|
| `uat-planner` | Evoluciona a **Test Designer** (propone casos desde requisitos + coverage) |
| `uat-guide` | Mantiene enriquecimiento junior; añade oracles sugeridos |
| `uat-runner` | Se convierte en dispatcher de executors (Fara CUA + script + playwright) |
| `uat-reporter` | Evoluciona a **Quality Judge** (sintetiza assessments + confidence + risk) |
| **NUEVO `uat-testability`** | Analiza cada scenario → recommended executor/oracles/review (Qase Automation Readiness) |
| **NUEVO `uat-reviewer`** | Presenta la review queue, captura decisiones + disagreements |
| **NUEVO `uat-coverage`** | Detecta gaps/duplicados entre requisitos y scenarios |

### 4.5 Skills

| Skill | Cambio |
|---|---|
| `uat-evidence` | Ampliar a EvidenceBundle (trace, network, a11y, geometry) |
| `uat-guided-mode` | Mantener (el humano sigue usando el wizard) |
| `uat-traceability` | Ampliar al Evidence Graph (requirement→scenario→execution→evidence→oracle→assessment→review→acceptance) |
| `uat-dashboard` | Nueva view: **Human Review Queue** |
| **NUEVO `uat-playwright-executor`** | Patrón Playwright como sensor + actuador (no juez) |
| **NUEVO `uat-oracles`** | Catálogo de oracles con contracts |

### 4.6 Dashboard kit (views)

| View | Cambio |
|---|---|
| `guided.html` | Mantener (wizard humano) |
| `interactive.html` | Ampliar: ver evidencia por scenario (trace/screenshots), assessments |
| `report.html` | Ampliar: verdict + confidence + disagreements |
| **NUEVO `review-queue.html`** | La pantalla central: 12 critical / 31 review / 147 auto-verified; tarjetas con machine assessment + evidencia + Approve/Reject/Needs-work |
| **NUEVO `evidence-viewer.html`** | Trace viewer, screenshots, console/network/a11y por execution |

### 4.7 Workflow y gates

```text
phase.uat.complete: requires uat-report + uat-verdict
    → NUEVO: release-uat-approved exige AcceptanceStatus != PENDING
    → NUEVO: uat-accepted event en ledger (con quién/cuándo/evidencia)
    → MANTENER: uat-skipped auditable (RNF-010)
```

### 4.8 REQs y ADRs

| Documento | Cambio |
|---|---|
| **NUEVO ADR-014** | "Human-Governed AI Quality: executor/evidence/oracle/review/acceptance separation" — supersede parcialmente ADR-012 §5 |
| `REQ-RF-019` | Ampliar: ejecución MAY ser humano, scripted, playwright, computer_use o mixto; assessment ≠ acceptance |
| `REQ-RF-020` | Ampliar: dashboard incluye review queue y evidence viewer |
| **NUEVO REQ** | Testability analysis (recomendación de executor/oracles) |
| **NUEVO REQ** | Sampling humano + disagreement dataset |
| **NUEVO REQ** | PASSED ≠ ACCEPTED (estados separados) |

---

## 5. Fases de implementación (work units)

> Cada fase deja el repo en estado verde (tests + gates). Orden = dependencias.

| Fase | Work unit | Entregable | Depende |
|---|---|---|---|
| **F0** | ADR-014 + diseño schema v3 | Documento de decisión + tipos de dominio | — |
| **F1** | Domain: schema v3 + migrador v2→v3 | `uat.rs` v3, `migrate_plan_v2_to_v3`, tests | F0 |
| **F2** | Gateway: PlaywrightExecutor + EvidenceCollector | `playwright.rs`, `evidence.rs`, tests con Playwright real | F1 |
| **F3** | Oracles deterministas | `oracles.rs` (exit_code/http/text/json_schema/dom/geometry/a11y/visual_diff) + tests | F2 |
| **F4** | CLI: `uat assess` + `uat run --executor playwright` | Evaluación de bundles, ejecución con evidencia completa | F2+F3 |
| **F5** | Testability agent + CLI `uat testability` | `uat-testability` agente, análisis por scenario | F1 |
| **F6** | Review policy engine + sampling + CLI `uat review` | Risk-based queue, sample 1-5%, `uat accept` | F3+F5 |
| **F7** | Disagreement dataset + ValidationSession + event log | `uat session`, `uat event log`, `UatDisagreement` | F6 |
| **F8** | ComputerUseExecutor (Fara CUA) + exploratory `uat mission` | Trajectory, budget, findings | F2+F7 |
| **F9** | Dashboard: review-queue + evidence-viewer + report ampliado | HTML kit v3 | F6+F7 |
| **F10** | Workflow gates + REQ/ADR updates + migración dogfood | `release-uat-approved` v3, ledger events, docs | F7+F9 |
| **F11** | Release v1.7.0 + dogfood del ciclo completo | UAT v3 usado contra el propio framework | F10 |

### Fases F12-F14 — Guided UAT Runner (ADR-015, `docs/uat/GUIDED-UAT-DESIGN.md`)

| Fase | Work unit | Entregable | Depende |
|---|---|---|---|
| **F12** | UAT Form DSL (domain) + Schema Validator + Wizard Compiler + renderer determinista | `UatStep v3` (instruction/expected/observation/check/evidence/branch), `UatCheck`, `UatCheckpoint`, `UatCompletionPolicy`, validación DSL, componentes UI del kit | F1, F9 |
| **F13** | Runner UX: inbox, wizard por paso, blind checks, ratings, evidence gates, AI diagnostics, checkpoints + modos Designer/Reviewer + sign-off inmutable + staleness | Guided Runner como app de primera clase; RELEASE ACCEPTANCE wizard; `UatAcceptanceRecord` con snapshot | F12, F6, F7 |
| **F14** | UX Form Agent + Form Quality Agent + Test Discovery Agent | Pipeline completo: criterio semántico → interacción óptima; anti-patrones (arXiv:2308.01386); descubrimiento desde la app real (Fara+Playwright) | F5, F8, F12 |

**Estimación:** F0-F3 core (schema+executors+oracles) ≈ 45% del valor; F6-F7 (governance) ≈ 20%; F12-F13 (Guided Runner) ≈ 25%; F8-F9 (Fara CUA + dashboard) ≈ 10%.

---

## 6. Decisiones abiertas (necesito tu input)

1. **¿Schema v3 rompe v2 o coexiste?** Recomiendo: `schema_version: 3` con migrador v2→v3 automático (patrón ya existente), validación acepta v1/v2/v3. El dashboard renderiza las 3.
2. **¿Playwright executor en Rust nativo o wrapper CLI?** Recomiendo wrapper sobre el CLI global (más rápido de iterar; el CLI ya existe). Rust nativo sería más tarde si hace falta.
3. **¿Fara CUA vía HTTP directo a llama.cpp o vía agente `cua-test-orchestrator`?** Recomiendo: gateway adaptador HTTP directo (ejecución reproducible en CLI), el agente queda como orquestador de alto nivel.
4. **¿El event log UAT va al ledger SQLite existente o a un store propio?** Recomiendo: eventos UAT como frames en el ledger existente (ADR-003), con `uat.event` prefix.
5. **¿La review queue es HTML autocontenido (patrón actual) o servidor local?** Recomiendo HTML autocontenido + localStorage (consistente con ADR-010/013); el servidor local queda para colaboración multi-persona futura.
6. **¿El sampling 1-5% se activa por defecto?** Recomiendo sí, configurable por proyecto (`uat.review.sampling: 0.02`).

---

## 7. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| Schema v3 grande → critical path lento | Fases incrementales; el renderer acepta v1/v2/v3; el migrador es automático |
| Fara 9B juzgando su propia ejecución (sesgo) | Oracle independiente por diseño; `visual_ai` opcional; M3 judge + humano |
| Playwright flaky en CI/local | Traces como evidencia; retries; flakiness_score en history |
| Modelo 9B no fiable para juicio fino de diseño | Solo smoke visual + regresión gruesa; lo fino queda al humano (policy risk_based) |
| Disagreement dataset con datos sensibles | Traces/logs pueden contener credenciales (Playwright docs) → redacción + local-only |
| El humano se convierte en cuello de botella | Sampling + risk_based + auto-verified; el humano solo ve lo que importa |
| Over-engineering (6 capas para un framework local) | Fases F0-F3 primero; F8-F9 solo si demuestran valor en dogfooding |

---

## 8. Lo que NO cambia

- Local-first, cero servidores/red (ADR-010).
- Cero intrusión en repos de proyectos (ADR-0011).
- Evidence sha256-pinned.
- Runner tipado sin shell.
- Agentes producen YAML, no HTML.
- `uat-skipped` auditable (RNF-010).
- El wizard guiado para juniors (sigue siendo el modo humano principal).

---

## 9. Criterios de salida (definition of done)

- [ ] `automation.status` eliminado del schema canónico (reemplazado por 4 ejes), migrador v2→v3 automático
- [ ] `uat run --executor playwright|cli|script|computer_use` produce EvidenceBundle content-addressable
- [ ] Oracles deterministas evaluables por CLI (`uat assess`) sin IA
- [ ] `uat testability` recomienda executor/oracles por scenario
- [ ] Review policy risk_based + sampling funciona; `uat review` muestra queue con evidencia
- [ ] PASSED ≠ ACCEPTED en el dominio; gate exige acceptance
- [ ] Human Review Queue renderiza en el dashboard
- [ ] Disagreement dataset capturado (machine PASS / human REJECT)
- [ ] Event log reconstruible (`uat event log` responde "¿por qué se aceptó este release?")
- [ ] 250+ tests workspace verde, clippy -D warnings, lint 0/0, 4 suites shell verdes
- [ ] Dogfooding: UAT v3 validando una release real del framework
