---
id: INC-DEBT-051
title: "`sddk release plan` y `release apply` exigen `Cargo.toml` y abortan en repos no-Rust, contra el principio de agnosticismo declarado en AGENTS.md §2.3"
status: resolved
resolved_at: 2026-10-02
resolved_by: ADR-0153
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-65
component: release
surface: crates/sddk-engine/src/version.rs
cluster_id: CL-RELEASE
related: [INC-DEBT-050, INC-DEBT-049]
references:
  - crates/sddk-engine/src/version.rs
  - crates/sddk-cli/src/release_cmd.rs
  - AGENTS.md
  - docs/debt/INC-DEBT-050-REMOTE-CASE-NORMALIZATION-REASSIGNS-PROJECT-IDS-WITHOUT-MIGRATION.md
  - tests/cycle-artifacts/p-63676b11dc0ef88f/session62-c3m2-knowledge-basis-revise-identity/RECEIPT.md
fingerprint: "release_plan_apply_hardcode_cargo_toml_breaks_stack_agnosticism"
---

## Qué es

`sddk release plan` y `sddk release apply` llaman a
`ensure_version_lockstep` (`crates/sddk-engine/src/version.rs`), que **lee
`root.join("Cargo.toml")` sin alternativa**. En un proyecto que no es Rust el
comando aborta antes de hacer nada útil.

## Evidencia (OBSERVED, session-65, ejecutada sobre PipelineK)

`/var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin` es Kotlin/Gradle, sin
`Cargo.toml`:

```text
$ sddk release plan --tag v0.45.0
error: VERSION LOCKSTEP ERROR: could not read
  /var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin/Cargo.toml:
  No such file or directory (os error 2)
```

Medido comando por comando, para no generalizar desde un caso:

| Comando | Resultado en un repo Kotlin/Gradle |
|---|---|
| `sddk release plan --tag <v>` | **ABORTA** — `VERSION LOCKSTEP ERROR` |
| `sddk release apply` | **ABORTA** — mismo predicado (`release_cmd.rs:847`) |
| `sddk release dist` | no evaluado: falla antes por argumentos requeridos |
| `sddk release verify --prefix <p>` | no evaluado: falla por ruta inexistente |

El fallo es **fail-closed y limpio**, no un corrupción: no escribe nada. Eso es
lo correcto en el modo de fallo, y lo único que falla es la **cobertura**.

## Por qué es un defecto y no una limitación legítima

AGENTS.md §2.3 declara, en la sección de convenciones duras:

> Este repo es Rust, por eso sus adapters concretos usan Cargo; la política SDDK
> es **agnóstica de lenguaje/build/test runner** y debe funcionar igual en repos
> **JVM, JS/TS, Python, Go, .NET, C/C++, Bazel** o polyglot **mediante
> adapters/capabilities**.

`ensure_version_lockstep` **no consulta ningún adapter ni capability**: abre la
ruta `Cargo.toml` directamente. El principio no se relaja donde toca, se
incumple en el único punto por el que pasan `plan` y `apply`.

**Bazel está listado expresamente** en el principio, y Bazel y Gradle son el
mismo caso: build system no-Cargo con su propia noción de versión.

Esto lo convierte en una desviación de un principio declarado por el propio
proyecto, no en una limitación del tooling aceptable. **La distinción importa**:
una limitación se acepta y se documenta; una desviación de una convención dura
se corrige o se deroga explícitamente.

## Por qué no lo llevaba nadie

`release_cmd.rs` es el camino de release del propio SDDK, siempre sobre un repo
Rust. Los tests de `release` se ejecutan en fixtures Rust, donde `Cargo.toml`
siempre existe: **el caso no-Rust no está cubierto en ninguna parte**, igual que
pasó con el contrato declarado del pin (INC-DEBT-049) y con el golden pin del
remote (INC-DEBT-050).

Es la tercera vez en esta sesión que el fallo real está en **lo que nadie
probó**, no en lo que alguien rompió.

## Alcance y encuadne

**Lo que NO se hace aquí y por qué.** El operador tomó la decisión operativa: no
usar `sddk release plan`/`dist` para construir PipelineK, y continuar por **la
ruta autorizada del propio proyecto** (su build Gradle + el
`pipelinek-release-harness`). Eso es exactamente lo que corresponde cuando la
herramienta no cubre el caso: no forzar la herramienta, no saltarse el control.

**Lo que sí queda pendiente de código, y es una decisión de diseño:**

El arreglo natural es que la versión se resuelva por **adapter/capability**, con
Cargo como una implementación entre otras, y que `plan`/`apply` **no aborten**
sino que declaren explícitamente qué adapter usaron y qué no pudo verificar.
Eso decide tres cosas y ninguna corresponde a un agente:

1. ¿Qué pasa cuando el proyecto **no expone** versión por ninguna vía? Hoy aborta
   con un error poco informativo; la alternativa es continuar declarando que la
   comprobación no se pudo hacer, o seguir abortando pero nombrando el problema
   en términos de adapter.
2. ¿Cómo se declara la versión en un proyecto no-Rust? Un `version.txt` del
   harness, el primer tag, `gradle.properties`, el `package.json`… Es un
   **contrato nuevo**, no una implementación.
3. ¿El lockstep sigue siendo exigible? Para SDDK es una salvaguarda real; para
   un proyecto que no versiona en un fichero único, quizá la regla correcta sea
   otra.

Por eso esto se registra como **deuda abierta con SCOPE pendiente**, no como
fix.

## Falsificadores exigidos cuando se implemente

- **F56** — `release plan --tag v0.45.0` sobre un fixture **sin** `Cargo.toml`
  no debe abortar con `VERSION LOCKSTEP ERROR`; debe producir un plan que
  declare qué adapter resolvió la versión.
- **F57** — el mismo fixture **con** `Cargo.toml` debe seguir exigiendo lockstep
  (no relajar el caso que sí cubre).
- **F58** — un fixture no-Rust **sin ninguna fuente de versión** debe fallar con
  un mensaje que nombre el adapter ausente, no un `No such file or directory`
  sobre una ruta que el usuario nunca mencionó.
- **F59** — la salida debe **nombrar el adapter usado**, para que la comprobación
  sea auditable y no un acto de fe.

## Consecuencia operativa medida

Mientras esta deuda esté abierta, `sddk release plan` **no es utilizable** sobre
ningún proyecto no-Rust. Para un framework que se declara agnóstico, eso acota
el alcance real del comando a un tipo de proyecto. No es un defecto de
configuración de un usuario: es del binario que se distribuye.

## Verificación de vigencia (session-65d) — la deuda sigue siendo real

El objetivo de la sesión obliga a comprobar que los criterios siguen vigentes
antes de tratarla como deuda. Los cuatro se sostienen:

| criterio | estado medido |
|---|---|
| AGENTS.md §2.3 declara la política agnóstica y lista Bazel | **vigente** — `AGENTS.md:90` lo dice literalmente |
| `ensure_version_lockstep` abre `Cargo.toml` sin consultar adapter | **vigente** — `version.rs:51`, sin cambios |
| Los call sites de `release plan` / `release apply` | **vigentes** — `release_cmd.rs:668` y `:847` |
| Existe un seam de adapter para resolver la versión de un proyecto no-Rust | **no existe** — los únicos son `ContextAdapter`, `EditorAdapter` y `ReconcileAdapter`; ninguno toca versión ni build system |

**El arreglo sigue siendo un contrato nuevo**, no un bug local: no hay seam al
que cablearlo, hay que diseñarlo. Sigue siendo `decision_request`, no trabajo
mecánico.

### Corrección del registro: hay un tercer call site, y NO es un tercer defecto

La entrada nombra dos. Al verificar se encontró un tercero,
`release_cmd.rs:947`, dentro de `local_release_preconditions`:

```rust
let version_lockstep_passed = ensure_version_lockstep(&context.root, current_tag).is_ok();
```

A primera vista parece el peor de los tres —no aborta, se traga el error— y la
hipótesis era que la ruta `local` se saltase el lockstep sin comprobarlo. **Es falso,
y se comprobó antes de escribirlo:** `crates/sddk-gateway/src/release.rs:205`
rechaza con `ReleaseError::Precondition("version lockstep check did not pass…")`
cuando el flag llega a `false`. Los tres call sites fallan cerrado: dos abortan
con el mensaje, el tercero anota y el gateway se niega.

Se consigna aquí para que nadie lo re-investigue como si fuera un agujero.

### Lo que el operador ya decidió

La ruta operativa para PipelineK no usa `sddk release plan` ni `dist`: se
continúa por la ruta autorizada del propio proyecto (build Gradle +
`pipelinek-release-harness`). Esa decisión **no cierra** esta deuda —acota su
alcance— y lo que queda pendiente sigue siendo la misma pregunta de diseño:
cómo resuelve su versión un proyecto que no tiene `Cargo.toml`.
# Addendum — sesión 65i: el parser del lockstep leía la versión de una DEPENDENCIA

> Añade a INC-DEBT-051. **No cierra la entrada**: el defecto de agnosticismo
> sigue abierto y verificado. Lo que se arregla aquí es un defecto **distinto**
> que la misma línea de código producía, y que solo apareció al ejercitar el
> parser con formas de fichero que ningún test cubría.

## Lo que se encontró

Los cuatro criterios de INC-DEBT-051 se verificaron vigentes contra el árbol
actual, incluido el caso real:

```
$ sddk release plan --tag v0.45.0     # en pipeline-kotlin (Kotlin/Gradle)
error: VERSION LOCKSTEP ERROR: could not read
  /var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin/Cargo.toml: No such file or directory
```

Y al leer la implementación aparece algo que la entrada no nombra: el
`lockstep` no «lee `Cargo.toml`», **parsea `Cargo.toml` a mano, línea a
línea**, dentro de una función de release. Y ese parser tiene tres defectos,
ninguno cubierto.

### (d) Leía la versión de una tabla de DEPENDENCIAS

```rust
if trimmed.starts_with('[') {
    in_workspace = trimmed.starts_with("[workspace");
}
```

`[workspace.dependencies]` **empieza por `[workspace`**. Con esa tabla antes
de `[workspace.package]` — orden legal, y el que emite el propio Cargo
cuando las dependencias se declaran primero — el parser leía la clave
`version` de una **dependencia** y la devolvía como versión del proyecto.

RED medido contra el código real, antes del arreglo:

```
assertion `left == right` failed: the lockstep read a dependency's version instead of the project's
  left: "9.9.9"
 right: "1.42.5"
```

**El fallo era silencioso**, que es lo que lo hace grave: no abortaba,
devolvía un veredicto seguro construido sobre un número que describe otra
cosa. Un tag `v9.9.9` habría **autorizado un release** sobre la versión de
una dependencia; un tag `v1.42.5` —el correcto— habría sido rechazado.

### (e) Abortaba en un `Cargo.toml` válido

El parser solo despejaba comillas dobles. `version = '1.2.3'` es TOML
válido —lo acepta el propio Cargo— y hacía fallar el release. RED medido:
`None` sobre un fichero que `toml::from_str` parsea sin dificultad.

### (f) Abortaba en repos de un solo crate

Una librería sin `[workspace]` declara su versión en `[package]`. El parser
solo miraba tablas `[workspace*]`, así que abortaba con un mensaje que
mencionaba `[workspace]` en un fichero que no tenía ninguna. Es **la imagen
invertida del defecto original**: el mismo predicado, en el otro sentido.

## Por qué no se arregló con «más formatos»

El arreglo mínimo podía haber sido añadir `maven.xml`, `gradle.properties` y
`package.json` a una lista. **Eso habría sido el mismo error una vez más**:
una lista escrita a mano de dónde mirar, cada entrada con su propio modo de
fallo, y otra que se desincroniza del resto. Es exactamente lo que produjo
los defectos (a) y (b) de esta entrada con las superficies del bundle
(INC-DEBT-056), donde la lista escrita a mano fue la quinta copia del
contrato.

Lo que sí es un arreglo del **sustituto**: usar el parser TOML real (`toml`,
la misma dependencia que `sddk-cli` ya usaba) y decidir la precedencia de
las tablas de forma explícita y total:

| orden | tabla | por qué |
|---|---|---|
| 1 | `[workspace.package].version` | la declaración del workspace |
| 2 | `[workspace].version` | workspaces antiguos, y lo que escriben los tests de este repo |
| 3 | `[package].version` | repositorio de un solo crate, sin workspace |

Ninguna es «la primera clave `version` que aparezca». Con la lista
explícita, añadir un formato nuevo es una entrada más **en el mismo sitio
que ya define la precedencia**, no un `if` más en el parser.

## Falsificadores: 4 mutaciones, 4 detectadas

| mutación | test que la mata |
|---|---|
| volver al prefijo de tabla (`[workspace*]`) | `lockstep_uses_the_project_version_not_a_dependencies` |
| quitar el fallback a `[package]` | `lockstep_does_not_confuse_package_and_workspace_tables` |
| quitar el fallback a `[workspace]` | `lockstep_passes_when_tag_matches_workspace_version` |
| degradar el error de parseo a tabla vacía | `lockstep_errors_when_cargo_toml_missing` y el resto |

La cuarta merece nombre: `unwrap_or(Table::default())` sobre un parseo
fallido convierte un error tipado en «no hay versión», que es la misma
suplantación que el `prompts_count = 0` de INC-DEBT-052 y el
`is_empty()` de INC-DEBT-054. Un fallo de parseo debe nombrar el fichero y
el error, no disfrazarse de ausencia.

## Un test que cambió de opinión

El primer RED de (d) forzaba `unwrap_err()` y luego inspeccionaba
`err.workspace_version`. Con el arreglo en su sitio el lockstep **pasa**
correctamente, y el test fallaba por eso: estaba afirmando el camino
interno, no la propiedad que ve quien llama.

Reescrito para afirmar el veredicto en las dos direcciones: el tag del
proyecto se **acepta**, y el de la dependencia se **rechaza**. La segunda
mitad es la que importa — si alguna vez volviera a leer `9.9.9`, el release
quedaría autorizado.

Es la segunda vez en esta sesión que un test RED resultó ser una descripción
incorrecta del comportamiento, y la segunda vez que la corrección no era
tocar el código sino **afirmar la propiedad en vez del camino**.

## Lo que sigue abierto

INC-DEBT-051 **no se cierra**. `ensure_version_lockstep` sigue leyendo
`Cargo.toml`, y un proyecto Kotlin, Gradle, Maven, npm o Bazel sigue
abortando. Cerrarlo requiere el **contrato que la entrada ya pedía**: la noción
de «dónde declara un proyecto su versión», que hoy no existe en ninguna parte
del engine — `grep -rln "project_version" crates/*/src/` → cero.

Ese contrato, sus tipos, su carga por lenguaje y su punto de integración son
un ciclo propio con SCOPE-CONTRACT y ADR. Lo que se ha hecho aquí es quitar
un defecto que hacía que el arreglo futuro fuera más difícil de verificar: hoy
el lockstep es correcto **para Rust**, y se puede demostrar con tests.

## Resolucion (2026-10-02) — ADR-0153, con los cuatro falsificadores medidos

Los cuatro falsos exigidos cuando se implementara (F56–F59), ejecutados uno a
uno contra el binario de este checkout y no deducidos:

| falsificador | medido | resultado |
|---|---|---|
| **F56** — un fixture **sin** `Cargo.toml` no aborta, produce plan | fixture Go, `go.mod` y nada mas | `exit 0`, plan producido, `version_authority.kind = tag_is_the_only_authority` |
| **F57** — el mismo fixture **con** `Cargo.toml` sigue exigiendo lockstep | tag correcto / tag discrepante | `exit 0` con `cross_checked` nombrando `Cargo.toml`; `exit 1` con `VERSION LOCKSTEP FAILED: project=1.0.0 vs tag=9.9.9` |
| **F58** — sin ninguna fuente, el error nombra lo que se busca y **no** un `No such file or directory` | repo vacio | `exit 1`, lista los 13 manifiestos buscados **y** la declaracion explicita; **cero** `No such file or directory` |
| **F59** — la salida nombra quien resolvio la version | los dos fixtures | proyecto Go: `ecosystems: ["go"]`; proyecto Rust: `candidates[0] = Cargo.toml (rust) = 1.0.0` |

**Reconciliacion de redaccion, declarada en vez de omitida:** F56 y F59 dicen
«adapter». El contrato elegido **no tiene adapters**: ADR-0153 es un registro
de **ecosistemas** con formatos declarativos, y por decision propia (criterio 3
de ese SCOPE) anadir un ecosistema es **solo datos**. La propiedad que F59
persigue —que la comprobacion sea auditable y no un acto de fe— se cumple con
el nombre del ecosistema y del manifiesto leido. Lo que cambia es el sustantivo,
no la exigencia.

**F58 estaba a medio camino y no se dio por bueno por parecer resuelto.** El
criterio 8 del SCOPE del lote 1 exigia que el error listara donde se busco **y
nombrara la declaracion explicita**; el codigo solo hacia la primera mitad, y el
test solo afirmaba la primera mitad — dos mitades de un mismo casi que se
confundian con el todo. Cerrado: el mensaje nombra `.sddk/version-source.json` y
el test lo mide.

**Estado de los siete criterios de aceptacion de ADR-0153:** `PASS=7 FAIL=0`,
ejecutados **uno a uno** por `bash tests/test_adr_0153_criteria.sh`, que reporta
el veredicto de cada criterio por separado para que un verde agregado no pueda
tapar uno rojo. El criterio 1 —«el codigo que resuelve no nombra ningun
manifiesto»— ademas esta **falsificado**: inyectar un `root.join("Cargo.toml")`
en el codigo de resolucion lo hace fallar.

**Lo que esto NO afirma.** `release plan` y `release apply` sobre un proyecto
no-Rust **no se han ejecutado de extremo a extremo contra un forge real**: F56,
F57 y F59 se midieron con la ruta local, que no necesita red. La ruta forge
comparte el mismo contrato de version y su parte de registro esta cubierta, pero
publicar de verdad por forge es una medicion que no se ha hecho.
