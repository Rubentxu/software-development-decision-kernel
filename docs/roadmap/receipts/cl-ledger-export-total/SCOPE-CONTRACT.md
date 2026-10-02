# SCOPE-CONTRACT — cl-ledger-export-total

**Cycle:** `p-63676b11dc0ef88f/ledger-export-total` (real, `OPEN/explore`)
**Date:** 2026-10-02T22:35:24Z
**Origin:** INC-DEBT-062, que `cl-ledger-watch-total` dejó **medida y con la
superficie leída** en vez de cerrada
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición

```
`ledger export --limit 5` -> 'exported 5 events to <path>', exit 0
total real                  -> 600        (leido de `ledger events`)
numeros que declara export  -> [5]
`ledger export --format json` -> exit 2: unexpected argument
ExportOutput                -> deriva Serialize, el resumen NO se serializa
```

## §1 — Qué es, y qué NO es

`export` trunca en silencio: escribe 5 de 600 y no dice nada de las 595.

**Lo que NO se afirma:** que deba escribir todo el ledger. `--limit` es una
petición legítima y `--limit 0` significa «todos» desde siempre. **El defecto es
no declarar que el límite dejó algo fuera**, y aquí es **peor** que en las otras
dos superficies porque trunca **en un fichero**: un texto que miente se lee, un
artefacto que parece completo se consume.

## §2 — Objetivo, falsable

1. **O1.** El resumen de `export` **declara cuántos eventos existían** para esa
   consulta, en el formato de texto, incluso cuando no queda nada fuera.
2. **O2.** `export` tiene una **forma legible por máquina** que lleva la misma
   declaración, y `--format` deja de ser un argumento desconocido.
3. **O3.** La forma en vigor es **`ExportOutput`, que ya existe**: cablearla en
   vez de añadir un segundo `json!` al lado. Un `Serialize` que se declara y no
   se usa es una afirmación que el código no sostiene.
4. **O4.** El total se toma **antes** del `.take(limit)`, del mismo vector que
   contiene la consulta entera — y el filtro (`--cycle` / `--frame`) se aplica
   **antes** de contar, no después.
5. **O5.** El payload del fichero exportado **no cambia**: una línea JSONL por
   evento, en orden ascendente de secuencia.

## §3 — No-objetivos

1. **NO** se cambia el formato del fichero exportado (JSONL, una línea por
   evento, orden ascendente).
2. **NO** se cambia `--limit 0` (significa «todos»), ni `--cycle`, ni `--frame`,
   ni su orden de aplicación.
3. **NO** se tocan `ledger events` ni `ledger watch`: ya declaran, y sus tests
   deben seguir verdes sin reescribirlos.
4. **NO** se añade ninguna API de storage: el total sale de `all_events.len()`,
   que ya está en la mano antes del `take`.
5. **NO** se escribe ningún fichero fuera de un temporal en la verificación.
6. **NO** se repite la auditoría de superficies que truncan: está medida y
   agotada (EXPLORATION §4).

## §4 — Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `LedgerExportArgs` | `ledger.rs:90-105` | `cycle`, `frame`, `limit` (def. 1000, 0 = todos), `output`. **No tiene `--format`** |
| selección de la consulta | `ledger.rs:485-491` | `list_cycle_events` → `list_frame_events` → `list_events`: **el vector completo** |
| el corte | `ledger.rs:492-497` | `.take(limit)` sobre ese vector: **el largo se tira aquí** |
| la escritura | `ledger.rs:499-510` | una línea JSONL por evento; `count` cuenta lo escrito |
| el resumen | `ledger.rs:519-528` | `format!("exported {} events to {}", count, path)`: **solo lo escrito** |
| la forma declarada | `ledger.rs:537-542` | `ExportOutput` **deriva `Serialize` y nunca se serializa** |
| precedente A | `ledger.rs:437-472`, `:651-657` | `ledger events`, cerrado por F63 |
| precedente B | `ledger.rs:~700-880` | `ledger watch`, cerrado ayer con `LedgerWatchSummary` |

## §5 — STOP conditions

1. **Si declarar obliga a mantener una segunda cuenta** —un `count` aparte del
   vector, un campo copiado a mano— **se para**. Es el defecto que este ciclo
   viene a cerrar, y el lote 2 de `cl-vault-node-projection` ya lo pagó una vez:
   la lista a mano que nadie derivaba.
2. **Si la forma legible por máquina obliga a cambiar el payload del fichero**,
   se para: el objetivo es honestidad del resumen, no otro formato de export.
3. **Si `export` no puede declarar el total con el filtro aplicado** (porque el
   orden ciclo → frame → limit dejara el total mal definido), se para y se
   escribe por qué. No se sustituye por el total sin filtro etiquetado: dos
   reglas que declaran el mismo hecho es la forma exacta del defecto.
4. **Si hay que reescribir** un test existente de `ledger export` en vez de
   ampliarlo, se para y se escribe por qué.

## §6 — Gates al cerrar

`cargo test --workspace` **sin reescribir ningún verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`test_changelog_coverage` · `test_debt_index_coherence` ·
`test_release_state_pointer` · `09-medir-export.py` pasa de **2/6 GAP a 0** ·
INC-DEBT-062 con `status: resolved` y su cierre escrito.
