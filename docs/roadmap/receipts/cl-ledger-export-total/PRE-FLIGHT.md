# cl-ledger-export-total — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/ledger-export-total` (real, `OPEN/explore`)
**Date:** 2026-10-02T22:35:24Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `866699ec`, `HEAD == origin/main`, árbol limpio
**Authority:** INC-DEBT-062, registrada al cierre del ciclo anterior con la
superficie leída · SCOPE-CONTRACT y EXPLORATION-REPORT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | El defecto está **medido**, no supuesto | OK — `09-medir-export.py`: declara `[5]`, hay **600**; y `--format` da `exit 2` |
| 2 | Criterios de cierre falsables | OK — O1–O5; STOP 1 prohíbe la segunda cuenta |
| 3 | SCOPE con objetivo, no-objetivos y STOP | OK — 5 objetivos, 6 no-objetivos, 4 STOP |
| 4 | Superficie mapeada y **leída**, con línea | OK — §4 del SCOPE, ocho filas |
| 5 | Radio de impacto medido | OK — **1** función (`run_ledger_export`) y su resumen; el payload del fichero **no se toca**, luego es aditivo |
| 6 | Riesgo de datos | **cero por construcción**: la medición escribe en un temporal y el mtime del ledger original se comprueba |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `LedgerExportArgs` | `ledger.rs:90-105` | sin `--format`: **una máquina no tiene forma de leer el resumen** |
| la consulta entera | `ledger.rs:485-491` | `list_events` / `list_cycle_events` / `list_frame_events`: ya está **entera en la mano** |
| el corte | `ledger.rs:492-497` | `.take(limit)`: el largo se tira **aquí**, no antes |
| el resumen | `ledger.rs:519-528` | `exported N events to PATH`: solo lo escrito |
| `ExportOutput` | `ledger.rs:537-542` | **deriva `Serialize` y nunca se serializa**: la forma declarada no es la que está en vigor |

## Lo que la medición evitó

1. **Añadir un `json!` nuevo al lado de un struct dead.** `ExportOutput` ya
   deriva `Serialize`; la lectura lo separa de la forma que el comando *podría*
   tener y que no tiene. Añadir una segunda forma sería repetir, dentro del
   arreglo, el defecto que se viene a cerrar. **Por eso O3 dice "cablear la que
   existe", no "añadir una".**
2. **Confundir el payload con el resumen.** `serde_json::to_string(event)`
   aparece en el cuerpo de `run_ledger_export` y es **el JSONL que el comando
   debe escribir**. El detector que lo confunde con el payload dio `True` dos
   veces seguidas, y la tercera forma —buscar en el fichero entero— encontró el
   `render_result` de `ledger events`. **Setima vez que un detector mide lo que
   tiene al lado.**
3. **Repetir la auditoría de superficies que truncan.** Está medida y agotada
   (EXPLORATION §4). Repetirla sería trabajo por analogía.

## Lote de este apply

**Lote 1 — los tests RED y nada más.**

| # | test | por qué cae hoy |
|---|---|---|
| R1 | el texto **declara** el total, incluso sin nada que dejar fuera | el resumen solo dice lo escrito |
| R2 | `--format json` existe y lleva `total_events` y `pending` | el flag no existe: `exit 2` |
| R3 | `written + pending == total_events` | `pending` no existe; si se escribiera a mano, esto lo detecta |
| R4 | con `--cycle X` el total es el de **ese** ciclo | el guard que impide la cuenta sin filtro |
| R5 | el **payload** sigue siendo una línea JSONL por evento, en orden | caracterización: fija que el arreglo no cambia el fichero |

R5 es de caracterización y **pasa hoy**, y se declara como tal: llamarlo RED
sería falso, como el guard de `vault search`, el de `vault graph` y el R6 de
`ledger watch`.

## Cobertura objetivo → guard

| objetivo | guard | por qué ese guard puede fallar con el defecto presente |
|---|---|---|
| **O1** declara el total en texto | R1 | hoy el resumen dice solo `[5]` y no puede ver los 600 |
| **O2** forma legible por máquina | R2 | `--format` da `exit 2`: no hay forma de leer nada |
| **O3** la forma en vigor es `ExportOutput` | R2 | un `json!` paralelo dejaría el `Serialize` dead otra vez, y R2 no lo nota: **el guard de O3 es estructural, lee el fuente** |
| **O4** total antes del `take`, con filtro | R3, R4 | un `count` aparte del vector, o un total sin filtro, pasa R1 y R2 y **falla** R3 o R4 |
| **O5** el payload no cambia | R5 | un arreglo que cambiara el JSONL dejaría R5 en rojo |

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`test_changelog_coverage` · `test_debt_index_coherence` ·
`test_release_state_pointer` · `09-medir-export.py` de **2/6 GAP a 0** ·
INC-DEBT-062 con `status: resolved` y el motivo escrito.
