---
id: INC-DEBT-039-RUN-STATE-VIEW-SCAFFOLD-READS-NO-LEDGER
status: open
severity: high
priority: P1
detected_at: 2026-09-29
detected_in_session: session-42
blocks: [CTX-COMPILER-001, CTX-003-paso-5, CTX-COMPILER-002]
references:
  - crates/sddk-cli/src/run_view.rs
  - crates/sddk-engine/src/run_view.rs
  - crates/sddk-engine/src/cold_start.rs
  - ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Shape.md
---

# INC-DEBT-039: `RunStateView` se construye sin leer el ledger y miente sobre su completitud

- **Estado**: open (high/P1)
- **Detectada**: session-42 (2026-09-29), al desbloquear CTX-003 paso 5
- **Bloquea**: `CTX-COMPILER-001`, CTX-003 paso 5, `CTX-COMPILER-002`

## Contexto

El objetivo 5 de C3j (deltas durables) está cerrado. El siguiente es
CTX-003 **paso 5**: `sddk context bootstrap` debe **compilar** una
`ContextCapsule`, no sólo recuperar una persistida. El único adaptador
`CapsuleInputs` que combina capsule + estado de run en producción es
`RecoveryCapsuleInputs` (`crates/sddk-engine/src/cold_start.rs:217`), y su
insumo es un `RunStateView`.

Al estudiar la ruta se encontró que **`sddk run view` no lee el ledger**, y el
propio código lo confiesa en tres comentarios consecutivos
(`crates/sddk-cli/src/run_view.rs`):

```rust
// For the v0 scaffold we read no storage; the run is "found" if a
// view can be built. Real implementation will query the ledger.        (línea 59)

// Heuristic for the scaffold: declared vs generated based on run_id prefix.  (línea 62)

// Build a minimal RunStateView from the run_id. Real impl reads ledger.      (línea 70)
```

Y entonces llama al constructor **con los tres vectores vacíos**:

```rust
let state = match build_run_state_view(
    run_id.clone(),
    as_of,
    origin,      // ← inventado por prefijo del nombre
    vec![],      // ← frontier
    vec![],      // ← blockers
    vec![],      // ← pending_decisions
    as_of,
)
```

## Por qué es P1 y no P3

No es "falta una integración". Es **una vista que afirma algo falso**.

`REQ-CurrentRunView-Shape.md:43` define `frontier` como *"Nodes whose
dependencies are resolved and that are eligible to execute next"*, y
establece que es vacío *"iff the run is terminal or no node is ready"*.

Un `vec![]` constante no distingue esos dos casos: dice "nada está listo"
cuando en realidad **nunca se consultó si algo estaba listo**. La vista es
indistinguible de la verdad, y esa indistinguibilidad es el defecto.

Lo que agrava:

1. **`origin` se inventa por el prefijo del `run_id`** (`R-decl` ⇒
   `Declared`, resto ⇒ `Generated`). Es una heurística de nombres, no un
   hecho del ledger. Dos runs con el mismo prefijo y distinto origen real
   producen vistas idénticas y erróneas.
2. **`ActionSurfaceView` se deriva de ahí.** Si `frontier` e
   `pending_decisions` están vacíos, cualquier política que dependa de
   ellos ("rechaza `Resume` en runs con `pending_decisions` vacías",
   REQ línea 110) evalúa sobre datos falsos. El defecto se propaga
   aguas abajo a la superficie de acciones.
3. **Es el bloqueo directo de CTX-003 paso 5.** Compilar una capsule con
   `RecoveryCapsuleInputs` alimentado por esta vista significaría persistir
   una capsule construida sobre frontier y decisiones inventados.

## Alcance medido (no estimado)

- `RunStateViewInputs` tiene **1 implementación**: `InMemoryRunStateViewInputs`
  (`cold_start.rs:99`), usada sólo por tests. `AgentHost` expone
  `with_run_state_view_inputs` (`agent_host.rs:305`) como punto de inyección
  y **nadie lo inyecta con una fuente real**.
- `build_run_state_view` tiene **1 consumidor de producción**:
  `crates/sddk-cli/src/run_view.rs:73`. El resto son tests.
- Las tablas que alimentarían la vista están **vacías en el ledger del
  operador**: `node_runs_v1: 0`, `workflow_run_events_v1: 0`,
  `decision_records_v1: 0`. `work_items_v1` tiene 81 filas pero sólo **2**
  con `cycle_id` real (las otras 79 usan su propio id como `cycle_id`,
  deuda de datos heredada y distinta).
- `event_snapshots_v1: 0` — no hay snapshots de proyección, así que una
  implementación de `RunStateView` tendría que proyectar desde
  `events_v1` (583 filas) o leer esas tablas vacías.

## Opciones

### (a) Implementar el adaptador de ledger — **pendiente de decisión de alcance**

`RunStateViewInputs` sobre `sddk-storage`, proyectando frontier/blockers/
decisiones desde `events_v1` + `work_items_v1`. Coste: medio-alto. Antes
hay que decidir qué es "frontier" cuando `node_runs_v1` está vacía, y esa
pregunta es de modelo, no de código.

### (b) Declarar el scaffold explícitamente no-prod y fallar cerrado

Hasta que haya fuente real, `sddk run view` debería devolver un estado
tipado que diga "sin fuente de run state" en vez de una vista vacía que
parece autoritativa. Coste: bajo. Convierte el defecto silencioso en
un fallo honesto, y es la opción que **no depende de decisiones de modelo**.

### (c) Distinguir "vacío" de "no consultado" en el tipo

Añadir un campo de procedencia a `RunStateView` (`Sourced` | `Unsourced`)
para que ningún consumidor pueda confundir una vista leída con una
construida. Coste: bajo, pero toca el tipo público y sus consumidores.

## Recomendación

**(b) primero, (a) después.** Un comando que devuelve una vista vacía
autoritativa sobre datos que no leyó es peor que un comando que dice "no
tengo fuente": el primero se usa en silencio y contamina decisiones; el
segundo se nota. (c) es el refuerzo natural cuando (a) llegue.

## Nota de método

Este hallazgo **no** era una deuda declarada: no estaba en
`docs/debt/README.md` ni en ninguna INC. Aparece sólo al intentar
ejecutar CTX-003 paso 5, es decir, al chocar con el objetivo. La lección
repetida de este repo (INC-DEBT-033, INC-DEBT-037) es que un objetivo
"terminado" en apariencia puede estar bloqueado por una deuda que nadie
anotó porque nadie llegó lo bastante lejos.
