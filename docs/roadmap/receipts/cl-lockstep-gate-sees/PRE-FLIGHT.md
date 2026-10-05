# PRE-FLIGHT — VA10 `cl-lockstep-gate-sees`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-070`
**Fecha:** 2026-10-05
**Predecesor:** VA9 `b9b59069` (ADR-0163), cerrado en `7ec3710d`
**Adenda:** la primera redacción de este documento afirmaba un `fail-open` en
`crates/sddk-engine/src/version.rs:190`. Esa afirmación era **falsa** y está
refutada con evidencia abajo. Lo que realmente justifica este bloque es otro, y
es más fuerte.

---

## Readiness: READY

---

## La refutación, primero

La primera redacción de este documento decía:

> **Un tag `v9.9.9` sobre un proyecto que declara `1.2.3` pasa la puerta.**
> `version.rs:190-191 … return Ok(authority)`. Un **fail-open con testigo medido**.

No es cierto. `refusal()` (`crates/sddk-engine/src/version.rs:230-305`) se
ejecuta en `:182`, **antes** del `let Some(...)` de `:190`, y devuelve `Some(...)`
—o sea, `Err`— para tres de los cuatro casos:

| Variante de `VersionAuthority` | `refusal()` | Puerta |
|---|---|---|
| `Unresolved` | `Some` en `:295` | **rechaza** |
| `Ambiguous` | `Some` en `:244` | **rechaza** |
| `Invalid` | `Some` en `:258` | **rechaza** |
| `ReleaseRefIsAuthority` | `None` en `:237` | pasa |

El único brazo que alcanza `:190` con `version() == None` es
`ReleaseRefIsAuthority`, y ese exige una **declaración explícita**: la fila
`go.mod` de `DEFAULT_DECLARATIONS` lleva `tag_is_authority: true`
(`crates/sddk-gateway/src/version_provider.rs:229`), igual que `MODULE.bazel` y
`WORKSPACE`; o el JSON `"authority": "tag"` de `:551`. El código lo dice con su
propio vocabulario: *"A declaration of absence, not silence."*

La medición que la primera redacción daba por testigo dice `productVersion: none`,
que tiene **cuatro** causas con cuatro veredictos distintos. Saltar de «ninguno» a
«pasa la puerta» fue un salto deYL, no una observación. La puerta nunca tuvo un
fail-open.

**Consecuencia para este bloque:** la capacidad sigue siendo necesaria, pero por
otro motivo. Ese motivo está medido en la sección siguiente.

---

## Qué cierra este bloque, con la premisa correcta

VA9 dio a SDDK la capacidad de **preguntar** al build tool, y la conectó **solo
al diagnóstico**. Lo que pasa es que para un build Gradle o Maven real eso no es
una limitación del informe: es que **no hay nada que autorizar**.

MEDIDO, sobre `/tmp/gradle-probe-groovy` (un build Gradle Groovy con `build.gradle`
y `settings.gradle`, sin `gradle.properties`):

```
$ sddk release version inspect --root .
providers_considered:  <14 providers>
providers_answering:   none
authority:             NotChecked
productVersion:        none
  - sddk.gateway.declaration-file/build.gradle  build.gradle no es fuente de version declarable
```

Los 14 providers responden, 13 con «no está en este target» y
`build.gradle` con «no es fuente de version declarable»
(`version_provider.rs:185-195`: `extraction: None`, porque la versión de Gradle
solo se conoce **evaluando** el build). `Unresolved` sale de ahí, y `Unresolved`
lo rechaza la puerta.

Es decir: **un proyecto JVM no puede publicar un release por SDDK.** No porque la
puerta sea permisiva, sino porque no hay evidencia y sin evidencia no hay
autorización.

MEDIDO, el mismo repo con la capacidad de VA10 conectada a la puerta:

```
$ sddk release version inspect --root . --evaluate-build
providers_considered:  …, sddk.gateway.build-model/gradle
providers_answering:   sddk.gateway.build-model/gradle
  - sddk.gateway.build-model/gradle [Observed] 1.2.3 declared at ././build.gradle
authority:             Observed
productVersion:        1.2.3
```

Eso es lo que cierra este bloque: **la capacidad llega a la puerta, y un build
Gradle pasa de «no hay nada que autorizar» a «hay una versión contra la que
comparar».**

---

## El defecto que este bloque también tiene que arreglar

`--build-tool` acepta cualquier nombre y **no es lo que dice**.

MEDIDO, con un ejecutable instrumentado en `PATH` sobre el mismo repo Gradle:

```
$ sddk release version inspect --root . --evaluate-build --build-tool mvn
providers_answering: sddk.gateway.build-model/mvn
  - sddk.gateway.build-model/mvn [Observed] 3.3.3 declared at ././build.gradle

$ cat argv.txt
properties
--offline
```

Dos cosas falsificadas a la vez:

1. **`mvn` recibió los flags de Gradle.** `properties --offline` es un goal
   inexistente en Maven (allí sería `help:evaluate -Dexpression=project.version`).
   El dato sale de `crates/sddk-cli/src/release_cmd.rs:2090-2094`, que fija los
   args sin mirar qué herramienta se pidió.
2. **El informe atribuyó la versión a `././build.gradle`**, un fichero que Maven
   nunca tocó. La atribución viene de `GRADLE_BUILD_FILES` (`version_provider.rs:775`),
   que decide aplicabilidad en términos de Gradle con independencia de la
   herramienta que se invocó.

O sea: la bandera nombra una herramienta, ejecuta otra cosa y **atribuye la
respuesta a un fichero que la herramienta no abrió**. Eso no es una limitación, es
evidencia fabricada — y en un proyecto cuya ley central es no mentir sobre lo que
se comprobó, es el peor tipo de defecto posible.

Las tres caras están atadas a «gradle» y hoy son tres constantes sueltas:
`BuildModelInvocation.program` (`:2091`), `GRADLE_BUILD_FILES` (`:775`) y
`parse_build_model` (`:824`). Separar una de las otras dos produce exactamente lo
que se midió arriba.

---

## Decisión

Dos cosas, atadas porque una sin la otra no arregla nada:

1. **`--evaluate-build` y `--build-tool` llegan a `release plan`, `release apply`
   y `ship`.** El registry que se pasa a la puerta es el que el operador pidió, y
   el default no cambia: sin la bandera no se pregunta, y el informe lo dice.

2. **`--build-tool` deja de aceptar cualquier cadena.** Pasa a ser un dialecto
   declarado, y un dialecto ata **programa, args, ficheros de build, parser y
   centinela**. Un nombre que no se reconoce es un error de la línea de comandos,
   no un Gradle silencioso.

El segundo punto es lo que hace honesta la primera capacidad: sin él,
`--evaluate-build` es la bandera que alguien activaría esperando que su
herramienta fuera preguntada, y lo que ocurriría es otra.

---

## Qué NO es objetivo

- **No se toca `ensure_release_ref_lockstep`.** La pregunta y su respuesta son
  correctas: `Unresolved` se rechaza y esa es la conducta debida.
- **No se evalúa por defecto.** Sigue costando 3 s y una JVM por target.
- **No se convierte el silencio en rechazo.** Ya lo es; este bloque no cambia eso.
- **No se añaden herramientas.** El dialecto declara las que se han medido. Una
  herramienta nueva es una fila con su medida, no una bandera que acepta
  cualquier nombre.

---

## Superficie

| Fichero | Qué |
|---|---|
| `crates/sddk-gateway/src/version_provider.rs` | `BuildToolDialect` que ata las tres caras; el provider toma dialecto, no invocación suelta |
| `crates/sddk-cli/src/release_cmd.rs` | los dos flags en `ReleaseArgs` y `ship`; `--build-tool` parseado como dialecto, sin default |
| `crates/sddk-gateway/tests/version_build_model_contract.rs` | los mutantes del dialecto |
| `crates/sddk-cli/tests/release_lockstep_sees_build_model.rs` | el testigo de puerta, falsificado |

**Ningún cambio en el dominio ni en el motor.** Si hiciera falta, el diagnóstico
estaría mal; no lo está.

---

## Riesgo de datos

**Cero.** Solo cambia qué registry se le pasa a una función que ya era pura, y
qué se acepta como nombre de herramienta.

## STOP conditions

1. Que el arreglo obligara a cambiar la semántica de la puerta. No: solo cambia
   su entrada.
2. Que hubiera que tocar el reducer o `version_inspection`. No.
3. Que un dialecto declarado tuviera que hardcodearse en el motor. No: vive en
   el gateway, que es donde vive el resto del conocimiento de herramientas.

## El riesgo de este bloque, escrito antes de empezar

El modo de fallo **no** es que la puerta siga autorizando lo que no debía —no
tenía ese defecto— . Es **convertir el silencio en una falsa autorización con
mejor apariencia**: que un proyecto sin evidencia suficiente pase a tener una
versión porque alguien escribió el nombre de una herramienta en la línea de
comandos.

Por eso el riesgo se mide en las dos direcciones, y por eso `--build-tool` no
puede significar «lo que sea»: un nombre no reconocido tiene que ser un error
visible, no un Gradle en disguise.
