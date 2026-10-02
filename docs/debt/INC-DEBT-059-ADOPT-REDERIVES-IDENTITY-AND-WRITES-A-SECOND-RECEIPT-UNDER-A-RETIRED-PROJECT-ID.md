---
id: INC-DEBT-059
title: "El store de alias se resuelve para `project resolve` y no para `adopt`: adopt re-deriva la identidad y `adopt apply` escribe un segundo recibo bajo el project_id retirado"
status: open
severity: high
priority: P1
detected_at: 2026-10-02
detected_in_session: session-69
component: identity
surface: crates/sddk-cli/src/lib.rs
cluster_id: CL-IDENTITY
related: [INC-DEBT-049, INC-DEBT-050]
references:
  - crates/sddk-cli/src/lib.rs
  - crates/sddk-engine/src/adoption.rs
  - docs/architecture/adrs/ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS.md
  - docs/debt/INC-DEBT-050-REMOTE-CASE-NORMALIZATION-REASSIGNS-PROJECT-IDS-WITHOUT-MIGRATION.md
  - docs/debt/INC-DEBT-049-READOPTION-REASSIGNS-PROJECT-IDSILENTLY-ORPHANS-HISTORY.md
fingerprint: "adopt_rederives_identity_and_writes_a_second_receipt_under_a_retired_project_id"
---

## Qué es

El store de alias de ADR-0152 se resuelve en **un** sitio, y ese sitio no es el
que usa `adopt`. Hay tres puntos de llamada de
`resolve_identity_honoring_pin*`:

| Punto de llamada | Superficie | Usa alias |
|---|---|---|
| `lib.rs:1551` | el wrapper | — |
| `lib.rs:1636` `resolve_project_ids` | `context`, inferencia de ciclo | sí |
| `lib.rs:1903` `run_project_resolve` | `sddk project resolve` | sí |
| `lib.rs:2133` `prepare_adoption_plan` | **todo `sddk adopt`** | **no** |

`prepare_adoption_plan` llama a `plan_adoption` (`adoption.rs:195`), y
`plan_adoption` llama a `resolve_project_identity` **directamente**
(`adoption.rs:203`). La tabla de aliases no se carga en esa ruta: es el único
camino de identidad del CLI que no la consulta.

El pin sí se respeta, pero por un mecanismo **paralelo** dentro de
`plan_adoption`, no por el resolver canónico. El resultado son **dos**
resolutores de identidad, que es exactamente lo contrario de lo que ADR-0152
autorizó al fijar un único punto de decisión.

## OBSERVED: los dos comandos discrepan sobre qué proyecto es el checkout

Reproducción fuera de este repositorio (regla de cero intrusión, AGENTS.md §1),
en `/var/home/rubentxu/repro-c3`, con `HOME`/`XDG_*` aislados. Secuencia:
checkout nuevo con remote; `adopt apply` con el pin puesto en `X`; pin retirado;
alias `Y -> X` declarado. Al final, el mismo checkout, sin pin, en el mismo
instante:

```
$ sddk project resolve --root ws --scope .
project_id: p-0000000000000aaa
identity_alias: p-c4319c598bc98be8 -> p-0000000000000aaa        # exit 0

$ sddk adopt status --root ws --scope .
status: absent
project_id: p-c4319c598bc98be8
receipt: .../projects/p-c4319c598bc98be8/workspaces/w-464b.../adoption.json
ledger: .../projects/p-c4319c598bc98be8/ledger.sqlite
detail: receipt and ledger registration are absent                # exit 1
```

`project resolve` resuelve `Y -> X` y lo declara. `adopt status` no ve el alias,
se queda en `Y`, **mira un ledger que no existe** y reporta `absent`. El alias
está declarado, el store lo tiene, el resolver lo aplica — y `adopt` responde
que ese proyecto no está adoptado. La mitad de la redirección es invisible
desde la superficie de adopción.

## OBSERVED: `adopt apply` recrea el huérfano que ADR-0152 existe para cerrar

Sobre ese mismo checkout, con el alias declarado:

```
$ sddk adopt apply --root ws --scope .
status: complete
project_id: p-c4319c598bc98be8

$ find .../data/sddk/projects -name adoption.json
.../p-0000000000000aaa/workspaces/w-4c77.../adoption.json
.../p-c4319c598bc98be8/workspaces/w-464b.../adoption.json   <-- NUEVO
```

Aparece un **segundo recibo de adopción bajo el id retirado**. `project resolve`
sigue diciendo `p-0000000000000aaa` inmediatamente después, así que el proyecto
queda con dos adopciones y la redirección declarada no evita ninguna de las dos.

Esto es la enfermedad que motivó ADR-0152. El ADR dice, textual:

> los 25 receipts huérfanos quedan resolubles sin tocar ninguno

y el store se construyó para que **no volvieran a aparecer**. El mecanismo los
vuelve a crear, de forma determinista, en cada `adopt apply` sobre un checkout
con alias, y **sin avisar**: el comando sale `complete`. Con 14 aliases
declarados en el storage real de esta máquina, la condición es alcanzable en 14
proyectos.

## Por qué ningún test lo cazó

```
$ grep -c alias crates/sddk-cli/tests/adoption_contract.rs
0
$ grep -c alias crates/sddk-cli/tests/project_pin_e2e.rs
0
```

**Cero** ocurrencias en los dos ficheros que cubren la superficie de `adopt` y la
del pin. Los tests viven a ambos lados de la costura y ninguno la cruza. Es la
**misma** forma que la mutación `resolve_bypasses_the_wiring` del lote 3 de
ADR-0152, que escapó por idéntica razón y que ya costó partir
`run_project_resolve_with` para poder cruzarla: aquí la costura existe y nadie la
cruza porque **nunca se cruzó**.

## Un doc afirma la convergencia que el código no tiene

`lib.rs:1699-1703`, doc de `ProjectPin`:

> **This doc used to be a false claim.** It said "every runtime context honors
> it" while only two of five independent resolvers did; `adopt status`, `config
> resolve` and cycle inference silently re-derived from the remote. Nothing
> tested the contract it asserted. All resolvers now go through
> [`resolve_identity_honoring_pin`]. INC-DEBT-049.

La última frase es **falsa**, y lo es por la misma función que el propio doc
nombra: `adopt status` sigue siendo un resolutor independiente. Se corrigió para
el pin y se dejó el mismo agujero para el alias, y al cerrarlo declara una
afirmación de convergencia que ningún test mide — que es el defecto que el doc
dice haber cerrado. Un doc no se ejecuta, así que el que lo vigila tendría que
ser un test **estructural** sobre los puntos de llamada.

## Impacto sobre ADR-0152

El criterio 3 («`sddk adopt status` y `sddk project resolve` dicen que
resolvieron a través de un alias, con el `from` y el `to`») es **ROJO**. No por
una línea que falte en el render: `AdoptionStatus` no tiene forma de declarar un
alias porque la identidad que recibe nunca pasó por la tabla.

Añadir el campo `alias_origin: Option<ProjectId>` a `AdoptionStatus` y su línea
de render **no cierra el criterio**: se midió que `plan.identity.alias_origin()`
es `None` **siempre**, porque `plan_adoption` no resuelve alias. El campo
serializaría `none` en el 100% de los casos — una declaración que nunca se
dispara, que es peor que no declararlo porque hace el criterio *parecer*
satisfacido a quien lea la estructura. Ese cambio se midió y se revirtió en vez
de commitearse.

**El ADR no se promueve.** C3 en rojo significa que los 6 criterios no se
suman.

El criterio 5 («audit reporta 0 receipts huérfanos») queda en la misma línea: hoy
reporta 1 divergencia por el par de skillgraph retirado, y aunque se reconciliara,
el mecanismo que lo produce seguiría vigente.

## Gravedad

`high`, no `critical`. La razón de esa elección, porque la diferencia importa:
esta INC crea un recibo duplicado bajo un id retirado, y ese recibo **es
resoluble** por el mismo alias que el mecanismo del store ya resuelve — que es
justo lo que `critical` significa para INC-DEBT-050 y lo que aquí **no** se
cumple. No hay pérdida de datos, no hay corrupción de `content_hash`, y el
`verify_stream_chain` de los ids canónicos no se toca: el criterio 6 de ADR-0152
sigue en pie. Lo que hay es un contrato roto y dos comandos que dan respuestas
distintas sobre el mismo estado, sin aviso. Eso es `high` en la taxonomía
(«broken contract»), y sube a `critical` en cuanto el mecanismo se aplique a un
proyecto cuyo id retirado **no** tenga alias declarado, porque entonces el
recibo no es resoluble por nadie.

## Reproduction

```bash
bash /var/home/rubentxu/repro-c3.sh    # alias + pin + adopt status
bash /var/home/rubentxu/repro-c3b.sh   # adopt apply -> segundo recibo
```

## Resolution (abierta)

Precondición de la corrección: **`SCOPE-CONTRACT` + `PRE-FLIGHT` propios**, y
test **RED antes** del arreglo, no después.

El arreglo tiene que mover la decisión, no añadirla. `plan_adoption` re-deriva
porque el engine es filesystem-free por diseño y la tabla la carga la CLI, así
que la forma correcta es que `prepare_adoption_plan` resuelva **una vez** por
`resolve_identity_honoring_pin_with` y pase al engine la identidad ya resuelta
—con `alias_origin` y `identity_source` intactos, porque `context_cmd.rs` lee
`identity_source` para decidir si reenvía el pin. Pasar el id resuelto como
`pinned_project_id` sería más corto y **incorrecto**: degradaría `identity_source` a
`Pinned` en el camino no pinado, que es la regresión que el comentario de
`adoption.rs:210-243` ya warnió una vez.

Criterios de cierre, todos falsables:

1. Un test **estructural** sobre los puntos de llamada: `prepare_adoption_plan` no
   llama a `resolve_project_identity` (ni a `plan_adoption` sin identidad
   resuelta). Falsificador: reintroducir la derivación y exigir fallo.
2. Un test e2e que cruza la costura: alias declarado + `adopt status` ⇒ el
   `project_id` reportado es el `to`, `identity_alias` nombra `from -> to`, y el
   ledger leído es el del `to`. Falsificador: borrar la línea de declaración y
   exigir fallo (es el falsificador que el criterio 3 ya exige).
3. Un test e2e de no-regresión: `adopt apply` sobre un checkout con alias
   **no** crea un recibo bajo el id retirado. Falsificador: contar los ficheros
   `adoption.json` antes y después.
4. El doc de `ProjectPin` corregido, y un test estructural que verifique que la
   afirmación de convergencia del doc se sostiene — o que el doc deja de
   afirmarla. Un doc que afirma una convergencia sin test es la clase que
   INC-DEBT-049 ya pagó una vez.
5. `scripts/migrate_project_identity.py audit` re-ejecutado sobre el storage real
   **después** del arreglo, para el criterio 5 de ADR-0152.

## Nota sobre el estado del árbol

El cambio que sí compila —`alias_origin` en `AdoptionStatus` más su línea de
render— **fue revertido sin commitear** en vez de dejar un campo que declara un
alias que nunca ocurre. No hay nada que recuperar: está en el historial de esta
sesión, no en un commit.
