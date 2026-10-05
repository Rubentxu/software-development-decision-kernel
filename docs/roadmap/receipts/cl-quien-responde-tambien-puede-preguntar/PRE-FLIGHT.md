# PRE-FLIGHT — VA12 `cl-quien-responde-tambien-puede-preguntar`

**Cycle:** `p-63676b11dc0ef88f/version-coherence-072`
**Fecha:** 2026-10-05
**Predecesor:** VA11 `1621c3fb` + `a84f79aa` (ADR-0165), publicado en `origin/main`

---

## Readiness: READY

---

## Qué falta, medido

VA11 dio a un proyecto la forma de declarar su versión sin que SDDK sepa su
tecnología. EsoOpens una pregunta que la propia capacidad de VA9 —*preguntar al
build tool*— dejó a medias en la superficie de la CLI.

**Tres comandos responden sobre la versión. Uno puede preguntar. Los otros dos no.**

### MEDIDO 1 — `release version matches` da la misma respuesta a dos tags distintos

`/tmp/gradle-probe-groovy`, `build.gradle` con `version = '1.2.3'`, Gradle instalado
y respondiendo:

```
$ sddk release version matches --tag v1.2.3      # el tag CORRECTO
productVersion: none
matches: false
detail: this target declares no product version, so there is nothing to compare against

$ sddk release version matches --tag v9.9.9      # el tag claramente INCORRECTO
productVersion: none
matches: false
detail: this target declares no product version, so there is nothing to compare against
```

**Byte a byte, la misma salida.**

Eso no es un mensaje impreciso: es un comando **cuya respuesta no depende de lo
preguntado**. La pregunta que existe —«¿nombra `v1.2.3` a la versión del producto?»—
no se ha hecho, y la salida tiene la forma de una respuesta. Un comando que devuelve
una constante no está midiendo la pregunta; está devolviendo su propia ausencia de
pregunta.

Y el `detail` afirma un hecho **falso**: el target **sí** declara versión, en un
fichero que SDDK localizó —el propio informe del rechazo de `release plan` lo nombra
como `build.gradle no es fuente de version declarable`— y que decidió no leer porque
la única forma honesta de leerlo es preguntarle a Gradle.

### MEDIDO 2 — el mismo repo, con `release plan` sí, y dice lo contrario

```
$ sddk release plan --tag v1.2.3 --evaluate-build --build-tool gradle
version_authority: resolved
release_target_provenance: la raiz del repositorio declara su propia version
```

El mismo repositorio, el mismo tag. Un comando dice «resolved» y el otro dice «no hay
nada contra lo que comparar». **El segundo no puede tener razón**, porque no preguntó
a quien tenía la respuesta.

### MEDIDO 3 — `release handoff` se niega por la misma causa

```
$ sddk release handoff --tag v1.2.3 --external-type digest-list --external-digest deadbeef
this target declares no product version, so there is nothing to hand off. Run
`sddk release version inspect` to see what was and was not looked at: no provider
produced a version.
```

El sobre que el productor entrega a quien certifica se construye con `productVersion`,
y el productor tiene prohibido rellenarlo a mano. Sin pregunta, no hay sobre — y el
motivo («no provider produced a version») vuelve a ser falso: el provider existe, y
nadie lo llamó.

---

## La causa, y es una sola

`release_cmd.rs:701-704`, escrito por el propio VA10:

```rust
// `release version matches` NO tiene `--evaluate-build`: MEDIDO, sus Args
// no declaran la bandera. Asi que aqui no hay pregunta que hacer, y se
// dice con el valor en vez de dejar que se deduzca de un default.
let ask = BuildAsk::never();
```

La observación es cierta y la conclusión no. «Sus `Args` no declaran la bandera» es
una descripción de por qué **no se añadió**, no una razón por la que **no debía
añadirse**. Lo que se escribió fue un límite de alcance convertido en diseño.

Y hay una asimetría medida que lo vuelve indefendible: `release version inspect` —
el comando **nacido para explicar qué se miró y qué no**— **sí** declara las dos
banderas (`release_cmd.rs:202-205`). El diagnóstico puede preguntar y el veredicto no.

---

## Decisión

**Dos cambios, y el segundo es el que importa.**

### 1. `--evaluate-build` y `--build-tool` en los dos `Args` que no los declaran

`VersionMatchesArgs` y `HandoffArgs` reciben las mismas dos banderas que ya tienen
`ReleaseArgs` y `VersionInspectArgs`. Sin default, por la razón que ADR-0164 escribió:
preguntar cuesta 3 s y levanta una JVM, y lo que se paga sin pedirlo es lo que nadie
revisa.

**Y no se toca el reducer ni el motor.** Es la misma `BuildAsk` que `plan` ya
construye, por la misma función `BuildAsk::of_parts`. Un segundo camino para decidir
si se observa sería el defecto que este bloque viene a cerrar.

### 2. Cuando no se preguntó, el mensaje lo dice

Es la parte que no es mecánica, y es la que hace que el bloque no sea cosmético.

`detail` e `error` hoy dicen **«this target declares no product version»**. Con la
bandera ausente, ese texto es falso siempre que la respuesta sería distinta de haber
preguntado. Y hay una categoría de target donde es literalmente falso: uno que
declara su versión en `build.gradle`, que SDDK encuentra y decide no leer.

La ley es la de VA11 y la de VA9, y es la misma:

> **Un mensaje cierto para todos los casos no distingue ninguno, y el operador va a
> la reparación equivocada.**

Aquí la reparación equivocada es peor que en los casos anteriores, porque es
**invisible**: el operador ve `matches: false` y `productVersion: none`, y lo lee
como «mi proyecto no declara versión», que es un hecho sobre su repo y no sobre lo
que SDDK miró.

El texto tiene que distinguir **tres** hechos y hoy distingue uno:

| hecho | texto |
|---|---|
| nadie preguntó al build tool | lo dice, y nombra la bandera |
| se preguntó y la herramienta no tiene versión | lo dice |
| el target declara que la lleva la release ref | ya lo dice, y es cierto |

---

## El criterio que hace falsable el bloque

**La respuesta de `matches` tiene que depender del tag.**

Eso es lo que hoy no ocurre, byte a byte, y es una aserción que se puede escribir sin
interpretación: con el build observado, `v1.2.3` casa y `v9.9.9` no, y las dos
salidas **no** pueden ser iguales. Un bloque que solo añade banderas es indistinguible
de un bloque que no arregla nada, porque el síntoma es una salida que no se mueve.

---

## STOP conditions

1. **Que tocara el reducer o el motor.** No: es la misma `BuildAsk` que `plan` ya usa.
2. **Que un proyecto que hoy publica dejara de publicar.** Eso sí se mide, en las dos
   direcciones: sin la bandera el comportamiento tiene que ser el mismo salvo el
   texto, y con la bandera tiene que coincidir con lo que `plan` ya dice.
3. **Que cambiar la respuesta de `matches` sin bandera.** No es objetivo: sin
   preguntar no hay versión, y decirlo distinto no es mejorarlo, es taparlo.
4. **Que el coste de preguntar saliera de este bloque.** No: `--evaluate-build` sigue
   siendo opt-in.

## El riesgo de este bloque, escrito antes de empezar

El riesgo no es que se rompa `plan`. Es que **`matches` y `handoff` se conviertan en un
segundo camino**: si el texto nuevo distingue tres hechos, cada uno necesita su
prueba, y tres textos escritos a mano son tres lugares donde el siguiente caso se
coloca en el equivocado.

La forma de vigilarlo: **una sola función** que produzca el texto, con una fila por
hecho, y un falsador que mute esa función entera. Si aparece una segunda rama con un
texto propio, es el defecto del bloque.

---

## Lo que NO se hace aquí, escrito para que no se expanda por inercia

- **No se hace que `matches` pregunde por defecto.** Preguntar cuesta; ADR-0164 ya
  decidió eso y no es de este bloque revisarlo.
- **No se amplía `DEFAULT_DECLARATIONS`.`build.gradle` sigue sin ser una fuente
  declarable, porque leer un build script con un regex es el defecto que originó
  VA9.
- **No se toca `release plan`.** Ya pregunta. Este bloque le quita una excusable a los
  otros dos.