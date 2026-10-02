# SCOPE-CONTRACT — cl-ledger-watch-total

**Cycle:** `p-63676b11dc0ef88f/ledger-watch-total` (real, `OPEN/explore`)
**Date:** 2026-10-02T21:12:36Z
**Origin:** la auditoría por criterio abierta al final de `cl-vault-node-projection`
—*«¿qué más declara el mismo hecho, y cada uno lo declara igual?»*— aplicada a
la lectura del ledger. F63 ya cerró `ledger events`; la misma pregunta sobre el
resto de superficies da dos respuestas, y una de ellas es un defecto medido.
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición

`/var/home/rubentxu/f63/03-medir-watch.py`, contra una **copia** del ledger real
(mtime verificado al final: el original no se toca).

```
`ledger watch --max-events 5` -> '[watch] emitted 5 events, exiting'
`ledger watch --max-events 5 --format json` -> {"__watch_complete":true,"emitted":5}
total real (ledger events) -> 591
GAP: `ledger watch` en TEXTO declara cuantos eventos existen en total  [5] vs 591
GAP: `ledger watch` en JSON declara cuantos eventos existen en total   None vs 591
```

## §1 — Qué es, y qué NO es

`ledger watch` **sí declara**, y es la superficie que la sesión-69f nombró «el
modelo del comportamiento correcto». La afirmación era cierta y aun así
llevó a la conclusión equivocada. Lo que falta es una sola cosa:

> **Declarar que ha emitido N no es declarar que había M.**

**Lo que NO se afirma.** Que `watch` deba emitir todo el ledger: `--max-events`
es una petición legítima y `--from-tail` pide explícitamente no ver el
histórico. **El defecto es no declarar que el tope dejó algo fuera**, no el tope.

## §2 — Auditoría por criterio, y su resultado

| superficie | veredicto |
|---|---|
| `ledger events` | **declara** — cerrado por F63 (`ledger.rs:248-256`) |
| `ledger watch` | **DEFECTO — este ciclo** (`ledger.rs:785-788`) |
| `ledger export` | **DEFECTO, misma clase** (`ledger.rs:512-525`), medido. **No-objetivo declarado**: un ciclo, una concernia (precedente 69h → 69i) |
| `cycle list` | **ya declara** (`cycle.rs:2064-2069`), con el motivo escrito |
| `telemetry status` | **declara** (`telemetry.rs:500`, `:554`) |

## §3 — Objetivo, falsable

1. **O1.** `ledger watch` **declara cuántos eventos existían** para esa
   consulta, en texto y en JSON. Un lector puede distinguir «se acabó» de
   «paré yo» sin contar líneas.
2. **O2.** `pending` es **derivado** (`total − emitted`), no un tercer número
   escrito a mano, y un test **comprueba que la aritmética cierra**. Es la
   lección del lote 2 de `cl-vault-node-projection`: una lista a mano que
   nadie derivaba era peor que el silencio que sustituyó.
3. **O3.** El total se cuenta con **el mismo predicado y el mismo cursor
   inicial** que usa el emisor. Un recuento paralelo —un `COUNT(*)` sin
   filtro, aunque sea 51,8× más barato— **no** puede pasar por declaración.
4. **O4.** La declaración aparece **también cuando no queda nada fuera**
   (`pending == 0`), como en R1 de F63: una declaración que solo aparece en el
   caso interesante no se puede leer de un log donde no ocurrió.

## §4 — No-objetivos

1. **NO** se cambia `ledger events`: ya declara, y sus tests siguen verdes.
2. **NO** se arregla `ledger export` aquí. Medido, declarado, y es otro ciclo.
   Su GAP **sigue abierto** y el gate de §7 lo nombra en vez de ocultarlo.
3. **NO** se cambia la emisión: ni el tope, ni el intervalo, ni los filtros,
   ni el tiempo de inactividad, ni el límite de seguridad de 5 minutos.
4. **NO** se añade API de storage ni se cambia la firma de
   `list_events_after`. La declaración sale de lo que el emisor ya llama.
5. **NO** se tocan `cycle list` ni `telemetry status`: ya declaran.
6. **NO** se escribe en el ledger real durante la medición.
7. **NO** se reescribe ningún test existente de `ledger watch`: el cambio es
   **aditivo**, y que sus aserciones sigan verdaderas es lo que lo demuestra.

## §5 — Superficie

| qué | dónde |
|---|---|
| args de `watch` | `crates/sddk-cli/src/ledger.rs:124-156` — `from_sequence`, `from_tail`, `frame`, `cycle`, `interval_ms`, `max_events`, `idle_timeout_ms` |
| bucle | `ledger.rs:715-775` — `list_events_after(after, 256)` + `retain` de ciclo y frame (`:727-732`) |
| ajuste de cursor inicial | `ledger.rs:707-713` — `--from-tail` carga `list_events()` para el último sequence |
| corte por tope | `ledger.rs:764-766` |
| pie y resumen | `ledger.rs:779-796` — texto `:788`, JSON `:785` |
| precedente ya aplicado | `ledger.rs:437-472` + `:651-657` — la forma de `ledger events` |
| el largo que ya se descarta | `sddk-storage/src/lib.rs:1022-1027` (`list_events_after` tira el largo), `:982-990` (`canonical_events` recorre todo cada llamada) |

## §6 — STOP conditions

1. **Si la declaración obliga a mantener una segunda cuenta** —un `COUNT`
   paralelo al filtro, una lista de campos, un contador aparte— **se para**.
   Es el defecto que este ciclo viene a cerrar, con otro nombre.
2. **Si el total filtrado no puede salir del mismo predicado que el emisor**
   (`cycle_id` / `frame_id`), **se para y se escribe por qué**. No se sustituye
   por el total sin filtro etiquetado: sería otra regla, y dos reglas que
   declaran el mismo hecho es la forma exacta del defecto.
3. **Si el coste medido de la declaración supera una iteración del bucle**, se
   para y se re-decide **con el número**, no con una intuición.
4. **Si hay que reescribir** un test existente de `ledger watch` en vez de
   ampliarlo, se para y se escribe por qué.
5. **Si declarar obliga a cambiar lo que `watch` emite** (orden, contenido,
   filtrado), se para: el objetivo es honestidad, no otra semántica.

## §7 — Gates al cerrar

- `cargo test --workspace` **sin reescribir ningún verde** salvo el declarado
- `cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings`
- `test_changelog_coverage` · `test_debt_index_coherence` ·
  `test_release_state_pointer`
- `03-medir-watch.py`: las **3** comprobaciones de `ledger watch` pasan
  (0/3 GAP). **El único GAP restante es `ledger export`**, que es no-objetivo
  declarado y queda nombrado en el recibo como ciclo siguiente. El script no
  puede salir en 0 mientras ese GAP siga abierto, y el recibo lo dice.
