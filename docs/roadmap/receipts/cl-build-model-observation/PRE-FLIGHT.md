# PRE-FLIGHT — VA9 `cl-build-model-observation`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-069`
**Fecha:** 2026-10-05
**Predecesor:** `version-coherence-068` cerrado en `a6278c26` (session-85)

---

## Readiness: READY

---

## Qué cierra este bloque

La medición de cierre de session-85 encontró que **SDDK no observa el
`ProductVersion` de PipelineK**, y que el refusal es correcto pero inútil:

MEDIDO en `pipeline-kotlin`:

- 37 targets, y SDDK se niega a elegir cuál es el producto.
- `v2/build.gradle.kts:75` declara `version = "0.47.0"` de verdad.
- SDDK responde `build.gradle.kts no es fuente de version declarable` →
  `authority: NotChecked`, `productVersion: none`.

Y el motivo del refusal está escrito en `version_provider.rs:180`:

> *Build scripts. Their version is only observable by EVALUATING the build's own
> model … so a reader for them belongs to a provider that can ask the tool, not to
> a pattern matcher.*

O sea: **el código ya sabe cuál es el arreglo**, y este bloque lo construye.

---

## Las mediciones que dan forma al bloque

Hechas antes de escribir nada, porque una de ellas cambia el diseño:

| Medición | Resultado |
|---|---|
| `gradle properties --offline`, DSL Groovy, fixture trivial | **exit 0, 3 s**, salida `version: 1.2.3` |
| lo mismo, **sin** declarar versión | `version: unspecified` |
| lo mismo, sin `build.gradle` | exit 0 |
| `gradle properties --offline`, **DSL Kotlin** | **FALLA en 3–9 s** dentro del compilador Kotlin de Gradle, sobre un script trivial |
| shim de asdf sin `.tool-versions` | `No version is set for command gradle`, exit 126 |

Tres consecuencias, y las tres están en el diseño:

1. **Preguntar a la herramienta funciona** y es rápido. 3 s por resolución es
   un coste real y hay que pagarlo a conciencia, no por defecto.
2. **`unspecified` es la ley del bloque.** La herramienta dice «no hay versión»
   con una palabra, y un provider que la tomara por una versión estaría
   inventando `unspecified` como si fuera `1.2.3`. Esa es la línea entre un
   provider y un reconocedor de patrones.
3. **El DSL Kotlin no compila en este entorno.** No es culpa del fixture: es
   `KotlinCompiler.kotlinCoreEnvironmentFor` reventando. El provider **no lo
   sortea** —no puede— y reporta el fallo con el mensaje de la herramienta,
   que es la regla que ya rige para `Invalid` en el resto del registry.

---

## Qué NO es objetivo

- **No se parsea ningún lenguaje de build.** Ni Kotlin DSL, ni Groovy, ni una
  expresión regular sobre ellos. El provider **pregunta** y lee la respuesta.
- **No hay fallback.** Si la herramienta no responde, el resultado es
  «no comprobado» o «ilegible», nunca un patrón. Un provider que degrada a
  regex reintroduce exactamente lo que `version_provider.rs:180` se niega a
  ser, y lo haría **en silencio**, que es la forma en que eso vuelve.
- **No se evalúa por defecto.** Invocar una herramienta externa es un coste y un
  acoplamiento al entorno; no se pagan sin que alguien los pida. El opt-in
  es una bandera, como `--naming` y `--role`, no un default del registry.
- **No se adapta ningún repo.** PipelineK no se toca. Este bloque le da a SDDK
  la capacidad de preguntar; que el proyecto conteste o no es suyo.

---

## Superficie declarada

| Fichero | Qué |
|---|---|
| `crates/sddk-gateway/src/version_provider.rs` | `BuildModelProvider`, registry con opt-in |
| `crates/sddk-cli/src/release_cmd.rs` | `--evaluate-build` y su `NOT_CHECKED` |
| `crates/sddk-gateway/tests/version_build_model_contract.rs` | contrato, nuevo |
| `docs/architecture/adrs/ADR-0163-*.md` | la decisión |

Y **no se toca** `version_inspection.rs` ni el reducer: el provider nuevo es un
`VersionResolverPort` más, y el reducer no tiene por qué enterarse de que
existe. Si el reducer necesitara un caso nuevo, el diseño estaría mal.

## Riesgo de datos

**Cero.** El provider solo lee: lanza `gradle properties`, que no escribe en el
proyecto evaluado —salvo su caché `~/.gradle`, que está fuera del árbol— y
devuelve una línea. El test mide el hash del árbol antes y después, como ya hace
`resolver_no_escribe_en_el_arbol`.

El riesgo real es de otro tipo: **el provider puede lanzar un proceso**. Eso es
superficie nueva de ejecución y por eso va **opt-in** y nunca por defecto.

## STOP conditions

1. Que la única forma de obtener la versión siga siendo un patrón. No lo es: se
   pregunta a la herramienta.
2. Que el provider necesite que el kernel sepa qué es Gradle. No: el kernel ve
   un `VersionResolverPort` más; el nombre de la herramienta vive en el adapter
   y llega por composición.
3. Que `unspecified` haya que distinguirlo con un caso especial en el reducer.
   No: es un `VersionProbe` que el reducer **ya** conoce (`Undeclared`).

## El riesgo de este bloque, escrito antes de empezar

El modo de fallo no es que el provider falle. Es que **funcione en los casos
fáciles y calle en los difíciles**, que es donde nadie lo mira:

- El fixture de prueba es Groovy y funciona. El proyecto real es Kotlin y no.
  Un contrato verde sobre Groovy y un provider que en un repo Kotlin falla
  silenciosamente es un contrato que no mide lo que dice medir.
- La herramienta responde rápido cuando todo va bien. Cuando algo va mal
  responde `unspecified` o un stacktrace, y un provider que mapea cualquiera de
  los dos a «no hay versión» convierte un fallo en una ausencia — **que es la
  clase de defecto que este bloque va a falsificar**.

La defensa es que el contrato tiene filas para los dos, y que la fila de fallo
exige el **mensaje de la herramienta**, no uno propio.
