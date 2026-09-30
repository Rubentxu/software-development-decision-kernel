---
id: INC-DEBT-042
title: context bootstrap reports status complete while delivering no capsule
severity: high
priority: P1
status: open
detected_at: 2026-09-30
detected_in_session: session-45f
component: cli
surface: crates/sddk-cli/src/context_cmd.rs
references:
  - crates/sddk-engine/src/cold_start.rs
  - crates/sddk-engine/src/context_capsule.rs
  - docs/roadmap/CURRENT.md
---

# INC-DEBT-042: `context bootstrap` reporta `status: complete` sin compilar la capsule

## Criterio verificable (y se sostiene hoy)

`sddk context bootstrap` debe, según su propio `--help` y el objetivo 3
de C3j (CTX-003), **compilar** una `ContextCapsule`. Hoy devuelve éxito
sin compilarla.

Observado con el binario release en el workspace real (session-45f):

```console
$ sddk context bootstrap --session probe-45e --root . --format json
{
  "status": "complete",
  "project_id": "p-63676b11dc0ef88f",
  ...
  "context_source": "fresh",
  "basis_revision": "empty",
  "capsule_id": null,
  "binding_ref": "sddk/context/bindings/probe-45e.json",
  "binding_written": false
}
EXIT=0
```

Tres campos delatan que la entrega no ocurrió:

- **`capsule_id: null`** — no hay capsule.
- **`context_source: fresh`** — se construyó desde cero, no leyó nada.
- **`binding_written: false`** — ni siquiera se persistió el binding.

Y aun así `status: complete` y **exit 0**.

## Por qué es P1

**Es el mismo defecto que `INC-DEBT-039`, un nivel más arriba.** Allí un
comando devolvía una `RunStateView` vacía que parecía autoritativa; aquí un
comando devuelve `complete` sin el artefacto que promete. En ambos casos
**la salida afirma algo que no se hizo**, y un consumidor que solo mire el
código de salida o el campo `status` se lleva una mentira.

La diferencia que lo hace P1 y no P3: `complete` es la señal que un
orquestador lee para decidir si el contexto está listo. Un bootstrap que
reporta `complete` con `capsule_id: null` permite encadenar trabajo sobre
una capsule que no existe, y el fallo aparece aguas abajo, lejos de su
causa.

## Causa raíz (estructural, verificada)

Solo hay **dos** implementaciones de `CapsuleInputs` en el workspace:

- `InMemoryCapsuleInputs` (`context_capsule.rs:651`) — test.
- `RecoveryCapsuleInputs` (`cold_start.rs:281`) — exige `RunStateView`.

`RecoveryCapsuleInputs::new(parent, rsv)` requiere un `RunStateView`, y la
única vía de inyección (`with_run_state_view_inputs`,
`agent_host.rs:305`) la usan **solo tests**. **No existe ninguna ruta de
producción que pueda compilar una `ContextCapsule`.**

El fallo cerrado de `run-view` (exit 4, `RUN_STATE_SOURCE_UNAVAILABLE`,
`INC-DEBT-039`) es correcto y no debe relajarse: es lo que impide que
`frontier` y `pending_decisions` se inventen. El problema es el otro lado:
cuando la fuente falta, el comando **no degrada a un estado honesto**.

## Lo que esta INC NO es

- **No es un bug de `INC-DEBT-039`.** El fail-closed de `run-view` es el
  comportamiento correcto. Abrir 039 para «hacer que bootstrap funcione»
  sería resolver un problema cuya solución honesta es la decisión de
  modelo que 039 ya declara bloqueante.
- **No es cosmético.** No hay forma de distinguir «compilé la capsule» de
  «no compilé nada» salvo leyendo campos secundarios que un consumidor no
  lee.

## Opciones

### (a) Degradar el estado cuando no hay capsule (fail-closed, coste bajo)

`status` debe distinguir `complete` de algo como `no_run_state_source` o
`degraded`, y el exit code debe ser distinto de 0 cuando `capsule_id` es
`null`. Es el mismo patrón que ya se aplicó en `INC-DEBT-039`: convertir
un defecto silencioso en un fallo honesto. **No requiere** la decisión de
modelo sobre `frontier`.

### (b) Compilar la capsule (opción (a) de `INC-DEBT-039`, coste alto)

Requiere el adaptador de ledger y, antes, la pregunta de modelo: *qué es
`frontier` cuando `node_runs_v1` está vacía*. Bloqueada por decisión del
operador, no por código.

### (c) Documentar que `complete` significa «contexto base resuelto»

Si el comando solo pretende resolver identidad/adopción/ciclo y la
compilación de capsule es un objetivo futuro, entonces `status: complete`
es **correcto pero engañoso por su nombre**, y la deuda es de contrato:
el nombre promete más de lo que entrega. Coste: bajo, pero deja el mismo
trampa para el consumidor.

**Recomendación: (a) primero.** Es la misma lección que 039 y no depende de
ninguna decisión de modelo. (c) es aceptable solo si además cambia el
nombre del estado, no solo si se documenta.

## Reproducción

```bash
sddk context bootstrap --session probe-45e --root . --format json
# observar: status=complete, capsule_id=null, context_source=fresh,
#           basis_revision=empty, binding_written=false, exit 0
```

## Nota de alcance

Observado **sin ciclo activo** (el estado por defecto del workspace). No
se ha arrancado un ciclo para comprobar el caso con ciclo activo:
`sddk cycle list` no existe como subcomando y no se inventa un comando
para forzar la prueba. Con un ciclo activo `context_source` podría dejar de
ser `fresh`, pero `capsule_id: null` seguiría requiriendo la misma fuente
inexistente. Queda declarado como límite de la observación, no como
suposición resuelta.
