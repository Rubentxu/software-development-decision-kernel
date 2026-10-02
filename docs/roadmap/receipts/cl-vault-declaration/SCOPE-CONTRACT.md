# SCOPE-CONTRACT — cl-vault-declaration

**Cycle:** `p-63676b11dc0ef88f/vault-declaration`
**Date:** 2026-10-02T21:05:00Z
**Authority:** misma clase que F63, medido en session-69f y escrito en
[SCOPE-CONTRACT §2.3-ter](../cl-ledger-declaration/SCOPE-CONTRACT.md)
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición

`/var/home/rubentxu/vault/01-medir.py`, contra el índice real de esta máquina y
contra índices sintéticos de 750 y 7500 documentos.

```
sddk vault search --db vault-index.sqlite --query cycle   -> 20 líneas, exit 0
SELECT COUNT(*) FROM vault_fts                            -> 75 documentos
sddk vault search ... --limit 0                            -> "no hits"
sddk vault search ... --format json                        -> [ ... ]  array desnudo
```

Las tres cosas de F63, una a una: tope por defecto sin declaración, `0`
significando cero, y un array JSON sin dónde llevar el total.

## §1 — Objetivo, falsable

1. **O1.** La salida declara cuántos documentos casan, cuántos se muestran y si
   trunca, **siempre**, también cuando no trunca.
2. **O2.** El JSON lleva `total_hits`, `shown` y `truncated`.
3. **O3.** `--limit 0` significa todos, como en `ledger export`, `ledger events` y
   `ledger watch`.
4. **O4.** Sin coincidencias declara cero, no imprime un `no hits` que se lee
   como «no hay nada» cuando lo que debe decir es «no se dejó nada fuera».

## §2 — No-objetivos

1. **NO** se cambia la firma de `sddk_vault::search_index`. Es API pública del
   crate y la usan 8 tests unitarios en `crates/sddk-vault/src/search.rs`. El
   total se obtiene con una función **nueva**, sin cambiar la existente.
2. **NO** se toca el índice FTS, su esquema ni `rebuild_search_index`. El lote
   cambia cómo se **presenta** el resultado, no cómo se busca.
3. **NO** se toca `vault graph`, `vault show`, `vault index` ni `vault validate`.
4. **NO** se escribe en ningún índice real. Los tests construyen el suyo en un
   temporal.
5. **NO** se cambia `--limit` por defecto (20). Que 20 sea razonable es criterio;
   que no se diga cuántos hay es mentira. El primero es discutible, el segundo no.

## §3 — La decisión de diseño, medida antes de tomarla

Aquí el total **no** viene gratis, y eso separa este lote del anterior:
`search_index` corta en SQL (`LIMIT ?2`, `crates/sddk-vault/src/search.rs:174`),
así que el total no está en memoria como en `ledger events`. Declararlo exige una
segunda consulta.

La alternativa barata era pedir `LIMIT n+1` y deducir «hay al menos uno más»
cuando salieran `n+1` filas. Se descartó **midiendo**, no por gusto:

```
índice real (75 docs)      búsqueda 0,376 ms   COUNT 0,119 ms   +31,7 %
índice sintético (750)     búsqueda 0,814 ms   COUNT 0,064 ms    +7,8 %
índice sintético (7500)    búsqueda 8,129 ms   COUNT 0,424 ms    +5,2 %
```

**El COUNT es más barato que la propia búsqueda, y el sobrecoste baja con la
escala.** La razón está medida, no supuesta: `ORDER BY rank LIMIT 20` tiene que
ordenar todos los matchs, mientras que `COUNT(*) … WHERE MATCH` solo los recorre.
El 31,7 % del índice real es coste fijo dominando sobre una consulta de 0,1 ms.

Por eso se paga el total **exacto** y no un «al menos N»: el dato completo sale
más barato que el parcial, y un total que a veces es exacto y a veces no obliga a
quien lo lee a descubrir cuál de los dos tiene.

## §4 — Radio de impacto, medido antes de cambiar nada

La vez anterior este recuento salió mal y rompió el build. Esta vez:

```
crates/sddk-vault/src/search.rs  -> 8 tests usan search_index() directamente
    NO se tocan: la firma no cambia
consumidores de `search_index` fuera del crate  : 0
tests de CLI del subcomando `vault search`      : 1
    crates/sddk-cli/tests/cli.rs:8541
        assert_eq!(hits.as_array().unwrap().len(), 1);
        assert_eq!(hits[0]["id"], "TERM-Auth");
```

**Uno**, y la diferencia con `ledger events` es que aquí la única pieza que se
rompe es la que el arreglo decide cambiar: una envoltura por un array. La
aserción que significa algo —«el único hit es TERM-Auth»— sobrevive igual que
sobrevivió allí.

Búsqueda exhaustiva sobre `--include=*.rs,*.sh,*.py,*.yml,*.kts,*.md,*.json,*.toml`
en todo el repo: `skills/`, `prompts/`, `agents/`, `scripts/`, `.github/` y
`tests/` no invocan `vault search`.

## §5 — STOP conditions

1. **Si hay que cambiar la firma de `search_index`**, se para. Su alternativa es
   añadir, y añadir no rompe a nadie.
2. **Si algún test verde hay que reescribirlo**, se para y se escribe por qué. La
   única excepción prevista es `cli.rs:8541`, y su aserción de fondo no se mueve.
3. **Si el sobrecoste del COUNT resultara ser material**, se para: la medición de
   §3 dice que no lo es, y si cambia, se vuelve a medir antes de seguir.
4. **Si el total declarado no cuadra con `COUNT(*)` sobre el índice real**, se
   para y se investiga antes de ajustar nada.
5. **Si trunca y no lo dice**, se para, aunque el resto sea correcto.

## §6 — Riesgo

1. **Compatibilidad del JSON.** El riesgo real, y está medido en §4: un
   consumidor en el repo, ninguno fuera del repo conocido.
2. **Coste**: el de §3, medido, ~5 % a escala realista.
3. **Riesgo de alcance**: `vault graph` y `vault show` también proyectan datos y
   podrían tener la misma clase de problema. §2.3 los excluye y por eso es STOP de
   alcance, no de comportamiento. **No se afirma que estén bien: se afirma que
   no se han medido**, y esa diferencia es la que evita repetir el error de
   `ledger watch`.
4. **Lectura, no escritura** sobre cualquier índice real.

## §7 — Gates al cerrar

`cargo test --workspace` sin reescribir ningún verde salvo `cli.rs:8541` ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`check_debt_index_coherence` · `test_changelog_coverage` ·
`test_adr_promotion_format` · `test_docs_script_contamination` ·
`test_gate_coverage` · `test_release_state_pointer` · falsificador propio que
compruebe O1–O4 contra un índice real, en solo lectura, declarando su recuento.
