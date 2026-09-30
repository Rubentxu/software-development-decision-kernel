---
id: INC-DEBT-042
title: context bootstrap reports status complete while delivering no capsule
severity: high
priority: P1
status: closed
detected_at: 2026-09-30
detected_in_session: session-45f
closed_in_session: session-46
component: cli
surface: crates/sddk-cli/src/context_cmd.rs
references:
  - crates/sddk-engine/src/cold_start.rs
  - crates/sddk-engine/src/context_capsule.rs
  - docs/architecture/adrs/ADR-0147-FRONTIER-SEMANTICS-AND-CYCLE-CAPSULE-INPUTS.md
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

## Lo que se implementó (opción a) — 2026-09-30, commit `fix(cli)`

La opción (a) está implementada y verificada en la interfaz real:

- `ContextBootstrapResult::status` ahora es **`no_capsule_source`** cuando no
  hay capsule, y `complete` solo cuando se recuperó una capsule durable. El
  literal incondicional `status: "complete"` desapareció, y con él el
  doc-comment que lo justificaba («Always `complete` on success»).
- La frontera CLI mapea el estado degradado a **exit code 4**, igual que
  `run-view` usa para `RUN_STATE_SOURCE_UNAVAILABLE`. El payload se emite
  íntegro: identidad, adopción, ciclo y binding siguen siendo observables.

Verificado ejecutando el binario, no solo con tests:

```console
$ sddk context bootstrap --session verify-45f --root . --format json
status          = no_capsule_source
capsule_id      = None
context_source  = fresh
basis_revision  = empty
adoption        = complete
binding_written = True
EXIT=4
```

**Lo que esto NO cierra.** El MUST de CTX-003 paso 5 sigue sin
satisfacerse: el comando ya no *afirma* haber compilado una capsule, pero
tampoco la compila. La brecha real permanece y sigue bloqueada por la
decisión de modelo sobre `frontier` (`INC-DEBT-039`). Lo que se corrigió
es la **mentira**, no la **carencia**: el defecto se degrada de «dice que
funciona» a «dice que no funcionó». La opción (b) —compilar de verdad—
sigue abierta y bloqueada.

Evidencia de tests: 3 tests nuevos (RED confirmado antes del fix: `left:
"complete"` y `left: 0`; GREEN después), más 1 test preexistente
actualizado. `cargo test -p sddk-cli`: **1326 passed, 0 failed, 2
ignored**. `cargo fmt --check` y `cargo clippy -p sddk-cli --all-targets
-- -D warnings`: exit 0.

Un hallazgo honesto sobre ese test preexistente: `bootstrap_without_active_
cycle_creates_project_binding` afirmaba `status == "complete"` mientras, tres
líneas más abajo, afirmaba `capsule_id.is_none()`. **El test codificaba la
contradicción como si fuera correcta.** Se actualizó a
`no_capsule_source`; el resto de su comportamiento (binding, idempotencia,
aislamiento de ciclo) no cambió. No se ajustó el test para que pasara: se
cambió la expectativa, y la causa del cambio está en el doc-comment.

## Reproducción

```bash
sddk context bootstrap --session probe-45e --root . --format json
# tras el fix: status=no_capsule_source, capsule_id=null, EXIT=4
```

## Nota de alcance

Observado **sin ciclo activo** (el estado por defecto del workspace). No
se ha arrancado un ciclo para comprobar el caso con ciclo activo:
`sddk cycle list` no existe como subcomando y no se inventa un comando
para forzar la prueba. Con un ciclo activo `context_source` podría dejar de
ser `fresh`, pero `capsule_id: null` seguiría requiriendo la misma fuente
inexistente. Queda declarado como límite de la observación, no como
suposición resuelta.

## CIERRE (2026-09-30, session-46) — opción (b) implementada vía ADR-0147

La carencia ya no existe: `context bootstrap` **compila de verdad** la
capsule del ciclo activo desde facts reales del ledger. La decisión de
modelo que bloqueaba (qué es `frontier` sin `node_runs`) quedó resuelta en
**ADR-0147** con tres decisiones: (D1) `frontier` solo se define para un
run existente, la ausencia de fila no es un frontier vacío; (D2) el
bootstrap compila a nivel CICLO con `CycleLedgerCapsuleInputs` (goal,
work items por estado, decisiones por tipo) leídos del ledger canónico;
(D3) la ruta run-level (`RecoveryCapsuleInputs`) queda como recovery
explícito pendiente del primer run real, re-scoped en INC-DEBT-039.

Implementación: `CycleFacts` + trait `CycleFactSource` (port, porque
`sddk-storage` es dev-dep del engine) + `CycleLedgerCapsuleInputs` en
`crates/sddk-engine/src/cold_start.rs`, y `StorageCycleFactSource` +
`compile_cycle_capsule` wired en el paso 4/5 del bootstrap en
`crates/sddk-cli/src/context_cmd.rs`. Con ciclo activo: `status: complete`,
`context_source: compiled`, capsule persistida bajo la clave `cycle-<id>`
y `basis_revision = capsule_id`. Sin ciclo (o ciclo sin facts):
`no_capsule_source` con exit 4 — la degradación honesta de la opción (a)
se mantiene como red de seguridad, ya no como estado normal.

Evidencia observada en session-46: test de integración
`bootstrap_with_active_cycle_compiles_capsule_from_ledger_facts` (LEDGER
real en tempdir: proyecto + ciclo + 3 work items done/active/paused + 2
decisiones accept/reject + lease activa; bootstrap compila y la capsule
durable lleva el cycle ref, el título del work item cerrado en relevant,
la decisión aceptada en decisions.accepted y el item pausado en
must_read). Suite: `cargo test -p sddk-cli --lib` 847 passed / 0 failed /
1 ignored; `cargo test -p sddk-engine --lib` 1351 passed; cold_start_tests
15/15. `cargo clippy -p sddk-cli -p sddk-engine --all-targets` limpio.

Hallazgo estructural del wiring, corregido en el mismo cambio: el bootstrap
leía `resolved.active_leases` para inferir el ciclo, pero
`resolve_cycle_context` devuelve ese campo **siempre vacío por contrato**
(la lease única viaja en `cycle_id`, y cero/ambiguas viajan como errores
tipados). El código muerto degradaba a `NoActiveCycle` incluso con una
lease activa, con lo que ningún bootstrap llegaba jamás a compilar. Fix:
leer `resolved.cycle_id`. Pinneado por el test de integración nuevo.

Límite residual declarado: el goal de la capsule es el `display_name` del
`CycleManifest` (el único resumen canónico nombrado por humanos que el
ciclo lleva); la entidad `Goal` de `sddk-domain` aún no tiene tabla de
persistencia propia, así que compilarla requeriría inventar facts. Queda
para el ciclo que introduzca persistencia de goals.
