# SCOPE-CONTRACT — cl-vault-node-projection

**Cycle:** `p-63676b11dc0ef88f/vault-node-projection`
**Date:** 2026-10-03T00:35:00Z
**Origin:** el cierre de `cl-vault-html-replica` dejó escrito que la auditoría
llegó a la tercera superficie **desde un defecto concreto**, no desde la pregunta
*«¿qué más declara el mismo hecho?»*. Esta es esa pregunta, aplicada a la
superficie que quedó como no-objetivo y por tanto **sin medir**.
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición

`/var/home/rubentxu/vault/08-medir-nodes.py`, contra el binario real y un vault
con `tags` en el frontmatter.

```
nodos incrustados: 2
campos de VaultNode: 8 -> [id, kind, path, title, status, tags, body, wikilinks]
campos en la exportacion: 6 -> [id, kind, path, status, title, wikilinks]
OMITIDOS: ['tags', 'body']
columnas de la tabla: [Id, Kind, Title, Status, Links, Backlinks]
el artefacto declara que omite: False
tags en el vault de entrada:  ['alpha', 'beta']
tags en el nodo exportado:    AUSENTE
status en el nodo exportado:  active
VEREDICTO: DEFECTO — omite sin declarar
```

## §1 — Qué es, y qué NO es

`VaultNode` (`index.rs:61-78`) deriva `Serialize` y tiene **ocho** campos.
`export_node` (`export.rs:75-84`) es un `serde_json::json!` **escrito a mano** con
seis. Es **la misma forma que `GraphExport` tenía** antes de `cl-vault-html-replica`:
una lista de campos a mano sobre un tipo que ya sabe serializarse, sin acoplamiento
de tiempo de compilación. Añadir un campo a `VaultNode` no llega a la exportación
y nada lo dice.

**La asimetría que lo hace defecto y no decisión:** `status` viaja **y tiene columna
propia** en la tabla; `tags` no viaja y no tiene columna. Son el mismo tipo de
metadato de frontmatter, y no hay razón derivable de por qué uno sí y el otro no.

**Lo que NO se afirma aquí.** Que `body` falte es, casi con seguridad, correcto:
incrustar el cuerpo de cada documento en un HTML autocontenido lo multiplica, y
`body` no es un campo de inspector sino el documento entero. **Lo que se afirma es
una cosa más estrecha y más difícil de discutir: el artefacto no dice que omite
nada.** Una proyección silenciosa y una proyección declarada se distinguen por
poder responder «¿esto es todo lo que hay?», y hoy no se puede.

Por eso el defecto **no es «faltan campos»**. Es **«una lista de campos escrita a
mano que puede quedarse vieja sin que nada lo note, y que no declara su
alcance»**. La solución de `cl-vault-html-replica` ya se-midió y aplica igual.

## §2 — Auditoría por criterio, y su resultado

La pregunta era *«¿qué más serializa un `GraphView` o una parte de él, y cada uno
declara lo mismo?»*. El resultado, **medido y no supuesto**:

| superficie | veredicto |
|---|---|
| `GraphView` del vault → `vault graph` (JSON) | **cerrada** en `cl-vault-graph` |
| `GraphView` del vault → `GraphExport` (HTML) | **cerrada** en `cl-vault-html-replica` |
| `VaultNode` → `export_node` (`__vault_nodes__`) | **DEFECTO — este ciclo** |
| `GraphView` del **dominio** (`sddk-domain/src/graph.rs:266`) | **DESCARTADO, por medición**: es una vista **prestada y filtrada** (`state: &'a GraphState`, `visible_nodes: Vec<String>`), sin `Serialize` y sin proyección de datos. No es una declaración del mismo hecho, luego no es de esta clase. Se lo miró porque **comparte nombre** con el del vault, y esa homonimia es justo lo que hace que una búsqueda por nombre produzca candidatos que no lo son |
| `ActiveGraphView` (`sddk-engine/src/active_graph_view.rs:22`) | **DESCARTADO, por lectura**: envuelve una `SemanticGraphProjection` canónica y falla sin ella (`ViewError::NoCanonicalAuthority`). No declara datos propios |

**De 5 superficies candidatas, 1 es defecto, 2 ya estaban cerradas y 2 se
descartan.** La reducción va escrita porque «5 candidatas» es el número que
viaja a un documento y se convierte en trabajo que nadie necesitaba.

## §3 — Objetivo, falsable

1. **O1.** El artefacto **declara su alcance**: dice cuántos campos de
   `VaultNode` viaja y cuáles no, en el HTML. Un consumidor puede responder
   «¿esto es todo?» sin leer el código.
2. **O2.** La lista de campos de la exportación deja de estar escrita a mano sobre
   un tipo que deriva `Serialize`: si `VaultNode` gana un campo, la decisión de
   incluirlo u omitirlo es **explícita y decompilable**.
3. **O3.** La asimetría `status` sí / `tags` no queda **resuelta por una regla**,
   no por herencia: o los dos viajan, o los dos se omiten con el motivo escrito.
4. **O4.** `body` sigue sin viajar, y el motivo queda escrito en el sitio donde se
   decide, no en un documento de sesión.

## §4 — No-objetivos

1. **NO** se cambia el HTML visible: ni la tabla, ni sus columnas, ni los estilos.
   Lo que se toca es el **dato incrustado** y la **declaración de su alcance**.
2. **NO** se cambia `VaultNode`, ni el parser, ni el índice.
3. **NO** se toca `sddk-domain::GraphView` ni `ActiveGraphView`: medidos y
   descartados en §2, y tocarlos sería trabajo por analogía de nombre.
4. **NO** se escribe en ningún vault real.
5. **NO** se reescribe el test existente de `export.rs`
   (`renders_self_contained_inspector`): sus siete aserciones tienen que seguir
   verdaderas, y eso es lo que demuestra que el cambio es aditivo.

## §5 — Superficie

| qué | dónde |
|---|---|
| `VaultNode` | `crates/sddk-vault/src/index.rs:61-78` — 8 campos, deriva `Serialize` |
| `export_node` | `crates/sddk-vault/src/export.rs:75-84` — 6 campos, `json!` a mano |
| incrustación | `export.rs:22` y `:68` — `window.__vault_nodes__` |
| tabla visible | `export.rs:45` — 6 columnas, sin `Tags` |
| precedente ya aplicado | `export.rs:116-130` — `impl From<&GraphView>`, el mismo remédio para `GraphExport` |

## §6 — STOP conditions

1. **Si declarar el alcance obliga a incrustar el cuerpo de los documentos**, se
   para: eso es un cambio de tamaño del artefacto, no de honestidad, y no se
   negocia dentro de este lote.
2. **Si incluir `tags` rompe el test existente**, se para y se escribe por qué.
3. **Si el acoplamiento de compilación resulta imposible** —es decir, si no hay
   forma de que añadir un campo a `VaultNode` obligue a decidir—, se para: O2 sin
   acoplamiento es una nota, y una nota no es un guard.

## §7 — Gates al cerrar

`cargo test --workspace` sin reescribir ningún verde salvo el declarado ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`test_changelog_coverage` · `test_debt_index_coherence` ·
`test_release_state_pointer` · `08-medir-nodes.py` pasa de
**DEFECTO — omite sin declarar** a **correcto**.
