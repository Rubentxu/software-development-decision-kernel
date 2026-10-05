# Cierre de `cl-build-model-observation` (VA9)

**Fecha:** 2026-10-05
**Commit:** `b9b59069` · ADR-0163
**Estado:** cerrado y publicado (`32891f20`)

---

## Qué mide esto

VA9 cierra la medición de `session-85`, que encontró que SDDK no observaba el
`ProductVersion` de un proyecto Gradle y que el motivo —un rechazo correcto—
estaba escrito en el propio código desde VA3.

Todas las cifras de aquí son **MEDIDAS antes de escribir una línea**, porque tres
de ellas cambiaron el diseño.

---

## Las cinco mediciones previas

| Medición | Resultado | Qué obligó |
|---|---|---|
| `gradle properties --offline`, fixture Groovy trivial | **exit 0, 3 s**, `version: 1.2.3` | Preguntar es viable y es rápido: vale un provider |
| lo mismo, sin declarar versión | `version: unspecified` | La ley del bloque: eso **no** es una versión |
| desde un submódulo `lib-b` | `Project ':lib-b'`, `version: 2.0.0` | `current_dir = target` basta; Gradle resuelve por cwd |
| desde `docs/guia` (sin build file propio) | exit 1, `Project directory '…' is not part of the build` | hace falta un criterio de **presencia** para no gastar 3 s en nada |
| DSL Kotlin, fixture trivial | **exit 1, 2 s**, `25.0.4.1` dentro del compilador Kotlin de Gradle | Se declara como límite del entorno; no se sortea |

Y una sexta, de entorno: el shim de asdf **exige** `.tool-versions` y sin él
responde `No version is set for command gradle` y sale 126. De ahí `--build-tool`.

---

## El defecto que encontró la medición, en el provider NUEVO

La primera version que reportaba el motivo de un fallo de Gradle tomaba la primera
línea no vacía del `stderr`, y esa línea es:

```
FAILURE: Build failed with an exception.
```

Cierta para **todos los fallos de Gradle que han ocurrido jamás**. Es decir: un
provider que fallaba cerrado con un motivo que no distinguía nada — el defecto que
`ProviderError::Unavailable` dice evitar, cometido por el provider que existe
para arreglar al anterior.

Corregido leyendo después de `What went wrong:`, con caída al primer motivo útil
para herramientas que no usan esa frase de Gradle.

**Y corregirlo cambió el diagnóstico, lo que obligó a re-medir.** El motivo que
ahora llega es el de verdad, y no el de un compilador: `Another Gradle
invocation is already using this v2 checkout`. Esa es una razón para no dar por
buena la medición anterior sin volver a mirarla.

---

## Estado final medido sobre PipelineK

| Momento | Qué dice SDDK |
|---|---|
| antes de VA9 | `build.gradle.kts no es fuente de version declarable` → `NotChecked` |
| con `--evaluate-build` | **`Invalid`**, con el motivo del propio Gradle: `Another Gradle invocation is already using this v2 checkout` |

El cambio es de **especie**, no de grado: antes SDDK no miraba; ahora **mira,
pregunta, y nombra lo que le pasó**. Lo que queda bloqueado es el **entorno** —
otra invocación de Gradle tiene ese checkout tomado, y no se tocan los demonios
del usuario para demostrarlo.

Verificado además, en el camino feliz:

- `chronos` (Rust): sigue observándose sin tocar nada.
- Fixture Groovy: `productVersion: 1.2.3` desde la herramienta.
- Submódulo `lib-b`: contesta `2.0.0`, **no** el `9.9.9` del padre.
- `unspecified`: `NotChecked` con el proyecto nombrado — **no** una versión.

**PipelineK sigue intacto y sin `.sddk/version-source.json`.**

---

## Deuda declarada, no abierta

- **Solo hay invocación para Gradle.** `BuildModelInvocation { program, args }`
  ya admite cualquier otra y añadirla sería una bandera más — pero ninguna está
  medida, y un provider sin medir es un provider que se supone.
- **El DSL Kotlin no compila en este entorno.** Es del entorno, no del código, y
  el provider lo reporta en vez de rodearlo.
