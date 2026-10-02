# VERIFICATION-REPORT — cl-ledger-export-total

**Cycle:** `p-63676b11dc0ef88f/ledger-export-total` (`OPEN/verify`)
**Subject SHA:** `04e129f7`
**Workspace:** 2.5.3 declarada, no publicada (último tag remoto `v2.5.2`)
**Date:** 2026-10-03

---

## §1 — Qué se afirma, y qué no

**Se afirma:** que `ledger export` declara cuántos eventos existían para su
consulta —filtro de `--cycle` / `--frame` incluido— además de cuántos escribió, y
que esa declaración existe en una forma que una máquina puede leer, producida por
el mismo struct que la forma de texto.

**No se afirma:** que `ledger export` deba escribir el ledger entero. `--limit` es
una petición legítima y `--limit 0` significa «todos» desde siempre. Lo que se
verifica es que el comando **no presente un fichero truncado como si estuviera
completo**, y que no invente un total que no corresponda a la consulta.

## §2 — Verificación de que el defecto estaba y de que ya no

El mismo instrumento (`09-medir-export.py`), antes y después, sobre una copia
byte-identica del ledger real:

| | antes (`f47db330`) | después (`04e129f7`) |
|---|---|---|
| texto | `exported 5 events to /…/export.jsonl` | `exported 5 of 608 events (603 not written) to /…/export.jsonl` |
| JSON | `--format json` → **exit 2**, unexpected argument | `{"path":"/…","written":5,"total_events":608,"pending":603}` |
| resumen serializado desde `ExportOutput` | **no** | **sí** |
| GAP en `ledger export` | **2 de 6** | **0 de 6** |

El total se lee de `ledger events`, la superficie hermana que ya declara: **los
tests y el instrumento comparan dos comandos entre sí**, no contra un literal.
El `mtime` de la copia del ledger se comprueba antes y después: la medición no
escribe en el ledger real.

## §3 — Falsificación: 5 mutaciones, 5 detectadas

`10-falsify-export.py` muta el producto en la dirección de cada modo de fallo y
exige que caigan los guards:

| Mutación | Cae |
|---|---|
| M1 `total_events` escrito a mano (constante) | **R1, R2** (+R3) |
| M2 `pending` escrito a mano (constante) | R3 |
| M3 total **sin filtro**, contado sobre todo el ledger | R4 |
| M4 declarar solo cuando `pending > 0` | R1 |
| M5 el payload pasa a un array JSON en vez de JSONL | R5 |

M3 es la que más importa: es la solución que se **descartó** en el ciclo anterior
para `ledger watch` (`COUNT(*)` sin filtro, 51,8× más barato y falso con
`--cycle`), y aquí el guard la detiene. R4 es el guard que le da nombre al ciclo.

**La primera pasada de este falsificador salió mal, y el defecto estaba en el
instrumento y en el guard, no en el producto** — ver §5.

## §4 — Perfil completo

| Comprobación | Resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5409 passed / 0 failed / 24 ignored / 283 binarios** |
| `cargo test -p sddk-cli` | **1456 passed / 0 failed** |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `tests/test_changelog_coverage.sh` | **PASS=66 FAIL=0** |
| `tests/test_debt_index_coherence.sh` | **PASS=12 FAIL=0** |

La aritmética cierra sola: base 5403 / 24 / 282, más un binario nuevo
(`ledger_export_declaration`) con 6 tests → 5409 / 24 / 283. **Ningún verde fue
reescrito**, ni del workspace ni de `sddk-cli`.

## §5 — Los tres defectos que la falsificación encontró, y dónde estaban

Ninguno estaba en el producto. Se corrigen **en el guard** y **en el
instrumento**, que es donde la regla dice que se corrigen.

### 5.1 — Un guard que no podía fallar

La primera pasada cerró con `MUTACIONES NO DETECTADAS: 1` para M1, porque R2 no
cayó —cuando **R1 y R3 la detectaron**. Medido antes de tocar nada con
`11-medir-fixture-r2.py`: **1 ciclo deja 1 evento, 2 dejan 2, 3 dejan 3**. El
fixture de R2 era de un solo ciclo, luego su total real era exactamente 1 y la
constante `1usize` **coincidía con la verdad**. La aserción no podía caer nunca.

Corregido en el guard: el fixture abre dos ciclos y afirma `total > 1`.

### 5.2 — El instrumento confundía dos cosas opuestas

`10-falsify-export.py` llamaba *"mutación no detectada"* a lo que era *"este guard
no disparó donde el propio instrumento esperaba"*. Son cosas con consecuencias
opuestas —la primera es defecto del producto, la segunda es la matriz del
falsificador mal escrita— y salían bajo el mismo encabezado y con el mismo
código de salida. Tres desenlaces ahora: `SOBREVIVIDA`, `DETECTADA`, `DERIVA`.

### 5.3 — La sonda de medición mentía tres veces, y se reparó por la razón equivocada

`09-medir-export.py` reportaba `GAP` en una línea que decía
`deriva Serialize=True, el RESUMEN se serializa=True`. Medido con
`12-medir-sonda.py`: **polaridad invertida** (no podía decir OK nunca), **leía
prosa** —encontró el literal `#[derive(Serialize)]` en la línea 594, dentro del
doc comment que explica por qué no se usa— y **anclaba un detalle de
implementación** que el arreglo abandonó a propósito.

La última es la que importa a futuro: una comprobación que falla cuando la
implementación mejora está midiendo el mecanismo, no la propiedad.

### 5.4 — Y la sonda reparada se falsificó antes de creérsela

Corregir un detector y verlo decir «OK» no prueba nada. `13-falsify-sonda.py`
mete el defecto de vuelta en el fuente y exige que la sonda vuelva a reportarlo:
**3 de 3**, incluida la que comprueba que un doc que cita el derive **no** cuenta
como capacidad.

## §6 — Deuda

**INC-DEBT-062** pasa a `resolved`. Severidad **high**, prioridad **P1**, y no
`critical` por el mismo razonamiento que bajó a `high` a INC-DEBT-060: **no hay
pérdida de datos**. El ledger está íntegro y el rodeo —`ledger events`, que ya
declara— existe. El daño era de visibilidad por el producto.

## §7 — Lo que NO se verificó, y se declara

- **No se verificó la instalación.** La release 2.5.3 no se ha construido ni
  publicado: sigue bloqueada por la **clave KMS**, que es una decisión del
  operador. Ningún resultado de este informe depende de la release.
- **No se verificó el bundle ni el manifest**, porque no se tocó ninguna
  superficie del bundle (`agents/`, `skills/`, `prompts/sddk/`, `assets/`,
  `specs/`, `docs/impeccable-reference/`). `MANIFEST.sha256` no se regeneró y no
  necesitaba regenerarse.
- **No se volvió a auditar la familia de superficies que truncan.** Está medida y
  agotada; repetirla produciría candidatos, no hallazgos.
- **No se ejecutó CI en la nube**, por la regla del repo: el gate es local y el
  cloud no bloquea.

## §8 — Contexto

**Real, no simulado.** El ledger que se mide es una copia byte-identica del de
`p-63676b11dc0ef88f`. Los fixtures de los tests corren contra el binario real en
un directorio temporal con `SDDK_DATA_DIR` y `SDDK_STATE_HOME` eliminados del
entorno, que es lo que impide que un test escriba en el ledger de verdad.
