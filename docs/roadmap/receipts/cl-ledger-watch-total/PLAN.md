# PLAN — cl-ledger-watch-total

**Cycle:** `p-63676b11dc0ef88f/ledger-watch-total` (`OPEN/plan`)
**Date:** 2026-10-02T21:12:36Z
**Alcance:** lo único que este ciclo cierra es la declaración de `ledger watch`.

---

## Lote 1 — tests RED (`test(cli)`)

1. **Nuevo** `crates/sddk-cli/tests/ledger_watch_declaration.rs`, reutilizando el
   arnés de `crates/sddk-cli/tests/ledger_events_declaration.rs` (Sandbox,
   `env_remove("SDDK_DATA_DIR")`, `adopt apply` + `cycle start`).
   - R1 texto declara el total, incluso sin nada que dejar fuera
   - R2 JSON lleva `total_events` y `pending`
   - R3 `emitted + pending == total_events`
   - R4 con `--cycle X` el total es el de ese ciclo
   - R5 con `--from-sequence N` el total cuenta solo lo posterior
2. Ejecutar **solo** ese test: debe **caer**. Árbol rojo a propósito y declarado.

## Lote 2 — implementación (`fix(cli)`)

3. `crates/sddk-cli/src/ledger.rs`:
   - `fn apply_watch_filters(&mut Vec<LedgerEvent>, Option<&str>, Option<&str>)`
     — el filtro **una vez**, invocado desde el bucle y desde el recuento.
   - `struct LedgerWatchSummary { emitted, total_events }` con
     `fn pending()` derivado.
   - Capturar `initial_cursor` tras el ajuste de `--from-tail`.
   - Sustituir el `return` del corte por tope por bandera + `break 'poll`.
   - Recuento final con `list_events_after(initial_cursor, i64::MAX)` + filtro.
   - Texto y JSON pintados **del mismo struct**.
4. `cargo test -p sddk-cli --test ledger_watch_declaration` → verde.
5. `cargo test -p sddk-cli` → ningún verde reescrito.

## Lote 3 — falsificación (`test(cli)`)

6. `/var/home/rubentxu/f63/07-falsify-watch.py`, que **muta el producto** y exige
   que los tests caigan:
   - **M1** `pending` escrito a mano (valor constante) → R3 cae
   - **M2** total contado desde el cursor **final** en vez del inicial → R5 cae
   - **M3** total **sin filtro** (el `COUNT(*)` que se descartó) → R4 cae
   - **M4** declaración solo cuando `pending > 0` → R1 cae
   Cada mutación se aplica sobre una copia del repo, se ejecuta el test, y el
   repo se restaura. Un FAIL del falsificador se corrige **en el guard**.

## Lote 4 — cierre

7. `cargo test --workspace --no-fail-fast` (tope 5400 aprox) — **sin reescribir
   ningún verde**.
8. `cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings`
9. `bash tests/test_changelog_coverage.sh` · `test_debt_index_coherence` ·
   `test_release_state_pointer`
10. `python3 /var/home/rubentxu/f63/03-medir-watch.py` → **0/3** GAP en
    `ledger watch`. El GAP de `ledger export` **sigue abierto** y se nombra.
11. `python3 /var/home/rubentxu/ce/06-scan.py` sobre lo tocado.
12. `CHANGELOG.md` sección `## [2.5.3]`, recibo con SHA, comandos y resultados,
    y `docs/roadmap/CURRENT.md` + `STATE.yaml` + `SESSION-JOURNAL.md` **en la
    misma concernia** que el cierre.
13. `git push origin main` **sin `--no-verify`**.

## Lo que este plan NO hace

- No arregla `ledger export`: medido, es la misma clase, y es otro ciclo.
- No toca `ledger events`, ni su test, ni la firma de `list_events_after`.
- No bumpea la versión: workspace 2.5.3 sobre tag `v2.5.2` → la siguiente
  release **es 2.5.3**, y el hook lo admite por la ruta del tag.

## Stop

Un STOP del SCOPE-CONTRACT que se dispare **termina el ciclo** con el motivo
escrito. En particular STOP 1: si aparece una segunda cuenta, el arreglo se
descarta aunque los tests pasen.
