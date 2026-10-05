# CLOSURE — VA10 `cl-lockstep-gate-sees`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-070`
**Fecha:** 2026-10-05
**Commit de código:** `2387c579`
**Cierra:** ADR-0164

---

## Qué se cerró

La capacidad de **preguntar al build tool** llega a la puerta de lockstep, y la
herramienta que se nombra es la que se ejecuta.

## La premisa de este ciclo era falsa, y eso es lo primero que hay que leer

El `PRE-FLIGHT` de este ciclo afirmaba en negrita:

> **Un tag `v9.9.9` sobre un proyecto que declara `1.2.3` pasa la puerta.**
> Un **fail-open con testigo medido**.

**No es cierto.** El error fue de razonamiento, no de código:

| `VersionAuthority` | `refusal()` (`version.rs:230-305`) | Puerta |
|---|---|---|
| `Unresolved` | `Some` en `:295` | **rechaza** |
| `Ambiguous` | `Some` en `:244` | **rechaza** |
| `Invalid` | `Some` en `:258` | **rechaza** |
| `ReleaseRefIsAuthority` | `None` en `:237` | pasa |

`refusal()` corre en `:182`, **antes** del `let Some(...)` de `:190`. El
`return Ok` solo se alcanza con `ReleaseRefIsAuthority`, que exige **declaración
explícita**: `go.mod`, `MODULE.bazel` y `WORKSPACE` llevan `tag_is_authority:
true` (`version_provider.rs:229`), o el JSON `"authority": "tag"` de `:551`. El
código lo llama *«a declaration of absence, not silence»*.

La medición citada decia `productVersion: none`, que tiene **cuatro** causas con
cuatro veredictos distintos.

**El `PRE-FLIGHT` se reescribió en lugar de anotarse con una adenda** porque
nunca se publicó: estaba sin commitear. Una adenda es para un recibo que la
gente ya leyó; aquí nadie había leído nada.

## Por qué la capacidad se mantiene igualmente

MEDIDO, sobre `/tmp/gradle-probe-groovy` (build Gradle Groovy, sin
`gradle.properties`):

```
$ sddk release version inspect --root .
providers_considered:  <14>
providers_answering:   none
authority:             NotChecked
  - sddk.gateway.declaration-file/build.gradle  build.gradle no es fuente de version declarable
```

Los 14 responden: 13 «no está en este target» y `build.gradle` «no es fuente de
versión declarable» (`version_provider.rs:185-195`: `extraction: None`, porque la
versión de Gradle solo se conoce evaluando el build). De ahí sale `Unresolved`,
y `Unresolved` **rechaza** la puerta.

Es decir: **un proyecto JVM no puede publicar un release por SDDK.** No porque la
puerta sea permisiva, sino porque no hay evidencia.

## El defecto que sí existía, medido con testigo

Con un ejecutable instrumentado en el `PATH`:

```
$ sddk release version inspect --root . --evaluate-build --build-tool mvn
  - sddk.gateway.build-model/mvn [Observed] 3.3.3 declared at ././build.gradle

$ cat argv.txt
properties
--offline
```

Dos falsificaciones: `mvn` recibió los flags de Gradle, y la atribución fue a un
fichero que Maven nunca abrió. Las cinco caras —programa, args, ficheros, parser,
centinela— estaban sueltas y nada impedía combinarlas mal.

## Verificación

**Falsadores, cada mutación cayendo por su propia comprobación:**

| mutación | cae |
|---|---|
| Maven recibe args de Gradle | `d2` |
| Maven busca ficheros de Gradle | `d3`, `d7` |
| Maven usa el parser de Gradle | `d8`, `d7` |
| control | 18 pasan |

- `crates/sddk-gateway/tests/version_build_model_contract.rs` — **18/18**
- `crates/sddk-cli/tests/release_lockstep_sees_build_model.rs` — **8/8**
- `crates/sddk-cli/tests/release_version_build_model.rs` — **3/3**
- suite `sddk-gateway` + `sddk-cli` — **PASS=1832 FAIL=0**
- `cargo clippy -D warnings` (ambos paquetes, `--all-targets`) — limpio
- `cargo fmt --check` — limpio
- `tests/test_adr_promotion_format.sh` — 67 accepted, **0 violaciones**
- `tests/test_changelog_coverage.sh` — **PASS=18 FAIL=0**

**MEDIDO sobre el binario, después del arreglo:**

```
$ sddk release version inspect --root . --evaluate-build --build-tool mvn
error: unknown build tool `mvn`; SDDK knows how to ask: gradle, maven

$ sddk release version inspect --root . --evaluate-build --build-tool maven   # repo con pom.xml
providers_answering: sddk.gateway.build-model/maven
  - sddk.gateway.build-model/maven [Observed] 4.5.6 declared at ./pom.xml

$ cat argv.txt
--offline help:evaluate -Dexpression=project.version -DforceStdout -q
```

## Un deadlock que apareció al construir esto

`resolve_release_target` recibía un registro **sin** dialecto mientras el
diagnóstico usaba uno **con** dialecto. Resultado medido: un repo Maven con solo
`pom.xml` no era release target — `release_targets` decide qué es target
preguntando quién declara versión, y el único que podía declararla se consultaba
**después** de esa decisión. Para preguntar hace falta un target, y para tener un
target hace falta declarar versión.

Ahora la misma `BuildAsk` decide qué es target, lo autoriza y lo informa. Una
función que decide con un registro distinto del que luego informa puede mirar dos
cosas distintas, y esa es la forma que tenía el resto de los defectos de este
ciclo.

## Dos defectos propios, medidos

1. **El falsador tenía un hueco.** Con el dialecto de Maven usando el parser de
   Gradle, `d5` y `d6` siguieron en verde: llamaban a `parse_maven_model`
   directamente y ejercitaban la ley pura sin ejercitar **que el dialecto la
   use**. `d8` cubre la conexión.

2. **`ETXTBSY` se reportaba como `ENOENT`.** MEDIDO 4 de 15 ejecuciones con 1219
   procesos en el host, y el texto mandaba a instalar una herramienta que ya
   estaba. `motivo_de_ejecucion` separa los dos, y solo `NotFound` dice «no la
   tengo». La flakiness venía de arrancar un script recién escrito por su shebang
   en tmpfs con el host saturado —el camino de `exec` del código cambiado es
   idéntico antes y después—, y arrancar `/bin/sh` lo hace imposible: medido
   **0 fallos en 20 ejecuciones**.

## Limitación declarada

`release version matches` y `release handoff` **no preguntan**: sus `Args` no
declaran `--evaluate-build` (medido). Usan `BuildAsk::never()` escrito en el
sitio de la llamada, no un default silencioso. Cerrarlos es trabajo de otro
ciclo.

## Lo que este bloque NO arregla

La cobertura de ecosistemas sigue siendo una **tabla de 13 filas** en
`DEFAULT_DECLARATIONS` (`version_provider.rs:129-243`). Un proyecto con
`build.sbt`, `composer.json`, `mix.exs`, `pubspec.yaml`, `Gemfile`, `*.csproj`,
`meson.build` o `BUILD.bazel` no es un target y no publica. La vía agnóstica ya
existe —`.sddk/version-source.json`— y es la que este bloque dejó de estorbar.
Invertir esa prioridad es el siguiente bloque, no éste.
