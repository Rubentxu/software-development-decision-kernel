---
id: ADR-0153-WHERE-A-PROJECT-DECLARES-ITS-VERSION
title: Resolve a project's declared version through a declarative source contract, because two of the eight ecosystems have no manifest to declare it in
status: superseded
proposed_at: 2026-10-02
accepted_at: 2026-10-02
superseded_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-source
accepted_by_cycle: p-63676b11dc0ef88f/version-source
supersedes: null
superseded_by: ADR-0157
component: release
surface: crates/sddk-engine/src/version_source.rs
closes: [INC-DEBT-051]
---

# ADR-0153 — Dónde declara su versión un proyecto: un contrato declarativo con dos clases de fuente

> **SUPERSEDED por [ADR-0157](ADR-0157-VERSION-AUTHORITY-IS-A-QUESTION-NOT-A-REGISTRY.md)
> (2026-10-05). Este documento se conserva íntegro y no es autoridad de nada.**
>
> La decisión de este ADR era correcta y su criterio de aceptación estaba
> medido y falsificado. Lo que ADR-0157 muestra es que el criterio dibujaba una
> frontera que el código no trazaba: el *lector* no nominaba manifiestos, pero
> el `REGISTRY` —doscientas líneas más arriba, en el mismo fichero, en el crate
> que **decide**— sí. Y una entrada unía tres ficheros de formatos distintos
> bajo un solo parser, que es una afirmación falsa sobre su formato.
>
> Lo que se conserva de aquí: la observación de que **Go y Bazel no declaran
> versión en ningún manifiesto**, que es la razón de existir del quinto
> veredicto en el modelo actual, y el criterio de que la declaración explícita
> del proyecto es un rescate acotado. Lo que no se conserva es el registro.
>
> El gate de sus siete criterios era `tests/test_adr_0153_criteria.sh`, y se ha
> sustituido por
> `tests/test_adr_0157_criteria.sh`, que además corrige un defecto del
> instrumento: el gate anterior decidía con un `grep` sobre `test result: ok`, y
> un criterio cuyos tests ya no existían reportaba **PASS** sin ejecutar nada.

**Status:** superseded (aceptado 2026-10-02)
**Date:** 2026-10-02
**Cycle:** `p-63676b11dc0ef88f/version-source`
**Closes:** INC-DEBT-051 (high/P1)
**Relates:** ADR-0042 (kernel agnóstico), SPEC-043 (`EcosystemProfileV1` como
precedente de descripción declarativa sin código de kernel), AGENTS.md §2.3

---

## Context

`ensure_version_lockstep` (`crates/sddk-engine/src/version.rs`) abre
`root.join("Cargo.toml")` sin alternativa. En un proyecto que no es Rust,
`sddk release plan` y `sddk release apply` abortan antes de hacer nada útil.
Reproducido sobre `/var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin`
(Gradle, sin `Cargo.toml`): exit 1 con `VERSION LOCKSTEP ERROR`.

AGENTS.md §2.3 declara, entre las convenciones duras, que la política SDDK es
agnóstica de lenguaje/build/test runner y debe funcionar igual en repos JVM,
JS/TS, Python, Go, .NET, C/C++, Bazel o polyglot **mediante
adapters/capabilities**. El lockstep no consulta ningún adapter: abre la ruta.

Eso convierte una limitación en una desviación de una convención dura, que es
la distinción que la propia deuda defiende: una limitación se acepta y se
documenta; una desviación se corrige o se deroga explícitamente.

**El hecho que impide el arreglo obvio.** Antes de diseñar se comprobó dónde
declaran su versión los ecosistemas del principio, sobre repos reales:

| ecosistema | manifiesto | versión |
|---|---|---|
| Rust | `Cargo.toml` | `[workspace.package] version` |
| JS/TS | `package.json` | `version` |
| Python | `pyproject.toml` | `project.version` (PEP 621) o `tool.poetry.version` |
| JVM/Gradle | `gradle.properties` | `version=` |
| .NET | `Directory.Build.props` | `<Version>` |
| C/C++ | `CMakeLists.txt` | `project(... VERSION x.y.z)` |
| **Go** | `go.mod` | **no existe** — el módulo no lleva versión |
| **Bazel** | `MODULE.bazel` | **no existe** — la versión vive en un `version.bzl` propio |

Go y Bazel son dos de los ocho ecosistemas que el principio nombra
explícitamente, y **ninguno de los dos tiene dónde declarar su versión**. Un
registro «manifiesto → versión» no puede cubrirlos sin inventarles un fichero
que no existe.

La salida fácil —una segunda rama para Gradle, una tercera para `package.json`—
ya está descartada por escrito en el propio doc comment de la función:

> Adding a second format here would be the same mistake as a third `cp -r` in
> the release staging (INC-DEBT-056) — a hard-coded list of places to look, each
> with its own failure mode.

---

## Decision

Un **contrato declarativo de fuente de versión**, en dos partes.

### 1. El registro: datos, no ramas

Cada ecosistema es una entrada de datos `VersionSourceSpec`:

- `ecosystem` — identificador estable.
- `manifest_paths` — candidatos, en orden de preferencia.
- `format` — uno de un conjunto **cerrado** de formatos de lectura
  (`Toml`, `Json`, `Properties`, `Xml`, `Cmake`, `PlainText`).
- `locator` — **dónde** está el valor, como datos: una ruta de claves
  (`["workspace","package","version"]`, `["version"]`) o, para los formatos
  con una etiqueta, el nombre de la etiqueta.
- `kind` — la clase de fuente (abajo).

Un lector genérico aplica `(format, locator)`. **No hay un `if` por ecosistema
en el código de resolución**: añadir un ecosistema es añadir una fila. Ese es
el criterio falsable que lo distingue de la lista codificada que el ADR quiere
evitar, y por eso hay un test que cuenta las entradas del registro y otro que
añade un ecosistema de prueba sin tocar código.

El precedente es `EcosystemProfileV1` de SPEC-043: «declarative, data-only
ecosystem description (no kernel code)». No se reutiliza ese tipo porque su
población en runtime no existe —`EcosystemProfileV1` no se construye fuera de
los tests—, y reutilizar un tipo sin población sería cablear el release a un
seam vacío.

### 2. Las dos clases de fuente

```text
Declares(manifest, format, locator)   →  hay una versión con la que comprobar
TagIsTheOnlyAuthority                 →  el ecosistema no declara versión;
                                        el tag ES la declaración
```

La segunda clase existe porque Go y Bazel no dan otra opción, y **no** es un
permiso para relajar el lockstep en general:

- Solo se concede a los ecosistemas cuya entrada la declara. Rust nunca la
  toma, y un test lo comprueba.
- El resultado distingue `CrossChecked` (había versión con la que comparar) de
  `TagIsTheOnlyAuthority` (no la había). Un release que solo pasó por la
  segunda clase **lo dice**, en vez de reportar un verde indistinguible del
  primero.

El coste se admite: para un proyecto Go, el lockstep no tiene contra qué
comprobar. Declararlo y exponerlo es preferible a fabricar una comprobación
contra un `git describe` que mentiría sobre lo que el proyecto declara.

### 3. La política de resolución, fail-closed

Dado el conjunto de fuentes candidatas detectadas:

| situación | resultado |
|---|---|
| exactamente una, y se lee | `CrossChecked` con esa versión |
| varias, **todas con la misma versión** | `CrossChecked`, declarando que hubo varias |
| varias, **con versiones distintas** | **error duro**, nombrando cada archivo, su ecosistema y su versión |
| ninguna detectada | **error duro**, listando los manifiestos buscados |
| la fuente declarada está pero **no se puede leer o parsear** | **error duro**, **sin** caer al siguiente candidato |

Ese último punto es la regla que más importa, y es la que se parece a
INC-DEBT-048 y ADR-0152: **un manifiesto roto no puede redirigir la autoridad
a otro sitio en silencio.** Si el `Cargo.toml` de un repo Rust está corrupto,
el resultado es un error, no un `package.json` que aparece como autoridad
porque se pudo leer.

### 4. La declaración explícita

Un repositorio polyglot no tiene respuesta automática, y no debe tenerla: qué
paquete se versiona al publicar es una decisión humana. `.sddk/version-source.json`
la declara; sin ella, un repo con versiones divergentes no se puede publicar.

Es una restricción deliberada. Prefiero «este repositorio necesita una
declaración» a «este repositorio se versiona como la primera cosa que se
encontró».

---

## Consequences

**Positive**

- `sddk release plan`/`apply` funcionan en los ocho ecosistemas del principio, o
  fallan diciendo por qué.
- La superficie de «dónde está la versión» está en **un** sitio, y es datos.
- Añadir un ecosistema no toca código de resolución.
- Un manifiesto corrupto falla en vez de cambiar de autoridad.
- Un release que solo pasó por `TagIsTheOnlyAuthority` es visible como tal.

**Negative / costes admitidos**

- Un repositorio polyglot con versiones divergentes **no se puede publicar**
  hasta que alguien escriba la declaración. Es un bloqueo deliberado.
- `TagIsTheOnlyAuthority` es un predicado más débil donde se concede, y hay que
  distinguirlo en cada consumidor o no sirve de nada.
- Un lector genérico por formato es código: los formatos del conjunto cerrado
  son código, aunque los ecosistemas sean datos. Un formato nuevo es código
  nuevo, pero **no una rama más en el lockstep**.
- Un manifiesto que declara la versión en un lugar no estándar (por ejemplo un
  `version.bzl` propio de Bazel) necesita su entrada; no se adivina.

**Lo que NO se decide aquí**

- Si la declaración explícita debe poder delegar en un subdirectorio del
  monorepo. Hoy cubre el caso «este repo declara cuál manda»; el caso «este
  monorepo tiene N paquetes versionados» es otro contrato.
- Si `spec-043` debe poblarse en runtime y estos dos registros deben fusionarse.
  Hoy son dos cosas **distintas** con dos consumidores, y unirlos sin
  poblaciones reales sería cablear el release a un seam vacío.

---

## Verification (falsable)

1. **El código que resuelve no nombra ningún manifiesto** — ni `Cargo.toml`, ni
   `package.json`, ni ninguno de los demás. Medido sobre el fichero entero: el
   `REGISTRY` **sí** los nombra, y tiene que, porque es el sitio donde vive el
   dato; lo que se prohíbe es que el *lector* los nombre. Verificado por
   `the_resolution_code_names_no_manifest`, que es **estructural** a propósito:
   un reader genérico que hardcodea un nombre se comporta *igual* mientras el
   nombre siga ahí, luego ningún test de comportamiento lo vería.

   > **Enmienda al redactar la aceptación.** El criterio estaba escrito como
   > «`Cargo.toml` no aparece en `version.rs`», y esa letra **no era
   > satisfacible por ninguna implementación correcta**: los tests de paridad
   > de Rust tienen que *construir* un `Cargo.toml` para comprobar que el
   > lockstep sigue igual que antes, y sus fixtures lo nombran. Medido: de las
   > trece apariciones del fichero, **cero** están en código de producción —seis
   > son fixtures, dos asserts sobre el mensaje de error y cinco comentarios
   > que cuentan la historia—. Se corrige el criterio para que diga lo que
   > significa, que es lo que el test hace cumplir.
2. Los ocho ecosistemas resuelven o fallan por una razón declarada, medido uno
   a uno sobre fixtures, no por inspección.
3. Añadir un ecosistema es solo datos: un test añade una entrada y comprueba que
   el lector la resuelve sin tocar código.
4. Un repo Rust se comporta **exactamente** igual que antes, incluido el
   mensaje de error. Un test de paridad lo fija.
5. Dos manifiestos con versiones distintas ⇒ error duro que nombra ambos.
6. Un manifiesto corrupto ⇒ error duro, y el test comprueba que **no** se
   consulta un candidato alternativo.
7. Go y Bazel ⇒ `TagIsTheOnlyAuthority`, distinto de `CrossChecked`.

**Cómo se comprueban los siete, uno a uno:**
`bash tests/test_adr_0153_criteria.sh` ejecuta cada criterio por separado y
reporta su propio veredicto, para que un `PASS` agregado no pueda tapar un
criterio rojo. Un criterio con varios tests exige que **pasen todos**. Estado
en la aceptación: **PASS=7 FAIL=0**, con el criterio 1 además **falsificado**
—inyectar un `root.join("Cargo.toml")` en el código de resolución lo hace fallar
y nombra el fichero y la regla—.
