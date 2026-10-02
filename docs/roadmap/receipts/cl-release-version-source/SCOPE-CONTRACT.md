# CL-release-version-source — SCOPE-CONTRACT: dónde declara su versión un proyecto

**Cycle:** `p-63676b11dc0ef88f/version-source` (cluster `CL-RELEASE`, de INC-DEBT-051)
**Baseline:** `main@512fbf3f` (workspace v2.5.3, declarada no publicada; último tag remoto v2.5.2)
**Opened:** 2026-10-02T10:30:00Z
**Owner:** orchestrator (ejecución directa)
**Authority basis:** AGENTS.md §3 (gates preautorizados) y el goal activo
**Closes:** INC-DEBT-051 (high/P1)

## 0. Vigencia verificada antes de escribir nada

El goal exige no tratar como deuda real una alerta cuyos criterios caducaron.
**Reproducido con el binario de este checkout (2.5.3, `512fbf3f`), no heredado:**

```text
$ sddk release plan --root /var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin --tag v0.45.0
error: VERSION LOCKSTEP ERROR: could not read
  /var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin/Cargo.toml: No such file or directory (os error 2)
exit 1
```

El repo tiene `build.gradle.kts`, `settings.gradle.kts` y `gradle/`, y **no**
tiene `Cargo.toml`. Criterio vigente.

## 1. Objetivo (falsable)

Que `sddk release plan` y `sddk release apply` resuelvan la versión declarada de
un proyecto **sin que ninguna función de release nombre un fichero de
manifesto concreto**, para al menos: Rust, JS/TS, Python, JVM/Gradle, Go,
.NET, CMake/C++ y Bazel.

**Criterio de salida, falsable por construcción:** el identificador
`Cargo.toml` **desaparece** de `crates/sddk-engine/src/version.rs` y de la
ruta de release, y los ocho ecosistemas se resuelven a través de un contrato
declarativo. Falsificadores: (a) borrar el registro declarativo debe hacer
caer la suite; (b) añadir un ecosistema nuevo debe ser **solo datos**, sin
tocar código de resolución — se comprueba con un test que lee el registro.

## 2. El hallazgo que decide el diseño

**No todos los ecosistemas declaran su versión en un manifiesto.** Se verificó
sobre los repos reales de esta máquina antes de diseñar nada:

| ecosistema | ¿dónde está la versión? |
|---|---|
| Rust | `Cargo.toml` → `workspace.package.version` |
| JS/TS | `package.json` → `version` |
| Python | `pyproject.toml` → `project.version` (PEP 621) o `tool.poetry.version` |
| JVM/Gradle | `gradle.properties` → `version=` |
| .NET | `Directory.Build.props` → `<Version>` (o el `.csproj`) |
| C/C++ (CMake) | `CMakeLists.txt` → `project(... VERSION x.y.z)` |
| **Go** | **`go.mod` no declara versión.** No hay dónde ponerla. |
| **Bazel** | **`MODULE.bazel` no declara versión.** Convención: un `version.bzl` propio. |

Un registro de "manifiesto → versión" **no puede cubrir Go ni Bazel**, que
están nombrados explícitamente en el principio de AGENTS.md §2.3. Y la salida
fácil —una lista codificada de formatos dentro de `ensure_version_lockstep`— es
exactamente lo que su propio doc comment descarta:

> Adding a second format here would be the same mistake as a third `cp -r` in
> the release staging (INC-DEBT-056) — a hard-coded list of places to look, each
> with its own failure mode.

Por eso el contrato tiene **dos** clases de fuente, no una.

## 3. No-objetivos

- **NO** hacer que `release plan` adivine la versión de un repositorio polyglot.
  Un repo con dos manifiestos que declaran versiones distintas no tiene
  respuesta automática, y un humano tiene que decidir qué paquete se versiona.
- **NO** relajar el lockstep en Rust. Para un repo Rust el predicado se
  cumple exactamente igual que antes; si algo cambia ahí, es una regresión.
- **NO** inventar un sistema de adapters nuevo. `EcosystemProfileV1` (SPEC-043)
  ya es el precedente de «descripción declarativa de ecosistema, sin código de
  kernel», y este contrato se le parece a propósito.
- **NO** tocar `sddk-engine` fuera de `version.rs` y el módulo nuevo.

## 4. Superficie

| fichero | cambio |
|---|---|
| `crates/sddk-engine/src/version_source.rs` | **nuevo**: registro declarativo + lector genérico + política de resolución |
| `crates/sddk-engine/src/version.rs` | `ensure_version_lockstep` pregunta al contrato; deja de abrir `Cargo.toml` |
| `docs/architecture/adrs/ADR-0153-…` | el contrato, con sus dos clases de fuente y el coste admitido |

## 5. Plan de test (scoped)

1. Rust: el lockstep se cumple y **falla** igual que antes (paridad medida).
2. Cada ecosistema del registro extrae su versión de su manifiesto.
3. Añadir un ecosistema es **solo datos**: test que lee el registro y cuenta.
4. Repo polyglot con versiones **distintas** ⇒ error duro nombrando ambos.
5. Repo polyglot con versiones **iguales** ⇒ OK, y dice que hubo varias.
6. Manifiesto **ilegible o corrupto** ⇒ error duro, **no** cae al siguiente
   candidato en silencio.
7. Go y Bazel ⇒ `TagAuthoritative`: el lockstep no tiene contra qué comprobar,
   y eso se **declara**, no se simula.
8. Cero candidatos ⇒ error que lista los manifiestos buscados y nombra la
   declaración explícita.

Cada criterio lleva falsificador. Un criterio sin falsificador ejecutable no
cuenta (§5 de AGENTS.md y el precedente de C3m).

## 6. STOP conditions

- Si el lockstep de un repo **Rust** cambia de comportamiento ⇒ parada. Este
  trabajo no puede mejorar la cobertura a costa del predicado que ya funciona.
- Si hace falta **un `if` por ecosistema** dentro de `version.rs` ⇒ parada. Es
  la forma que el contrato existe para evitar, y un test la delata.
- Si un manifiesto roto hace que la resolución pase a otro candidato ⇒ parada.
  Es la clase «autoridad equivocada en silencio», la misma que ADR-0152
  acababa de cerrar para la identidad.
- Si el ADR se propone como `proposed` sin la decisión y los costos ⇒ no se
  implementa.

## 7. Riesgos

| riesgo | por qué importa | mitigación |
|---|---|---|
| Un registro declarativo se convierte de facto en una lista codificada | es el mismo defecto con otro nombre | el registro es **datos puros** y un test cuenta los Adaptadores declarados; añadir uno es añadir una entrada |
| `TagAuthoritative` se lee como «lockstep desactivado» | lo es, en parte, y hay que decirlo | el resultado distingue `CrossChecked` de `TagIsTheOnlyAuthority`, y ambos se exponen |
| El fallo de Go/Bazel se tapa declarando tag autoritativo en todos | dejaría el predicado vacío en todas partes | `TagAuthoritative` está permitido **solo** para los ecosistemas cuya entrada lo dice, y el criterio 1 comprueba que Rust nunca lo toma |
| Un polyglot queda sin poder publicar | inconvenient, no peligroso | es el punto: sin declaración humana no hay respuesta correcta |

## 8. Fuera de alcance

- INC-DEBT-050 / 049 (cerradas o en curso en C3m).
- La clave del KMS, que bloquea v2.5.3 y es del operador.
- Publicar la release: primero verde local, después release.
