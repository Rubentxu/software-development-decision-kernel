# RECEIPT — cl-build-identity

**Cycle:** `p-63676b11dc0ef88f/cl-build-identity`
**WorkItem:** remedio de **INC-DEBT-064** (high/P1, open)
**Date:** 2026-10-03

---

## Qué entrega

`sddk dev build-id` dice **qué commit es el binario y de dónde salió ese dato**, y
`--check` lo compara con un checkout nombrando la relación. Es el remedio de
INC-DEBT-064: `sddk 2.5.3` y `sddk 2.5.3` no son necesariamente el mismo binario,
y nada lo decía.

| Fichero | Qué es |
|---|---|
| `crates/sddk-cli/build.rs` | embebe commit, procedencia y suciedad; degrada sin fuente ni repositorio |
| `crates/sddk-cli/src/dev/build_id.rs` | la identidad, la comparación y sus 11 guards |
| `crates/sddk-cli/src/dev/mod.rs` | registra `DevCommand::BuildId` |
| `crates/sddk-cli/tests/cli.rs` | 2 guards e2e sobre el binario real |
| `docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-dev-help.txt` | **una línea**: el subcomando nuevo |

## Las tres decisiones que no son evidentes

**1. La identidad la fija quien lanza el build, no el script de build.**
`SDDK_GIT_SHA` es la fuente de verdad. El `build.rs` cae a `.git` como
**diagnóstico**, y STOP 6 le prohíbe decidir. La razón está medida, no supuesta:
con un crate mínimo se probaron cuatro escenarios y tres fallan —
congelada sin `rerun-if-changed`, congelada con ella sobre `.git/HEAD` porque
`HEAD` no cambia al commitear, y **congelada e incorrecta** con `packed-refs`
(declaraba `9b3f0076` con el HEAD en `247e808d`). Un detector que emite un valor
obsoleto sin señal es peor que no tener detector.

**2. `--version` no se toca, y el motivo es una línea de un script de shell.**
`install.sh:416` resuelve la versión con `printf '%s' "$out" | awk '{print $NF}'`.
Hoy el último campo de `sddk 2.5.3` es `2.5.3`; añadir el commit **al final**
haría que fuera `)` y el instalador guardaría un paréntesis como versión.

**3. `--is-ancestor` es lo único que prueba una dirección.** Dos commits
distintos no dicen cuál es más nuevo, así que cuando el del binario no es
ancestro del HEAD la relación es `diverged` y **no se afirma en qué dirección
van**. Y con `--check`, una relación que no se pudo establecer **sale con
código 1**: STOP 3 hecho código.

## Verificación

| Comprobación | Resultado |
|---|---|
| guards de `dev::build_id` | **11 passed / 0 failed** |
| guards e2e en `tests/cli.rs` | **2 passed / 0 failed** |
| `cargo test --workspace --no-fail-fast` | **5427 passed / 0 failed / 24 ignored / 283 binarios** |
| `23-falsify-build-identity.py` (6 mutaciones) | **6/6 DETECTADA**, 0 no medibles |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cli_golden` | verde tras regenerar **una** línea |
| `tests/test_changelog_coverage.sh` | **PASS=69 FAIL=0** |
| scanner de caracteres no latinos | CLEAN |

La aritmética del workspace cierra sola: base 5414 / 24 / 283, más los **13**
tests nuevos —11 in-module en `build_id.rs` y 2 e2e en `tests/cli.rs`— →
**5427 / 24 / 283**. Los 283 binarios y los 24 ignorados son los mismos, y el
golden se regeneró con una línea, no se reescribió.

Comportamiento observado, con los dos binarios:

| | `source: git` | `source: env` |
|---|---|---|
| `dev build-id` | declara commit y `dirty: true` | lo mismo |
| `dev build-id --check` | `relation: Unknown`, **exit 1** | `relation: Matches`, **exit 0** |

## Falsificación: los seis casos y quién los caza

| Mutación | Cae |
|---|---|
| M1 la identidad derivada de `.git` pasa a ser concluyente | **R6** + R4 |
| M2 toda relación cuenta como respuesta | **R4** |
| M3 sin identidad concluyente se declara `Matches` | **R4** |
| M4 `Behind` por desigualdad en vez de por ancestría | **R5** (el caso nuevo) |
| M5 la suciedad se inventa cuando no se sabe | **R3** (el caso nuevo) |
| M6 el `build.rs` hace panic | **R2** |

**M4 y M5 sobrevivieron a la primera pasada, y los dos eran guards que solo
fijaban el caso donde el defecto no se manifiesta.** Se corrigieron **en el
guard**, nunca en el producto:

- **R3** construía la identidad a mano y probaba que la estructura admite
  `dirty: None`, **sin ejercitar la lectura** que convierte el `"unknown"` del
  build en `None` — que es donde se escondería un `Some(false)`. Se extrajo
  `from_raw` y el guard pasó a ejercitar la lectura con las tres entradas que
  el `build.rs` puede emitir.
- **R5** no tenía el caso en que el commit del binario **no** es ancestro del
  HEAD, que es el único donde «comprobar la ancestría» y «comprobar que son
  distintos» dan respuestas **opuestas**. Sin él, la prueba anterior pasaba con
  `rev-parse` en lugar de `merge-base --is-ancestor`.

Que M4 caiga con el guard nuevo y **no** con el viejo es la prueba de que
añadirlo aportó algo: el viejo no puede detectarla, y no por un fallo suyo, sino
porque en su caso las dos implementaciones coinciden.

## Los instrumentos fallaron, y ninguno era el producto

**Tres guards se detectaban a sí mismos.** R1 buscaba `version(` en su propio
módulo, y su versión anterior buscaba `DevCommand::Version` en la línea que la
contenía. Es la **tercera vez** en esta serie que un guard textual busca una
cadena que su propio código tiene — la primera fue R5 en
`cl-release-forge-testability`. Ahora el token se ensambla en runtime.

**El falsificador falló tres veces.** Comparaba nombres cortos contra los
completos que imprime `cargo test` (`dev::build_id::tests::r6_…`), y las cuatro
detecciones salían como `DETECTADA_POR_OTRO` aunque los guards caían bien: el
mismo modo de fallo del extractor ciego del ciclo forge, en su otra forma — allí
no capturaba el nombre, aquí lo captura y no lo reduce. Su criterio de «árbol
limpio» exigía un árbol vacío, imposible con el trabajo en curso a medias, y
abortaba en la primera mutación sin medir nada. Y una de sus mutaciones **añadía
un brazo inalcanzable** al `match`: inerte, luego no media nada, y reportarla
como `SOBREVIVIDA` habría sido un falso positivo sobre el guard.

## El golden cayó, y el delta es una línea

`cli_golden_surface_matches_blessed_snapshot` falla al añadir un subcomando, que
es exactamente lo que su propio contrato predice. El delta revisado antes de
regenerar: **una línea añadida**, `build-id`, y todo lo demás desplazado. Ninguna
eliminación, ningún renombrado — que es lo que el contrato llama regresión.

## Lo que NO se hizo, y por qué

- **No se tocó `BUNDLE.toml` ni el contrato del bundle** (STOP 4). `build.rs` y
  `dev/build_id.rs` están bajo `crates/`, que no es superficie de
  `MANIFEST_SURFACES`; comprobado, no supuesto.
- **No se cambió `--version`** (STOP 2), por el motivo medido de arriba.
- **No se derivó la identidad de `.git` como fuente de verdad** (STOP 6), por los
  cuatro escenarios medidos.
- **No se tocó `Cargo.toml`**: `build.rs` se detecta solo, sin `build =` explícito.
- **No se publicaron** los valores de `SDDK_GIT_SHA` en el pipeline de release.
  Eso es lo que convierte el diseño en "`env` es la fuente de verdad" y es
  concernia propia: `release.sh` tendría que exportarla antes del paso 3. **Sin
  eso, una release publicada seguirá declarando `source: git`**, que es correcto
  pero poco útil. Queda declarado, no escondido.

## Estado en la autoridad

Ciclo en `OPEN/build`, `B-direct`, sequence 1. **0 gates**: la transición
`phase.build.complete.b-direct` exige `implementation-complete` y
`implementation-receipt`, que se graduarán con este recibo.

---

# Corrección posterior: `--check` pasaba sobre un binario atrasado

> Añadido tras publicar `032e9553`. **Este bloque se conserva aunque se corrija**,
> porque explica por qué el defecto era invisible hasta que se miró con un binario
> real en lugar de con un test.

## El defecto, medido

Construyendo el binario con `SDDK_GIT_SHA` apuntando a un commit que **no** es el
HEAD — que es exactamente el escenario de INC-DEBT-064 — la salida fue:

```text
commit: dc69e6f2676c9c70c812c58f4b5195f6826f15fc
source: env
checkout_head: 032e9553decb697a3cba42897e76ae2a8a71b907
relation: Behind
reason: el commit del binario (dc69e6f2) es ancestro del HEAD del checkout, luego
        el checkout tiene trabajo que el binario no contiene
EXIT=0
```

**El texto y el código de salida se decían lo contrario en la misma pantalla.** El
`reason` afirma que al binario le falta trabajo; el `EXIT=0` afirma que no le falta
nada. Para cualquiera que use `--check` como gate —un script, un pipeline, un
humano leyendo solo el código— un binario obsoleto se presenta como conforme.

## Por qué estaba, y por qué los tests no lo vieron

La causa es `is_answer()`, que metía `Behind` en el conjunto que pasa. La lectura
detrás era: "`Behind` es una respuesta, luego pasa". Y lo es — **pero respecto a una
pregunta distinta**. `is_answer` contestaba *¿hubo relación?*, y el código de salida
tenía que contestar *¿este binario es el del checkout?*.

STOP 3 dice «una relación que **no se pudo establecer** no sale con 0», y eso se
cumplía: `Behind` sí establece la relación, la dirección va probada con
`--is-ancestor`. Lo que STOP 3 no dice —y lo que el módulo sí tiene que decir— es que
**una relación establecida y suficiente para responder no tiene por qué ser
suficiente para pasar**. Establecida y aun así insuficiente: el binario no contiene
el trabajo del checkout. Es el mismo principio de STOP 3 un nivel más arriba, y el
lectura de STOP 3 como techo en vez de como suelo lo dejó pasar.

Los tests no lo vieron por una razón distinta y declarada: el guard unitario fijaba
`is_answer` —el predicado, no el efecto— y el guard e2e fijaba el **formato**, nunca
el código de salida. Entre ambos, el código de salida no lo fijaba nadie.

## El arreglo

`is_answer` desaparece y le sigue `is_current`, que es `Matches` y nada más. El
nombre nuevo dice lo que el viejo no decía: el código de salida responde a una
sola pregunta. Con el mismo binario y el mismo checkout, `Behind` pasa de **0** a
**1**; verificado reconstruyendo, no por lectura.

## Falsificación, incluida la del guard que no detectaba nada

- El guard unitario R4 estaba cambiado a `Matches` y `Behind`, es decir, fijaba el
  comportamiento defectuoso. Corregido **en el guard** (el código era correcto
  sobre su pregunta y la pregunta estaba mal), y se le añadió el caso `Behind`
  como no pasante, que es el que antes no existía.
- Reintroduciendo el defecto en el producto, R4 **cae** con el motivo que lo
  explica. Falsificado.
- El guard e2e nuevo, con el defecto puesto y el entorno normal, **PASABA**. Cuarta
  vez en esta serie que un guard resulta ciego: sin `SDDK_GIT_SHA` el binario de
  test cae por el fallback `.git`, que STOP 6 declara no concluyente, luego la
  relación es siempre `unknown` y la rama `behind` —la única que hacía pasar al
  defecto— no se alcanza nunca. Falsificado de verdad construyendo el binario de
  test en un commit que no es el HEAD (`SDDK_GIT_SHA=$(git rev-parse HEAD~1)`),
  donde **sí cae** con el código contradictorio. La condición de ceguera queda
  escrita en el propio guard, porque un guard que parece cubrir un caso y no lo
  cubre es peor que un guard ausente.

## Lo que este defecto dice del proceso

No lo encontró ninguna mutación del falsificador. Lo encontró **usar el producto
como se usa**: declarar un commit viejo en la construcción y mirar qué salía. Las
seis mutaciones de `23-falsify-build-identity.py` pasaban en verde sobre un
`--check` que era peor que no tenerlo. Un falsificador que solo muta la
implementación no ve los defectos de la *interfaz observable*.
