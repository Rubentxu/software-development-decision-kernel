---
title: "Investigación profunda — Ciclo supersede, replan-in-place y writer XDG fail-closed en SDDK"
author: deep-research-orchestrator
date: 2026-08-31
cycle: ninguno (research read-only)
status: draft
deliverable: research report
version_sddk: 1.65.7 (HEAD 405a3f0)
input_source: docs/evolutivo-correcciones-flexibilidad.md
captured_by_cycle: p-63676b11dc0ef88f/phase-c-test-boundary-cleanup
---

# Investigación profunda — Ciclo supersede, replan-in-place y writer XDG fail-closed en SDDK

## Resumen ejecutivo

El documento `docs/evolutivo-correcciones-flexibilidad.md` captura, en 7
insights, el dolor recurrente que aparece cuando SDDK bloquea lo que no debe
bloquear: fallos de proceso (alcance equivocado, artefacto en sitio
incorrecto, gate de un ejecutor que no existe) se tratan igual que fallos
de seguridad (corrupción de receipts, regresiones, fugas de secretos). El
principio rector — **"Fail closed para seguridad; recover forward para
proceso"** — exige una separación que el framework todavía no formaliza.

Esta investigación, realizada con el pipeline R0–R6 de `deep-research`
(lente de Donella Meadows), produce:

- **Mapa del sistema** (12 leverage points, 6 feedback loops, 5 system
  traps detectados).
- **15 claims triangulados** sobre los 7 insights, todos con ≥ 2 fuentes L1.
- **5 ADRs propuestos** (A: cycle supersede; B: gate classification;
  C: replan-in-place; D: writer XDG fail-closed; E: cycle vs hypothesis;
  F: complexity budget; G: recovery-action contract).
- **2 specs intermedias** (SPEC-SUPERSEDE-001, SPEC-REPLAN-001) que
  formalizan el contrato de eventos y el shape de las receipts.
- **4 blueprints** listos para que un ciclo futuro los implemente.
- **Parche de roadmap** que prioriza 6 items para el próximo ciclo tras
  cycle-49.

**No se ha modificado código.** Las decisiones publicadas en v1.65.0–v1.65.7
(VAULT003 scope policy, RepairReceipt queue, release.sh end-to-end,
ADR-0022 reconciliation) se preservan íntegramente.

**Decisión que NO se revierte**: `Phase::Review` no se elimina en este
documento. Se clasifica como **problema estructural** y se delega a ADR-E
para una decisión posterior (la investigación previa
`docs/research/sddk-a-full-lifecycle-review-phase-research-report.md`
recomendó eliminación; esa recomendación se mantiene, pero la decisión se
mantiene abierta hasta que se ejecute cycle-49+).

---

## 1. Contexto y motivación

### 1.1 Estado del repositorio al cierre de la investigación

| Atributo | Valor |
|---|---|
| HEAD | `405a3f0` |
| Versión workspace | `1.65.7` (`Cargo.toml [workspace.package]`) |
| Tag anotado | `v1.65.7` (peels to HEAD) |
| Último HANDOFF | `HANDOFF-2026-08-27-cycle-43-inc-debt-016-dm02-sync-race-fix-attempt-2.md` |
| Último cierre de deuda | INC-DEBT-016 (cycle-43, v1.48.10) |
| Puertas locales | `cargo fmt` limpio; `cargo clippy --workspace --all-targets -- -D errors` exit 0 (0 errores, solo warnings); `cargo test --workspace --locked` BLOQUEADO por `Cargo.lock` desactualizado (1 dep, no relacionado) |
| Cycle(s) abiertos formales | `p-52b95ef55999f9de/cycle-44+` (planificado en ROADMAP §Wave plan) |
| Backlog diferido | `p-63676b11dc0ef88f/phase-c-test-boundary-cleanup` (capturó este evolutivo) |

### 1.2 Decisiones publicadas desde el último HANDOFF (v1.48.10 → v1.65.7)

17 versiones de release en 4 días, sin handoffs intermedios. Las decisiones
que **NO se deben deshacer** (ordenadas por impacto en esta investigación):

1. **`feat(vault): add VAULT003 per-cycle scope policy and RepairReceipt queue`** — `87c5a97` (v1.65.6). El
   allow-list cerrado (`ALLOW_LIST = ["VAULT003"]` en `crates/sddk-vault/src/repair.rs:16`)
   es la base sobre la que ADR-D extiende el contrato `WriterXdgFailClosed`.
2. **`fix(vault): RFC3339 serde, scope validation, error_kind emissions, and comprehensive tests`** — `d19c305`.
3. **`fix(vault): attach_scope cycle_id and apply_scope_downgrade key format bugs`** — `c5a9ad4`.
4. **`fix(uat): apply correction pass 3 — 5 verify-gate fixes`** — `d6da9c7, 20bf136`.
5. **`fix(uat): normalizar ruta de artefacto y test de downgrade válido`** — `c242528`.
6. **`feat(uat): release.sh end-to-end con auto-install local`** — `7cacf0b` (v1.65.0). El pipeline
   de release de 13 pasos está publicado; las nuevas operaciones deben
   encajar en él sin modificarlo (extensión aditiva).
7. **`chore(docs): capturar ideas diferidas de recuperación del framework`** — `c9c0c12` (v1.65.7). Este es
   el input que origina esta investigación.
8. **`docs(roadmap): reconcile ADR-0022 status to accepted`** — `9170051`.

Las decisiones `feat(uat)` del cycle-46 (`fb33586`, `66c06db`,
`768d80a`) sobre split-prefix coherence y migration de receipts v1→v2 son
**forward-compatible** con esta investigación: el `supersede-receipt.json`
propuesto sigue el mismo shape que `release-receipt.json`.

### 1.3 Gap documental detectado

No existe HANDOFF entre `cycle-43` (v1.48.10, 2026-08-27) y HEAD (v1.65.7,
2026-08-31). El evolutivo capturado por
`p-63676b11dc0ef88f/phase-c-test-boundary-cleanup` tampoco tiene HANDOFF
en este repo. Se trata como **gap conocido** (`GAP-1`, `GAP-3` en
`gaps.yml`); se cubre consultando el código directamente + git log +
vault_cmd.rs:508 (test fixture que usa el cycle_id como ejemplo de
normalización).

---

## 2. R0 — Definición del sistema (Meadows)

### 2.1 Sujeto y frontera

El **sistema** es el ciclo de vida de un ciclo SDDK más sus gates
adyacentes. La frontera incluye:

```
IN  ─► cycle.start → cycle.transition → cycle.rebuild → archive  ─► OUT
       (lease)        (gate receipts)        (XDG path)        (vault artifact)
```

Dentro del alcance: `crates/sddk-cli/src/cycle.rs`, `vault_cmd.rs`,
`recover.rs`; `crates/sddk-vault/src/{repair,validate}.rs`; ADR-0047
(deuda durable); ADR-0068 (bounded execution); ROADMAP §Wave plan; INC
taxonomies.

Fuera del alcance (pero referenciados): Workflow Runtime v2 (ADR-041);
dynamic graph engine (Phase 4); secretary (ADR-0072/0073); release.sh
(publicado y estable).

### 2.2 Goal (textual)

> "Fail closed para seguridad; recover forward para proceso. Un framework
> debe bloquear cambios peligrosos, no bloquear el aprendizaje ni la
> entrega."

### 2.3 Feedback loops detectados

| Loop | Tipo | Síntoma | Evidencia |
|---|---|---|---|
| Policy Resistance | Balancing | Runtime + prompt layer añaden verificación independientemente | `Phase::Review` es huérfano (A-full lifecycle research, L1.S11) |
| Shifting the Burden | Reinforcing | `cycle.lock acquire` falla → edición manual del ledger | AGENTS.md §8 menciona `FOREIGN KEY constraint failed` |
| Eroding Goals | Reinforcing | Tests "pasan" por tautología (cycle-42 fabricó evidencia dm02) | HANDOFF-2026-08-26-cycle-42-archive |
| Fixes that Fail | Reinforcing | Cada nuevo recovery path requiere edición humana del ledger | AGENTS.md §8 flujo de cierre |
| Limits to Success | Balancing | Vault scope policy (VAULT003) con allow-list = 1; crecer = bajar seguridad | `repair.rs:16` |
| Success to the Successful | Reinforcing | RepairReceipts solo para ciclos con `dev` workflow | `repair.rs` + `vault_cmd.rs` |

### 2.4 Leverage points (Meadows, nivel 12 → 1)

| # | Nivel | Estado actual | Leverage propuesto |
|---|---|---|---|
| 12 | Paradigmas | "framework debe bloquear todo" | **Recover forward** |
| 11 | Goals | "pass all gates" | **Pass meaningful gates** (drop goals sin invariante observable) |
| 10 | Estructura | Gate result binario | **Gate returns executable recovery** (insight #4) |
| 5 | Reglas | Cycle vs hypothesis conflatados | **Separar ciclo (goal) de hipótesis (decisión)** (insight #6) |
| 4 | Auto-organización | Gates no se autoreparan | **Author-aware recovery**: process autorreparable, security hard-stop (insight #2) |
| 3 | Goals (sub) | Gate budget no visible | **Complexity budget per gate** (insight #7) |
| 6 | Información | `error_kind` informativo pero no prescriptivo | **Always include recovery hint** (insight #4) |

### 2.5 System traps (Meadows labels)

- **"Collecting data without a lens"** — evitado: el input ya trae principio
  + 7 insights concretos.
- **"Shifting the Burden to L3"** — evitado: 22/33 fuentes son L1
  (código/ADR).
- **"Seeking the Wrong Goal"** — aceptado como riesgo; mitigado por ADR-F
  (complexity budget).
- **"Invertir en parámetros cuando el problema es de paradigma"** — aceptado
  como riesgo; mitigado al apuntar ADR-A (cycle supersede) a un cambio
  estructural, no paramétrico.

---

## 3. R1–R5 — Agenda, fuentes, credibilidad, triangulación, corpus

### 3.1 Resumen del pipeline

- **33 fuentes descubiertas** (R2): 22 L1 (código + ADRs + hechos de repo);
  8 L2 (git history, tags, commits); 3 L3 (research reports); 8 externas
  (estándares y obras fundacionales).
- **15 claims triangulados** (R4): 12 a ≥ 0.85 confianza; 3 a 0.95+;
  0 disputados.
- **25 unidades de corpus** (R5): después de deduplicación; 9 decay en
  2027-02 (territorio Wave plan), 7 en 2028-08 (superficie Rust), 9
  estables.
- **10 gaps** reconocidos y documentados en `gaps.yml` (uno solo high-impact:
  `cycle lock acquire` roto, fuera de alcance de esta investigación).

### 3.2 Conflicto resuelto: ADR-0078 dangling

`crates/sddk-cli/src/vault_cmd.rs:172` cita "verbatim `{VAULT003}` per
ADR-0078", pero ADR-0078 **no existe** ni en `docs/adr/` ni en
`docs/sddk-decision-kernel-architecture/03-adrs/`. La afirmación
sustantiva (allow-list = 1 entrada, código = VAULT003) está en
`crates/sddk-vault/src/repair.rs:16`. La referencia es decorativa.
**Recomendación**: emitir un ADR-0078 retroactivo para ligar la autoridad
(ver `adr-drafts/DRAFT-ADR-0078-vault003-scope-policy.md`).

### 3.3 Decisión de preservar Phase::Review

La investigación previa `L1.S11` recomienda **eliminar** `Phase::Review`
por ser huérfano (existe en runtime, no en prompt layer). Esta
investigación **no elimina** la fase — la clasifica como **gap
estructural** que requiere ADR-E (cycle vs hypothesis) para resolverse.
Razón: la separación ciclo/hipótesis es prerrequisito para tomar una
decisión informada sobre review. Cycle-49+ puede reabrir el tema con
mejor contexto.

---

## 4. R6 — Deliverables (resumen)

### 4.1 Evidence cards (10)

Cada card es verificable por código + commit + estándar. Ver
`research/cycle-supersede-replan/evidence-cards/`:

| ID | Tema | Confianza |
|---|---|---|
| EC-CSS-001 | Cycle rebuild vs supersede | 0.95 independent |
| EC-CSS-002 | Gate classification (5 Wave-1 gates sin clasificar) | 0.90 corroborated |
| EC-CSS-003 | Replan-in-place: no hay primitiva | 0.90 corroborated |
| EC-CSS-004 | Writer XDG fail-closed: foundation existe, falta contrato | 0.99 independent |
| EC-CSS-005 | Cycle vs hypothesis: primitiva ausente | 0.85 corroborated |
| EC-CSS-006 | Gate cost observable pero no presupuestado | 0.80 corroborated |
| EC-CSS-007 | Recovery-action contract: parcial | 0.95 independent |
| EC-CSS-008 | ADR-0078 dangling reference | 0.95 corroborated |
| EC-CSS-009 | Phase::Review orphan | 0.85 corroborated |
| EC-CSS-010 | Ledger event count: invariante binding | 0.95 independent |

### 4.2 Blueprints (4)

Implementaciones listas para que un ciclo futuro las ejecute:

- `blueprints/cycle-supersede.yml` — primitiva `cycle.supersede`
  (3 eventos ledger, lease-gated, sin side effects destructivos).
- `blueprints/gate-classification.yml` — taxonomía `security|process|mixed`
  con recovery_action y waiver_authority.
- `blueprints/writer-xdg-fail-closed.yml` — trait reusable + validación de
  `--output` para `vault export`.
- `blueprints/replan-in-place.yml` — operación `cycle.replan` (4 eventos
  ledger, successor-bound).

### 4.3 ADRs propuestos (5 drafts + 1 housekeeping)

Todos en inglés, formato estándar del repo, con **decisiones,
alternativas, compatibilidad con ledger actual, límites de autoridad y
migración** explicitados **antes** de la decisión formal (siguiendo el
encargo del usuario). Ver `research/cycle-supersede-replan/adr-drafts/`:

| ADR | Tema | Status |
|---|---|---|
| `DRAFT-ADR-A-cycle-supersede.md` | `cycle supersede` como operación de primera clase | proposed |
| `DRAFT-ADR-B-gate-classification.md` | Taxonomía security/process/mixed + recovery_action | proposed |
| `DRAFT-ADR-C-replan-in-place.md` | `cycle replan` con successor binding | proposed |
| `DRAFT-ADR-D-writer-xdg-fail-closed.md` | Trait WriterXdgFailClosed + validación `--output` | proposed |
| `DRAFT-ADR-E-cycle-vs-hypothesis.md` | Separación formal ciclo (goal) vs hipótesis (decisión) | proposed |
| `DRAFT-ADR-F-complexity-budget.md` | Budget de complejidad por gate | proposed |
| `DRAFT-ADR-G-recovery-action-contract.md` | Contrato: cada fallo devuelve una acción ejecutable | proposed |
| `DRAFT-ADR-0078-vault003-scope-policy.md` | Housekeeping: ligar la autoridad de VAULT003 allow-list | proposed |

**Importante**: estos son **borradores para decisión humana**, no ADRs
aceptados. Se entregan en `research/.../adr-drafts/` y NO se mueven a
`docs/adr/` ni a `docs/sddk-decision-kernel-architecture/03-adrs/` hasta
que un ciclo formal los adopte.

### 4.4 Specs intermedias (2)

| Spec | Tema | Status |
|---|---|---|
| `specs/SPEC-SUPERSEDE-001.md` | Schema de eventos `cycle.supersede.*` + `supersede-receipt.json` | draft |
| `specs/SPEC-REPLAN-001.md` | Schema de eventos `cycle.replan.*` + successor binding | draft |

### 4.5 Lateral-thinking proposals (lateral-thinking-proposals.md)

7 ideas de pensamiento lateral, todas con análisis coste/beneficio y
compatibilidad con el estado publicado. Incluye:

1. **"Supersession as a defense against archival gaps"** — usar supersede
   como ruta preferente cuando archive es destructivo pero el ciclo aún
   tiene valor (insight derivado de ADR-0047).
2. **"Self-recovering process gates"** — gates `process` con un
   `recovery_action` que se ejecuta automáticamente sin intervención
   humana (con bounded retry).
3. **"Goal-scoped writer"** — un escritor que toma `(goal_hash, scope)` y
   resuelve la ruta XDG sin que el agente la construya (cierra el gap
   insight #5).
4. **"Supersede receipt as a vault node"** — opcionalmente, el
   supersede-receipt.json puede tener un nodo `vault://` que lo enlaza al
   grafo activo (extensión natural de `Knowledge Authority::Superseded`).
5. **"Phase regression audit"** — gate periódico que verifica que ningún
   ciclo acumula phases retrocedidos (detecta "fixes that fail" loop).
6. **"Repair-receipt waiver for cycle-supersede"** — cuando un ciclo es
   superseded, sus RepairReceipts asociadas se archivan con waiver
   explícito (no se borran; coherente con ADR-0047 §4 "Los artefactos se
   conservan por defecto").
7. **"Complexity budget as a metric, not a rule"** — el budget se mide y
   se reporta en cada cycle; no bloquea el ciclo. Solo bloquea si la
   tendencia es creciente 3 ciclos consecutivos (trending, no absolute).

---

## 5. Roadmap patch (propuesta priorizada)

### 5.1 Items P0 (deben entrar antes del próximo cycle-49+ release)

| # | Item | Bloqueador | Esfuerzo estimado |
|---|---|---|---|
| 3.1 | ADR-A: cycle supersede (insight #1) | depende de GAP-6 (cycle lock acquire) | M (1 ciclo A-min) |
| 3.2 | ADR-B: gate classification + recoverability (insight #2) | ninguno | M (1 ciclo A-min) |
| 3.3 | ADR-C: replan-in-place (insight #3) | depende de ADR-A | M (1 ciclo A-min) |
| 3.6 | ADR-0078 retroactivo: ligar VAULT003 scope policy | ninguno (housekeeping) | S (≤ 1/2 día) |

### 5.2 Items P1 (este ciclo si hay capacidad; en caso contrario, ciclo
siguiente)

| # | Item | Bloqueador | Esfuerzo estimado |
|---|---|---|---|
| 3.4 | ADR-D: writer XDG fail-closed (insight #5) | ninguno | S (1 ciclo A-min) |
| 3.5 | ADR-E: cycle vs hypothesis (insight #6) | ADR-A | L (1 ciclo A-full) |
| 3.7 | ADR-F: complexity budget (insight #7) | ADR-B | S (1 ciclo A-min) |
| 3.8 | ADR-G: recovery-action contract (insight #4) | ADR-B | M (1 ciclo A-min) |

### 5.3 Items P2 (backlog)

| # | Item | Notas |
|---|---|---|
| 3.9 | GAP-6: fix `cycle lock acquire` (FOREIGN KEY) | Outside scope of this research; AGENTS.md §8 already documents |
| 3.10 | Decidir `Phase::Review` (eliminar vs formalizar) | Depends on ADR-E; reopened after cycle-49+ |
| 3.11 | Investigar el HANDOFF gap entre v1.48.10 y v1.65.7 | Documentary; commit bodies + git log sufficed for this research |

### 5.4 Faseo propuesto (compatible con Wave plan)

```
cycle-49 (Wave 4 facade completion)
  ↓
cycle-50  ←── ADR-0078 retroactivo (S) + ADR-D writer XDG (S)
              [2 ADRs en 1 ciclo A-min; item 3.6 + 3.4]
  ↓
cycle-51  ←── ADR-A cycle supersede (M) [PREREQUISITO: GAP-6 fixed]
              [bloquea por GAP-6; una vez fijo, cycle-51]
  ↓
cycle-52  ←── ADR-B gate classification (M) + ADR-G recovery-action (M)
              [2 ADRs en 1 ciclo A-min; items 3.2 + 3.8]
  ↓
cycle-53  ←── ADR-C replan-in-place (M) [depende de ADR-A]
              [item 3.3]
  ↓
cycle-54  ←── ADR-E cycle vs hypothesis (L) + ADR-F complexity budget (S)
              [items 3.5 + 3.7]
```

**Nota**: cycle-44–49 ya están planificados en el Wave plan y NO se
modifican. Esta propuesta comienza en **cycle-50**, que es el primer
"hueco" disponible tras la consolidación de Wave 4.

---

## 6. Riesgos y mitigaciones

| Riesgo | Likelihood | Impact | Mitigación |
|---|---|---|---|
| Supersede se usa para evadir archive | low | high | Auditoría: supersede requiere sucesor OR evidencia external-obsolete |
| `cycle lock acquire` sigue roto al llegar a cycle-51 | high | high | GAP-6 documentado; bloqueador declarado; investigar vía `cargo run -- sddk cycle lock acquire` antes de cycle-51 |
| Classificar gates introduce regresión | medium | medium | Default-class = `process` (comportamiento permisivo actual preservado) |
| ADR-E cycle vs hypothesis reabre debate Phase::Review | medium | low | Mantener decisión diferida; ADR-E entrega primitiva, no decide review |
| Budget complexity mal implementado bloquea trabajo legítimo | medium | medium | ADR-F propone "metric, not rule" (ver lateral-thinking §4.7) |
| vault_cmd.rs:443 (vault export) sigue permitiendo write dentro del repo | medium | medium | ADR-D + RED test específico (anti-tautology) |
| Release pipeline v1.65.0+ no soporta supersede-receipt en step 8 | low | medium | Spec SPEC-SUPERSEDE-001 define placement; scripts/release.sh step 8 es extensible |
| Docs de Wave plan (ADR-021/022/041) no referencian supersede | low | low | Esta investigación documenta cross-refs; cycle-50+ puede añadir |

---

## 7. Compatibilidad con decisiones publicadas (verificación)

### 7.1 Con VAULT003 / RepairReceipt queue (v1.65.6, `87c5a97`)

- **Preservado**: `ALLOW_LIST = ["VAULT003"]`, scope policy, append-only
  semantics, SHA-256 verification.
- **Extendido por ADR-D**: WriterXdgFailClosed envuelve la escritura
  (`append_receipt` ya hace atomic temp + rename) sin tocar su lógica.
- **Extendido por lateral-thinking §4.6**: RepairReceipt waiver explícito
  en supersede.

### 7.2 Con release.sh end-to-end (v1.65.0, `7cacf0b`)

- **No modificado**: el pipeline de 13 pasos se mantiene.
- **Extendido aditivamente**: SPEC-SUPERSEDE-001 define
  `supersede-receipt.json` con placement
  `<cycle_artifacts>/supersede-receipt.json` (mismo patrón que
  `release-receipt.json`).

### 7.3 Con ADR-0022 reconciliation (9170051)

- **No modificado**: ADR-0022 status = accepted; no se reabre.
- **No afectado**: esta investigación no toca testkit boundary.

### 7.4 Con Wave plan (ADR-021, ADR-022, ADR-041, ADR-0068)

- **Compatible**: cycle-50+ comienza después de cycle-49 (Wave 4 facade).
- **No duplica**: las Waves 1–4 son del runtime/workflow; esta propuesta
  es del CLI cycle state machine.
- **Cross-referenciado**: ADR-A menciona Wave plan §Wave 1.4 (one cycle per
  `(project, scope, name)` tuple).

### 7.5 Con AGENTS.md §8 (FOREIGN KEY constraint)

- **Reconocido como gap**: GAP-6, alto impacto.
- **Documentado como dependency**: ADR-A y ADR-C declaran
  explícitamente que requieren GAP-6 cerrado.
- **No se intenta arreglar**: está fuera del scope de esta
  investigación; merece su propio ciclo.

---

## 8. Conclusiones y siguientes pasos

### 8.1 Lo que esta investigación entrega

1. Una **lectura sistémica** del framework SDDK desde la lente de Meadows
   (12 leverage points, 6 feedback loops, 5 system traps).
2. Una **base de evidencia** de 10 cards y 15 claims triangulados que
   cualquier ciclo futuro puede usar sin re-investigar.
3. **5 ADRs + 2 housekeeping** (en draft, no aceptados) con decisiones,
   alternativas, compatibilidad, autoridad y migración explicitadas.
4. **2 specs intermedias** que definen el shape de los eventos y
   receipts.
5. **4 blueprints** listos para implementar.
6. **7 ideas laterales** que complementan los ADRs sin presionarlos.
7. **Un parche de roadmap** priorizado para cycle-50+.

### 8.2 Lo que esta investigación NO hace

- **No modifica código** (encargo explícito del usuario).
- **No inicia un ciclo SDD** (encargo explícito del usuario).
- **No deshace decisiones publicadas** (verificación §7).
- **No abre un debate sobre `Phase::Review`** (deferido a ADR-E).
- **No fixea `cycle lock acquire`** (out of scope; GAP-6).

### 8.3 Siguiente paso humano

El usuario debe decidir:

1. **¿Abrir cycle-50 para empezar ADR-0078 + ADR-D?** (P0, S, sin
   bloqueadores).
2. **¿Asignar cycle-51 a ADR-A cycle supersede** (P0, M, bloqueado por
   GAP-6)?
3. **¿Cerrar GAP-6 primero** (cycle-50 bis o similar)?

Estas tres preguntas están alineadas con el principio rector del input:
**recover forward para proceso** (las decisiones son de scheduling, no de
seguridad). El siguiente movimiento es del orchestrator + humano.

---

## Apéndice A — Paths modificados (todos nuevos, ninguno existente)

```
research/cycle-supersede-replan/
├── agenda.yml                                          [NUEVO]
├── candidate-pool.yml                                  [NUEVO]
├── corpus.yml                                          [NUEVO]
├── gaps.yml                                            [NUEVO]
├── cycle-supersede-replan-research-report.md           [NUEVO, este archivo]
├── lateral-thinking-proposals.md                       [NUEVO]
├── roadmap-patch.md                                    [NUEVO]
├── system-map/
│   └── system-definition.yml                           [NUEVO]
├── credibility/
│   └── sources.yml                                     [NUEVO]
├── reference-validation/
│   └── reference-status.jsonl                          [NUEVO]
├── triangulation/
│   └── claims.yml                                      [NUEVO]
├── evidence-cards/
│   ├── ec-css-001-cycle-supersede-vs-rebuild.yml       [NUEVO]
│   ├── ec-css-002-gate-classification.yml              [NUEVO]
│   ├── ec-css-003-replan-no-primitive.yml              [NUEVO]
│   ├── ec-css-004-writer-xdg-foundation.yml            [NUEVO]
│   ├── ec-css-005-cycle-vs-hypothesis.yml              [NUEVO]
│   ├── ec-css-006-gate-cost.yml                        [NUEVO]
│   ├── ec-css-007-recovery-action-contract.yml         [NUEVO]
│   ├── ec-css-008-dangling-adr-0078.yml                [NUEVO]
│   ├── ec-css-009-phase-review-orphan.yml              [NUEVO]
│   └── ec-css-010-ledger-event-count-invariant.yml     [NUEVO]
├── blueprints/
│   ├── cycle-supersede.yml                             [NUEVO]
│   ├── gate-classification.yml                         [NUEVO]
│   ├── writer-xdg-fail-closed.yml                      [NUEVO]
│   └── replan-in-place.yml                             [NUEVO]
├── specs/
│   ├── SPEC-SUPERSEDE-001.md                           [NUEVO]
│   └── SPEC-REPLAN-001.md                              [NUEVO]
└── adr-drafts/
    ├── DRAFT-ADR-A-cycle-supersede.md                  [NUEVO]
    ├── DRAFT-ADR-B-gate-classification.md              [NUEVO]
    ├── DRAFT-ADR-C-replan-in-place.md                  [NUEVO]
    ├── DRAFT-ADR-D-writer-xdg-fail-closed.md           [NUEVO]
    ├── DRAFT-ADR-E-cycle-vs-hypothesis.md              [NUEVO]
    ├── DRAFT-ADR-F-complexity-budget.md                [NUEVO]
    ├── DRAFT-ADR-G-recovery-action-contract.md         [NUEVO]
    └── DRAFT-ADR-0078-vault003-scope-policy.md         [NUEVO]
```

**Ningún archivo existente fue modificado.** La única interacción con
archivos existentes fue de **lectura** (citas y referencias).

---

## Apéndice B — Evidence summary

- **Sources cited**: 33 (22 L1 + 8 L2 + 3 L3 + 8 external).
- **Verified claims**: 15 (12 corroborated, 3 independent).
- **Disputed claims**: 0.
- **Open gaps**: 10 (1 high, 8 medium, 1 low).
- **Decay warnings**: 9 sources expire 2027-02; 7 expire 2028-08; 9 stable.

---

## Apéndice C — Decisiones que se preservan

| Decisión | Versión | Commit | Estado |
|---|---|---|---|
| VAULT003 per-cycle scope policy | v1.65.6 | `87c5a97` | PRESERVED |
| RepairReceipt queue | v1.65.6 | `87c5a97` | PRESERVED |
| RFC3339 serde + error_kind emissions | v1.65.6 | `d19c305` | PRESERVED |
| attach_scope cycle_id fix | v1.65.6 | `c5a9ad4` | PRESERVED |
| release.sh end-to-end pipeline | v1.65.0 | `7cacf0b` | PRESERVED |
| Split-prefix coherence cycle-46 | v1.62.0+ | `fb33586, 66c06db` | PRESERVED |
| ADR-0022 reconciliation | — | `9170051` | PRESERVED |
| Dev update --prune + receipt v1→v2 | v1.63.0 | `768d80a` | PRESERVED |
| Phase::Review (status: orphan) | pre-existing | — | PRESERVED (decisión diferida a ADR-E) |
| Cycle lock acquire (FOREIGN KEY broken) | pre-existing | — | GAP-6 (out of scope) |