---
id: ADR-0160-WHO-IS-RESPONSIBLE-FOR-A-RELEASE
title: Un release puede tener varios responsables, y un rol es una declaración de hasta dónde llega cada uno
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-068
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-068
supersedes: null
superseded_by: null
extends: [ADR-0157, ADR-0158, ADR-0159]
component: release
surface: crates/sddk-domain/src/release_role.rs
closes: []
---

# ADR-0160 — Quién es responsable de un release, y dónde termina cada responsabilidad

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0157 (la versión se resuelve preguntando a providers),
  ADR-0158 (varios productos por repositorio), ADR-0159 (un tag no es una versión)
- **Relaciona:** ADR-0069 (`DeliveryEnvelope::evaluate`), ADR-0155 (el núcleo
  no nombra proveedores)
- **Superficie:**
  - `crates/sddk-domain/src/release_role.rs` — `ReleaseRole`, `may_promote`,
    `may_publish`, `CandidateHandoff`, `RoleRefusal`, `promotion_distance`
  - `crates/sddk-cli/src/release_cmd.rs` — `--role` y la puerta que lo exige

---

## Contexto

El flujo de release estaba escrito con una sola forma: construir, etiquetar,
publicar una release estable. Los tres pasos, el mismo actor, el mismo sitio, el
mismo momento.

Eso no es una ley, es una costumbre, y es falsa para un reparto ordinario:
**un repositorio produce material candidato, y otra cosa lo certifica y lo
promueve.** El productor no es una versión más pequeña del publicador. Es una
responsabilidad distinta, y obligarle a hacer el trabajo del otro obliga a
elegir entre dos cosas malas: que lo haga todo, o que la diferencia sea invisible.

### Lo que ya existía y por qué no se reusa

MEDIDO antes de escribir una línea, porque dos de los tres candidatos podrían
haber sido autoridades:

- **`DeliveryKind` no es lo mismo y no se fusiona.** Es una taxonomía cerrada de
  **qué efectos están prohibidos en un ciclo**, inmutable desde
  `phase.specify.complete`, guardada en `cycle.manifest.delivery_kind`, con
  `EvaluationPolicy::evaluate` sobre efectos observados. Un rol responde a
  **quién es responsable de qué paso**, sobre otro sujeto —el target— y con otro
  ciclo de vida: un ciclo se abre y se cierra, el papel de un repositorio en la
  cadena de release dura mucho más. Se decide mecánicamente, así que no es el
  STOP de «dos autoridades competidoras»: son dos preguntas.
  MEDIDO también que `EvaluationPolicy::evaluate` **no tiene consumidor de
  producción** —solo lo ejercitan sus tests—, y por eso este ADR **no** se apoya
  en la lista de efectos prohibidos: apoyarse en algo que nadie lee es cambiar de
  dependencia a una por otra que tampoco se lee.
- **`ReleaseChannel` sí se reusa, y su retículo es la autoridad.** Los cuatro
  canales y `promotion_target` / `can_promote` ya existían y ya eran
  canónicos. Este ADR **no crea un retículo**: le dice a cada rol hasta dónde
  puede llegar, y pregunta al retículo si un paso es legal.

---

## Decisión

### 1. Un rol es un techo, y el techo no es un destino

`ReleaseRole::ceiling()` dice **hasta dónde me dejan llegar**, no «qué pasos
puedo dar». La pregunta «¿puede este actor dar este paso?» la hace
`may_promote`, y la hace preguntando **las dos** cosas: el retículo y el techo.
Las dos deben cumplirse.

Y se preguntan en ese orden —el retículo primero— por una razón que se puede
explicar sin contexto: una regla del retículo es una propiedad de los canales y
**no depende de quién pregunta**, luego es el rechazo más fundamental de los dos.
Preguntar el rol primero informaría «ese paso no es tuyo» de un movimiento que los
canales no le permitirían a nadie.

### 2. El techo se lee con el retículo, y no comparando dos canales

**Este es el hallazgo que el falsador hizo y la lectura no**, y es el que más
enseña del bloque.

`ReleaseChannel` se declara `Stable, Candidate, Edge, Dev` —el orden en que se
**lee** la cadena de promoción, de su final a su principio—, luego su `Ord`
derivado es el **revés**: `Stable` ordena antes que `Candidate`, y
`Stable <= Candidate` es **cierto**.

La primera versión de este bloque escribió el techo como `channel <= ceiling`.
MEDIDO: un `CandidateProducer` pasaba a poder llegar a `Stable` — que es
literalmente lo que el módulo existe para impedir — y la fila que lo mide la
detectó en el falsador, no la revisión del código.

Un orden derivado que contradice el retículo del dominio no es un detalle al que
haya que tener cuidado: es una trampa con sistema de tipos alrededor, y el
sistema de tipos no avisa. Por eso la comparación correcta camina
`promotion_distance`, que usa el `promotion_target` que ya existía. Y el techo se
lee **al revés** que una promoción, que es lo que hace ser un techo: `channel`
tiene que ser un **antepasado** del techo, no su sucesor.

### 3. Llegar no es publicar, y por eso hay dos preguntas

Un certificador tiene que **llegar** a `stable` para poder certificar dentro. Si
«llegar» implicara «publicar», el certificador publicaría, es decir certificaría
su propio trabajo.

Por eso `may_publish` no deduce de `may_reach`, y la fila que lo fija se llama
`llegar_no_es_publicar` y afirma que el techo de un certificador y el de un
publicador **son iguales** — que es justo lo que hace que el techo no baste.

### 4. Producir es un final, y terminar bien no es terminar incompleto

`terminates_here()` es cierto para el productor y falso para los otros tres.

Es el fallo **silencioso** del bloque: el handoff se emite, y si el flujo se queda
esperando una certificación externa que el productor no controla, el trabajo que
sí podía hacer se detiene con él. Desde fuera, terminar y quedarse a medias se
ven igual si nadie escribe la ley.

### 5. El sobre es opaco a propósito

`CandidateHandoff` lleva target, versión de producto, referencia, secuencia,
revisión, artefactos direccionables por contenido, referencias de evidencia y el
tipo de handoff externo **en las palabras del productor**.

No lleva veredicto, y esa es la parte: un sobre que llevara un veredicto sería
una segunda autoridad —las reglas del productor reescritas por una herramienta que
no las tiene—. La semántica concreta de una candidata externa sigue siendo de
quien la produjo, y SDDK entender el sobre es el alcance completo de lo que
reivindica.

### 6. El rol es una declaración, y tiene superficie para declararse

`--role candidate_producer|certifier|promoter|full_publisher` en `release` y en
`ship`. Sin superficie, el tipo sería un artefacto declarado que nadie consume, que
es el defecto que este repositorio ha encontrado siete veces.

La puerta se pregunta en los **dos** puntos de entrada —plan y apply— y la regla
vive en el dominio y se pregunta una vez. Que la pregunten los dos no es
duplicar la regla: es no dejar un camino por el que no se pregunte. Y va **antes**
que el lockstep, para que un target que declara no ser el publicador no lea un
desajuste de versión en un release que no iba a existir.

---

## Consecuencias

**Positivas**

- Un repositorio que solo produce candidatas puede decirlo, y el release lo tiene
  en lugar de asumir que publica.
- El rechazo nombra el rol y su techo, así que «¿entonces qué soy yo?» tiene
  respuesta.
- `DeliveryKind` y `ReleaseRole` quedan explícitamente separados, con el motivo
  escrito, para que una reescritura no los fusione.

**Negativas / costes admitidos**

- **`--role` es por release, no una configuración del repositorio.** Es el mismo
  coste que `--naming` y por el mismo motivo: no hay superficie de configuración
  de release en SDDK, y construirla es otro bloque. Un default por invocación sigue siendo un default, y
  quien olvide la bandera en el próximo release se lleva el rechazo.
- **La emisión del handoff NO está en este bloque.** El tipo, las leyes y la
  puerta existen y se prueban; el comando que **construye** un `CandidateHandoff`
  y lo entrega, no. Es lo que queda de VA6 y está declarado aquí en vez de
  dejar un sobre que nada construye.
- `EffectKind` sigue sin consumidor de producción. Este ADR no lo arregla y no se
  apoya en él.

---

## Criterios de aceptación, medidos uno a uno

| # | Criterio | Dónde se mide |
|---|---|---|
| T1 | El publicador publica con las puertas abiertas | `release_role_falsification` |
| T2 | El techo del productor lo para aunque todo lo demás lo permita | `release_role_falsification` |
| T3 | Y el paso que sí es suyo lo puede dar | `release_role_falsification` |
| T4 | El retículo no lo permite a nadie | `release_role_falsification` |
| T5 | Las puertas cerradas son un motivo propio | `release_role_falsification` |
| T6 | Un certificador no publica: llegar no es publicar | `release_role_falsification` |
| T7 | Un promotor tampoco | `release_role_falsification` |
| T8 | El productor se queda en su techo | `release_role_falsification` |
| T9 | Producir es un final, y los otros siguen después | `producir_es_un_final` |
| T10 | El techo no separa a un certificador de un publicador | `llegar_no_es_publicar` |
| T11 | El `Ord` derivado va al revés que la promoción, y no se usa | `el_orden_derivado_de_los_canales_no_es_el_de_la_promocion` |
| T12 | El sobre lleva revisión, referencia, secuencia y artefactos | `el_sobre_lleva_lo_que_hace_falta_y_nada_mas` |
| T13 | `--role` cambia lo que el release hace | `cli.rs` |
| T14 | Un rol que no existe dice cuáles hay | `cli.rs` |
| T15 | El de fábrica no cambia de comportamiento | `cli.rs` |

### Falsificación

Tres mutantes, uno por conjunción, y **cada uno muere en la fila donde el otro
conjunto se cumple**: `M1 ceiling-ignores-role` (se cae el techo), `M2
lattice-ignored` (se cae el retículo), `M3 publishing-ignored` (se cae «solo
publica quien publica»).

**Y una lección del instrumento, que es la segunda vez que aparece en este bloque
y merece decirse porque es la que costó el diseño entero.** La primera versión de
este falsador metió los tres mutantes en **una sola** tabla, la de `may_publish`.
Los tres sobrevivieron, y el motivo es que el techo **es redundante** en esa
función: `publishes()` solo es cierto para el rol cuyo techo es `Stable`, luego
quitar el techo de ahí no cambia ninguna respuesta. No era un defecto del código
—era una fila que no distinguía la ley que decía medir—, y por eso las dos
preguntas están ahora en **tablas separadas**: el techo se falsifica por
`may_promote`, que es donde significa algo, y publicar por `may_publish`.

Y la otra mitad, que ya es patrón en este bloque: un mutante tiene que
**reemplazar** la respuesta, no llevar la verdadera en un `&&` al lado. La primera
versión de M1 y M3 lo hacía, y por eso no podían discrepar nunca — un falsador que
lleva la respuesta correcta dentro no es un falsador, es una tautología con
apariencia de medir.

La ley que este bloque **no** puede falsar con un mutante —`producir_es_un_final`,
`llegar_no_es_publicar`, `el_orden_derivado…`— tiene nombre y forma propias, y la
razón está escrita en cada una: o la función que la sostiene es una propiedad del
tipo, o la garantía es de compilación, o la fila no distingue. Una ley con nombre
la puede citar un gate, que es lo que evita que una reescritura se la lleve sin
querer.
