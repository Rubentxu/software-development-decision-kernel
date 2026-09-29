# SPEC-005 — Durable context, session binding and hypermedia handoff

## Intent

Eliminar la dependencia de handoffs narrativos y de memoria de sesión.

## Reutilización obligatoria

- `ContextCompiler` / `ContextCapsuleV2`
- `ContextDelta` / `ContextBridge`
- `AgenticSessionRef` / `BindingTarget`
- `ContextLease` / Contribution envelope
- `ContextReadRecord`

No crear una segunda memoria de contexto.

## Requisitos

### CTX-001 — Durable CapsuleStore

Debe existir implementación productiva persistente (SQLite/CAS o store canónico equivalente) detrás del seam actual. Restart de proceso no pierde capsule/basis.

### CTX-002 — Durable SessionBindingStore

Bindings host↔project/workitem/run/task sobreviven restart sin importar transcript.

### CTX-003 — Bootstrap operation

Una operación de alto nivel (`context bootstrap` o equivalente) MUST:

1. resolver project/workspace;
2. converger adoption sin interacción si no hay conflicto;
3. inferir cycle/run cuando no sea ambiguo;
4. reconstruir context basis;
5. compilar capsule;
6. persistir/read-reuse por digest;
7. devolver hypermedia representation.

### CTX-004 — Cycle inference

Si existe exactamente un active lease/cycle candidate, bootstrap lo resuelve automáticamente. 0 => typed no-active state. N => typed ambiguity con candidate actions.

### CTX-005 — Adoption ensure

Repeated bootstrap en proyecto adoptado MUST ser no-op semántico. Pregunta humana sólo ante conflicto no reparable automáticamente.

### CTX-006 — Context contents

Capsule compacto SHOULD incluir refs/summaries de:

- goal/frontier;
- active work items;
- decisions;
- invariants;
- blockers/negative knowledge;
- relevant evidence;
- provider observations;
- semantic relations;
- prior failed approaches cuando sean relevantes.

### CTX-007 — Progressive disclosure

Detalle pesado queda referenciado. `context.expand` expande refs con read tracing.

### CTX-008 — Delta

Después del bootstrap, cambios materiales se entregan como ContextDelta basis-bound. Stale/out-of-order se rechaza como ya define ContextBridge.

### CTX-009 — Handoff

Handoff = nueva representación de StepRun + ContextDelta + resource relations + affordances. El free-text summary MAY existir para humanos, pero no es la autoridad.

### CTX-010 — Context read provenance

Cada execution/attempt registra resource IDs leídos y hashes cuando existan.

### CTX-011 — Transcript boundary

Transcript permanece host-owned. SDDK persiste sólo semantic refs/contributions/evidence necesarios.

### CTX-012 — Context lease

Mutating contribution MUST referenciar ContextLease/basis; stale lease falla antes de canonical mutation.

## Acceptance

CTX-UAT-001..015.
