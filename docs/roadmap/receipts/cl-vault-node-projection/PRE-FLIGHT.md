# cl-vault-node-projection — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/vault-node-projection`
**Date:** 2026-10-03T00:40:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `4822ddd1` (`docs(roadmap): session-69j, la regla del operador aplicada al unico critical abierto`), `HEAD == origin/main`, árbol limpio
**Authority:** el no-objetivo que `cl-vault-html-replica` dejó explícitamente
**sin medir** · SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | El defecto está **medido**, no supuesto | OK — `08-medir-nodes.py`: 6 de 8 campos, 0 declaración |
| 2 | Criterios de cierre falsables | OK — O1–O4; STOP 3 exige acoplamiento real o se para |
| 3 | SCOPE con objetivo, no-objetivos y STOP | OK — 4 objetivos, 5 no-objetivos, 3 STOP |
| 4 | Superficie mapeada y **leída**, con línea | OK — §5 del SCOPE |
| 5 | Radio de impacto medido | OK — **1** consumidor, y el cambio es **aditivo** sobre el JSON incrustado |
| 6 | Riesgo de datos | **cero por construcción**: no se escribe en ningún vault real |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `VaultNode` | `crates/sddk-vault/src/index.rs:61-78` | **8 campos**, deriva `Serialize`: ya sabe su propia forma |
| `export_node` | `export.rs:75-84` | **`json!` a mano con 6 campos**: la forma exacta que `GraphExport` tenía |
| incrustación | `export.rs:22`, `:68` | `window.__vault_nodes__`, lo que el falsificador extrae |
| tabla visible | `export.rs:45` | 6 columnas; `Status` sí, `Tags` no |
| remedio ya probado | `export.rs:116-130` | `impl From<&GraphView>`: el mismo camino para el grafo |

## Lo que la medición evitó

1. **Confundir «omite» con «omite mal».** `body` casi con seguridad no debe
   viajar: es el documento entero y el HTML es autocontenido. La afirmación
   defendible **no es que falten campos**, es que **nada declara el alcance**.
   Un lote que hubiera>array adding campos habría hecho el artefacto más grande
   sin tocar el defecto.
2. **Contar candidatos por homonimia.** `sddk-domain` tiene **otro** `GraphView`.
   Buscar «GraphView» lo trae, y trae también `ActiveGraphView`. Leídos: uno es
   una vista **prestada y filtrada** sin `Serialize`, el otro envuelve una
   proyección canónica y **falla sin ella**. **Ninguno de los dos declara datos
   propios**, luego no son de esta clase. Es la sexta vez que el número de
   candidatos se reduce al leerlos, y por eso la reducción va escrita.

## Lote de este apply

**Lote 1 — los tests RED y nada más.**

| # | test | por qué cae hoy |
|---|---|---|
| R1 | el artefacto **declara** cuántos campos viaja y cuáles no | no hay ninguna declaración |
| R2 | la lista de campos no puede **quedarse vieja**: añadir un campo a `VaultNode` rompe la compilación o falla el test | hoy es un `json!` a mano, sin acoplamiento |
| R3 | `status` y `tags` viajan **o se omiten los dos**, por una regla | hoy `status` viaja y `tags` no, sin razón derivable |
| R4 | `body` **no** viaja | caracterización: fija que el arreglo no convierte el HTML en el vault entero |

R4 es de caracterización y **pasa hoy**, y se declara como tal: llamarlo RED sería
falso, como el guard de `vault search` y el de `vault graph`.

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`test_changelog_coverage` · `test_debt_index_coherence` ·
`test_release_state_pointer` · `08-medir-nodes.py` pasa de
**DEFECTO — omite sin declarar** a **correcto**.
