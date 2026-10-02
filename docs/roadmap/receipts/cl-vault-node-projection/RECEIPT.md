# RECEIPT — cl-vault-node-projection

**Cycle:** `p-63676b11dc0ef88f/vault-node-projection`
**Closed:** 2026-10-03T01:05:00Z
**Workspace:** 2.5.3 (declarada, **no publicada**; último tag remoto `v2.5.2`)
**Baseline:** `4822ddd1` (`docs(roadmap): session-69j…`)

---

## §1 — Qué se cambió

| commit | concernia | contenido |
|---|---|---|
| `593bb9e5` | `docs(roadmap)` | SCOPE-CONTRACT y PRE-FLIGHT (`Readiness: READY`) |
| `ddb92cca` | `test(vault)` | 3 RED + 1 caracterización + 1 guard de `NodeKind` (árbol **rojo a propósito**) |
| este ciclo | `fix(vault)` | `NodeProjection` + `projection_note` + `OMITTED_NODE_FIELDS` |
| este ciclo | `chore(changelog)` | entradas `fix(vault)` y `test(vault)` |

**Producción tocada:** `crates/sddk-vault/src/export.rs` — **un solo fichero**:
`export_node` deja de ser un `json!` a mano, y la página gana una frase de
alcance. El HTML visible —tabla, estilos, columnas— **no se toca**.

## §2 — Comandos ejecutados y resultados

| comando | resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5396 passed, 0 failed** (baseline 5391, **+5**) |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `python3 ce/06-scan.py` | **CLEAN** |
| `vault/08-medir-nodes.py` | **DEFECTO — omite sin declarar** → **correcto: omite y lo declara** |
| `test_changelog_coverage` | **PASS=57 FAIL=0** |
| `test_debt_index_coherence` · `test_docs_script_contamination` · `test_gate_coverage` · `test_release_state_pointer` | exit 0 |

**Contexto real vs. sintético:** el script de medición construye un vault de
markdown en un árbol temporal con `XDG_*` propio y `SDDK_DATA_DIR` eliminado, y
extrae el JSON **del HTML que produce el binario real**, no de una estructura en
memoria. **Ninguna medición de este ciclo se hizo contra el vault real**, y el
ciclo no escribe en él.

## §3 — Objetivos y STOP

| objetivo | veredicto | evidencia |
|---|---|---|
| **O1** el artefacto declara su alcance | **CUMPLIDO** | R1; la página dice «carries 7 of 8 fields… Not carried: body» |
| **O2** la lista deja de estar escrita a mano | **CUMPLIDO, CON UNA CORRECCIÓN** | ver §4 — la afirmación de compilación era falsa y se corrigió |
| **O3** `status` y `tags` por una regla | **CUMPLIDO** | R3: los dos viajan |
| **O4** `body` no viaja, con el motivo escrito | **CUMPLIDO** | R4; el motivo está en `OMITTED_NODE_FIELDS` |

| STOP | veredicto |
|---|---|
| 1 — declarar el alcance no puede obligar a incrustar cuerpos | **no se cruzó**: R4 falla si `body` aparece, y el script confirma que el texto del cuerpo no está en la página |
| 2 — incluir `tags` no rompe el test existente | **no se cruzó**: las 7 aserciones de `renders_self_contained_inspector` siguen verdaderas sin reescribirlas |
| 3 — el acoplamiento tiene que poder fallar | **SE CRUZÓ, y lo que encontró está en §4** |

## §4 — El falsificador encontró un defecto en el remedio, y en el guard

Esto es lo que más valor tiene del ciclo, y va antes que cualquier número.

### 4.1 — O2 era falsa

El SCOPE y el doc de `NodeProjection` afirmaban que derivar la proyección por
`From<&VaultNode>` hacía que **añadir un campo a `VaultNode` fuera error de
compilación**. **Es falso, y medido:**

```
1. anadir `mutant_field: String` a VaultNode
2. actualizar parser.rs para satisfacerlo
3. cargo build -p sddk-vault   ->  EXIT 0    (PASA)
```

Un `From` entre dos tipos **distintos** no es exhaustivo por ningún lado, y
`NodeProjection` no es `VaultNode`: el compilador no tiene nada que exigir. **El
mismo doc de `GraphExport::from(&GraphView)` afirmaba lo mismo desde el ciclo
anterior**, así que la afirmación falsa llevaba dos commits viva y nadie la había
falsificado. Corregidos los dos docs, con la medición escrita al lado.

### 4.2 — El guard que sí existe, tampoco era el que se creía

R2 es el test que sujeta el invariante, y tuvo **dos versiones previas que no
medían lo que declaraban**:

1. **Repetía la lista de ocho campos como literal.** El defecto bajo prueba con
   otro sombrero: una lista escrita a mano de lo que tiene el struct, en el
   sitio cuyo trabajo es notar cuándo deja de cuadrar.
2. **Construía un `VaultNode { … }` literal** para serializarlo. Entonces, al
   mutar, el error era `missing field mutant_field` **en el fichero de test**, y
   **ninguna aserción llegaba a ejecutarse**. La mutación quedaba «detectada»
   por el motivo equivocado, y el mensaje que habría servido —el que nombra el
   campo y dice qué se transporta y qué se omite— no se imprimía nunca.

La sonda es ahora un nodo **parseado de un fixture real**, así que el test
**siempre compila** y es la aserción la que informa. Con la mutación activa:

```
`mutant_field` is a field of `VaultNode` that the export drops, and the
artifact does not mention it. ... Carried: ["id", "kind", "path", "status",
"tags", "title", "wikilinks"], omitted: ["body", "mutant_field"]

the export must DECLARE that it carries a projection of the node, not the whole
node. Today it carries 7 of 9 fields (...)
```

R2 nombra el campo; R1 recalcula el recuento. **Eso sí es un guard**, y se
obtuvo después de medir que el que había no lo era.

### 4.3 — El script de medición también

Su primera versión buscaba la palabra `omit` en el HTML. Eso acopla el guard a
una redacción: cualquier declaración honesta que no use esa palabra sale como
defecto, y cualquier texto decorativo que la contenga sale como correcto. **El
guard, no el producto**, era lo que estaba mal. Reescrito para medir la
sustancia —que cada campo omitido esté nombrado y que se declare un recuento—,
que es lo mismo que exige R2.

## §5 — La auditoría por criterio, y su reducción

La pregunta era *«¿qué más serializa un `GraphView` o una parte de él, y cada uno
declara lo mismo?»*:

| superficie | veredicto |
|---|---|
| `GraphView` → `vault graph` | cerrada en `cl-vault-graph` |
| `GraphView` → `GraphExport` | cerrada en `cl-vault-html-replica` |
| `VaultNode` → `export_node` | **este ciclo** |
| `sddk-domain::GraphView` | **descartado, por medición**: vista prestada y filtrada, sin `Serialize` |
| `ActiveGraphView` | **descartado, por lectura**: envuelve una proyección canónica y falla sin ella |

**5 candidatas, 1 defecto, 2 ya cerradas, 2 descartadas.** Los dos descartes
salieron por **homonimia**: `sddk-domain` tiene **otro** `GraphView`, y el nombre
compartido es exactamente lo que hace que una búsqueda por nombre los traiga.
Sexta vez en esta sesión que el número de candidatos se reduce al leerlos.

## §6 — Límites declarados

1. **Que `body` no viaje es una decisión, no una medición.** El ciclo la sostiene
   con el motivo escrito, y R4 la fija; si el criterio del inspector cambia, el
   guard es lo primero que hay que revisar.
2. **La tabla visible sigue sin columna `Tags`.** El dato **viaja** en el JSON
   incrustado y la página lo declara, pero la tabla no lo muestra. Es coherente
   con el no-objetivo 1 (no se toca el HTML visible) y **queda dicho** para que
   nadie lo lea como que `Tags` está en la tabla.
3. **La omisión de `body` se nombra en inglés** (`Not carried: body`) porque el
   artefacto está en inglés. El script de medición dejó de depender de esa
   palabra por eso mismo.

## §7 — UAT

No hay ejecución UAT: no hay superficie de usuario final ni página en
funcionamiento que recorrer —el cambio es sobre el JSON incrustado y una frase
de alcance—. **No se declara PASS de UAT.** No se ha tocado ninguna fila de
`docs/roadmap/UAT-MATRIX.md`.

---

## §8 — LOTE 2: el lote 1 se falsificó a sí mismo

El §4 cuenta tres correcciones. Esta es la cuarta, y la más grave, porque la
causó **el remedio de este mismo ciclo**.

### 8.1 — La segunda lista podía hacer mentir a la página

El lote 1 resolvió O1 con una constante `OMITTED_NODE_FIELDS` escrita a mano,
con el razonamiento de que una omisión **declarada** es mejor que una silenciosa.
Se añadió `"tags"` —un campo que la proyección **sí** transporta— y:

```
cargo test -p sddk-vault   ->  todos los tests verdes

la página dice:
  each node below carries 7 of 8 fields of VaultNode.
  Not carried: body, tags.
```

**Dos afirmaciones en la misma línea que no pueden ser ciertas a la vez**: 7 de 8
transportados más 2 omitidos son 9. Y la segunda es directamente **falsa** sobre
`tags`, que viaja en el JSON incrustado.

**Eso es peor que el silencio que sustituyó.** Una proyección incompleta se puede
completar; una página que afirma algo falso hay que dejar de creer. El lote 1
cambió un defecto por otro, y este último es más grave que el primero.

**Por qué ningún test lo cazó:** R2 compara la proyección contra el **tipo** y
nunca contra la **lista**. El hueco era exactamente la dirección que nadie
miró: la lista es la segunda fuente de verdad sobre el mismo hecho, y un test
que compara A contra B no dice nada de A contra C.

### 8.2 — La constante desaparece

La frase se **deriva**: se serializa un nodo dos veces —una por `NodeProjection`,
otra por `VaultNode`— y se nombra la diferencia. Es lo mismo que haría a mano
quien leyera las dos salidas, y **no queda ninguna entrada que alguien pueda
editar** para volver a decir algo falso. `body` sigue sin transportarse porque
`NodeProjection` no tiene el campo, no porque una lista lo diga.

### 8.3 — R5, y la prueba de que añade algo

R5 comprueba que la frase sea **aritméticamente cerrada y cierta**: el recuento
transportado, el total y el número nombrado tienen que cerrar, y nada nombrado
puede estar presente. Falsificado con **la misma mutación** que destapó el
defecto:

```
r5 ... FAILED
the page names 2 omitted fields, and the difference between what it claims to
exist ({...8 campos...} vs {...7 campos...}) is 1. Two numbers about the same set
that do not close is a sentence that cannot all be true.

r1 ... ok        <-- SEGUIA EN VERDE
```

Que **R1 siguiera en verde** con esa mutación es la prueba de que R5 aporta algo
que R1 no daba. Sin ese dato, «he añadido un test» no sería una afirmación.

### 8.4 — El guard más simple detectó lo que los tests no detectaron

`clippy -D warnings` falló con `constant OMITTED_NODE_FIELDS is never used`. La
constante had quedado muerta al derivarse el conjunto, y **`dead_code` es
exactamente la condición que una segunda fuente de verdad debe cumplir** para no
poder mentir: o se usa, o no existe. Ninguno de los cinco tests del ciclo vio el
problema; el lint que comprueba que no quede código muerto sí.

## §9 — Verificación del lote 2

| comando | resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5397 passed, 0 failed**, 281 binarios (baseline 5396, **+1**, R5) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `python3 ce/06-scan.py` | **CLEAN** |
| `test_changelog_coverage` | **PASS=58 FAIL=0** |
| `vault_node_projection` | 6 verdes; `vault_html_replica_declaration` 3; librería 27 — **ninguno reescrito** |
