---
id: INC-DEBT-043
title: context bootstrap enlaza la sesión a un ciclo explícito que no existe
severity: high
priority: P1
status: resolved
detected_at: 2026-09-30
detected_in_session: session-49
closed_in_session: session-49
component: cli
surface: crates/sddk-cli/src/context_cmd.rs
references:
  - crates/sddk-cli/src/cycle.rs
  - skills/sddk-cycle-resume/SKILL.md
  - docs/architecture/adrs/ADR-0147-FRONTIER-SEMANTICS-AND-CYCLE-CAPSULE-INPUTS.md
  - docs/debt/INC-DEBT-042-CONTEXT-BOOTSTRAP-COMPLETE-WITHOUT-CAPSULE.md
  - tests/uat_ctx_005_explicit_cycle_migration.sh
---

# INC-DEBT-043: `--cycle` explícito inexistente producía un envelope resuelto y un binding a una ficción

## Criterio verificable (y se sostenía antes del fix)

`--cycle` es una **referencia**: apunta a un ciclo. Un caller que la entrega
equivocada —un id caducado, un typo, el id de otro proyecto— debe obtener un
fallo tipado, no una respuesta con forma de éxito. Y el comando que responde
es la puerta de entrada de la reconstrucción de contexto: la skill
`skills/sddk-cycle-resume/SKILL.md` la documenta dentro del MISMO envelope
`cli_context` que `sddk cycle status`, y su tabla de reglas duras exige
`BLOCK with argv, exit code, output digest, and recovery action` ante
cualquier referencia inválida.

Observado contra el binario release publicado **v2.3.2** (`~/.local/bin/sddk`),
en ledger aislado, con un ciclo real presente para que «no encontrado» no
pudiera ser un artefacto de un ledger vacío:

```console
$ sddk context bootstrap --root "$WT" --session s-missing-exit \
    --cycle "$PROJECT/does-not-exist" --format json
{
  "status": "no_capsule_source",
  "project_id": "p-0359a7b04609b752",
  "adoption": "complete",
  "cycle": { "state": "explicit", "cycle_id": "cycle-does-not-exist" },
  "context_source": "fresh",
  "basis_revision": "empty",
  "capsule_id": null,
  "binding_ref": "sddk/context/bindings/s-missing-exit.json",
  "binding_written": true
}
EXIT=4
```

Cuatro cosas fallan a la vez, y ninguna es un accidente de redacción:

1. **`state: "explicit"` afirma que la referencia resolvió.** No resolvió.
2. **`cycle_id` viaja al consumidor.** Un agente que pueble `cli_context`
   desde aquí lleva un ciclo inexistente como si fuera el suyo.
3. **`binding_written: true` lo persiste.** El binding durable
   (`sddk/context/bindings/s-missing-exit.json`) apunta a la ficción; sobrevive
   al proceso que la inventó.
4. **El error no existe**, así que no hay nada queMHAgent
   branch. No hay `cycle_id` resuelto, pero el texto lo identifica como
   `no_active_cycle` y el envelope es coherente con ese estado.

## Por qué es P1 y no P3

Es **el mismo defecto que INC-DEBT-039 y 042, un nivel más arriba**, y esa
familia ya tiene dos veredictos: la salida afirma algo que no se hizo, y el
consumidor que lee solo el exit code o el campo de estado se lleva la mentira.
La diferencia con 042 es que allí la carencia era de *capsule* (un artefacto
que faltaba); aquí la carencia es de **identidad**: se afirma la existencia de
un ciclo que no está en el ledger.

Y hay una asimetría que lo hace más grave: el estado degradado **no era
silencioso**, pero tampoco era *coherente*. `EXIT=4` y
`status: no_capsule_source` dicen "algo no está completo", y eso es verdad.
El problema es que el único campo que el consumidor usa para saber **sobre qué
ciclo está trabajando** —`cycle`— no observa esa degradación: `state` dice
`explicit` y trae un `cycle_id`. Es el único campo que el caller controla, y
por tanto el único que no puede haber salido mal por casualidad. Un
`no_capsule_source` honesto con `state: ambiguous` habría sido correcto —
`no_capsule_source` con `state: explicit` no lo es, porque afirma una
resolución que el runtime nunca verificó.

## Causa raíz (estructural, verificada)

La ruta explícita nunca consulta el ledger. En `bootstrap`, `args.cycle` pasa
por `resolve_cycle_context` (que cortocircuita la inferencia por contrato:
`cycle.rs:238`) y el `cycle_id` resultante se copia tal cual a
`BootstrapCycleState::Explicit` sin ninguna lectura de `cycles`:

- `context_cmd.rs` paso 4/5: `compile_cycle_capsule` lee facts del ciclo; si
  no hay ninguno devuelve `Ok(None)` y el comando degrada a
  `no_capsule_source`. **Eso no es validación**: la ausencia de facts y la
  ausencia del ciclo son el mismo caso, y el código lo trataba como ausencia
  de facts.
- paso 6: el `target` se construye con `BindingTarget::Run { run_ref }` a
  partir del id sin comprobar que exista.

El comentario que documentaba esa decisión (en `compile_cycle_capsule`) decía
literalmente «a `--cycle` reference that does not exist yet. The bootstrap
still binds — the target is a reference». **Era una decisión de diseño
explícita, y por eso los fixtures la fijaban.** La premisa que la sostenía
—«una referencia es una referencia, no hay que validarla»— es correcta para
*sesión ≠ run* y falsa para *la referencia tiene que apuntar a algo*.

## Lo que esta INC NO es

- **NO es un bug de INC-DEBT-042.** 042 regula la relación
  `status ↔ capsule_compilada`. Aquí la relation rota es
  `state: explicit ↔ ciclo_existente`. Son ejes distintos y el fix de 042 no
  podía cubrirlo: con un ciclo inexistente, `Ok(None)` es la respuesta
  *correcta* del compilador.
- **NO es un cambio de la semántica de inferencia.** `cycle.rs` no se toca.
- **NO elimina el binding explícito.** Sesión ≠ run sigue siendo el contrato.

## Resolución (fail-closed en la frontera, tipado como el hermano)

Se añade `ContextBootstrapError::CycleNotFound` y, en el paso 6 de
`bootstrap` —justo antes de elegir el `target`—, toda referencia **explícita**
se prueba contra el ledger con `Storage::cycle_exists`:

- `--cycle` explícito que no existe ⇒ `Err(CycleNotFound)` ⇒ el CLI devuelve
  **exit 1** por `failure()`, sin envelope en stdout y **sin escribir
  binding**.
- El mensaje nombra la referencia rota y ofrece la misma acción de
  recuperación que el hermano: `cycle not found: <id>` /
  `recovery: create the record or fix the reference` — la misma forma que
  emite `sddk cycle status --cycle <desconocido>` (`STORAGE_NOT_FOUND`). Dos
  superficies que la skill documenta juntas dejan de discrepar.
- **Exento**: el ciclo *resuelto por inferencia* no se re-consulta, porque el
  resolver ya lo leyó de una fila de lease viva. Re-validarlo sería trabajo
  duplicado.

El blast radius es el binding, no la capsule: la comprobación va **después** de
resolver la basis, así que un ciclo con capsule durable sigue reconectando
(`context_source: recovered`) igual que antes. Ningún test preexistente
cambió de expectativa más allá de los dos fixtures reparados abajo.

## Evidencia

**RED observado antes del fix** (binario release v2.3.2 publicado, el que los
usuarios tienen): los dos tests nuevos fallan y el payload del fallo es el
envelope fantasma de arriba, con `binding_written: true`.

**Verde después**: `cargo test -p sddk-cli --lib context_cmd::tests` =
**25 passed / 0 failed** (2 nuevos + 2 fixtures reparados + 21 intactos).

**Falsador RED→GREEN observado**: neutralizar la llamada a `cycle_exists`
(dejando la variante de error y toda la firma en su sitio) devuelve
**2/2 RED**. La comprobación es lo que sostiene el comportamiento, no la mera
existencia de la variante.

**Detector end-to-end**: `tests/uat_ctx_005_explicit_cycle_migration.sh`
(MIG-UAT-001) ejecutado contra el binario publicado pre-fix produce
**exactamente 4 fallos, todos en la sección fail-closed** (el error no nombra
la referencia, no es tipado, se emitió envelope `state: explicit`, se persistió
binding). Las otras 30 aserciones **PASS**, lo que acota el defecto con
precisión: el runtime ya honraba «explícito vence a inferencia»; lo único
que faltaba era negarse a una referencia rota. Contra el binario con el fix:
**34 ok / 0 FAIL**.

## Fixtures que fijaban el fail-open (hallazgo secundario)

Dos tests preexistentes usaban identificadores de ciclo **que nunca se
insertaban en el ledger**, así que no podían distinguir «enlaza una
referencia» de «enlaza una ficción»:

- `explicit_cycle_binds_run_target_without_inference` (`cycle-explicit`)
- `explicit_cycle_reads_its_own_capsule` (`explicit`)

Ambos affirmed una intención correcta —sesión ≠ run, y que un ciclo explícito
lee **su** capsule— con un fixture que hacía imposible que la intención se
cumpliera. Se repararon plantando un ciclo **real** (`plant_real_cycle`) y
**conservando la aserción original sin relajarla**: el contrato que probaban no
cambia, lo que cambia es que ahora se cumple. Es la cuarta vez que la
corrección de un defecto aparece en un artefacto de test que afirmaba lo
contrario; la quinta sería no mirar.

## Reproducción

```bash
# Pre-fix (v2.3.2 publicado)
sddk context bootstrap --root "$WT" --session s --cycle "$PID/does-not-exist" --format json
# → state: explicit, binding_written: true, EXIT=4

# Post-fix
sddk context bootstrap --root "$WT" --session s --cycle "$PID/does-not-exist" --format json
# → error[...]: cycle not found: <id>
#     recovery: create the record or fix the reference
#   EXIT=1, sin stdout, sin binding
```
