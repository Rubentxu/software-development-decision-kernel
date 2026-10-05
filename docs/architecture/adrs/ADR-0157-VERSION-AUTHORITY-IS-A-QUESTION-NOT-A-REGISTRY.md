---
id: ADR-0157-VERSION-AUTHORITY-IS-A-QUESTION-NOT-A-REGISTRY
title: La version de un producto se resuelve preguntando a providers, no leyendo un registro que nombra tecnologias
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-068
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-068
supersedes: [ADR-0153]
superseded_by: null
component: release
surface: crates/sddk-domain/src/version_authority.rs
closes: []
---

# ADR-0157 — La versión de un producto se resuelve preguntando, no leyendo un registro

- **Status:** `accepted` (2026-10-05)
- **Supersedes:** ADR-0153 (los nombres de fichero se quedan en el adapter)
- **Relaciona:** ADR-0158 (que producto es un release, y por que esa eleccion no se automatiza), ADR-0155 (la misma ley aplicada al puerto de code intelligence),
  ADR-0042 (kernel agnóstico), ADR-0152 (identidad de proyecto en storage),
  AGENTS.md §2.3
- **Superficie:**
  - `crates/sddk-domain/src/version_authority.rs` — el modelo y el reducer
  - `crates/sddk-gateway/src/version_provider.rs` — los providers y el registro
  - `crates/sddk-engine/src/version.rs` — la regla del lockstep
  - `crates/sddk-cli/src/release_cmd.rs` — la composición

---

## Contexto

ADR-0153 resolvió que un proyecto que no es Rust no pudiese planear un release.
Lo hizo con un **registro** en `sddk-engine`: ocho filas, cada una con su
ecosistema, sus ficheros y su extractor, más una función que las recorría. La
decisión era correcta y su criterio de aceptación —«el *lector* no nombra
ningún manifiesto»— estaba medido y falsificado.

Medido sobre el resultado, tres cosas aparecen:

1. **El registro estaba en el crate que decide.** El lector no nominaba
   manifiestos; el `REGISTRY`, doscientas líneas más arriba en el mismo
   fichero, sí: `Cargo.toml`, `package.json`, `gradle.properties`,
   `pyproject.toml`, `go.mod`. La distinción que el criterio dibujaba —«el
   lector no, el dato sí»— no es una distinción que nadie trace en el punto de
   llamada, y el mismo fichero llevaba las dos cosas. Un gate que escanea la
   mitad del fichero certifica una frontera que el código no traza.
2. **La entrada unía cosas que no van juntas.** `jvm_gradle` listaba tres
   ficheros —`gradle.properties`, `build.gradle.kts`, `build.gradle`— con **un
   solo formato**, el de `clave=valor`. El segundo y el tercero se leían con
   un parser que no era el suyo, y eso no es una aproximación: es una
   afirmación falsa sobre el formato, silenciosa cuando el fichero encaja.
   MEDIDO: un `setup.py` —Python— se parseaba como TOML porque compartía lista
   con `pyproject.toml`, el parseo fallaba, y como el fallo abortaba la
   resolución **entera**, se perdía la versión que `package.json` ya había
   declarado.
3. **La ley de «fail closed» era código, escrita tres veces.** `NoSource`,
   `NotDeclared`, `Undeclared` y `Invalid` se distinguían por ramas, y
   distinguir «no found» de «found pero roto» era trabajo de cada rama.

Y el hallazgo que abrió el bloque, con un consumidor real al otro lado: un
`gradle.properties` de configuración legítima, **sin `version=`**, abortaba la
resolución con «could not find `version`» y el siguiente fichero de la lista no
se leía nunca. No un `plan` con menos información: **ningún plan**.

---

## Decisión

**SDDK define las preguntas, los contratos y las decisiones. Los providers saben
cómo obtener evidencia de herramientas concretas.**

1. **El modelo es puro y no depende de nadie.** `sddk-domain`
   (`version_authority.rs`) define `ProductVersion` —una identidad opaca, no un
   esquema—, `ReleaseTarget`, `VersionProbe`, `VersionEvidence`,
   `VersionObservation`, `VersionAuthority` y `reduce`. No sabe de nada cómo se
   obtuvo una observación, y un fitness escanea el módulo y falla si aparece un
   nombre.
2. **Un provider por fuente, con su capability versionada.**
   `VersionResolverPort` con `PRODUCT_VERSION_OBSERVATION =
   "product-version.observation/v1"`. El registro pregunta la capability, no
   pregunta quién es cada provider: la identidad es **procedencia**, nunca
   entrada de decisión.
3. **Los nombres de fichero viven en el adapter**, uno por fichero, cada uno con
   **su** formato y **su** localizador. Un fichero sin extractor es una
   declaración honesta de que existe y no es fuente declarable —`build.gradle.kts`
   se resuelve evaluando el modelo de Gradle, no leyendo Kotlin con un regex— y
   no un hueco.
4. **Un provider no decide.** Con dos ficheros que discrepan hay dos
   observaciones y el reducer nombra el conflicto. Un provider único tendría que
   elegir, y elegir es una política; donde una implementación elige y otra
   podría elegir distinto sin que nada lo note, hay una segunda autoridad.
5. **Cinco respuestas, no cuatro.** `NotApplicable`, `Undeclared`, `Declared`,
   `Invalid` y `ReleaseRefIsAuthority`. La quinta existe porque Go y Bazel
   **no declaran** versión de producto y su convención es que la lleva la
   release ref: sin ella, migrar el registro habría convertido un release
   legítimo en un fallo cerrado. No es `Undeclared` porque el silencio no es una
   declaración, y no afirma identidad —no dice que la etiqueta *sea* la
   versión— sino una relación: «no hay versión de producto, y la referencia la
   lleva».
6. **La ley es *fail closed, but not fail first*.** Ausente y no-declarado ceden
   al siguiente; ilegible **falla cerrado**; una versión declarada gana a una
   ausencia declarada; y `Invalid` gana a todo, para que un manifiesto roto no
   quede tapado por la convención de otro fichero.
7. **La declaración del proyecto es un provider más.**
   `.sddk/version-source.json` se lee desde el adapter, y sus condiciones
   —«solo rescata una ausencia, nunca una fuente ilegible ni una
   divergencia»— ya no están escritas a mano: son la precedencia del reducer.
   El rescate sigue funcionando y ha dejado de ser código.
8. **El motor conserva una sola pregunta**: ¿la release ref nombra la versión
   que los providers observaron? Su convención es un prefijo `v` y nada más, y
   está escrita y con test, porque un recorte es cómo dos valores distintos
   acaban presentándose como el mismo.

---

## Consecuencias

**Positivas**

- El kernel no nombra ninguna tecnología para resolver versiones, y eso es un
  test con dientes en los dos extremos (dominio y motor), no una nota.
- Añadir una forma de obtener una versión es **registrar un provider**: no
  toca el motor ni el dominio. Demostrado con un provider falso en el test de
  aceptación del puerto, sin una línea de engine modificada.
- Un fichero presente que no declara versión no puede tapar a otro que sí. Es el
  caso del consumidor, y ahora es un fixture de conformance.
- El motor se testea sin disco: su ley no necesita un directorio temporal.
- `release plan` distingue **encontrado** de **verificado**, y **declaró que no
  tiene versión** de **no dijo nada**.

**Negativas / costes admitidos**

- **La salida de `release plan` cambia de forma y de vocabulario.** Un
  `Cargo.toml` solo pasa de `kind: cross_checked` a `verdict: resolved`, y el
  texto añade `nothing was cross-checked`. Es una corrección y no un
  logotipo nuevo: `CrossChecked` sobre una única declaración era el
  falso verde que este ADR cierra. MEDIDO antes de cambiarlo: no hay ningún
  consumidor fuera del workspace —ni script, ni otro repositorio— que lea
  `version_authority`; los únicos lectores son tests y el registro del
  resultado de un apply, que no se deserializa en ninguna parte.
- **La convención del prefijo `v` sigue siendo una convención.** Declararla y
  medirla es honesto; reemplazarla por una relación `ReleaseRef` declarada es
  trabajo posterior, y está escrito en el sitio donde vive para que nadie añada
  un segundo recorte al lado.
- `ReleaseRefIsAuthority` es un concepto de la release dentro del modelo de
  versión. Se admite porque sin él se pierde un comportamiento certificado, y se
  acota porque **no afirma identidad**: afirma dónde está la versión, no qué
  es.
- Un target con dos productos y sin selector sigue sin tener respuesta. El tipo
  existe (`ReleaseTarget`) pero la selección es trabajo posterior; hasta
  entonces, la composición nombra un target y solo uno.
- `crates/sddk-engine/src/rules/baseline.rs` sigue leyendo el manifiesto del
  workspace y sus `members` para la línea base de verificación de cambio. **No
  es** resolución de versión y queda fuera de este ADR, pero es el sitio donde
  queda acoplamiento a una tecnología concreta en el motor, y conviene que
  conste por nombre y no por memoria.

---

## Criterios de aceptación, medidos uno a uno

`bash tests/test_adr_0157_criteria.sh` ejecuta cada criterio por separado y
reporta su propio veredicto, para que un `PASS` agregado no pueda tapar un
criterio rojo. Cada criterio exige que **pasen todos** los tests que lo componen,
y además que **alguno se haya ejecutado de verdad**: un criterio cuyos tests no
existen reporta `NOT_APPLIED`, nunca `PASS`. Ese requisito no es un detalle —
la primera versión de este gate habría dado `PASS=7 FAIL=0` con los siete
criterios sin un solo test, porque `cargo test` con un filtro que no casa nada
imprime `test result: ok. 0 passed`.

| # | Criterio |
|---|---|
| C1 | El módulo que decide no nombra tecnología concreta, en el dominio **y** en el motor |
| C2 | El escáner del fitness puede ver un nombre, y la excepción que tiene no puede crecer |
| C3 | Los ocho ecosistemas del principio resuelven o fallan por una razón declarada |
| C4 | Un repo Rust conserva el lockstep, incluido el texto del rechazo |
| C5 | La convención de la release ref es un prefijo y nada más |
| C6 | Dos declaraciones que discrepan son un error duro que nombra las dos |
| C7 | Un manifiesto roto falla cerrado y no cede la autoridad |
| C8 | Go y Bazel toman el camino de la release ref, distinto de cross-validación |
| C9 | Un fichero presente que no declara versión no tapa a otro que sí |
| C10 | Un mecanismo nuevo se añade registrando un provider, sin tocar el motor |
| C11 | La resolución es de solo lectura |
| C12 | El silencio no se confunde con una convención declarada |
| C13 | La convención declarada no es una prioridad sobre una versión leída |
| C14 | Workspace verde |

### Falsificación

Cada ley del reducer tiene un mutante que la mata si deja de cumplirse, y cada
falsador corre contra el camino de producción. `M7` —la ausencia declarada
tapa a la versión leída— y `M8` —la ausencia se degrada a silencio, que es el
que se colaría porque el veredicto sigue siendo correcto en cuanto a que no hay
versión— son los dos de la quinta ley, y los dos mueren.
