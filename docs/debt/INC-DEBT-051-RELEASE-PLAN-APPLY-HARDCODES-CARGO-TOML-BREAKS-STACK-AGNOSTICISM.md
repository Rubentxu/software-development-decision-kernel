---
id: INC-DEBT-051
title: "`sddk release plan` y `release apply` exigen `Cargo.toml` y abortan en repos no-Rust, contra el principio de agnosticismo declarado en AGENTS.md §2.3"
status: open
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
