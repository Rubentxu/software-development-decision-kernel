# VERIFICATION-REPORT — `cl-build-identity`

Ciclo `p-63676b11dc0ef88f/cl-build-identity`, fase `verify`, path `B-direct`.
Transición de cierre: `phase.verify.complete.b-direct`.

Toda la evidencia de este informe se **re-ejecutó en esta sesión** sobre el
`HEAD` que nombra el puntero. Nada se hereda de la fase `build`: una cifra de
tests heredada no es evidencia de nada, es una promesa de que hubo tests.

---

## 1. Qué se verifica

Que `sddk dev build-id` diga **qué commit es este binario**, con la procedencia
del dato, y que `--check` sirva para **decidir** si el binario corresponde al
checkout — no solo para imprimir una relación.

La razón de ser está medida y es el motivo de que el gate sea binario: sin él,
`sddk 2.5.3` y `sddk 2.5.3` son indistinguibles aunque uno lleve 1,24 días de
código distinto por debajo (INC-DEBT-064).

## 2. El comportamiento observable, medido con dos binarios reales

El criterio que separa un detector útil de uno decorativo es si **distingue**,
no si responde. Se mide con el **mismo binario** contra dos checkouts distintos:

| Escenario | Checkout | Salida | Exit | Lo que demuestra |
|---|---|---|---|---|
| binario al día | `33221181` (HEAD) | `relation: Matches` | **0** | el gate pasa cuando corresponde |
| el mismo binario, checkout atrasado | `c30dcf89` (HEAD~1) | sin conclusionar | **1** | el gate **falla**, porque el binario tiene trabajo que el checkout no tiene |
| binario atrasado | HEAD más nuevo que el binario | `relation: Behind` | **1** | detecta el caso que motiva INC-DEBT-064 |
| binario sin identidad | cualquiera | `relation: unknown` | **1** | no pasa cuando no sabe |

**La fila 2 es la que faltaba y la que se corrigió.** Con el código de
`11c8e1d9` y anteriores, ese mismo escenario devolvía **exit 0** mientras su
propia línea de `reason` decía «el checkout tiene trabajo que el binario no
contiene». El texto y el código de salida diciéndose lo contrario en la misma
pantalla.

## 3. `tests-pass`

Evidencia del gate: **un** comando, con `argv`, `exit_code` y `output_digest`,
que es la forma que exige REQ-IPV (spec-v2, cycle-44). Un lote anidado bajo
`commands` se rechaza con `ENGINE_INVALID_PASS_EVIDENCE`; se comprobó.

- `cargo test --workspace --no-fail-fast` → **exit 0**
  — 5428 passed / 0 failed / 24 ignored / 283 binarios.

Lote acotado al SUT, con digest individual por comando
(`CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets`):

| Comando | Exit |
|---|---|
| `cargo test -p sddk-cli --lib dev::build_id` (11 guards) | 0 |
| `cargo test -p sddk-cli --test cli build_id` (3 e2e) | 0 |
| el mismo e2e con `SDDK_GIT_SHA` clavado a un commit que **no** es el HEAD | 0 |
| `cargo fmt --check` | 0 |
| `cargo clippy -p sddk-cli --all-targets -- -D warnings` | 0 |
| `bash tests/test_changelog_coverage.sh` | 0 — PASS=72 FAIL=0 |
| `bash tests/test_debt_index_coherence.sh` | 0 — PASS=12 FAIL=0 |
| `bash tests/test_release_build_identity.sh` | 0 — PASS=22 FAIL=0 |
| scanner de redacción sobre lo añadido | 0 — CLEAN |

**El tercer comando no es redundante.** El guard e2e de `--check` es **ciego
por defecto**: sin `SDDK_GIT_SHA` el binario de test se construye por el
fallback `.git`, que STOP 6 declara no concluyente, luego la relación es
siempre `unknown` y la rama `behind` —la única que hacía pasar al defecto— no se
alcanza nunca. Se comprobó que **pasaba con el defecto puesto**. Con el SHA
clavado a un commit que no es el HEAD, la relación llega a ser `behind` y el
guard sí cae. Sin ese tercer comando, la evidencia del lote daría una cobertura
que no existe.

## 4. `policy-compliant`

Evidencia: `bash tests/test_build_identity_policy.sh` → **exit 0, PASS=8
FAIL=0**. El instrumento existe para que este gate no lo afirme un agente: cada
comprobación ejecuta y devuelve código de salida.

| Política | Cómo se comprueba | Resultado |
|---|---|---|
| STOP 2 — `--version` intacto | guard e2e `r1_version_last_field_is_a_bare_semver` | ok |
| STOP 4 — bundle intacto | `MANIFEST.sha256` sin cambios en el rango | ok |
| STOP 6 — el fallback no decide | guards `r6_the_git_fallback_never_concludes` y `r6_unknown_is_never_reported_as_coming_from_git` | ok |
| redacción | scanner sobre **las líneas añadidas** del rango, no los ficheros enteros | CLEAN, 13 ficheros con delta |
| shell | `shellcheck` sobre el shell del rango | sin avisos |
| gates documentales | changelog e índice de deuda | PASS |

**Dos cosas que el instrumento NO midió, declaradas en vez de dadas por buenas:**

- **La regla de cero intrusión (AGENTS.md §1).** Es cierto por construcción —
  solo se han tocado ficheros bajo el CWD — pero el script no lo demuestra y no
  se presenta como si lo hiciera.
- **El umbral de ficheros sin seguimiento de `release.sh`.** Es más estricto
  que el preflight solo para lo que entra en el binario. Es una decisión de
  contrato del gate de release y es del operador.

**Y una corrección de honestidad del propio informe:** la primera versión de
este instrumento fallaba por dos motivos que eran suyos, no del producto. Un
`cargo test | grep -q` con `pipefail` devolvía error porque `grep -q` sale al
primer match y `cargo test` recibe SIGPIPE — el guard se ponía verde por el
motivo equivocado en cuanto la salida crecía. Y el escáner de redacción pasabas
los ficheros enteros, con lo que reportaba los 27 caracteres no latinos
**históricos** de `SESSION-JOURNAL.md` —deuda declarada que este cambio no
introduce— en vez de medir el delta. **Medir el cambio y medir la historia no
es lo mismo**, y es el mismo error que el del `case` gloton de la biseca 5:
mirar el fichero entero cuando lo que se pregunta es por la diferencia.

## 5. Falsificación, que es lo que da valor a lo anterior

Un guard que no se ha visto fallar no es evidencia. Los doce guards del módulo
se mutaron con `23-falsify-build-identity.py` — 6 mutaciones, 6 detectadas, 0 no
medibles—, y en esa pasada **M4 y M5 sobrevivieron**: eran guards que solo
fijaban el caso donde el defecto no se manifiesta. Se corrigieron en el guard.

El defecto mayor lo encontró el producto en uso, no una mutación: se construyó
un binario declarando un commit que no era el HEAD y se miró qué salía. **Un
falsificador que solo muta la implementación no ve los defectos de la interfaz
observable.**

| Guard | Cómo se vio caer |
|---|---|
| `r4_only_matches_makes_a_check_pass` | reintroduciendo `Behind` en el conjunto que pasa |
| `build_id_check_exit_code_agrees_with_the_relation_it_prints` | con `SDDK_GIT_SHA=HEAD~1` |
| predicado de identidad de `release.sh` | M1 broaden, M2 eliminación del bloque |
| filtro de fuentes sin seguimiento | M3 broaden del `grep -E` |

Las tres mutaciones de `release.sh` se aplican al **source real**, no a una
copia: la primera versión del guard llevaba una copia del predicado, con lo que
M1 no tenía nada que detectar. Y un guard que se ancla a la primera coincidencia
mide algo distinto de lo que cree — `release.sh` tiene cinco `grep -E` y la
extracción sin ancla se llevó el de la línea 418.

## 6. Lo que esta verify NO declara

- **INC-DEBT-064 sigue `open`**, y el changelog ahora dice «avanza» en lugar de
  «cierra». La condición sigue viva: el binario del PATH va atras hasta que haya
  release, bloqueada por la clave KMS.
- **`dev doctor` no invoca `build-id --check`.** El detector existe y funciona,
  pero la detección depende de que alguien la pida. Cablearlo al doctor es lo que
  convertiría el detector en **mecanismo** en vez de en dato, y es la lección de
  INC-DEBT-061 aplicada donde toca: *«el arreglo no está roto, no está
  desplegado»* como mecanismo y no como dato de un caso. Es el siguiente paso
  declarado y no forma parte de esta verify.
- **La ruta forge contra un GitHub real**: `NOT_RUN`. Tres escrituras
  privilegiadas sobre un repositorio ajeno (AGENTS.md §1).
- **Ninguna matriz UAT nueva se ejecutó en esta sesión.** Esta verify es la que
  debería producirla y no la ha producido todavía; lo que hay aquí son gates y
  mediciones de comportamiento, no una matriz UAT.

## 7. Corrección de un dato que los punteros tenían mal

Los punteros de la biseca 5 afirman que readquirir el lease incrementa el
`fencing_token` «ya va por 2». **Medido: es 1.** El incremento ocurre al
*reemplazar* un lease caducado, pero la transición de fase lo **liberó** —
borró la fila— y un lease ausente arranca en 1. La afirmación general es
cierta; el número concreto era falso, y se corrige aquí y en los punteros.
