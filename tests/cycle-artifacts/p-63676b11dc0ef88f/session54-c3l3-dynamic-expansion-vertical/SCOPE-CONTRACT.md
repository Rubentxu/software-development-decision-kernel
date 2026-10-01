# SCOPE-CONTRACT — C3l.3 Dynamic Workflow Expansion E2E real

**Slice:** `session54-c3l3-dynamic-expansion-vertical`
**Roadmap:** `docs/roadmap/ROADMAP.md` §C3l.3 · **Paquete:** `docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md:191`
**Abierto:** 2026-10-01 · **Baseline:** `dc343722` (= `origin/main`, workspace `2.4.2`)

---

## 1. Defecto verificado (no inferido del enunciado del paquete)

Cada hueco se comprobó contra el código, con la ruta que lo demuestra.

| # | Hueco | Evidencia |
|---|-------|-----------|
| D1 | **No hay identidad estable de trigger/proposal.** `Engine::cycle_replan` recibe `event_id` del caller (`cycle_replan.rs:84`); nada deriva la identidad del contenido del trigger. | `crates/sddk-engine/src/cycle_replan.rs:84` |
| D2 | **El test W02 fija el defecto como esperado**: dos replans con el *mismo delta* producen `replan_count == 2`, con el mensaje *"the contract is bounded counter, not dedup"*. El propio test esquiva el dedup usando un `event_id` nuevo. Esto es exactamente el patrón que C3l.2 ya prohibió para S7a. | `crates/sddk-engine/tests/aiw_s4_dynamic_expansion.rs:483-487` |
| D3 | **Replay no idempotente a nivel de proyección.** El append canónico usa `INSERT OR IGNORE` sobre `event_id` (evento: NO duplica secuencia), pero `update_cycle_with_event` ejecuta el `UPDATE cycles` **incondicionalmente** después. Replay ⇒ evento dedupeado + `replan_count` re-incrementado ⇒ **divergencia ledger↔proyección**. | `event_store.rs:246` vs `storage/lib.rs:686-705` |
| D4 | **`cycle_replan` no valida authority.** Cero llamadas a `AuthorityContext::validate`; la superficie `WritableSurface::PlanRevisions` existe y está poblada pero la ruta de replan la bordea. | `grep authority cycle_replan.rs` → 0 matches |
| D5 | **No existe paso de orchestration** entre proposal de Secretary y `ReplanDelta`. | aus en la ruta |
| D6 | **`cycle_replan` nunca toca `PlanRevisionLineageV1`** — bumpea un contador y restagea la fase; no hay revisión N+1 con lineage. Y la ejecución incremental no existe: `compile_plan_to_revision` es **compile-only por diseño declarado** (persistencia y ejecución fuera de scope, DW-RUNTIME-002/003/004/005). | `execution_graph_compiler.rs:9-13` |

**Consecuencia:** C3l.3 **no es una composición de test**. Requiere superficie de producción nueva. La clasificación honesta vigente (`IMPLEMENTED_NOT_VERIFIED`) era correcta.

## 2. Vertical mínima obligatoria (del paquete)

```text
Observation / EvidenceGap
        ↓
Secretary proposal
        ↓
Orchestration decision
        ↓
Authority validation
        ↓
typed ReplanDelta
        ↓
PlanRevision N+1
        ↓
incremental execution
        ↓
receipt
```

## 3. Diseño — identidad estable como clave de idempotencia

Se reutiliza la deduplicación que **ya existe** en el ledger en vez de inventar un registro nuevo:

- `ExpansionTrigger` produce un **fingerprint content-addressed** `sha256(canonical_json(trigger))`. Es la clave de idempotencia; dos triggers con el mismo contenido tienen el mismo fingerprint.
- Ese fingerprint viaja en el `event_id` (`cycle.expansion.applied:{fingerprint}`) y en el payload de los eventos.
- **Guarda de replay:** antes de ejecutar la vertical, se listan los eventos del ciclo y se busca un `cycle.expansion.applied` con ese fingerprint. Si existe → la expansión **ya ocurrió**: se devuelve el recibo existente con `0` revisiones nuevas y `0` ejecuciones nuevas. Esto cierra D1 y D3 sin una tabla nueva (ADR §2.7: una autoridad canónica por concepto; el ledger ya es la autoridad append-only).
- **Authority** se valida contra `WritableSurface::PlanRevisions` antes de cualquier mutación (D4).
- **Lineage:** `PlanRevisionLineageV1::derive` ya computa `revision_id = sha256(parent|mutation|normalized_identity)` — determinista por construcción. El replay con el mismo trigger sobre el mismo tip **no** puede derivar una segunda revisión distinta, porque el guard de replay corta antes.
- **Ejecución incremental:** la selección es la diferencia de conjuntos `child.nodes \ parent.nodes` sobre `ExecutionGraphRevision`. Se registra un **execution ledger** por expansión con exactamente esos nodos. Esto prueba el exit gate *"nodos previos no se re-ejecutan"* sin fingir un runtime de operadores.

## 4. Límite declarado por adelantado (anti-falso-verde)

La ejecución registrada es de **selección y contabilidad de nodos despachados**, no de evaluación real de operadores. El runtime de operadores es DW-RUNTIME-003/004/005, explícitamente fuera de scope del propio compiler. **Este slice no puede mover `AIW-S4` a VERIFIED**; lo deja en `IMPLEMENTED → re-verificable` con el residual declarado, igual que C3l.2 hizo con S7a. Declararlo aquí *antes* de escribir código, para que el recibo no pueda reinterpretar el alcance después.

## 5. Falsificadores congelados (los 7 del paquete)

| # | Falsificador | Invariante que debe observationar |
|---|--------------|----------------------------------|
| F1 | trigger duplicado | `revision_delta_count == 1` |
| F2 | proposal replay | `revision_delta_count == 1`, `new_node_execution_count == 1` |
| F3 | authority denied | 0 cambio canónico (ni revisión, ni evento, ni snapshot) |
| F4 | delta inválido | 0 cambio canónico |
| F5 | fallo de ejecución del nodo nuevo | revisión durable, nodo marcado fallido, 0 ejecuciones contadas como éxito |
| F6 | restart entre proposal y apply | estado durable; el guard de replay sobrevive al reopen |
| F7 | plan base cambió antes del apply | 0 cambio canónico (falla cerrado) |

## 6. Exit gate (del paquete, verificable sin leer el nombre del test)

1. exactamente una expansión ante replay;
2. nodos previos no se re-ejecutan;
3. denied/invalid no cambia el plan;
4. la nueva revisión tiene lineage y receipts.

## 7. UAT

`AT-UAT-006` (nueva revisión + sólo nodo nuevo) · `AT-UAT-007` (1 revisión y 1 ejecución total) · `AT-UAT-008` (authority deny ⇒ 0 cambio canónico). `boundary_class = IN_PROCESS/SQLITE`.

## 8. Reglas de ejecución

- RED antes de GREEN en cada falsificador; el RED se observa con el tipo/superficie **ausente**, no con una aserción blanda.
- No tocar los tests W01..W11 existentes para que passen; W02 se **reescribe** porque fijaba el defecto (D2) — con el RED registrado.
- Testing scoped durante apply (`prompts/sddk/change-scoped-testing.md`); perfil completo reservado a verify/release.
- Sin commits hasta que el verde esté commiteado (lección de session-50: falsar sobre código sin commitear costó la implementación).
