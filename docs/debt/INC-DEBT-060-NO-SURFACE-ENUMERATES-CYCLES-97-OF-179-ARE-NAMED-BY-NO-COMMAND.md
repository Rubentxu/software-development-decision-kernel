---
id: INC-DEBT-060
title: "Ninguna superficie del producto enumera los ciclos: 97 de los 179 de un proyecto no los nombra ningún comando, y 91 de esos declaran estar OPEN"
status: open
severity: high
priority: P1
detected_at: 2026-10-02
detected_in_session: session-69c
component: cycle
surface: [crates/sddk-storage/src/lib.rs, crates/sddk-cli/src/cycle.rs]
cluster_id: CL-STATE-AUTHORITY
related: [INC-DEBT-049]
references:
  - crates/sddk-storage/src/lib.rs
  - crates/sddk-cli/src/cycle.rs
  - crates/sddk-cli/src/ledger.rs
---

# INC-DEBT-060 — La autoridad no puede enumerar su propio estado

> ## ⚠️ CORRECCIÓN DE MAGNITUD (2026-10-02, session-69d)
>
> **El titular y el cuerpo de más abajo usan cifras que han resultado estar mal.**
> No se reescriben: se conservan como evidencia de lo que se creía al detectar, y
> esta corrección va delante para que nadie los lea sin ella.
>
> **Lo que se creía:** 179 filas en `cycles`, 97 no nombradas por ninguna
> superficie, 91 de esas `OPEN`, 81 manifiestos ilegibles.
>
> **Lo que se ha medido:** la tabla `cycles` de ese ledger **no contiene 179 ciclos
> de este proyecto**. Contiene **dos poblaciones**:
>
> | `project_id` | filas | ids | manifiesto | eventos |
> |---|---|---|---|---|
> | `p-63676b11dc0ef88f` | **100** | `<project_id>/<slug>` | 98 legibles, 2 ilegibles | 77 ciclos tienen ≥1 evento, **23 no** |
> | `__spine_import__` | **79** | slug desnudo (`AGENT-HOST-001`, …) | las 79 con `{}` | **0**, ninguna |
>
> `__spine_import__` **no es un marcador**: es una fila real de la tabla `projects`
> del mismo ledger (`display_name: "Spine Import Project"`, workspace
> `spine-import`). Son registros importados de spine, no ciclos de este proyecto.
>
> **Cifras correctas:** de los **100 ciclos de `p-63676b11dc0ef88f`, ninguna
> superficie los enumeraba, 23 no los nombraba, 17 de esos 17 son `OPEN`, y 2
> tienen manifiesto ilegible.** La clase de defecto no cambia; la magnitud sí, y
> era casi 4× mayor de lo declarado.
>
> **Cómo se encontró:** el falsificador de R6 (el del propio remedio) falló
> contra un producto correcto. Falló porque su baseline contaba
> `SELECT COUNT(*) FROM cycles` sin filtro de proyecto. Es la **tercera** vez en
> este ciclo que un falsificador falla por un guard mal escrito y no por un
> defecto del producto. Se deja escrito porque el número que sostenía un
> documento publicado era el del guard.

## Qué es

El ledger del proyecto `p-63676b11dc0ef88f` tiene **179 filas en la tabla
`cycles`**. Ninguna superficie del producto nombra **97** de ellas, y **91** de
esas 97 declaran `status: OPEN`.

Un ciclo que el ledger afirma abierto y que ningún comando nombra no es un ciclo
abierto: es una fila que la autoridad no puede presentar. La premisa del proyecto
—*SDDK es la autoridad exclusiva del estado operativo*— se cumple para leer y
falla para enumerar.

## Cómo se midió

Falsificador: `/var/home/rubentxu/ro/03-alcance.py` — **PASS=7 FAIL=0 SKIP=1**.
Read-only: `SELECT` directo y comandos de solo lectura.

```
ciclos en la tabla cycles          : 179
ciclos que NOMBRA alguna superficie:  82
ciclos que NO nombra ninguna       :  97   <- de ellos, 91 con status OPEN
```

Las superficies que existen, y qué alcanzan:

| Superficie | Alcanza | Por qué no llega al resto |
|---|---|---|
| `sddk cycle status` | 0 | Infiere el ciclo actual por **lease vivo** (`list_active_cycle_leases_for_project(project_id, now_ms)`, `cycle.rs:341`). Los 30 leases del proyecto están **caducados**, así que responde `no active cycle found`. Es correcto según su semántica: no hay ciclo *activo*. No es un falso negativo, es otra pregunta. |
| `sddk ledger events` | 82 | Solo los ciclos que emitieron algún evento. Y **trunca en 50 eventos por defecto** (590 en total), así que sin `--limit` solo ve 19 ciclos. |
| `Storage::get_cycle(id)` | 1 | Por id. Hay que **saber** el id, y no hay superficie que lo dé. |
| `list_cycles` | **no existe** | `grep 'fn list_cycles'` sobre `crates/sddk-storage/src/lib.rs` → 0. No hay enumeración en el storage. |
| `sddk cycle list` | **no existe** | `sddk cycle --help` no lista ningún subcomando de enumeración. |

## Por qué 97 ciclos no tienen evento

No es legado anterior al fact log, que era la explicación benigna:

- `events_v1` arranca el **2026-08-31T11:06:00Z**;
- los ciclos sin evento son del **2026-09-07** en adelante.

Es decir: el sistema de hechos ya existía y estos 97 filas se escribieron igual
sin hechos detrás.

La tabla `cycles` muestra **dos vías de escritura** distintas:

| `created_at` | Ciclos | ¿Con evento? | `manifest_json` |
|---|---|---|---|
| RFC3339 (`2026-09-07T19:24:17…Z`, con `T` y `Z`) | 100 | 82 sí, **18 no** | nativo / `null` |
| Con espacio (`2026-09-07 19:24:17`) | 79 | **ninguno** | `{}` |

De los 97 sin evento: **79** con timestamp de espacio (importados, `manifest_json`
vacío) y **18** con timestamp RFC3339 (escritura nativa, sin evento). Los 18 son
el caso más incómodo, porque no son ni legado ni importación: los escribió el
camino normal y no dejaron hecho.

## Clasificación por AGENTS.md §2.7

La tabla `cycles` se comporta como **Projection**, y una proyección debe ser
reconstruible desde los hechos. 97 filas no lo son: sus hechos no existen. Eso las
convierte de hecho en **Object sin Fact** — estado durable que ningún hecho
sostiene y que ninguna proyección puede regenerar si se pierde.

## Severidad: `high`, y por qué no `critical` ni `medium`

No es `critical`: **no hay pérdida de datos**. Las filas están ahí y son
legibles; nada se ha borrado ni corrompido. La lista de `critical` exige bloquear
release, pérdida de datos o ruptura de frontera de seguridad, y ninguna se cumple.

No es `medium` tampoco, y esta es la parte discutible. `medium` es «degrada
funcionalidad no core; **existe workaround**». El workaround aquí existe y es
`sqlite3 ledger.sqlite 'select * from cycles'` — es decir, **rodear el producto
entero y leer su almacenamiento a mano**. Un rodeo que consiste en dejar de usar
la herramienta no es un rodeo: es la razón por la que la incidencia es `high`. Se
declara el criterio escrito para que pueda ser challenging: si se considera que
leer el storage a mano es un rodeo aceptable, esta deuda baja a `medium` y la
decisión es del operador, no mía.

## Lo que esta deuda NO es

- **No es** la parte abierta de INC-DEBT-049. Ahí el problema era historial bajo
  otra identidad; ahí la premisa **hoy es falsa**: el alias
  `p-995939af668a53d8 → p-63676b11dc0ef88f` ya está declarado y ese id hermano
  tiene **0 eventos y 0 ciclos**, luego no hay historia que quede fuera de vista.
- **No es** un falso negativo de `cycle status`. Su semántica es «ciclo con lease
  vivo» y responde bien a esa pregunta.
- **No es** pérdida de datos. Las 97 filas son legibles.

## Magnitud declarada

Solo se midió `p-63676b11dc0ef88f`. El storage de la máquina tiene **333
directorios de proyecto**, así que la magnitud total es **mayor** y no se midió.
Que sea mayor no cambia el hecho y por eso se declara SKIP, no PASS.

## Falsificadores exigidos cuando se implemente

1. **F60** — sembrar 3 ciclos en un storage limpio y ejecutar la nueva
   superficie de enumeración ⇒ devuelve **3**, no 0 y no un subconjunto.
2. **F61** — un ciclo `CLOSED` aparece en la enumeración ⇒ falla si se filtra por
   estado; la enumeración es de todos los ciclos, no de los activos.
3. **F62** — un ciclo **sin eventos** aparece en la enumeración ⇒ falla si la
   superficie se construye sobre `events_v1`, que es exactamente el error que
   produce estos 97.
4. **F63** — `sddk ledger events` **sin** `--limit` declara cuántos eventos hay
   del total, o falla; hoy devuelve 50 de 590 sin decirlo.

## Remedio, no aplicado

Dos piezas, y la segunda no depende de la primera:

1. `Storage::list_cycles(project_id, estado_opcional)` más `sddk cycle list`.
   F63 debe entrar en el mismo lote porque hoy el truncamiento silencioso hace
   que **cualquier** lectura por eventos subestime el estado, que es la misma
   familia de defecto que esta incidencia.
2. Decidir qué se hace con las **97 filas sin hecho**: si son Objects sin Fact
   legítimos, su estado necesita una fuente de verdad declarada; si son residuo
   de una importación, limpiarlas es destructivo y es decisión del operador.
   **No se hace nada sin esa decisión.**

## Addendum session-69c — cómo se encontró

No salió de leer el roadmap. Salió de medir la premisa de INC-DEBT-049 («la
autoridad no ve su propia historia») y, al hacerlo, chocar con que
`sddk cycle status` decía `no active cycle found` mientras la tabla tenía 107
ciclos no terminales. La primera hipótesis —«eso es un falso negativo»— era
**falsa**: el código infiere por lease vivo y los 30 leases estaban caducados.
La segunda hipótesis, ya leída la implementación, sí: el hueco no es de
resolución sino de **enumeración**, y por eso no lo cubría INC-DEBT-049.

Dos FAIL de la medición fueron del falsificador, no del producto: contar 160
ciclos inalcanzables porque `ledger events` trunca en 50 por defecto, y exigir
un reparto de estados copiado de un cálculo anterior en vez de comparar contra
el valor medido.

## Addendum session-69d — la corrección, medida y con su propia evidencia

El remedio (`cl-cycle-enumeration`) construyó el falsificador que la misma deuda
exigía, y ese falsificador falló contra un producto **correcto**: la lista
devolvió 100 y el baseline decía 179. Antes de tocar el código —que era el
movimiento fácil y el equivocado— se midió de dónde salía la diferencia, porque
un FAIL puede ser el guard y no el producto, y las dos reparaciones son
opuestas: `list_cycles` no debe enumerar filas de otro proyecto.

Medición (`04-spine-import.py`, read-only, `mode=ro`):

```
todas las filas de cycles                          : 179
  project_id = p-63676b11dc0ef88f                   : 100
  project_id = '__spine_import__'                   :  79   <- otro proyecto
forma de cycle_id
  <project_id>/<slug>                               : 100
  slug desnudo                                      :  79
proyectos registrados en ESTE ledger
  ('p-63676b11dc0ef88f', 'sddk-framework')
  ('__spine_import__', 'Spine Import Project')      <- fila real, no marcador
de los 100 de este proyecto
  nombrados por un evento                           :  77
  NO nombrados por ninguna superficie               :  23   (17 OPEN)
  manifiesto legible                                :  98
  manifiesto ilegible                               :   2
de los 79 de __spine_import__
  con algun evento                                  :   0
```

**Las 79 filas con `{}` que el documento atribuyó a los «97 sin hecho» son, una
a una, la población de `__spine_import__`.** No son ciclos de este proyecto: son
registros importados de spine, con su propio proyecto, su propio workspace y su
propio `display_name` en el mismo ledger. Los 23 que sí son de este proyecto
tienen `manifest_json` nativo o `null`, y solo 2 son ilegibles.

### Qué cambia y qué no

| | Antes | Ahora |
|---|---|---|
| filas de `cycles` atribuidas al proyecto | 179 | **100** |
| ciclos no nombrados por ninguna superficie | 97 | **23** |
| de esos, `OPEN` | 91 | **17** |
| manifiestos ilegibles | 81 | **2** |
| superficie de enumeración | no existe | **`Storage::list_cycles` + `sddk cycle list`** |

**La clase de defecto no cambia y por eso la deuda sigue `open` y `high`:** sigue
sin existir superficie para declarar qué son las 79 filas importadas, y 17 ciclos
que el ledger afirma `OPEN` no estaban —y sin el remedio no están— nombrados por
nada. Lo que cambia es el número, y bajarlo a `medium` sobre la evidencia
corregida es decisión del operador, no se toma aquí.

### Lo que la corrección abre, y no se cierra aquí

Las 79 filas de `__spine_import__` son un **sujeto distinto** con su propio
problema, no una parte de este: un proyecto que existe dentro del ledger de otro,
cuyo workspace es el literal `'spine-import'` (no una ruta) y cuyos 79 registros
no tienen evento ni manifiesto. Que sean o no alcanzables desde algún checkout es
una pregunta que **no se ha medido** en esta sesión y se declara no medida.
SCOPE-CONTRACT §2.4 ya excluía la deuda de procedencia de esas importaciones del
alcance de `cl-cycle-enumeration`, y esta corrección confirma que la exclusión
estaba bien puesta: no eran el mismo problema.
