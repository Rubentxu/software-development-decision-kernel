---
id: ADR-0163-PREGUNTAR-AL-BUILD-TOOL-NO-LEER-SU-SCRIPT
title: La versión de un build se pregunta a la herramienta, y no sale de un patrón sobre su script
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-069
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-069
supersedes: null
superseded_by: null
extends: [ADR-0157, ADR-0158, ADR-0159, ADR-0160, ADR-0161, ADR-0162]
component: release
surface: crates/sddk-gateway/src/version_provider.rs
closes: []
---

# ADR-0163 — Preguntar al build tool, no leer su script

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0157 (la versión se resuelve preguntando a providers),
  ADR-0161 (el veredicto explica su porqué)
- **Superficie:**
  - `crates/sddk-gateway/src/version_provider.rs` — `BuildModelProvider`,
    `BuildModelInvocation`, `parse_build_model`, `version_registry_with`
  - `crates/sddk-cli/src/release_cmd.rs` — `--evaluate-build`, `--build-tool`
- **Cierra** la medición de `session-85`, que encontró que SDDK no observaba el
  `ProductVersion` de PipelineK.

---

## Contexto

`version_provider.rs:180` llevaba desde VA3 diciendo por qué un build script no
era fuente de versión:

> *Build scripts. Their version is only observable by EVALUATING the build's own
> model … so a reader for them belongs to a provider that can ask the tool, not to
> a pattern matcher.*

Es decir: el código sabía el arreglo y no lo tenía. MEDIDO en `session-85` sobre
el repo que motivó la frase, `v2/build.gradle.kts`:

- El monorepo tiene **37 targets**, y SDDK se niega a elegir cuál es el producto.
- `v2/build.gradle.kts:75` declara `version = "0.47.0"`, de verdad.
- Y el fichero menciona `version` **tres veces en comentarios** —una de ellas con
  un `0.44.0` que el proyecto ya había corregido— antes de la asignación real.

**Un patrón se habría llevado un número que el proyecto ya dejó atrás.** No es
hipotético: está en el fichero que dio la frase.

---

## Decisión

### 1. Un provider más, y el reducer no se entera

`BuildModelProvider` es un `VersionResolverPort` más, exactamente como los otros.
**No se toca `version_inspection.rs` ni `reduce`.** Si el reducer hubiera
necesitado un caso nuevo, el diseño estaría mal: el provider traduce «lo que dijo
la herramienta» a un `VersionProbe`, que el reducer ya sabe reducir.

### 2. Preguntar es **opt-in**, y el informe lo declara

MEDIDO: `gradle properties --offline` tarda **3 segundos** y levanta una JVM, por
target. Ponerlo en el registro por defecto significa que cada diagnóstico de un
repositorio JVM paga eso por cada target, y que un target colgado retrasa el
diagnóstico que se pidió *precisamente porque algo va mal*.

Así que es `--evaluate-build`, y **sin** ella el informe lleva:

> *the build tool was NOT asked what its model says; that costs a process per
> target, so it happens only with `--evaluate-build`*

Sin esa línea el informe sería internamente contradictorio: declararía el
provider en `providers_considered` —que es lo que hace, porque el registro se
imprime entero— nombrando uno que no se ejecutó.

### 3. `--build-tool`, porque `gradle` y `./gradlew` son dos hechos

MEDIDO: el shim de asdf **exige** `.tool-versions` y sin él responde `No version
is set for command gradle` y sale 126. Un proyecto con wrapper y uno sin wrapper
no son el mismo caso, y suponer cuál es es la misma operación que este bloque
viene desarmando en todas las capas anteriores.

### 4. **No hay fallback**

Si la herramienta no responde, la respuesta es que no respondió. Nunca un
patrón. Un provider que degrada a «me lo leo yo» reintroduce el defecto **en
silencio**, que es la forma en que vuelve.

---

## La ley del bloque: `unspecified` no es una versión

MEDIDO: Gradle contesta `version: unspecified` cuando el build no declara
versión.

Esa palabra **no** es un valor. Un provider que la tomara por una versión
publicaría una release llamada `unspecified` — y nadie vería un error, porque el
número saldría limpio. Es la forma más silenciosa de mentir que tiene una
herramienta.

Y hay un segundo hecho que la distingue de un silencio: el provider **no
necesita un caso nuevo** para ella. Es un `VersionProbe::Undeclared`, que el
reducer ya reduce. La herramienta diciendo «no tengo» y el provider oyendo «no
contestó» son dos hechos distintos, y por eso `parse_build_model` tiene tres
salidas y no dos.

---

## El defecto que la medición encontró en el provider nuevo

MEDIDO, ejecutándolo: la primera versión reporting el motivo de un fallo de
Gradle tomaba **la primera línea no vacía** del `stderr`, y esa línea es:

```
FAILURE: Build failed with an exception.
```

Que es cierta para **todos los fallos de Gradle que han ocurrido jamás**. Es
decir: un provider que fallaba cerrado con un motivo que no distinguía nada —
exactamente el defecto que `ProviderError::Unavailable` dice evitar, cometido por
el provider que existe para arreglar al anterior.

Corregido leyendo **después** de `What went wrong:`, con caída al primer motivo
útil cuando una herramienta no usa esa frase de Gradle. El motivo que ahora
llega es del propio Gradle: `Another Gradle invocation is already using this v2
checkout`, que además **cambió el diagnóstico** y obligó a re-medir el Kotlin DSL
para confirmar que su fallo era real y no un artefacto de concurrencia. Lo fue.

---

## Consecuencias

### Positivas

- SDDK observa la versión de un proyecto Gradle **preguntándola**, y el valor
  viene con la procedencia del proyecto que contestó —MEDIDO: desde un
  subdirectorio Gradle responde `Project ':lib-b'` y da **su** versión, no la del
  padre—.
- El provider es **hermético en tests**: usa una herramienta falsa, así que la
  ley se falsifica sin JVM. Una ley que necesita la herramienta instalada para
  falsificarse es una ley que nadie falsifica.

### Negativas y costeadas

- **Opt-in significa que la mayoría de los informes no lo saben.** Es el coste de
  no pagar 3 s por target sin pedirlo, y el informe lo dice en vez de callarlo.
- **El DSL Kotlin no compila en este entorno** (MEDIDO, reproducible: 2 s, motivo
  `25.0.4.1` dentro de `KotlinCompiler.kotlinCoreEnvironmentFor`). El provider no
  lo sortea —no puede— y reporta el fallo con el mensaje de la herramienta. Se
  declara, no se oculta: es una limitación del entorno, no del código.

### Deuda declarada, no abierta aquí

- Solo hay invocación para **Gradle**. La abstracta
  (`BuildModelInvocation { program, args }`) ya admite cualquier otra, y añadirla
  es una bandera más —pero no se ha medido ninguna, y un provider sin medir es
  un provider que se supone.

---

## Verificación

| # | Ley | Suite |
|---|-----|-------|
| B1 | La versión la manda la herramienta, con su procedencia | `b1_la_version_la_manda_la_herramienta` |
| B2 | `unspecified` **no** es una versión | `b2_unspecified_no_es_una_version` |
| B2b | El proyecto sin versión dice cuál y que no tiene | `b2b_el_proyecto_sin_version_dice_cual_y_que_no_tiene` |
| B3 | El motivo no es el banner | `b3_el_motivo_no_es_el_banner` |
| B3b | Una herramienta que no usa la frase de Gradle también habla | `b3b_una_herramienta_que_no_usa_la_frase_de_gradle_tambien_habla` |
| B4 | Una herramienta ausente no es una que falló | `b4_una_herramienta_ausente_dice_que_no_esta` |
| B5 | Preguntar no escribe en el árbol | `b5_preguntar_no_escribe_en_el_arbol_del_proyecto` |
| B6 | Un directorio que no es build no se consulta | `b6_un_directorio_que_no_es_build_no_se_consulta` |
| B7 | La ley vive en una función pura | `b7_la_ley_vive_en_una_funcion_pura` |
| C1 | Sin la bandera, el informe declara que no se preguntó | `sin_la_bandera_el_informe_declara_que_no_se_pregunto` |
| C2 | Con la bandera, la versión llega de la herramienta | `con_la_bandera_la_version_llega_de_la_herramienta` |
| C3 | Una herramienta que no existe no se confunde con una que falla | `una_herramienta_que_no_existe_no_se_confunde_con_una_que_falla` |

### Falsificación

**La ley está en una función pura a propósito**, y esa es la decisión de diseño
que más cosas evita: `parse_build_model` es un `&str → BuildModelAnswer`, y sus
mutantes se ejecutan sin lanzar nada. B7 la casa contra la **forma real** de la
salida, incluida la línea `runtimeVersion: 11.0` que un `contains` en vez de un
`==` se habría llevado.

**B2 lleva el mutante escrito al lado**, y muere donde el código no: el mutante
devuelve `unspecified` como versión, el código real no devuelve ninguna.

Y **B6 mide algo más de lo que parece**: que un directorio sin fichero de build
propio no gaste un proceso. MEDIDO, ese criterio separa `docs/guia` —que sin él
lanzaría 3 s para que Gradle contestara `Project directory '…' is not part of the
build`— de `lib-b`, un submódulo que **sí** declara versión propia y que sin él
se perdería.
