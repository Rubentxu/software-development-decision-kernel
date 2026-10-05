---
id: ADR-0159-UNA-RELEASE-REF-NO-ES-UNA-VERSION
title: Un tag no es una versión, y la convención con la que se compara se declara en vez de estar cableada
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-068
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-068
supersedes: null
superseded_by: null
extends: [ADR-0157, ADR-0158]
component: release
surface: crates/sddk-domain/src/release_ref.rs
closes: []
---

# ADR-0159 — Un tag no es una versión, y su relación se declara

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0157 (la versión se resuelve preguntando a providers),
  ADR-0158 (un repositorio puede contener varios productos)
- **Relaciona:** ADR-0069 (`DeliveryEnvelope::evaluate`), ADR-0155 (el núcleo
  no nombra proveedores), AGENTS.md §2.3
- **Superficie:**
  - `crates/sddk-domain/src/release_ref.rs` — `ReleaseRef`,
    `CandidateSequence`, `VersionNaming`, `BindOutcome`, `SourceRevision`
  - `crates/sddk-engine/src/version.rs` — el lockstep recibe la convención
  - `crates/sddk-cli/src/release_cmd.rs` — `--naming`, y su registro

---

## Contexto

ADR-0157 y ADR-0158 dejaron la versión resuelta y con producto. Lo que
siguiente quedó supuesto era lo más viejo de los tres:

> `git tag == ProductVersion`.

No lo está. Un tag es **cómo se llama** un release; una versión es **qué es**
el producto. Lo que los unía era una línea, en el crate que decide:

```rust
name.strip_prefix('v').unwrap_or(name) == version
```

Una línea que era correcta por accidente en cada caso que el repositorio
tenía, y por accidente no es una decisión. Tres cosas estaban mal a la vez:

1. **La convención era invisible.** Un recorte de un carácter parece
   exactamente una coerción, y una coerción es como dos valores distintos
   acaban presentándose como uno — la peor clase de verde, porque un release
   se autoriza sobre un número que no es el del proyecto.
2. **`-rc2` no estaba modelado.** `v1.0.0-rc2` y `v1.0.0` difieren en un
   sufijo y en un mundo: una es candidata, la otra es estable. Tratarlo como
   cadena a normalizar tira lo único que las distingue.
3. **El canal no era parte de la referencia.** `ReleaseChannel` ya existía
   como tipo canónico, con retículo de promoción, y la referencia no lo
   mencionaba: dos referencias de canal distinto eran el mismo valor con dos
   nombres.

### Lo que ya existía y no hay que volver a inventar

MEDIDO antes de escribir una línea:

- **`ReleaseChannel` ya es canónico** (`crates/sddk-domain/src/channel.rs`):
  cuatro canales con `promotion_target` y `can_promote`. Este ADR **no crea un
  canal**; lo usa.
- **`DeliveryKind` no es lo mismo y no se fusiona.** Es una taxonomía cerrada
  de *qué efectos están prohibidos* en un ciclo, inmutable desde
  `phase.specify.complete`, y con `EvaluationPolicy::evaluate` sobre efectos
  observados. Un rol de release responde a *quién es responsable de qué paso*,
  que es otra pregunta, sobre otro sujeto y con otro ciclo de vida —lo declara
  VA6, y este ADR solo deja escrito que no son la misma autoridad.
- **`tag_version` existe en dos sitios con dos sentidos.** Como campo de
  `VersionLockstepError` contenía el nombre de la referencia tras recortarle el
  prefijo, y se renombra `release_ref`. Como columna de UAT en el control
  plane es la versión del tag de una evidencia. Homónimos, no competencia.

---

## Decisión

### 1. La relación se comprueba por construcción, no por recorte

La naming **construye** el nombre que la versión tendría, y las dos cadenas se
comparan exactamente.

**La dirección es el diseño.** Generar un nombre es enumerable; parsear un
nombre no lo es: no hay forma de enumerar los cortes que una cadena admite, y
una comprobación que no se puede enumerar no se puede falsificar.

De ahí una consecuencia que parece restrictive y es lo contrario:
`name_for` devuelve `Option<String>`. Una función que puede devolver un nombre
sin la secuencia de candidata es una función que puede mentir, y los llamadores
que necesitan un nombre son exactamente los que lo publicarían.

### 2. Un prefijo declarado es obligatorio

`Exact` significa exacta en las dos direcciones; `VPrefixed` exige el prefijo en
las dos. Antes el prefijo era **opcional de facto**, porque un `strip_prefix`
que no casa no hace nada: la convención era una sugerencia, y una sugerencia no
puede rechazar nada, luego no puede autorizar nada.

**Esto es un cambio de comportamiento y por eso va en el changelog**: un tag
sin `v` ahora se rechaza donde antes pasaba.

### 3. `NamingHasNoRoomForCandidates` es un caso aparte, no un desajuste

Porque la **reparación** es distinta. No es que el nombre esté mal —eso manda a
renombrar la referencia—, es que el proyecto no ha declarado cómo nombra
candidatas, y la reparación es declarar la convención. Presentarlos como el
mismo error manda a la reparación equivocada.

### 4. El canal y la secuencia son **entradas**, nunca deducidos del nombre

`ensure_release_ref_lockstep` recibe un `&ReleaseRef` que ya lleva canal y
secuencia. `ensure_version_lockstep_detailed` —la convenience— construye
`ReleaseRef::new(nombre, Stable)`, y lo hace **explícito y comentado**: un
nombre solo es una referencia estable, y que el texto parezca el de una
candidata no lo convierte en una. Adivinar el canal por el texto sería parsear
el nombre, que es lo que este módulo prohíbe.

`CandidateSequence` es `NonZeroU32` porque `-rc0` es una candidata sin
predecesora: una pregunta que nadie hizo y que nadie puede responder.

### 5. `SourceRevision` es un cuarto concepto, y este módulo no fabrica versiones

Una revisión dice **dónde estaba el código**, una versión **qué es el producto**,
una referencia **cómo se llama este release**, y un canal **en qué vía va**. Un
build cuyo código se movió y cuya versión no es un hecho normal, y una
herramienta que confunda revisión y versión no puede representarlo.

La garantía que sostiene la frontera no es una comparación: es que este módulo
**recibe** versiones y **no devuelve ninguna**. Por eso la ley se mide sobre las
**firmas**, y no con un `assert!` de igualdad de cadenas.

### 6. El motor NO tiene naming por defecto

La primera versión de este bloque dejó `default_version_naming()` en
`sddk_engine::version`, y eso estaba mal por una razón que no se ve en la
firma: **un crate que decide ya ha decidido la convención de todos los
proyectos** en el momento en que pone un valor por defecto. Ponerlo de
fábrica no lo hace menos cableado, lo hace **invisible**, que es peor.

Así que la convención es un **parámetro** de las tres funciones de lockstep, y
la declara quien publica: `--naming v_prefixed|exact` en `release` y en `ship`.
El rechazo lleva **siempre** la convención aplicada, porque es un hecho —el
release se autorizó bajo `v_prefixed`— y un hecho en un rechazo no puede ser
consejo equivocado. Un consejo condicional tiene que acertar en su diagnóstico,
y el día que falle enseñará a despreciarlo.

`PrefixedCandidate` existe y **no** es alcanzable desde la bandera: necesita
prefijo, separador y marcador, y meterlos en una palabra los escondería. El
error distingue «no existe» de «existe y no se puede decir así», que son dos
errores con dos reparaciones.

### 7. El plan y el outcome registran la convención

`release_naming` viaja en los dos. Un registro que dice `release_target: runtime`
sin decir si el tag tenía que ser `v0.47.0` o `0.47.0` no lo puede volver a
comprobar nadie, ni quien lo escribió.

---

## Consecuencias

**Positivas**

- `v1.2.3-rc2` no puede autorizar una release estable por ninguna vía.
- Un proyecto que etiqueta sin `v` tiene una salida declarada, no un rechazo
  sin salida.
- `ProductVersion`, `ReleaseRef`, `ReleaseChannel` y `SourceRevision` son cuatro
  tipos, no tres y una cadena.

**Negativas / costes admitidos**

- **Un tag sin `v` se rechaza donde antes pasaba.** Es deliberado, es lo que un
  prefijo obligatorio significa, y está en el changelog.
- **`--naming` es por release, no una convención guardada en el repo.** Un
  default de CLI en cada invocación sigue siendo un default, y quien olvide la
  bandera en el próximo release se lleva el rechazo. Se declara como lo que es
  porque la alternativa —una configuración de release que este bloque no tiene—
  es otro bloque.
- **`PrefixedCandidate` no es alcanzable desde la CLI.** Es una tercera naming que
  existe para quien la construya en código, no para quien escribe una bandera.
- `SourceRevision` está **aislada**: existe el tipo y la frontera, y todavía no
  hay un camino de producción que la recorra. Se dice aquí en vez de fingir que
  está en uso.

---

## Criterios de aceptación, medidos uno a uno

| # | Criterio | Dónde se mide |
|---|---|---|
| T1 | `v1.2.3` nombra a `1.2.3` bajo la naming por defecto | `release_ref_falsification` |
| T2 | Una candidata NO nombra a la versión de su producto | `release_ref_falsification` |
| T3 | Con naming declarada, la candidata sí nombra si la secuencia casa | `release_ref_falsification` |
| T4 | Una secuencia que no es la del nombre no nombra | `release_ref_falsification` |
| T5 | Un prefijo declarado es obligatorio en las dos direcciones | `release_ref_falsification` |
| T6 | `Exact` nombra sin prefijo y no con prefijo | `release_ref_falsification` |
| T7 | Un prefijo declarado más largo no es un superprefijo | `release_ref_falsification` |
| T8 | Una versión distinta no es la versión | `release_ref_falsification` |
| T9 | El desajuste dice los dos lados | `release_ref_falsification` |
| T10 | Ninguna firma del módulo devuelve una `ProductVersion` | `release_ref_falsification` |
| T11 | No hay conversión entre revisión y versión | `release_ref_falsification` |
| T12 | El motor no fabrica ninguna naming | `version_authority_fitness` (engine) |
| T13 | El motor recibe un `&ReleaseRef` con canal, no una cadena | `version_authority_fitness` (engine) |
| T14 | Sin `v` se rechaza y el mensaje dice qué nombre daría | `version_authority_fitness` (engine), `version.rs` |
| T15 | `--naming exact` hace pasar el mismo tag sin prefijo | `cli.rs` |
| T16 | Una naming que no existe dice cuáles hay | `cli.rs` |
| T17 | El plan registra la convención aplicada | `cli.rs` |

### Falsificación

Cuatro mutantes, cada uno rompiendo **una** ley, y cada uno tiene que morir **en
la fila que ejerce su ley**: `M1 contains`, `M2 strip-suffix`, `M3
optional-prefix`, `M4 sequence-ignored`. Y una quinta ley, `M5`, que **no es un
mutante** y por eso tiene otra forma: la frontera entre revisión y versión se
sostiene en las firmas, y eso se mide sobre el fuente con un fitness que tiene su
propio control.

**Tres instrumentos se rompieron a sí mismos** y esa parte es la que más
enseña del bloque:

1. **Dos mutantes de dos leyes.** `M3` comparaba el nombre contra la versión
   desnuda *o* con `v`, así que reventaba la fila de la candidata —que es la
   fila de `M2`—; `M4` construía `v{version}-rc2` siempre, con lo que reventaba
   la primera fila antes de llegar a la suya. Los dos reescritos para romper
   **una sola** ley: `M3` quita el prefijo declarado respetando secuencia,
   marcador y separador, y `M4` lee la secuencia del texto en vez de comparar
   la declarada.
2. **Una ley que no era un mutante.** `M5` definía una función local y exigía
   que devolviera `false`: no había nada que falsar, porque la función era del
   test. `ProductVersion::new("1.2.3")` también es una versión válida, luego
   comparar las dos cadenas **sí** da `true` y el test solo se contradecía a sí
   mismo. Salió del molde de mutante a un fitness de source-level.
3. **Dos falsos positivos del mismo tipo y por la misma causa.** El fitness de
   `M5` preguntaba «¿la línea menciona un tipo?» en vez de «¿la línea
   **devuelve** ese tipo?», y marcó `BindOutcome::message`, que **recibe** una
   versión y devuelve un `String`. El fitness que sustituyó al default del
   motor repitió el error: buscaba `-> VersionNaming` en la línea entera y
   marcaba `-> Option<VersionNaming>`, que es una función capaz de decir «no
   tengo ninguna» —justo lo **opuesto** de tener un default—.

   Los dos escáneres comparan ahora el **tipo de retorno** y los dos tienen el
   caso **negativo** en su control, porque un guard que solo tiene el caso
   positivo no demuestra que sepa cuándo **no** disparar. Un guard que se
   dispara sobre algo legítimo entrena a su lector a saltárselo, que es como un
   guard desactivado se parece a uno que pasa.
