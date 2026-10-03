---
id: INC-DEBT-067
title: "La vista del operador dice 'Cycle completed' y 'Nada por ahora' para TODOS los ciclos, incluidos los abiertos y los que esperan aprobacion humana, porque las dos frases son constantes y no derivaciones"
status: open
severity: high
priority: P1
fingerprint: "cycle_narrative_operator_view_asserts_completed_and_no_action_regardless_of_state"
fingerprint_aliases: []
cluster_id: CL-VERIFICATION
created: 2026-10-04
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-04
detected_in_session: session-68
component: sddk-cli (run_cycle_narrative) / sddk-engine (cycle_narrative)
surface: crates/sddk-cli/src/cycle.rs:1618 y crates/sddk-engine/src/cycle_narrative.rs:321,492,500
related: [INC-DEBT-065, INC-DEBT-066]
references:
  - crates/sddk-cli/src/cycle.rs
  - crates/sddk-engine/src/cycle_narrative.rs
---

## Qué es

`sddk cycle narrative` se describe a sí mismo como la **operator view** (su propio
`--help`: «Render a deterministic markdown narrative for a cycle (operator view)»,
`--audience maintainer` por defecto). Es, por tanto, **la forma en que una persona lee
el estado del ciclo**. Y sus dos afirmaciones centrales **no derivan de nada**:

1. **`"Cycle completed."`** es el valor por defecto de `what_was_done` cuando el
   llamante no pasa `--what-was-done` (`crates/sddk-cli/src/cycle.rs:1618`).
   **No consulta el `status` del ciclo.** Es una cadena literal.
2. **`"Nada por ahora."`** es lo que el pie `Necesito de ti` imprime cuando
   `n.human_action_required` es `None` (`cycle_narrative.rs:500`) — y ese campo
   **no tiene productor en todo el workspace**: se declara en `:286`, se inicializa a
   `None` en el constructor (`:321`) y **no se le asigna ningún `Some(..)` en ninguna
   parte**. Las únicas cuatro apariciones del símbolo en el repo son la declaración del
   campo, ese `None` de construcción, la rama que lo renderiza y un fixture de test
   (`:565`). **El pie no puede decir otra cosa que "Nada por ahora", ni por construcción.**

## OBSERVED — el falsificador, y por qué no es un default inofensivo

Si el defecto fuera «el estado no se propaga», bastaría un ciclo. Se compararon **tres,
elegidos para que la respuesta correcta difiera en los tres ejes**:

| Ciclo | `status` | `phase` | `runtime_state` | Lo que dice el narrative |
|---|---|---|---|---|
| `cycle-45-build-remediate-archive` | **CLOSED** | archive | — | `Cycle completed.` / `Nada por ahora.` |
| `cycle-46-install-coherence` | **OPEN** | build | — | `Cycle completed.` / `Nada por ahora.` |
| `c3n-production-boundary-certification` | **OPEN** | design | **approval-waiting** | `Cycle completed.` / `Nada por ahora.` |

**Las tres filas renderizan idénticas.** Un `CLOSED` y dos `OPEN` en fases distintas
producen byte a byte la misma afirmación sobre su estado. Eso descarta la lectura
cómoda —«el default es aceptable porque el caller puede sobrescribirlo»—: el problema
no es el default, es que **el valor por defecto es la respuesta para todos los casos**,
incluido el que más la necesita.

## Consecuencia viva en este repo, no teórica

El ciclo `c3n-production-boundary-certification` lleva **desde `2026-10-03` en
`approval-waiting`**, con una solicitud de aprobación registrada
(`surface.cycle_state#cycle_supersede`, `request_hash sha256:c128d51c…`) que **solo un
operador humano puede conceder** —-y que este agente no se concedería a sí mismo,
porque aprobar una superficie gobernada desde quien pide la mutación anula el gate—.
Mientras tanto, **la vista del operador afirma que el ciclo está completado y que no
hace falta nada de él.** Quien se guíe por ella no concede la aprobación, y el ciclo no
avanza: el bloqueo es invisible justo en la vista diseñada para hacerlo visible.

## Por qué `high` y no `critical`

No hay pérdida de datos ni corrupción: **`sddk cycle status` —la vista de máquina— es
correcta** (`status: OPEN`, `phase: design`, `runtime_state: approval-waiting`, lease con
su `fencing_token`), y la frente a ella es la que miente. El daño está acotado a la
proyección legible por una persona. Se sube a `critical` si la proyección llega a
escribir estado —es decir, si algún día `narrative` alimenta un artefacto o una
transición en vez de solo imprimir—.

## Misma clase que ya está registrada

Es la forma de **INC-DEBT-066** (un `PASS` que certifica algo que la fila no exige) y la
de **INC-DEBT-065** (`reactive_verify`, un contrato escrito sin nadie detrás): **una
superficie que existe, se renderiza y no tiene productor.** Aquí la diferencia es que la
superficie *sí* tiene productor —el render— y lo que no tiene productor es el **dato**,
y el render **presenta la ausencia de dato como una afirmación positiva**.

## Cómo refutarla (o por qué es `open` y no `resolved`)

El defecto se puede comprobar con un ciclo `OPEN` en `approval-waiting` cuyo narrative
diga algo distinto de `Cycle completed.` y cuyo pie `Necesito de ti` nombre la
aprobación pendiente. **Hoy ninguno de los dos ocurre**, y no hay test que lo exija:
`grep -rn human_action_required` sobre `crates/` devuelve 4 líneas y ninguna es una
asignación.

Cierre = (1) `what_was_done` **derivado del `status`/`phase`/`runtime_state`** leídos
del ciclo, no una constante; (2) `human_action_required` **poblado** desde el estado real
—aprobaciones pendientes, `blockers`, `runtime_state != proceeding`— con el
`None` **visto** como «no se pudo determinar» y no como «no hace falta nada»; (3) un test que
afirme que **un ciclo `OPEN` en `approval-waiting` no renderiza `Cycle completed.`**,
y su falsificación por mutación de los tres estados.
