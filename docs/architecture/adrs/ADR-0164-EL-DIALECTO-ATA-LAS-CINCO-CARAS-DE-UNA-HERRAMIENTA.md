---
id: ADR-0164-EL-DIALECTO-ATA-LAS-CINCO-CARAS-DE-UNA-HERRAMIENTA
title: La herramienta que se nombra es la que se ejecuta, y el nombre que no se sabe es un error
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-070
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-070
supersedes: null
superseded_by: null
extends: [ADR-0163, ADR-0157, ADR-0159]
component: release
surface: crates/sddk-gateway/src/version_provider.rs
closes: []
---

# ADR-0164 — El dialecto ata las cinco caras de una herramienta

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0163 (preguntar al build tool), ADR-0159 (el tag no es la versión)
- **Superficie:**
  - `crates/sddk-gateway/src/version_provider.rs` — `BuildToolDialect`,
    `UnknownBuildTool`, `parse_maven_model`, `motivo_de_ejecucion`
  - `crates/sddk-cli/src/release_cmd.rs` — `BuildAsk`, `dialect_asked`,
    `LocalReleaseQuestion`
- **Adenda correctiva:** corrige la premisa del `PRE-FLIGHT` de este mismo ciclo,
  que afirmaba un `fail-open` en `version.rs:190` que no existe. La corrección
  está al final y no cambia la decisión.

---

## Contexto

ADR-0163 dio a SDDK la capacidad de **preguntar** al build tool y la conectó al
diagnóstico. Lo dejó con un defecto medido.

`--build-tool` aceptaba cualquier cadena y `BuildModelInvocation` traía programa
y argumentos sueltos. Tres caras quedaban **desalineadas** sin que nada lo
impidiera:

| cara | dónde vivía |
|---|---|
| programa | `--build-tool`, lo que el operador escribiera |
| argumentos | `vec!["properties", "--offline"]`, fijo en `release_cmd.rs` |
| fichero de build | `GRADLE_BUILD_FILES`, fijo en el gateway |
| parser | `parse_build_model`, dialecto Gradle |
| centinela | `"unspecified"`, palabra de Gradle |

MEDIDO, con un ejecutable instrumentado en el `PATH` sobre un build Gradle:

```
$ sddk release version inspect --root . --evaluate-build --build-tool mvn
providers_answering: sddk.gateway.build-model/mvn
  - sddk.gateway.build-model/mvn [Observed] 3.3.3 declared at ././build.gradle

$ cat argv.txt
properties
--offline
```

Dos falsificaciones a la vez. **`mvn` recibió los flags de Gradle** —`properties`
no es un goal de Maven— y el informe **atribuyó la respuesta a
`././build.gradle`**, un fichero que Maven nunca abrió.

Cada cara era correcta por separado. Las tres juntas fabricaron una evidencia, y
la bandera que nombraba la herramienta era la que decidía cuál se usaba. Eso no
es una limitación: es un mecanismo de fabricar evidencia atribuida, en un subsistema cuya
ley central es no mentir sobre lo que se comprobó.

## La decisión

**Un dialecto es las cinco caras, y solo se puede construir desde su nombre.**

```rust
pub enum BuildToolDialect {
    GradleProperties,
    MavenHelpEvaluate,
}
```

El tipo ata programa, argumentos, ficheros de build, parser y centinela. No
existe la forma de elegir uno de cada: `BuildModelInvocation` dejó de tener
campos públicos, así que el par desalineado **no se puede escribir**.

**`--build-tool` no tiene default.** `BuildToolDialect::parse` devuelve
`UnknownBuildTool`, que dice cuáles se saben preguntar. Un nombre desconocido es
un error de la línea de comandos, no un Gradle en disguise.

**La pregunta viaja como un valor.** `BuildAsk` sustituye a la pareja de
banderas por las que la puerta podía pasar `evaluate_build` y olvidar
`build_tool` —un provider que lanza un proceso con el nombre equivocado—.

**La misma pregunta decide qué es un target.** `resolve_release_target` recibía
un registro sin dialecto mientras el diagnóstico usaba uno con dialecto. Eso
producía un deadlock medido: un repo Maven con solo `pom.xml` no era release
target, porque `release_targets` decide qué es target preguntando quién declara
versión, y el único que podía declararla —el build tool— se consultaba después.
Para preguntar hace falta un target, y para tener un target hace falta declarar
versión.

## Un defecto que apareció al construir esto

`ETXTBSY` —el binario existe y otro proceso lo tiene ocupado— se reportaba con
el texto de `ENOENT`. MEDIDO: 4 de 15 ejecuciones de una suite fallaron así, con
1219 procesos en el host, y el texto decía que faltaba una herramienta.

Eso contradice el motivo por el que existe `b4`: «no tengo la herramienta» y «la
herramienta falló» son dos reparaciones opuestas. `motivo_de_ejecucion` los
separa, y `NotFound` es el único que dice «no la tengo».

La flakiness del ETXTBSY **no** venía del código cambiado —el camino de `exec`
es idéntico antes y después— sino de arrancar un script recién escrito por su
shebang en tmpfs con el host saturado. Arrancar `/bin/sh` y pasar el script como
argumento lo hace imposible, y medido: **0 fallos en 20 ejecuciones**, antes
4-5 de 12-15.

## Un hueco que el falsador encontró en sí mismo

Con `MavenHelpEvaluate` usando el parser de Gradle, `d5` y `d6` siguieron en
verde. Llamaban a `parse_maven_model` directamente, así que ejercitaban la ley
pura sin ejercitar **que el dialecto la use**. Son dos cosas distintas y una suite
que solo cubre la primera acepta un mutante que rompe la segunda. `d8` cubre la
conexión.

## Verificación

Falsadores, con cada mutación cayendo por su propia comprobación:

| mutación | cae |
|---|---|
| Maven recibe args de Gradle | `d2` |
| Maven busca ficheros de Gradle | `d3`, `d7` |
| Maven usa el parser de Gradle | `d8`, `d7` |
| control, sin mutación | 18 pasan |

`crates/sddk-gateway/tests/version_build_model_contract.rs` — 18/18.
`crates/sddk-cli/tests/release_lockstep_sees_build_model.rs` — 8/8.
`crates/sddk-cli/tests/release_version_build_model.rs` — 3/3.

MEDIDO después del arreglo, sobre el binario:

```
$ sddk release version inspect --root . --evaluate-build --build-tool mvn
error: unknown build tool `mvn`; SDDK knows how to ask: gradle, maven

$ sddk release version inspect --root . --evaluate-build --build-tool maven   # repo con pom.xml
providers_answering: sddk.gateway.build-model/maven
  - sddk.gateway.build-model/maven [Observed] 4.5.6 declared at ./pom.xml

$ cat argv.txt
--offline help:evaluate -Dexpression=project.version -DforceStdout -q
```

## Consecuencias

- **`release version matches` y `release handoff` no preguntan.** Sus `Args` no
  declaran `--evaluate-build` (medido), así que usan `BuildAsk::never()` escrito
  en el sitio de la llamada. Es una limitación declarada de esos dos comandos.
- **Una herramienta nueva es una variante de dialecto con su medida**, no una
  bandera que acepta cualquier palabra. Maven entra con su comando medido y un
  parser propio porque su salida no comparte formato ni centinela con Gradle.
- **El kernel sigue sin conocer herramientas.** `AdapterFact` y
  `parse_build_model` viven en el gateway, como todo el resto.

---

## Adenda correctiva al `PRE-FLIGHT` de este ciclo

La primera redacción de `PRE-FLIGHT.md` afirmaba, en negrita:

> *Un tag `v9.9.9` sobre un proyecto que declara `1.2.3` pasa la puerta.*
> Un **fail-open con testigo medido**.

**No es cierto**, y el error fue de razonamiento, no de código. `refusal()`
(`version.rs:230-305`) corre en `:182`, **antes** del `let Some(...)` de `:190`,
y devuelve `Err` para `Unresolved` (`:295`), `Ambiguous` (`:244`) e `Invalid`
(`:258`). El `return Ok` solo se alcanza con `ReleaseRefIsAuthority`, que exige
declaración explícita: `go.mod`, `MODULE.bazel` y `WORKSPACE` llevan
`tag_is_authority: true` (`version_provider.rs:229`), o el JSON
`"authority": "tag"` de `:551`. El propio código lo llama *«a declaration of
absence, not silence»*.

La medición que citaba como testigo decía `productVersion: none`, que tiene
**cuatro** causas con cuatro veredictos distintos. Saltar de «ninguno» a «pasa
la puerta» fue un salto de silogía, no una observación.

**La capacidad de este bloque sigue siendo necesaria, pero por otro motivo, y es
mayor:** un build Gradle o Maven real, sin `gradle.properties`, da
`authority: NotChecked` con 14 providers consultados y ninguno respondiendo — y
`Unresolved` lo rechaza la puerta. Sin esta capacidad, **un proyecto JVM no
puede publicar un release por SDDK.** No porque la puerta sea permisiva, sino
porque no hay evidencia y sin evidencia no hay autorización.

Este ADR no cambia la decisión. Cambia la **razón** que se escribió, que era
falsa, por la que se midió, que no lo era.
