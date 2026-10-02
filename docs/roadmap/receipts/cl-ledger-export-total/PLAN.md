# PLAN — cl-ledger-export-total

**Cycle:** `p-63676b11dc0ef88f/ledger-export-total` (`OPEN/plan`)
**Date:** 2026-10-02T22:35:24Z
**Alcance:** la declaración de `ledger export`, y nada más

---

## Lote 1 — tests RED (`test(cli)`)

1. **Nuevo** `crates/sddk-cli/tests/ledger_export_declaration.rs`, reutilizando
   el arnés de `crates/sddk-cli/tests/ledger_events_declaration.rs` (Sandbox,
   `env_remove("SDDK_DATA_DIR")`, `adopt apply` + `cycle start`) y el patrón de
   `crates/sddk-cli/tests/ledger_watch_declaration.rs` (el total de referencia
   se **lee** de `ledger events`, no se escribe a mano).
   - R1 el texto declara el total, incluso sin nada que dejar fuera
   - R2 `--format json` existe y lleva `total_events` y `pending`
   - R3 `written + pending == total_events`
   - R4 con `--cycle X` el total es el de ese ciclo
   - R5 el payload sigue siendo una línea JSONL por evento, en orden
     ascendente (**caracterización**, y se declara como tal)
2. Ejecutar **solo** ese test: debe **caer**. Árbol rojo a propósito y declarado.

## Lote 2 — implementación (`fix(cli)`)

3. `crates/sddk-cli/src/ledger.rs`:
   - `LedgerExportArgs` gana `format: OutputFormat` con default `text`
   - `ExportOutput`: `count` → `written`, más `total_events`, más
     `fn pending()` derivado
   - `total_events = all_events.len()` **antes** del `.take(limit)`
   - `fn export_text(&ExportOutput) -> String` para texto, y
     `serde_json::to_string(&ExportOutput)` para JSON: **los dos leen el mismo
     struct**
4. `cargo test -p sddk-cli --test ledger_export_declaration` → verde.
5. `cargo test -p sddk-cli` → ningún verde reescrito.
6. **Guard estructural de O3**: un test que lee el fuente y exige que la forma
   JSON la produzca `ExportOutput` y no un `json!` paralelo. Sin él, O3 no tiene
   guard — un `json!` nuevo dejaría el `Serialize` dead otra vez y **todos** los
   tests en verde.

## Lote 3 — falsificación (`test(cli)`)

7. **Instrumento nuevo, que este plan escribe en el lote 3:**
   `/var/home/rubentxu/f63/10-falsify-export.py`, que **muta el producto** y
   exige que los tests caigan:
   - **M1** `total_events` escrito a mano (constante) → R1 y R2 caen
   - **M2** `pending` escrito a mano (constante) → R3 cae
   - **M3** total **sin filtro** (contado sobre el vector sin filtrar) → R4 cae
   - **M4** declarar solo cuando `pending > 0` → R1 cae
   - **M5** el payload pasa a un array JSON en vez de JSONL → R5 cae
   Cada mutación se aplica sobre el fichero real, se ejecuta el test, y el
   fichero se restaura **por bytes** — no con `git checkout`, que repone el
   último commit y borra trabajo sin commitear. El falsificador del ciclo
   anterior se encontró ese defecto **perdiendo** un arreglo a medio hacer.

## Lote 4 — cierre

8. `cargo test --workspace --no-fail-fast` (tope 5410) — **sin reescribir
   ningún verde**.
9. `cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings`
10. `bash tests/test_changelog_coverage.sh` · `test_debt_index_coherence` ·
    `test_release_state_pointer`
11. `python3 /var/home/rubentxu/f63/09-medir-export.py` → **0/6 GAP**
12. INC-DEBT-062 con `status: resolved` y el motivo escrito, más su entrada en
    el changelog; recibo con SHA, comandos y resultados; `CURRENT.md`,
    `STATE.yaml` y `SESSION-JOURNAL.md` **en la misma concernia**.
13. `git push origin main` **sin `--no-verify`**.

## Lo que este plan NO hace

- No arregla `ledger watch`: cerrado y verificado en el ciclo anterior.
- No cambia el payload del fichero exportado, ni `--limit 0`, ni los filtros.
- No repite la auditoría de superficies que truncan: medida y agotada.
- No bumpea la versión: workspace 2.5.3 sobre tag `v2.5.2` → la siguiente
  release **es 2.5.3**.

## Stop

Un STOP del SCOPE-CONTRACT que se dispare **termina el ciclo** con el motivo
escrito. STOP 1 en particular: si aparece una segunda cuenta —un `count` aparte
del vector, un campo copiado a mano— el arreglo se descarta aunque los tests
pasen, porque un guard que no puede fallar no es un guard.
