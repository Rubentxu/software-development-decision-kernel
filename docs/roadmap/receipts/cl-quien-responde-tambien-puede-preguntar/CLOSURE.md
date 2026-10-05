# Cierre de `cl-quien-responde-tambien-puede-preguntar` (VA12)

**Fecha:** 2026-10-05
**Ciclo:** `p-63676b11dc0ef88f/version-coherence-072` · ADR-0166
**Estado:** cerrado y publicado

---

## Qué mide esto

VA9 dio a SDDK la capacidad de **preguntar** al build tool. VA10 le ató las cinco
caras de una herramienta. Y el hueco que ninguno de los dos cerró es que esa
capacidad **no llegaba a todos los comandos que la necesitan**.

Tres comandos responden sobre la versión de un producto. Uno podía preguntar. Los
otros dos contestaban **sin poder preguntar**, y su respuesta no lo decía.

---

## La medición que loopened, y era de las que no se ven leyendo

`/tmp/gradle-probe-groovy`, `build.gradle` con `version = '1.2.3'`, Gradle
instalado y respondiendo:

```
$ sddk release version matches --tag v1.2.3      # el tag CORRECTO
productVersion: none
matches: false
detail: this target declares no product version, so there is nothing to compare against

$ sddk release version matches --tag v9.9.9      # el que el proyecto contradice
productVersion: none
matches: false
detail: this target declares no product version, so there is nothing to compare against
```

**Byte a byte, la misma salida.**

Un mensaje impreciso se puede arreglar. Esto no: un comando cuya respuesta no
depende de lo preguntado **no está midiendo la pregunta**, y ningún ajuste de
redacción lo arregla.

### El segundo hecho, que es una contradicción entre dos comandos

```
$ sddk release plan --tag v1.2.3 --evaluate-build --build-tool gradle
version_authority: resolved
```

El mismo repositorio, el mismo tag. Uno dice «resolved» y el otro «no hay nada
contra que comparar». El segundo no puede tener razón, porque no preguntó a quien
la tenía.

### El tercero, y es el peor porque no necesita ninguna herramienta

`Cargo.toml` en `4.2.0` **y** `.sddk/version-source.json` en `9.9.9`:

```
productVersion: none
matches: false
detail: this target declares no product version, so there is nothing to compare against
```

**`this target declares no product version` es exactamente lo contrario de lo que
pasa.** El target declara *dos* versiones y se contradicen.

Y mientras, en la misma invocación y sobre el mismo repo,
`sddk release version inspect` ya lo decía bien:

```
authority: Conflict
  - sddk.gateway/.sddk/version-source.json [Observed] 9.9.9 declared at .sddk/version-source.json
  - sddk.gateway.declaration-file/Cargo.toml        [Observed] 4.2.0 declared at Cargo.toml
```

`matches` miraba el mismo veredicto y lo tiraba.

---

## El hueco ya estaba declarado tres veces

| dónde | qué decía |
|---|---|
| `cl-lockstep-gate-sees/CLOSURE.md:147` | «`matches` y `handoff` **no preguntan**: sus `Args` no declaran la bandera» |
| `ADR-0164:156` | lo mismo, en la deuda que ADR-0164 dejó escrita |
| `cl-project-declares-its-version/CLOSURE.md:144` | lo mismo, un bloque después |

Tres veces nombrado, tres veces cerrado el bloque sin tocarlo. **Declarar un
defecto tres veces no es cerrarlo**, y esa es la razón de que este bloque exista:
el patrón de este repo es que un defecto que solo se declara vuelve, y lo que lo
cierra es quitarle la posibilidad de reaparecer.

---

## Decisión, y su criterio

**Un tipo, no dos campos por `Args`.** `BuildAskArgs` lleva las dos banderas y los
cuatro `Args` lo aplanan. Copiar las banderas es copiar la deriva; con un tipo
único **no se puede** volver a tener un comando que responda sin poder preguntar.

De paso caen los tres constructores de `BuildAsk` —`of`, `of_inspect`,
`of_parts`—, porque la razón que el propio código daba para tenerlos era «no un
`From` porque dejaría abierta la pregunta de dónde salió esto». Al aplanar un
único tipo **la razón desaparece**: ahora sí se sabe de dónde sale.

**`BuildAsk::never()` desaparece, y su ausencia es la prueba.** Los dos comandos
que lo llamaban leen ahora `of_parts(&args.ask)`, luego el constructor se queda
sin un solo uso. No es limpieza: es la prueba estructural de que ya no existe el
camino de código que produce una `BuildAsk` sin banderas.

**`measured`, porque arreglar solo el texto no cerraba nada.** Si preguntar y no
preguntar dieran el mismo `matches: false` y solo cambiasen las frases, la salida
estructurada seguiría sin depender de lo que se hizo — **el mismo defecto un
nivel más arriba**—. `measured` es `false` **solo** cuando nadie ha mirado lo
suficiente. Un `Conflict` con sus dos declaraciones es **lo más mirado que hay**,
no lo menos.

MEDIDO antes de tocar la forma del JSON: ningún script, workflow ni otro
repositorio lee el JSON de estos dos comandos; los únicos consumidores son tests
Rust y la documentación.

---

## Estado final medido, sobre el binario

| caso | antes | después |
|---|---|---|
| Gradle, tag correcto, sin preguntar | `matches: false`, «declares no product version» | `measured: false` + «did not ask the build tool» con las dos banderas |
| Gradle, tag correcto, `--evaluate-build --build-tool gradle` | **imposible** | `productVersion: 1.2.3`, `matches: true` |
| Gradle, tag erróneo, **con** la pregunta | **imposible** | `productVersion: 1.2.3`, `matches: false` — **la salida cambia** |
| dos fuentes en desacuerdo | «declares no product version» | **nombra las dos** (4.2.0 y 9.9.9) y dice que se contradicen |
| fuente ilegible | «declares no product version» | «a source exists and could not be read» |
| `go.mod` + `authority: "tag"` | correcto | **intacto**, y ahora con `measured: true` |
| `handoff` sobre build Gradle | se negaba a construir el sobre | **`productVersion: 1.2.3` en el sobre** |
| `--build-tool sbt` | n/a | error que dice qué sí sabe preguntar: `gradle, maven` |

### El camino real, con los comandos exactos

```bash
cd /tmp/gradle-probe-groovy
BIN=/var/home/rubentxu/wt-target/debug/sddk
SEED="--fallback-seed 00000000-0000-4000-8000-000000000001"

$BIN release version matches --root . --no-infer --tag v1.2.3 $SEED \
     --evaluate-build --build-tool gradle
# productVersion: 1.2.3 / matches: true / measured: true

$BIN release handoff --root . --no-infer --tag v1.2.3 $SEED \
     --external-type digest-list --external-digest deadbeef \
     --evaluate-build --build-tool gradle
# productVersion: 1.2.3 / envelope_sha256: sha256:dd59480a...
```

---

## Los falsadores, y qué encontró cada uno

**11/11** en `crates/sddk-cli/tests/release_version_matches_asks.rs`, cada uno
cayendo por su propia comprobación.

| mutación | qué rompe | cae |
|---|---|---|
| **M1** | `measured` siempre `true` | 1 test |
| **M2** | los cuatro textos vuelven al único viejo | **5 tests** |
| **M3** | `handoff` vuelve a `version_registry()` | 1 test (el fitness de fuente) |

### Un defecto que introdujo el arreglo

`handoff` añadía su propia frase «Run `sddk release version inspect`…» encima de
un texto que **ya** remitía ahí. Salía dos veces, con punto y todo. Un mensaje que
dice dos veces lo mismo no informa más: entrena al lector a leer el primero y
saltar el segundo — **ese defecto es de VA10 y lo corrigió entonces**; repetirlo
aquí sería perderlo en la traducción. Hay un test que lo cuenta: el que falla si
la pista sale más de una vez.

### Un helper que se rompió a sí mismo

El helper de salida compone `sddk release …` con `--root` y `--scope`. La primera
versión los ponía **antes** de la ruta del subcomando, y los ocho tests que la
usaban caían con `unexpected argument '--root' found`. Como el helper de aserción
exige salida 0, ese error **de la herramienta** se reportaba como un fallo de la
ley.

Un helper que construye mal la línea de comandos no mide la ley: la esconde
detrás de un error suyo. Es la cuarta vez en cinco bloques que el instrumento es
lo que se rompe primero.

---

## El límite del bloque, declarado y no maquillado

**Ningún test hermético cubre que `handoff` use `ask.registry()`.** Los dos
registros solo se distinguen cuando `evaluate` es verdad, y eso exige arrancar un
proceso con un programa que se busca en `PATH` — y
`crates/sddk-cli/src/dev/update.rs:26` ya dejó escrito que en edition 2024
`set_var` es `unsafe` y **ningún test puede tocar `PATH`**.

Se cubre con un **fitness de fuente con su control**: mira el **receptor** de la
llamada (`ask.registry()` y `version_registry()` se distinguen ahí, y
`version_registry_asking` —legítimo— no se llama así), y el control planta el
defecto en una copia del cuerpo para exigir que el escáner lo vea. Sin ese
control, el test de arriba pasaría igual de verde con un escáner que no lee nada.

Es un fitness, y un fitness es **un instrumento más débil que un
comportamiento**. Se declara como límite del bloque, no como resultado. La
razón está escrita porque el repo ya se ha comido dos fitness que disparaban
sobre algo legítimo: el de `-> VersionNaming` que marcaba `-> Option<VersionNaming>`
—una función capaz de decir «no tengo ninguna», justo lo opuesto a tener un
default— y el que buscaba `candidates[0]`.

---

## Una medición caducada que no se arrastró

El doc de `ReleaseArgs` afirmaba que sin `--evaluate-build`,
`ensure_release_ref_lockstep` devolvía `Ok` con un tag que el proyecto
contradecía. **Es falso**: `refusal()` corre en `version.rs:182`, antes del
`let Some` de `:190`, y devuelve `Err` para `Unresolved`, `Ambiguous` e `Invalid`.

Es la premisa que la adenda correctiva de ADR-0164 ya retiró. No se reescribe
porque medir de nuevo sale de otro bloque, pero **no se arrastra tampoco**: una
medición que el código contradice, escrita al lado de la bandera que se iba a
reutilizar, volvería a tener autoridad por vecindad. Es la segunda vez en dos
bloques que un `PRE-FLIGHT` afirma algo falso y hay que corregirlo en el sitio.

---

## Una limitación del guard, medida

Los dos escáneres de contaminación —cirílico/CJK y CamelCase en prosa— **no cazan
la basura sin cambio de caja**. Se coló `instrumentoménos fuerte que un
comportamiento`, que es indecible y no tiene transición minúscula→mayúscula. Se
encontró leyendo, no escaneando.

Un guard de contaminación que no ve un tipo de contaminación es un guard medio
activado, que es como un guard desactivado se parece a uno que pasa. Se declara
aquí; el arreglo es de otro bloque.

---

## Lo que sigue abierto, y es lo que sigue

- **La tabla sigue decidiendo quién publica si el proyecto no se declara.** Sin
  cambios, y sigue siendo una decisión de **política**.
- **`release version matches` y `release handoff` ya no son un hueco declarado**:
  está cerrado aquí. Las dos referencias que quedaban en deuda
  (`cl-lockstep-gate-sees/CLOSURE.md:147` y el `CLOSURE` de VA11) son historia y
  este documento es el que manda.
- **Sin cambios y sin tocar:** seis de los siete targets built-in sin cuerpo,
  `AdapterFact`/`EvidenceResolver` con 532 líneas sin consumidor, cero
  observabilidad estructurada, cinco de cinco workflows de CI en
  `workflow_dispatch`, 553 `unwrap`/`expect` en producción.

Nada de eso se resuelve aquí, y ninguno lo bloquea.