# GUIDED-UAT-DESIGN — Guided UAT Runner: AI-generated, machine-observed, human-validated, evidence-backed

**Status:** IMPLEMENTED
**Date:** 2026-08-10
**Supersedes:** n/a (extiende `PLAN-uat-v3-quality-control-plane.md` F0-F11)
**Core rule:** los agentes NUNCA generan HTML/JavaScript arbitrario. Generan una **especificación declarativa** (`UAT Form DSL`); un **renderer determinista** la convierte en wizard, formularios, checks, branching y pantallas de revisión. UX, seguridad, accesibilidad y trazabilidad quedan controladas por el framework.

---

## 1. Principios de diseño

1. **Agentes especifican, el renderer dibuja.** `Test Designer → Form UX Agent → Schema Validator → Wizard Compiler → UI Renderer`. El agente produce YAML/JSON validado; el frontend lo compila a componentes conocidos.
2. **Nunca preguntar al humano lo que la máquina ya sabe.** Si Playwright conoce URL/viewport/browser/console/HTTP/DOM/ARIA, no se pregunta. Si Fara observa un modal o un texto cortado, se muestra como sugerencia validable (Correct/Incorrect/Not relevant).
3. **El humano observa, no pulsa "Next".** Blind checks, observaciones con respuesta esperada oculta, ratings con escala — para evitar el efecto "Next, Next, Next" sin mirar.
4. **Checks con identidad visual distinta**: determinista (verde ✓), browser (verde ✓), IA (azul/morado ◉ con confidence), humano (○ pendiente hasta respuesta).
5. **Checkpoint humano > confirmación por click.** Bloques de pasos con verificación automática + checkpoint de aprobación.
6. **FAIL no es un formulario vacío**: al fallar, la app ya recolectó screenshot/trace/console/network/DOM y propone causa + título de defecto (AI diagnostics).
7. **AI-generated ≠ AI-trusted.** Procedencia en cada artefacto (agente, modelo, based_on, confidence, human reviewed) + detección de staleness cuando la UI cambia.
8. **Bloqueo por política, no por HTML.** `blocking`, `required`, `evidence_required` son datos del DSL; el renderer impide continuar sin cumplirlos.
9. **Tres modos de interfaz**: Designer (QA/autor), Runner (validador humano), Reviewer (responsable UAT/sign-off).

---

## 2. Modelo de datos — escenario como grafo de pasos

```text
Scenario
  ├── Step
  │     ├── Instruction        (qué hacer)
  │     ├── ExpectedResult     (qué debe ocurrir; visibility hidden = blind)
  │     ├── ObservationRequest (qué observar / responder)
  │     ├── Check[]            (auto/browser/AI/human; blocking; required)
  │     ├── EvidenceRequirement(qué capturar; accepted kinds)
  │     └── Branch             (on pass/fail/blocked → goto)
  ├── Checkpoint               (bloque humano: approve/reject con evidencia máquina)
  ├── ReviewPolicy             (risk_based; require_human_when; sampling)
  └── CompletionPolicy         (cuándo el scenario está completo)
```

```yaml
scenario:
  id: UAT-127
  title: Create workspace
  intent: >
    Verify that a new user can create and enter a workspace.
  steps:
    - id: create-workspace
      instruction:
        type: form_action
        fields: [{ label: Workspace Name, value: UAT Workspace }]
        action: { label: Create Workspace }
      expected:
        text: The workspace dashboard opens.
      checks:
        - { type: http, status: 201 }
        - { type: dom, role: heading, text: UAT Workspace }
        - { type: visual_ai, criterion: "Dashboard visible, no overlays" }
        - { type: human_observation, question: "Does it appear usable?" }
      evidence:
        screenshot: required
      on_failure:
        require_comment: true
        offer_create_defect: true
```

---

## 3. UAT Form DSL — vocabulario cerrado

### 3.1 Inputs humanos

```text
confirm | yes_no | pass_fail | single_choice | multi_choice | text | textarea |
number | rating | date | duration | select | checkbox | checklist
```

### 3.2 Evidencia

```text
screenshot | video | file | annotation | browser_trace | console | network |
log | url | clipboard
```

### 3.3 Validaciones automáticas (oracles)

```text
http | json | text | dom | aria | geometry | visual_diff | visual_ai |
accessibility | performance | cli | database | custom_script
```

### 3.4 Elementos informativos

```text
instruction | warning | expected_result | tip | reference | image | code | link | example
```

### 3.5 Control de flujo

```text
next | previous | skip | block | retry | branch | repeat | goto | stop
```

### 3.6 Check — campos

```yaml
check:
  id: CH-03
  type: human_rating | blind_observation | human_confirmation | http | dom | ...
  executor: auto | playwright | fara | human
  oracle: exit_code | http | text | dom | aria | geometry | visual_diff | visual_ai | accessibility | human
  visibility:
    expected_result: visible | hidden     # hidden = blind check
  required: true
  blocking: true
  confidence_requirement: 0.85            # oracles semánticos
  evidence_requirement:
    required: true
    accepted: [screenshot]
  comment:
    required_when: [fail, partial]
```

---

## 4. Máquina de estados del runner

```text
ASSIGNED → STARTED → [STEP n]
                      ├─ pass ──────────────► next step (branch on pass)
                      ├─ fail ──► DIAGNOSE ─► defect capture ─► next/branch on fail
                      ├─ partial ─► comment required ─► next
                      └─ blocked ─► environment diagnostics ─► branch on blocked
[CHECKPOINT] ─► approve | reject ─► continue
COMPLETED ─► EVIDENCE BUNDLE ─► ASSESSMENT ─► ACCEPTANCE (PASSED != ACCEPTED)
```

Branching dinámico en el DSL:

```yaml
on:
  pass: { goto: step-4 }
  fail: { goto: diagnose-modal-failure }
  blocked: { goto: environment-diagnostics }
```

---

## 5. Tipos de check — identidad visual

| Tipo | Ejemplo | Icono/color |
|---|---|---|
| Determinista | `✓ API returned HTTP 201` | ✓ verde |
| Browser medido | `✓ Heading "UAT Workspace" found` | ✓ verde |
| IA (VLM/LLM) | `◉ Dashboard visually usable — Fara PASS 91%` | ◉ azul/morado |
| Humano pendiente | `○ Layout understandable — [Yes][No][Partially]` | ○ hasta respuesta |

El humano entiende al instante qué validó una máquina y qué está validando él.

---

## 6. Blind checks y observaciones (anti "Next, Next, Next")

```yaml
check:
  type: blind_observation
  question: "What confirmation message appears?"
  oracle: { expected: "Workspace created successfully" }
  visibility: { expected_result: hidden }
```

El tester escribe/observa sin conocer lo esperado; la app compara y muestra `✓ MATCH`. Convierte al humano en **observador real**, no en pulsador.

### Observación guiada (el agente conoce la respuesta, no la muestra)

```yaml
observation:
  question: "Which message appears after saving?"
  options: [Saved, Project saved successfully, Successfully updated, No message appears]
```

### Rating subjetivo (UX)

```yaml
check:
  type: human_rating
  question: "How easy was it to find Create workspace?"
  scale: { min: 1, max: 5, anchors: {1: Very difficult, 5: Immediate} }
  require_comment_below: 3
```

---

## 7. Evidence Required y Actual Result

- `evidence.required: true` + `accepted: [screenshot]` → la UI impide `Continue` sin evidencia.
- **Actual Result es concepto de dominio** (`actual: {workspace_count: 1}`), autocompletado por agentes:

```text
AI observed: Workspace "UAT Workspace" detected.
[✓ Correct] [Edit observation]
```

- Regla UX: **never ask a human to manually re-enter machine-observable information**.

---

## 8. Human Checkpoints

```text
steps 1-4 → automatic verification → HUMAN CHECKPOINT → steps 5-9 → ... → FINAL ACCEPTANCE
```

```yaml
checkpoint:
  id: CP-1
  title: Account Creation
  evidence_summary:
    machine: { passed: 8, total: 8 }
    fara: { assessment: likely_correct, confidence: 0.96 }
    anomalies: []
  validation:
    - "The resulting UX corresponds to the requirement."
    - "No unexpected UI state is visible."
    - "I consider this section acceptable."
  actions: [approve, reject]
```

---

## 9. AI-generated diagnostics (FAIL inteligente)

Al pulsar FAIL, el runner presenta:

```text
Failure detected — evidence already collected:
  ✓ screenshot  ✓ browser trace  ✓ console  ✓ network failures  ✓ current DOM  ✓ Fara trajectory

Possible cause:
  POST /api/workspaces returned 403.
Likely category: Authorization / Backend
Suggested defect title: "Workspace creation fails with HTTP 403"
Observed: ... Expected: ...
[Create defect] [Something else happened]
```

---

## 10. Pipeline de generación (agentes)

```text
Requirement
  → Scenario Agent (propone scenarios)
  → Coverage Agent (gap analysis)
  → Testability Agent (executor/oracles recomendados)
  → UX Form Agent (transforma criterio semántico → mejor interacción)
  → Critic Agent (anti-patrones; schema validation)
  → HUMAN APPROVAL
  → Published UAT (procedencia + staleness tracking)
```

### UX Form Agent — responsabilidad

No decide QUÉ probar (eso es Test Designer). Transforma un criterio en interacción óptima:

```yaml
# criterio: "After saving, the project appears in the list"
→ blind_observation: "Enter the number of projects visible"
→ machine_check: DOM contains project
→ human_confirmation: project visually identifiable
→ screenshot: auto-captured
```

### Form Quality Agent — anti-patrones (test smells, arXiv:2308.01386)

```text
❌ ambiguous instruction         ❌ expected result absent
❌ human asked to observe        ❌ leading question
   machine-verifiable info
❌ duplicated check              ❌ subjective criterion without scale
❌ failure without evidence      ❌ step too large
❌ excessive number of steps     ❌ no recovery path
❌ hidden prerequisite
```

---

## 11. Test Discovery Agent (generación desde la app real)

```text
Requirement + application running + code/knowledge graph
  → Test Discovery Agent
      ├── explora con Fara (CUA)
      ├── inspecciona DOM via Playwright
      ├── observa llamadas API
      └── captura screenshots
  → Actual Application Model
  → Generated Guided UAT (flujo real descubierto, no inventado)
```

Ejemplo: requisito "A user can delete a workspace" → descubre modal de confirmación + escribir nombre + redirect → genera wizard de 5 pasos.

---

## 12. Procedencia y staleness

```yaml
provenance:
  generated_by: ScenarioAgent
  model: MiniMax-M3
  based_on: { requirement: REQ-721, commit: 8acc72, exploration: EXP-81 }
  confidence: 0.91
  human_reviewed: { by: Ana, at: 2026-08-10 }
```

Staleness: cuando la UI cambia (`Create project` → `New project`), el sistema marca los UAT afectados:

```text
⚠ Scenario may be stale — affected: UAT-17, UAT-18, UAT-21
[Review proposed updates]
```

---

## 13. Tres modos de interfaz

| Modo | Rol | Pantallas | Status |
|---|---|---|---|
| **Designer** | QA / product owner | requirements, scenarios, AI suggestions, coverage, form editor, testability | ✅ Implementado (dashboard editor) |
| **Runner** | Validador humano | wizard, inbox, evidence, observation, pass/fail, checkpoints, AI diagnostics | ✅ Implementado (guided.html) |
| **Reviewer** | Responsable UAT | evidence, AI assessment, disagreements, defects, sign-off wizard, release acceptance | ✅ Implementado (guided.html + releaseAcceptanceView) |

### Runner — pantalla inicial (inbox)

```text
MY VALIDATIONS
  Requires your attention — 7
  UAT-341 Graph Explorer  HIGH  12 min   preflight 18 checks + 2 human  [Start]
  UAT-327 Authentication  MEDIUM  5/8 completed  [Continue]
  UAT-312 Settings  HIGH  Blocked — environment unavailable  [View]
```

### Reviewer — RELEASE ACCEPTANCE (wizard final)

```text
RELEASE ACCEPTANCE — Release 0.14
  142 scenarios: 119 machine ✓ | 17 human ✓ | 4 conditional ⚠ | 2 rejected ✕
  Critical requirements: ✓✓✓⚠✓
  Open defects: P0 0 | P1 1 | P2 4
  AI assessment: confidence HIGH — main risk: CSV export on Firefox
  ○ Accept release  ○ Accept conditionally  ○ Reject release
  Decision justification [____________]
  □ I reviewed the outstanding risks.
  [ Sign off ]
```

### Firma de aceptación inmutable

```yaml
acceptance:
  decision: accepted | accepted_conditional | rejected
  actor: { id: user:421 }
  timestamp: 2026-08-10T14:12:51Z
  plan_version: sha256:...
  evidence_snapshot: sha256:...
  outstanding_findings: [ISSUE-381]
  justification: "Known Firefox issue does not block release."
```

Válida incluso si los tests cambian después (snapshot inmutable).

---

## 14. Renderer — restricción arquitectónica

```text
NO:  AI → generate_html() → <div><script>...
SÍ:  Agent → Guided Test Specification (YAML validado)
     → Schema Validator → Wizard Compiler → UI Renderer → HTML/WASM/SPA
```

- Componentes deterministas (kit existente `assets/uat-dashboard/` ADR-013 se extiende).
- Sin HTML/JS arbitrario del agente → seguridad, a11y y trazabilidad controladas.
- Misma especificación → mismo HTML (RNF-001, determinismo).

---

## 15. Encaje con el plan UAT v3 (fases)

La capa Guided UAT Runner se integra como **fase F12 (Form DSL + renderer) y F13 (Runner UX + checkpoints + diagnostics)**, con dependencias:

| Fase | Nueva responsabilidad | Depende de |
|---|---|---|
| **F12** | `UAT Form DSL` (domain: UatStep v3 con instruction/expected/observation/check/evidence/branch, UatCheck, UatCheckpoint, UatCompletionPolicy) + Schema Validator + Wizard Compiler + renderer determinista | F1, F9 (kit) |
| **F13** | Runner UX (inbox, wizard por paso, blind checks, ratings, evidence gate, AI diagnostics, checkpoints) + Designer/Reviewer modes + sign-off wizard + staleness | F12, F6, F7 |
| **F14** | UX Form Agent + Form Quality Agent + Test Discovery Agent (pipeline §10-11) | F5, F8, F12 |

## 16. REQs nuevos propuestos

| REQ | Contenido |
|---|---|
| REQ-RF-024 | Guided UAT Runner: renderer determinista de la spec declarativa; agentes nunca generan HTML/JS |
| REQ-RF-025 | UAT Form DSL: vocabulario cerrado (inputs/evidence/oracles/informativos/flujo/checks) |
| REQ-RF-026 | Blind checks + observaciones + ratings; anti "Next, Next, Next" |
| REQ-RF-027 | Human checkpoints + AI diagnostics en FAIL + Actual Result de dominio |
| REQ-RF-028 | Tres modos (Designer/Runner/Reviewer) + sign-off inmutable con snapshot |

## 17. Criterios de salida

- [x] Un agente genera spec YAML validada → el renderer produce el wizard sin HTML del agente
- [x] Blind check: el tester no ve el expected hasta confirmar; MATCH automático
- [x] Evidence gate: `Continue` bloqueado sin evidencia requerida
- [x] FAIL presenta diagnostics automáticos (screenshot/trace/console/network + causa propuesta + defect)
- [x] Checkpoint humano aprueba/rechaza bloque con resumen máquina
- [x] Sign-off inmutable con plan_version + evidence_snapshot sha256
- [x] Staleness detecta cambios de UI y marca UAT afectados
- [x] 350+ tests workspace verde, clippy -D warnings, lint 0/0
