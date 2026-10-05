# RECEIPT — session-85, cierre del evolutivo `version-coherence-068`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-068`
**Fecha:** 2026-10-05
**Rama:** `fix/version-coherence-068` → mergeada a `main` por fast-forward
**Commits:** 19 (`62b5a88d..267fab14`), publicados a `origin/main` **sin
`--no-verify`**

---

## Lo que se cierra

El evolutivo de **agnosticidad de versión/release** (VA1–VA8), con tres ADRs
nuevos y cuatro modificados, todo en `main` y con la suite del workspace en
verde.

| Bloque | Commit | Qué |
|---|---|---|
| VA1–VA3 | `b32978ed`… | El registro tecnológico sale del kernel; la versión se resuelve **preguntando** a providers (ADR-0157) |
| VA4 | `0b1b15e5`, `a6799318`, `cc855df1` | Varios targets por repositorio, con `--target` y sin adivinar (ADR-0158) |
| VA5 | `135e812c`⁻ | Un tag no es una versión; la convención es **declarada** (ADR-0159) |
| VA6 | `312205c4` | Roles productor/certificador/promotor con techo en el retículo (ADR-0160) |
| VA7 | `fae6bb9c` | `sddk release version inspect`: el veredicto por fin explica su porqué (ADR-0161) |
| VA8 | `135e812c` | `sddk release handoff`: el sobre que construye y entrega (ADR-0162) |
| Aterrizaje | `267fab14` | Rebase, changelog `2.12.0` reconstruido, tres punteros de versión alineados |

---

## La medición de cierre: **parcial, y con el motivo medido**

La pregunta era: ¿observa SDDK el `ProductVersion` de PipelineK sin que el repo
declare una versión por segunda vez?

### `chronos` (Rust) — **sí, y sin tocar nada**

```
$ sddk release version inspect        # en chronos, sin flags
providers_answering: sddk.gateway.declaration-file/Cargo.toml
  - sddk.gateway.declaration-file/Cargo.toml [Observed] 0.11.0 declared at Cargo.toml
authority: Observed
productVersion: 0.11.0
```

MEDIDO también, y es la parte que importa:

- **El `NOT_CHECKED` derivado apareció solo por tener una fuente**: *«only one
  source declared this value; nothing corroborated it from an independent second
  source»*. Esa clave es condicional en el código, así que su aparición aquí es
  la ley funcionando sobre un repo real y no sobre un fixture.
- `chronos` **no tiene** `.sddk/version-source.json` y su `git status` está
  limpio. Cero declaraciones nuevas.

### PipelineK (Kotlin/Gradle) — **no, y el motivo está escrito en el código**

> **ADENDA, posterior a este recibo.** La medición de arriba dice que SDDK
> *no mira* `build.gradle.kts`. Eso era verdad al publicarse y **ha dejado de
> serlo**: `b9b59069` (ADR-0163) añade la capacidad de **preguntar** a la
> herramienta de build con `--evaluate-build`. Lo que queda es distinto y hay que
> no leer este parrafo como el estado actual: hoy SDDK **pregunta**, y lo que se obtiene
> en ese checkout es `Invalid` con el motivo del propio Gradle
> (`Another Gradle invocation is already using this v2 checkout`), porque otra
> invocación tiene el checkout tomado. Es un bloqueo **del entorno**, no del
> código, y se nombra en vez de esconderse. El repo sigue intacto y sin segunda
> declaración. Detalle en `docs/roadmap/receipts/cl-build-model-observation/`.

MEDIDO en `/var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin`:

1. **El monorepo tiene 37 targets** y SDDK se niega a elegir:
   `37 release targets found and nothing names one … Which product to release is
   a decision, and sddk will not make it.` — con `--target` funciona.
2. `v2/build.gradle.kts:75` declara `version = "0.47.0"` de verdad, no en un
   comentario.
3. SDDK responde `build.gradle.kts no es fuente de version declarable` →
   `authority: NotChecked`, `productVersion: none`.

**Y ese refusal es correcto, no un defecto.** El motivo está escrito en
`crates/sddk-gateway/src/version_provider.rs:180`:

> *Build scripts. Their version is only observable by EVALUATING the build's own
> model — it can come from a provider reference, a version catalogue, or a
> convention plugin — so a reader for them belongs to a provider that can ask the
> tool, not to a pattern matcher.*

Es decir: SDDK **sabe** que hay una versión ahí y **no la adivina**, porque
adivinarla con un regex sobre Kotlin DSL sería un reconocedor de patrones
haciéndose pasar por provider — y el fichero lo confirma: `v2/build.gradle.kts`
tiene al menos **tres** menciones de `version` en comentarios y una asignación
real. Un regex que cogiera la primera se llevaría un 0.44.0 de una comentario.

El repo **no se tocó** y **no** recibió `.sddk/version-source.json`. Que es
exactamente lo que se pidió: no adaptar PipelineK para satisfacer a SDDK.

---

## El siguiente bloque, y por qué está declarado y no empezado

**VA9 — un provider que pueda preguntar a Gradle.** No es «un parser más»: es el
único camino honesto, y el código actual ya lo dice con esas palabras. El
provider evaluates `gradlew properties` (o equivalente) y devuelve lo que el
build dice de sí mismo.

**Por qué no se empieza aquí.** Tres razones, y las tres son medidas:

1. **Es un bloque nuevo**, no una extensión de VA8: toca el registry de
   providers, su contrato de conformidad (`version_provider_contract`, C1..C10)
   y necesita un proyecto Gradle real en el banco de pruebas.
2. **Necesita su propio PRE-FLIGHT**, y la superficie no está mapeada con línea
   todavía.
3. **El riesgo de hacerlo a medias es alto**: un provider que «más o menos» lee
   Gradle es peor que no tenerlo, porque reintroduce el reconocedor de patrones
   que `version_provider.rs` se niega a ser.

**Y hay un dato que lo hace más necesario de lo que parecía:** PipelineK declara
en su propio código que *«the root project.version is the SOLE authority for
every subproject's publication version»*. Los 37 targets que SDDK encuentra son
37 módulos que **heredan** la versión del padre y no la declaran. Hoy SDDK dice
`NotChecked` en los 37, lo cual es cierto pero poco útil: la respuesta que falta
no está en los hijos, está en el padre, y un provider que evaluara el modelo del
build la daría sin cambiar nada del repo.

---

## Mediciones de esta sesión

| Puerta | Resultado |
|---|---|
| `cargo test --workspace` (tras el rebase) | **PASS=5620 FAIL=0 IGNORED=24**, 302 binarios |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 errores |
| `cargo fmt --check` | limpio |
| Gate de changelog (2b) | **PASS=16 FAIL=0** — los 13 commits desde `v2.11.4` representados |
| `tests/test_adr_0157_criteria.sh` | **PASS=26 FAIL=0 NOT_APPLIED=0** |
| `tests/test_changelog_merge.sh` | PASS=34 FAIL=0 |
| `tests/test_adr_promotion_format.sh` | 65 accepted, 0 violaciones |
| `tests/test_gate_coverage.py` | PASS |
| `tests/test_push_prevention_coherence_mutation.sh` | 7 medidos como se esperaba, 0 no |
| Falsador de VA7 | 10/10 |
| Contrato de providers | 11/11 |
| Render de `inspect` | 5/5 |
| Falsador de `handoff` | 8/8 |

---

## Discrepancia que queda abierta, y es real

**El ciclo `version-coherence-068` sigue en fase `specify`** con un único
artefacto (`exploration-report.md`), mientras ocho bloques están implementados,
probados y publicados en `main`.

MEDIDO: `sddk cycle status` → `phase: specify`; `git log origin/main..HEAD`
antes del push → 19 commits.

No se corrige a mano. Mover el ciclo por sus transiciones es trabajo del
proceso con sus gates, y hacerlo sin evidencia sería escribir un estado que no
se midió. **Se declara como discrepancia y se deja que el operador o el proceso la
resuelvan con la evidencia que ya existe arriba.**

Igual de importante: **`docs/roadmap/CURRENT.md` dice «session-83» y el último
bloque del diario es «session-84 bis 7».** Eso ya estaba asi antes de esta
sesión; no lo he tocado porque arreglarlo es otro bloque y hacerlo mal es peor
que dejarlo escrito.

---

## Deuda declarada y **no** abierta

1. **`DeclaredAuthorityProvider::provider_id()`** es `sddk.gateway/<ruta>`
   mientras las demás son `sddk.gateway.declaration-file/<fichero>`, y por eso
   `providers_considered` sale heterogéneo. No se toca: los provider ids son
   superficie pública.
2. **`PrefixedCandidate` sigue fuera de `VersionNaming::NAMED`**, así que
   `--naming` no lo selecciona. Intencionado: tiene tres parámetros y un nombre
   de una palabra los escondería. Se construye por sus partes.
