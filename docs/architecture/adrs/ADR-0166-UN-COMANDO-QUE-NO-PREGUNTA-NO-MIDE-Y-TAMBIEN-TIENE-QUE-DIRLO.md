---
id: ADR-0166-UN-COMANDO-QUE-NO-PREGUNTA-NO-MIDE-Y-TAMBIEN-TIENE-QUE-DIRLO
title: Un comando que no pregunta no mide, y un false que significa «no lo sé» no puede tener la forma de un false
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-072
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-072
supersedes: null
superseded_by: null
extends: [ADR-0165, ADR-0164, ADR-0163, ADR-0161]
component: release
surface: crates/sddk-cli/src/release_cmd.rs, crates/sddk-cli/src/ship.rs, crates/sddk-cli/src/lib.rs
closes: []
---

# ADR-0166 — Un comando que no pregunta no mide

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0165 (declarar o no entender), ADR-0164 (el dialecto ata las
  cinco caras), ADR-0163 (preguntar al build tool), ADR-0161 (un veredicto no
  explica su propio porqué)
- **Ciclo:** `cl-quien-responde-tambien-puede-preguntar`

---

## Contexto

Tres comandos responden sobre la versión de un producto. Uno podía preguntar. Los
otros dos no.

### MEDIDO 1 — la respuesta no dependía del tag

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

**Byte a byte, la misma salida.** Eso no es un mensaje impreciso: es un comando
**cuya respuesta no depende de lo preguntado**. No mide la comparación, contesta
que no ha mirado, con la forma de una respuesta.

### MEDIDO 2 — el mismo repo, con `plan` sí, y dice lo contrario

```
$ sddk release plan --tag v1.2.3 --evaluate-build --build-tool gradle
version_authority: resolved
```

El mismo repositorio, el mismo tag. Un comando dice «resolved» y el otro «no hay
nada contra que comparar». El segundo no puede tener razón, porque no preguntó a
quien la tenía.

### MEDIDO 3 — el peor, y no necesita ninguna herramienta

`Cargo.toml` en `4.2.0` **y** `.sddk/version-source.json` en `9.9.9`:

```
productVersion: none
matches: false
detail: this target declares no product version, so there is nothing to compare against
```

**`this target declares no product version` es exactamente lo contrario de lo que
pasa.** El target declara *dos* versiones y se contradicen, y `release version
inspect` —el comando hermano, en la misma invocación, sobre el mismo repo— ya lo
dice bien:

```
authority: Conflict
  - sddk.gateway/.sddk/version-source.json [Observed] 9.9.9 declared at .sddk/version-source.json
  - sddk.gateway.declaration-file/Cargo.toml [Observed] 4.2.0 declared at Cargo.toml
```

`matches` lo tiraba.

### La causa: un límite de alcance disfrazado de diseño

`release_cmd.rs:701-704`, escrito por el propio VA10:

```rust
// `release version matches` NO tiene `--evaluate-build`: MEDIDO, sus Args
// no declaran la bandera. Asi que aqui no hay pregunta que hacer, y se
// dice con el valor en vez de dejar que se deduzca de un default.
let ask = BuildAsk::never();
```

La observación es cierta y la conclusión no. «Sus `Args` no declaran la bandera»
describe por qué **no se añadió**, no por qué **no debía añadirse**.

Y había una asimetría que lo volvía indefendible: `release version inspect` —el
comando **nacido para explicar qué se miró y qué no**— sí declaraba las dos
banderas. El diagnóstico podía preguntar y el veredicto no.

### Y el hueco ya estaba declarado tres veces

`cl-lockstep-gate-sees/CLOSURE.md:147`, ADR-0164:156 y el `CLOSURE` de VA11. Tres
veces nombrado y tres veces cerrado el bloque sin arreglarlo. **Declarar un defecto
tres veces no es cerrarlo**, y esa es la razón de que este bloque exista.

---

## Decisión

### 1. Un tipo, no dos campos por `Args`

`BuildAskArgs` lleva las dos banderas y los cuatro `Args` lo aplanan: `release`,
`release version inspect`, `release version matches`, `release handoff`.

Copiar las banderas es copiar la deriva. Con un tipo único **no se puede** volver
a tener un comando que responda sin poder preguntar, que es el defecto.

De paso caen los tres constructores de `BuildAsk` —`of`, `of_inspect` y
`of_parts`—, porque la razón que el propio código daba para tenerlos era «no un
`From` porque dejaría abierta la pregunta de de dónde salió esto». Al aplanar un
único tipo **la razón desaparece**: ahora sí se sabe de dónde sale.

### 2. `BuildAsk::never()` desaparece, y su ausencia es la prueba

Los dos comandos que lo llamaban leen ahora `of_parts(&args.ask)`, luego el
constructor se queda sin un solo uso. **No es limpieza: es la prueba estructural de
que ya no existe el camino de código que produce una `BuildAsk` sin banderas.**

Y un segundo defecto, independiente, que la falta de banderas escondía: `handoff`
resolvía con `version_registry()` y **no** con `ask.registry()`. Poner las
banderas sin cambiar el registro habría dado un `handoff` que acepta
`--evaluate-build` y lo ignora, que es **peor** que no aceptarlas.

### 3. `measured`: un `false` que significa «no lo sé» no puede tener la forma de un `false`

MEDIDO antes de tocar la forma: ningún script, workflow ni otro repositorio lee el
JSON de estos comandos; los únicos consumidores son tests Rust y la documentación.

Si solo se arreglara el texto, preguntar y no preguntar darían el mismo
`matches: false` y solo cambiarían las frases. La salida estructurada seguiría sin
depender de lo que se hizo —**el mismo defecto un nivel más arriba**—. Por eso el
campo va en el campo.

`measured` es `false` **solo** cuando nadie ha mirado lo suficiente: no se preguntó
al build tool y ningún fichero respondió. En cualquier otro caso se miró, y lo que
se encontró —nada, una contradicción, un fichero ilegible— es un hecho. Un
`Conflict` con sus dos declaraciones es **lo más mirado que hay**, no lo menos.

### 4. `sin_version`: una función, cinco hechos

| autoridad | `measured` | qué dice |
|---|---|---|
| `ReleaseRefIsAuthority` | sí | la lleva la referencia — el único texto que ya era cierto |
| `Ambiguous` | sí | **nombra las dos declaraciones** y se niega a elegir |
| `Invalid` | sí | una fuente existe y no se pudo leer, que no es silencio |
| `Unresolved` + preguntado | sí | se preguntó y no había |
| `Unresolved` + no preguntado | **no** | no se preguntó, con las dos banderas nombradas |

Una función y no cinco textos porque cinco textos escritos a mano son cinco sitios
donde el siguiente caso se coloca en el equivocado.

`handoff` usa **la misma** función, porque un productor que lee «no hay nada que
entregar» cuando lo que hay es una contradicción va a arreglarlo en el sitio
equivocado.

---

## Consecuencias

**Lo que cambia para el usuario.** `matches` y `handoff` aceptan `--evaluate-build`
y `--build-tool`. Con ellas, `handoff` construye el sobre sobre un build Gradle con
`productVersion: 1.2.3` — **imposible antes**.

**Lo que no cambia.** Preguntar sigue siendo opt-in y `--build-tool` sigue sin
default. `release plan` no se toca: ya preguntaba. `DEFAULT_DECLARATIONS` no se
amplía: `build.gradle` sigue sin ser una fuente declarable, porque leer un build
script con un regex es el defecto que originó VA9.

**Una medición caducada que no se arrastra.** El doc de `ReleaseArgs` afirmaba que
sin `--evaluate-build` `ensure_release_ref_lockstep` devolvía `Ok` con un tag que el
proyecto contradecía. **Es falso**, y es la premisa que la adenda correctiva de
ADR-0164 ya retiró: `refusal()` corre en `version.rs:182`, antes del `let Some` de
`:190`. No se reescribe porque medir de nuevo sale de otro bloque, pero no se
arrastra tampoco: una medición que el código contradice, escrita al lado de la
bandera que se va a reutilizar, volvería a tener autoridad por vecindad.

---

## Lo que queda declarado, no abierto

**Ningún test hermético cubre que `handoff` use `ask.registry()`.** Los dos
registros solo se distinguen cuando `evaluate` es verdad, y eso exige arrancar un
proceso con un programa que se busca en `PATH` — y
`crates/sddk-cli/src/dev/update.rs:26` ya dejó escrito que en edition 2024 ningún
test puede tocar `PATH`. Se mide con un **fitness de fuente con su control**, que
mira el receptor de la llamada y planta el defecto en una copia para exigir que el
escáner lo vea. Y el camino real queda medido sobre fixture, con los comandos
exactos en el `CLOSURE`.

Es un fitness, y un fitness es **un instrumento más débil que un
comportamiento**: mira el texto del fuente y no lo que el programa hace con él.
Se declara como límite del bloque, no como resultado.

---

## Alternativas descartadas

**Arreglar solo el texto.** Preguntar y no preguntar seguirían dando el mismo
`matches: false`; solo cambiarían las frases. La salida estructurada habría
seguido sin depender de lo que se hizo.

**Que `matches` pregunte por defecto.** Preguntar cuesta 3 s y levanta una JVM. Es
la razón que ADR-0164 ya decidió y no es de este bloque revisarla.

**Ampliar `DEFAULT_DECLARATIONS` con `build.gradle`.** Leería un build script con
un regex, que es exactamente el defecto que originó VA9: un patrón se lleva un
número que el proyecto ya dejó atrás, y no es hipotético.

**Copiar en `matches` el informe entero de `inspect`.** Sería una segunda
representación del mismo hecho en dos sitios, y es lo que este repositorio ha
encontrado siete veces. `matches` nombra el veredicto y remite; `inspect` sigue
siendo el que enumera.