# C3m-identity-alias — SCOPE-CONTRACT: alias de identidad de proyecto a nivel de storage

**Cycle:** `p-63676b11dc0ef88f/identity-alias` (C3m, Semantic & Boundary Convergence)
**Baseline:** `main@e5b03b7d` (workspace v2.5.3, declarada no publicada)
**Opened:** 2026-10-02T08:45:00Z
**Owner:** orchestrator (ejecución directa)
**Authority basis:** AGENTS.md §3 (gates preautorizados); **ADR-0152 `status: proposed` — la implementación NO arranca hasta que el operador lo acepte**.
**Closes:** INC-DEBT-050 (critical/P1), parte de INC-DEBT-049 (high/P1)

## 1. Objetivo (falsable)

Que `sddk` resuelva la identidad de un proyecto a través de un alias
almacenado, de modo que los **25 receipts huérfanos** de esta máquina se
resuelvan a la identidad que tiene su historial, **sin escribir una sola fila
en el fact log**.

**Criterio de salida:** `scripts/migrate_project_identity.py audit` reporta
**0** receipts huérfanos, y el recuento total de filas en las seis tablas
append-only de los 15 ledgers afectados es **exactamente el mismo** que antes
(3.477 más lo que se añada después). Si el número de filas cambia, el criterio
falla.

## 2. Lo que ya está medido (OBSERVED, esta máquina, 2026-10-02)

| magnitud | valor |
|---|---|
| receipts con `project_id` que no coincide con la derivación actual | 25 de 148 |
| proyectos afectados | 15 |
| filas en tablas append-only | 3.477 |
| proyectos migrables reescribiendo el fact log | **0** |
| proyectos con dos identidades vivas | 8 (6 con ciclos en ambos lados, 2 con el nuevo vacío) |
| ciclos que existen sólo del lado nuevo | 51 |
| colisiones de nombre corto al unificar | 1 (`r6-workers-probe-wiring`) |
| receipts de este repo con pin ya aplicado | 1 checkout, `pinned_at: 2026-10-01T11:48:55Z` |

## 3. No-objetivos

- **NO** tocar una sola fila de `events_v1` ni de las otras cinco tablas
  append-only. El trigger lo prohíbe y el hash lo detectaría.
- **NO** fusionar los dos ledgers de un proyecto con dos identidades. Exige
  reescribir `cycle_id`, y hay una colisión de nombre corto.
- **NO** borrar `.sddk/project-pin.json` de este checkout en este ciclo. El pin
  y el alias coexisten hasta que el alias esté desplegado y verificado; quitar
  el pin antes sería dejar el checkout sin la mitigación que hoy funciona.
- **NO** re-derivar ni «arreglar» ids antiguos. Un alias no cambia lo que se
  emitió; cambia a qué apunta lo que se resuelve hoy.

## 4. Superficie

| fichero | cambio |
|---|---|
| `crates/sddk-domain/src/identity.rs` | tabla `project_aliases`, resolución transitiva, detección de ciclo |
| `crates/sddk-cli/src/lib.rs` | encadenar el alias en `resolve_identity_honoring_pin` (**un solo punto**) |
| `crates/sddk-cli/src/lib.rs` | `sddk project alias --from --to --reason` (`--reason` obligatorio) |
| `crates/sddk-cli/src/lib.rs` | `sddk adopt status` y `project resolve` **declaran** que resolvieron por alias |
| `crates/sddk-storage/` | migración de esquema de la tabla, append-only |

## 5. Plan de test (scoped)

Anclado a los criterios 1–6 de **ADR-0152 § Verification**. Cada criterio trae
su falsificador; un criterio sin falsificador que se pueda ejecutar no cuenta.

1. Sin alias, la resolución no cambia. Falsificador: sembrar un alias que no
   aplica y exigir identidad idéntica.
2. Ciclo `A -> B -> A` ⇒ **error duro**, salida no cero, nombra el ciclo.
   Falsificador: un guard que sólo avisa es FAIL.
3. La declaración aparece en la salida de `adopt status` y `project resolve`.
   Falsificador: borrar la línea y exigir que el test falle.
4. Borrar un alias falla (append-only). Falsificador: ejecutar el borrado.
5. Los 25 aliases aplicados ⇒ `audit` en 0 y recuento de filas append-only
   idéntico. Falsificador: cambiar el número de filas ⇒ FAIL.
6. `verify_stream_chain` sigue OK sobre el stream canónico de al menos un
   proyecto con fact log. **Es el criterio que ningún otro cubre.**

## 6. STOP conditions

- Si el hash de algún evento cambia al aplicar un alias ⇒ **parada inmediata**:
  significa que se tocó el fact log.
- Si `resolve_identity_honoring_pin` deja de ser el único resolutor ⇒ parada.
  Ese fue el defecto que W2c corrigió (cinco resolutores desincronizados).
- Si el recuento de filas append-only no coincide exactamente ⇒ parada, antes
  de intentar «ajustar» nada.
- Si el operador no acepta ADR-0152, este ciclo no arranca. El documento queda
  como propuesta.

## 7. Entregables

- `ADR-0152` aceptado (o rechazado con motivo).
- Tabla, resolución, comando y declaración, con los 6 criterios verdes.
- Los 15 aliases declarados sobre el storage real, cada uno con su `reason`.
- `migrate_project_identity.py audit` en 0, con el apéndice del criterio 5.
- INC-DEBT-050 cerrada; INC-DEBT-049 reducida a su parte no resuelta por esto.

## 8. Riesgos

| riesgo | por qué importa | mitigación |
|---|---|---|
| Alias mal escrito manda una identidad a otro sitio | no hay comparación posible: el código no sabe qué alias es «correcto» | acción explícita y auditada, `--reason` obligatorio, y **declaración visible** en la salida |
| La indirección se vuelve permanente | cada resolución futura pasa por la tabla | es el mismo coste que un rename en git, y la misma razón por la que se acepta |
| Un alias tapa una separación real | dos proyectos distintos podrían converger sin querer | `--to` debe existir ya en el storage: no se crea un destino nuevo |
| Ciclo de aliases | resolución no terminante | error duro, con el ciclo nombrado |
| Que el criterio 5 «pase» sin migrar nada | un verde vacío es peor que un gate ausente | los criterios 1–4 y 6 tienen que pasar también; y 5 compara un número, no un bool |

## 9. Fuera de alcance

- INC-DEBT-051 (contrato de versión por adapter) — otro ciclo.
- INC-DEBT-057 (corrupción de script en docs/) — requiere que quien escribió
  cada frase diga qué quiso decir.
- La clave del KMS, que bloquea v2.5.3 y es del operador.
- Publicar el alias en una release: primero verde local, después release.

---

## Addendum — lote 1 (dominio) ejecutado, y un defecto que encontró el falsificador

**PRE-FLIGHT:** `PRE-FLIGHT.md`, `Readiness: READY`.
**Superficie tocada:** `crates/sddk-domain/src/identity.rs` y nada más, como
el pre-flight acotaba. Ni schema, ni CLI, ni storage.

### Lo implementado

`ProjectAlias` (from/to/reason/created_at), `AliasTable` (append-only por
construcción: no hay `remove`), `AliasResolution` (id final más los saltos,
para que la declaración del ADR regla 4 tenga de dónde servirse), y las dos
variantes de error.

### Verificación

```text
cargo test -p sddk-domain --lib identity                      57 passed; 0 failed
cargo test -p sddk-domain --lib                               574 passed; 0 failed
cargo fmt --check                                             limpio
cargo clippy -p sddk-domain --all-targets -- -D warnings      limpio
cargo check --workspace --all-targets                         exit 0
```

El `check --workspace` no es opcional aquí: `IdentityError` es un enum
**público** y añadir una variante rompe cualquier `match` exhaustivo de otro
crate. Los usos que hay son todos construcciones, pero eso se verificó
compilando, no leyendo.

### El falsificador encontró un defecto real en MI test

**6 mutaciones, 5 detectadas. La que no:** `no_cycle_detection`. Al quitar la
detección de ciclo, la suite seguía **verde**.

La causa: el test afirmaba sobre el **texto** del error (`contains("cycle")`).
Sin detección, un ciclo de dos saltos no se colgaba — corría hasta el tope de
16 saltos y devolvía `AliasCycle` igualmente, con un mensaje que casaba con las
aserciones. El test pasaba **por el motivo equivocado**.

Arreglo: `AliasChainTooLong` es ahora una variante **distinta** de `AliasCycle`,
y los tests afirman sobre la variante con `matches!`, no sobre el texto. Un
ciclo y una tabla malformada son defectos distintos con arreglos distintos — re-
apuntar un alias frente a reparar la tabla — y un test que no los distingue no
puede certificar ninguno.

Re-falsificado: **6/6 detectadas**, con `no_cycle_detection` rompiendo 2 tests.

Es la cuarta vez en esta sesión que un falsador encuentra un defecto que leer
no habría encontrado, y la primera que encuentra uno **en el trabajo de esta
sesión** en vez de en algo heredado.

### Lotes que quedan

| lote | superficie | por qué separado |
|---|---|---|
| 2 | persistencia de la tabla | un fallo de lógica y uno de cableado tienen que ser distinguibles por el resultado |
| 3 | `resolve_identity_honoring_pin` + `sddk project alias` + declaración en la salida | el cableado es donde se re-introduce el defecto de los cinco resolutores |

Ninguno de los dos toca el storage real de la máquina. Los 15 aliases se
aplican después, con su propio criterio de recuento de filas.

---

## Addendum — lote 2 (el store) ejecutado, y una razón para no ser SQLite

**Commit:** `2d7fac4c`. **Superficie:** `crates/sddk-cli/src/project_alias.rs`
(nuevo) más una línea de `lib.rs` que lo registra. Ni dominio, ni storage.

### Apartarse del propio SCOPE: JSON en vez de tabla SQLite

La §4 de este contrato pedía «migración de esquema de la tabla, append-only»
en `crates/sddk-storage/`. **No es lo que se hizo**, y la razón es el modo de
fallo, no la comodidad:

El fallo que este store tiene que hacer visible es **redirigir la identidad de
un proyecto entero al sitio equivocado sin avisar**. Un JSON se lee con `cat`,
se diff-ea y se pega en un bug report. Una tabla SQLite exige una herramienta
de consulta, y su inspección es un acto deliberado — que es exactamente lo que
no va a pasar si nadie sospecha que hay un alias. Quien mire `$XDG_STATE_HOME`
se encuentra el pin y el alias juntos, en el mismo sitio y con el mismo
formato; quien mire una tabla que no sabía que existía, no mira nada.

Encima: son quince filas, y un store global no pertenece al sistema de
migraciones **por ledger** de `sddk-storage`. Un alias se resuelve *antes* de
saber qué ledger es; meterlo en el ledger del destino es un huevo y gallina.

El precedente también es del repo: el pin vive en `.sddk/project-pin.json`.

### Verificación

```text
cargo test -p sddk-cli --lib project_alias     15 passed; 0 failed
cargo fmt --check -p sddk-cli                  limpio
cargo clippy -p sddk-cli --all-targets         limpio
```

Falsificado con **6 mutaciones, 6/6 detectadas**. El contador de tests pasados
baja de forma monótona (13→10→9→9→8→7) porque el falsificador **acumula** y
nunca revierte entre mutaciones: eso confirma que cada mutación se aplicó de
verdad, en vez de producir un verde falso. Hubo que hacer una copia de
referencia antes de lanzarlo, porque el fichero sale del run degradado.

La sexta mutación no compilaba la primera vez: `ProjectId` no implementa
`Ord`, así que el `sort_by` era inválido. Una mutación que no compila es una
mutación **inválida**, no un hueco del test — se corrige la mutación.

### Dos expectativas mías que el código contradijo

1. **`no-es-un-id` es un `ProjectId` válido.** El test afirmaba que
   "no-es-un-id" sería rechazado; el validador lo acepta. Se añadió además un
   test **positivo** de ese mismo id, para que nadie "arregle" la expresión
   regular pensando que estaba mal.
2. **El store rechaza el ciclo en la segunda declaración**, no en una
   escritura posterior. La expectativa original era la intención declarada y
   resultó **peor**: un ciclo escrito y detectado más tarde es un fichero en
   disco que ya no resuelve, que es justo el estado que este store no debe poder
   alcanzar. Se conserva el código y se corrigió la expectativa.

### El doc mentía sobre lo que el código hacía

`declare_alias` documentaba que rechazaba un `to_id` que ya fuese `from_id` de
otro alias. **No lo hace**: el ciclo lo rechaza `store_alias_table_at`, un paso
después, al resolver toda la tabla antes de escribir. El doc se corrigió para
decir lo que el código promete y **dónde** rechaza de verdad, en vez de
cambiar el código para que encajara con el doc.

### Dos corrupciones de prosa en el propio fichero nuevo

`suBINspección` donde debía decir «su inspección», y `unaHop` donde debía decir
«una cadena de un salto». Misma familia que INC-DEBT-057 pero **en ASCII**: el
guard de docs mira alfabetos no latinos, no mayúsculas pegadas dentro de una
palabra, así que no las ve. Corregidas porque aquí la palabra original **es**
reconstruible — al contrario que en los 13 ficheros de INC-DEBT-057, donde
sustituir sería fabricar.

> **Deuda que esto abre:** el guard de `test_docs_script_contamination.py` no
> cubre la clase ASCII. Está anotado como pendiente, no como cerrado.

---

## Addendum — lote 3 (el cableado), y una mutación que escapó

**Superficie:** `crates/sddk-domain/src/identity.rs`,
`crates/sddk-engine/src/paths.rs`, `crates/sddk-cli/src/lib.rs`.

### Dos hallazgos de diseño que salieron al cablear

**1. `project resolve` era un segundo resolutor, y por un motivo tonto.** La
rama del pin descartaba `remote_url` y `fallback_seed`, así que la superficie
no tenía más remedio que re-derivar la identidad para recuperar los datos que
muestra. El arreglo no es "calcularlo dos veces": es **derivar primero y dejar
que el pin sobrescriba el id**, con lo que `resolve_xdg_paths` deja de tirar
datos y `project resolve` pasa a usar el resolutor único. Se conserva el caso en
que el pin resuelve aunque la derivación no pueda (sin remote y sin seed): ese
camino existía antes y el pin es justo lo que hace resoluble un checkout sin
remote utilizable.

**2. El alias se aplica a la rama pinada también, y es deliberado.** El pin es
por checkout y **no converge**: un checkout pineado antes de la re-adopción
resuelve al id retirado para siempre. Si el alias se aplicara solo a identidades
derivadas, el único caso que más necesita redirigir sería el único que no
alcanzaría el redirect — y el pin quedaría como huida permanente. Pin contesta
"qué id es este checkout"; alias contesta "qué proyecto es ese id".

El orden importa y no es intercambiable: alias-primero dejaría que un pin
reintrodujera un id retirado, y pin-después dejaría que un checkout obsoleto se
eximiera del redirect. Hay una mutación para cada mitad de esa frase.

### Verificación

```text
cargo test -p sddk-cli --lib        901 passed; 0 failed; 2 ignored
cargo test -p sddk-domain --lib     574 passed; 0 failed
cargo test -p sddk-engine --lib    1376 passed; 0 failed; 1 ignored
cargo fmt --check (3 crates)        limpio
cargo clippy (3 crates, -D warnings) limpio
cargo check --workspace --all-targets  exit 0
```

10 tests nuevos: 8 de cableado del resolutor y 2 de la salida.

### La mutación que escapó: `resolve_bypasses_the_wiring`

**7 mutaciones, 6 detectadas. La séptima dejó la suite EN VERDE.**

La mutación reinstala en `project resolve` la forma antigua: una rama de pin que
devuelve el id del pin sin consultar jamás el alias. Es el defecto exacto que
este lote existe para cerrar, y no lo detectó ningún test.

La causa es que los tests vivían a los dos lados de la costura y ninguno la
cruzaba: los de `resolve_identity_honoring_pin_with` ejercitan el resolutor, los
de `project_resolution_text` ejercitan el renderizador, y la propiedad que
importa — *que esta superficie pase por el resolutor único* — vive en el medio.
Un test por debajo y otro por arriba no cubren el empalme.

Arreglo, en el mismo espíritu que el del lote 1: **la costura se hace
testeable en vez de sujetarla con variables de entorno.**
`run_project_resolve` se parte en la lectura de entorno y
`run_project_resolve_with(args, table)`, que recibe la tabla como el resolutor
recibe la suya. El test end-to-end monta un pin obsoleto y un alias, y exige el
id post-alias **y** la declaración. Re-falsificado: **7/7 detectadas**, con esa
mutación rompiendo precisamente ese test.

> Un test que necesita un `$XDG_STATE_HOME` real para comprar hermeticidad no
> es hermético: es un test que nadie va a escribir, porque el resto de pruebas se
> ejecutan en paralelo y `set_var` es una carrera. Partir la función es más
> trabajo y es la única variante que se puede probar de verdad.

### El comando

`sddk project alias --from --to --reason` con **`--reason` obligatorio** y
**`--to` tiene que tener ledger ya**. Ese segundo chequeo es la diferencia entre
"fusiono dos historias" y "apunto este proyecto a la nada": un alias a un
`project_id` sin estado detrás resuelve todos los comandos a un ledger vacío, y
lo hace **con éxito**. Fallar en la declaración es el único sitio donde ese
error todavía es barato. Hay `--dry-run`, que previsualiza contra la tabla
actual, así que también ensaya el caso de cerrar un ciclo.


