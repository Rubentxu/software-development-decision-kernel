# HANDOFF — DW-RUNTIME-005 SHIPPED → DEC-PLANE-001 starting point

> **Cycle:** `p-63676b11dc0ef88f/dw-runtime-005-restart-survival`
> **Status:** SHIPPED ✅
> **Release:** `v1.89.7` (HEAD `65cbb74` + housekeeping `8d0cf4a`)
> **Spine reconciled:** `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml` `order: 180` (PROPOSED → SHIPPED)
> **Path:** A-min
> **Date shipped:** 2026-09-08
> **Next cycle:** **DEC-PLANE-001** — Introduce `CurrentRunView` for declared and generated workflow state

---

## 1. What DW-RUNTIME-005 shipped

Exit gate cerrado: *"Persisted generated DAG can stop, resume and replay to equivalent state under UAT."*

### Final 5 commits (orden cronológico)

```
b6e2cd4 feat(engine): UAT end-to-end restart-survival del exit gate DW-RUNTIME-005 (S6b)
8115186 fix(engine): tolerar IdempotencyConflict tipado al re-registrar marcadores de Sequence en replay
15ef8fd feat(engine): restart-survival de workflow runs en execute (S6b slice 2)
db62c67 feat(storage): event-sourcing de transiciones de workflow run (base S6b restart-survival)
ada4ae2 feat(engine): DW-RUNTIME-005 S6a exit gate con outputs terminales
```

### Alcance técnico aterrizado

| Slice | Commit | Qué cierra |
|---|---|---|
| **S6a — exit gate real** | `ada4ae2` | `execute()` retorna éxito con `outputs` terminales (no `Default::default()`). Fix self-deadlock del MutexGuard del scrutinee en `match self.store.lock()…load_run()` (la rama `Ok(None)` re-lockeaba el mismo mutex en `record_run`). Acumula `completed_node_outputs` en `apply_outcomes_to_state` y los publica en `run.outputs` al llegar a `AllComplete`. Sustituye stub `runtime_exit_gate.rs` por 3 tests reales. |
| **Base event-sourcing S6b** | `db62c67` | Append-only `workflow_run_events_v1` con `load_run_replay(run_id) → Vec<RunEvent>` para reconstruir la línea de transiciones de un run persistido. |
| **S6b slice 2 — restart survival en `execute()`** | `15ef8fd` | `execute()` consulta el replay: si el run está en estado terminal → `Err(RuntimeError::AlreadyTerminal)` (no se re-ejecuta); si está en `Running` (no terminal) → rehace el replay desde la última transición persistida hasta terminal. Aporta el contador compartido de operadores para idempotencia. |
| **Fix IdempotencyConflict tipado** | `8115186` | La UAT S6b expuso que `Sequence::evaluate` re-registra marcadores `seq-tick-{node}-{step}` ya persistidos y `SqliteGraphStore` devuelve `StorageError::IdempotencyConflict` tipado (REQ-WFR4-PAR-003), no la forma legacy `StorageError::Other("idempotency")`. La rama de tolerancia solo cubría legacy → `EvalFailed`. Ahora tolera la forma tipada y el run termina `Completed` tras replay. |
| **UAT determinista S6b** | `b6e2cd4` | Tres tests file-backed (`SqliteGraphStore`, no `:memory:`): (1) stop mid-flight + replay equivalente sobre Sequence 5 Task leaves; (2) `AlreadyTerminal` al reiniciar un run ya completado; (3) idempotencia de intentos (1 fila por hijo, sin duplicados). Golden run sin interrupción usado como referencia de equivalencia. |

### Verificación de exit gate

- UAT `b6e2cd4` corre con `cargo test -p sddk-engine --test uat_restart_survival` (o archivo homónimo) sobre `SqliteGraphStore` real (file-backed).
- Equivalencia probada: `run.state`, `run.outputs` y mapa de node states del run re-ejecutado == golden run sin interrupción.
- Replay determinista: contador de operadores leg1 == golden (no se duplica trabajo).
- Idempotencia: 1 fila de `attempts` por hijo del DAG generado.
- Terminal no se re-ejecuta: restart sobre `Completed` corta con `Err(AlreadyTerminal)` y el contador compartido no crece.

### Spec coverage

- `REQ-EXIT-001` (execute retorna éxito con outputs terminales) — ✅
- `REQ-EXIT-002` (terminal-no-reejecucion) — ✅ (UAT leg 2)
- `REQ-WFR4-PAR-003` (IdempotencyConflict tipado) — ✅ (forma tipada tolerada en replay)

---

## 2. Pre-existing bug fixed in scope

**Self-deadlock de `execute()`** descubierto durante S6a (commit `ada4ae2`):

```rust
// ANTES (deadlock en run fresh):
match self.store.lock()...load_run() {
    Ok(Some(run)) => ...,
    Ok(None) => self.store.lock()...record_run(...),  // re-lockea el mismo MutexGuard
}

// DESPUÉS (guard liberado):
let load_result = self.store.lock()...load_run();
match load_result { ... }
```

Confirmado por backtrace `gdb`. Era prerequisito para S6b (sin el fix, ningún `execute()` sin run pre-insertado colgaba). No documentado en specs anteriores — hallazgo de implementación.

---

## 3. Spine reconciliation

```yaml
# docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml
- order: 180
  id: DW-RUNTIME-005
  horizon: H2
  status: SHIPPED # reconciled 2026-09-08: cycle p-63676b11dc0ef88f/dw-runtime-005-restart-survival; release v1.89.7; ...
  depends_on: [DW-RUNTIME-004]
  objective: Close end-to-end replay and resume vertical slice.
  exit_gate: Persisted generated DAG can stop, resume and replay to equivalent state under UAT.
```

Cambio de `PROPOSED → SHIPPED` con evidence inline (mismo formato que `DW-RUNTIME-001..004`). El horizonte H2 (DW-RUNTIME-001..005) queda completo: el runtime es capaz de compilar, persistir, ejecutar Sequence/Conditional/Parallel y sobrevivir restart con replay determinista.

---

## 4. Next cycle — DEC-PLANE-001

**Goal:** Introduce `CurrentRunView` for declared and generated workflow state.

**Spine ref:** `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml` `order: 190`, `horizon: H3`, `status: PROPOSED`.

**Exit gate:** *One projection exposes frontier, blockers, pending decisions and available actions.*

**Scope preliminar (a aterrizar en spec):**
1. **Tipo `CurrentRunView`** — proyección inmutable sobre `WorkflowRun` persistido + ledger + planning graph que combina declared runs (ciclo family) y generated runs (DW-RUNTIME-005).
2. **`frontier: Vec<NodeRef>`** — nodos habilitados para ejecutar a continuación según dependencias resueltas y estado del run.
3. **`blockers: Vec<BlockerRef>`** — dependencias no satisfechas, decisiones pendientes o precondiciones de policy.
4. **`pending_decisions: Vec<DecisionRef>`** — Choice guards con `ChoiceOutcome::Pending` o marcadores equivalentes.
5. **`available_actions: Vec<ActionKind>`** — taxonomía cerrada (start, resume, abort, approve, escalate, …) derivada del estado del run y de la policy activa.
6. **`sddk run view <run-id>`** — comando CLI que materializa `CurrentRunView` en JSON legible.
7. **Contrato durable** — specs en `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView*.md`.

**Dependencias satisfechas:**
- ✅ `DW-RUNTIME-005` (replay + resume vertical slice cerrado).
- ✅ `PLN-LEDGER-004` (proyecciones deterministas status/next/blocked/show/graph).

**Decisión abierta (preguntar antes de spec):**
- ¿`CurrentRunView` debe distinguir declared vs generated en el shape, o unificarlos y dejar que el caller pregunte `view.kind()`? Trade-off: shape unificado reduce superficie, shape separado deja invariantes de cada origen explícitas.
- ¿`available_actions` se calcula eagerly al construir la view, o se expone como método lazy (`view.available_actions(&policy)`)? Trade-off: eager simplifica consumers, lazy permite cambiar policy sin reconstruir.

**Path propuesto:** A-lite (propose → spec → tasks → apply → verify → debt-verify → release → archive) por la naturaleza arquitectural del cambio (nueva superficie durable consumida por CLI y futura Decision Plane).

---

## 5. Operational notes

### Environment (ya en su sitio)

- `sddk` CLI: `/home/rubentxu/.local/bin/sddk`, versión actual `1.89.7`.
- Working dir: `~/Proyectos/agentesIA/sddk-framework/` (no `~/.sddk-shared/`).
- Framework resolved: `~/.local/share/sddk/framework/1.89.6/` (binary `1.89.6`; bundle `1.89.6` se actualizará al instalar `1.89.7`).

### Housekeeping aplicado en esta sesión de cierre

- **Stash huérfano descartado** (`839e1d2`): traía una sección "Intelligent Change-Scoped Testing" en `AGENTS.md` (656 líneas) y un `Cargo.lock` a `1.72.1`. La sección **no existe** en el bundle SDDK actual (`~/.local/share/sddk/framework/current/`); es contamination de un `AGENTS.md` upstream distinto (probablemente `gentle-ai/agents.md`) inyectada por error durante `sddk dev install` en otra sesión. El propio mensaje del stash advertía *"REVISAR provenance"*. Decisión: descartar. Si vuelve a aparecer, investigar qué install la introdujo antes de merge.
- **Cargo.lock regenerado**: estaba residual al bump `1.89.6 → 1.89.7` (`cargo check -p sddk-cli` lo corrigió). Commit `8d0cf4a` lo limpia.
- **Spine reconciliado**: `EXECUTION-SPINE.yaml` con evidence inline (refs `15ef8fd`, `db62c67`, `8115186`, `ada4ae2`, `b6e2cd4`).

### Cycle management (patrón reciente)

```bash
# Validar que el ciclo cierra limpio
git status --porcelain
git log --oneline -5

# Próximo release (v1.89.7) — entry point canónico
bash scripts/release.sh --dry-run    # validar gate (pasos 0-8)
bash scripts/release.sh              # flujo completo 13 pasos
```

---

## 6. References

- Spine: `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml` (order 170..195)
- Roadmap: `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §3 determinism rule
- Timeline: `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-TIMELINE.md`
- Cycle context map: `docs/sddk-decision-kernel-architecture/02-roadmap/CYCLE-CONTEXT-MAP.yaml`
- Handoffs previos H2: `docs/handoff/HANDOFF-2026-09-*.md` (DW-RUNTIME-001..004)
- RELEASING: `docs/RELEASING.md` §8 (flujo canónico 13 pasos)
