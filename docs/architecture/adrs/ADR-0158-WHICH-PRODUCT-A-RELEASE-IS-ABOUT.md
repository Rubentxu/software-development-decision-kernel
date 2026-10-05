---
id: ADR-0158-WHICH-PRODUCT-A-RELEASE-IS-ABOUT
title: Un repositorio puede contener varios productos, y elegir cual se publica es una decision que no se toma sola
status: accepted
proposed_at: 2026-10-05
accepted_at: 2026-10-05
cycle: p-63676b11dc0ef88f/version-coherence-068
accepted_by_cycle: p-63676b11dc0ef88f/version-coherence-068
supersedes: null
superseded_by: null
extends: [ADR-0157]
component: release
surface: crates/sddk-domain/src/version_authority.rs
closes: []
---

# ADR-0158 — Qué producto es un release, y por qué esa elección no se automatiza

- **Status:** `accepted` (2026-10-05)
- **Extiende:** ADR-0157 (la versión se resuelve preguntando a providers)
- **Relaciona:** ADR-0152 (identidad de proyecto en storage), ADR-0155 (el
  núcleo no nombra proveedores), AGENTS.md §2.3
- **Superficie:**
  - `crates/sddk-domain/src/version_authority.rs` — `TargetSelector`,
    `select_target`, `TargetSelectionError`
  - `crates/sddk-gateway/src/version_provider.rs` — el escaneo y la precedencia
  - `crates/sddk-cli/src/release_cmd.rs` — `--target` y el registro en el plan

---

## Contexto

ADR-0157 dejó el modelo de versión resuelto y agnóstico. Lo que no tenía es
una forma de que haya **más de un producto**: la composición fabricaba un único
`ReleaseTarget` con la raíz del repositorio y resolvía. Con un solo producto
eso es correcto. Con varios, era incorrecto de dos maneras a la vez, y las dos
invisibles:

1. **La raíz no declara nada y el producto está un nivel más abajo.** No había
   plan: el rechazo era «no declaro version», que es un rechazo sin dirección.
2. **La raíz declara una versión y hay otros productos dentro.** Se publicaba
   la de la raíz sin decir nada de los demás — indistinguible, para quien lee
   el plan, de un repositorio con un solo producto.

Un número de versión sin producto es la misma ambigüedad que ADR-0157 quitó,
movida un nivel más arriba.

### Lo que ya existía y no hay que volver a inventar

MEDIDO sobre el dominio antes de escribir una línea:

- **`ReleaseChannel` ya es canónico** (`crates/sddk-domain/src/channel.rs`):
  cuatro canales —`stable`, `candidate`, `edge`, `dev`— con `promotion_target`
  y `can_promote`. Este ADR **no crea un canal**; lo reutiliza, y por eso VA5
  no tendrá un `ReleaseChannel` nuevo sino una relación declarada con este.
- **`target_id` ya existe** en el código, con **otro significado**: el destino
  de un ticket de admisión o de una promoción de backlog. Es un homónimo, no
  una autoridad competidora —promover un ítem no es publicar un producto—, y por
  eso `ReleaseTarget` se llama como se llama y no se fusiona con nada. Queda
  escrito para que la palabra «target» no se lea como una sola cosa.

---

## Decisión

### 1. Elegir target es una función pura, con cuatro respuestas

`select_target(targets, selector)` en el dominio. No mira ficheros, no mira
tecnologías y no ordena por nada que no sea identidad. Sus cuatro fallos están
separados **porque las reparaciones son cuatro**:

| Fallo | Situación | Reparación |
|---|---|---|
| `NoTarget` | no hay sujeto | no es un conflicto: no hay nada que mirar |
| `AmbiguousTarget` | varios, y el pedido no dice cuál | elegir un producto |
| `UnknownTarget` | se nombró uno y no está | corregir el nombre |
| `AmbiguousTarget` (identidad repetida) | dos directorios con el mismo nombre | una colisión real |

Fusionar los dos últimos es lo que este ADR rechaza: quien recibe «hay varios»
busca un segundo producto, y quien recibe «ese no existe» corrige el nombre. Es
el mismo error de confusión que la distinción `Unresolved` / `ReleaseRefIsAuthority`
en el modelo de versión, y por eso se escribe igual.

### 2. Tres leyes, con falsador propio

- **Ante varios no se elige, y el orden de la lista tampoco decide.** Una lista
  de candidatos que cambia al reordenar el disco no es una lista de la que
  nadie pueda depender.
- **Un nombre que no casa no cae al «único que hay».** Es el defecto más
  tentador: convierte el nombre en decoración y devuelve otro producto en un
  monorepo sin que nada lo diga.
- **La lista de candidatos va en orden canónico**, por la misma razón que el
  reducer ordena las observaciones: un error cuyas variantes cambian al reordenar
  no se puede comparar con nada, y un error que no se puede comparar no puede
  comprobarse.

### 3. Los targets se descubren, con profundidad declarada

El escaneo pregunta «¿hay aquí algo que **pueda** declarar una versión?», que es
más débil que «¿declara una versión?», y a propósito: un directorio con una
declaración ilegible sigue siendo un target, y que lo diga o no el reducer, con
la evidencia delante. Encontrar un candidato y saber qué declara son dos
preguntas.

**La profundidad por defecto es `2`, y el número está justificado**:
`packages/alpha` está dos niveles debajo de la raíz porque `packages` agrupa y
no es un producto. MEDIDO: con `1` el escaneo encuentra el directorio agrupador
—que no declara nada— y declara que un monorepo con un producto no tiene
ninguno. Diez de los catorce tests de conformance cayeron por eso, que es la
forma que tiene un número sin justificación de fallar en silencio. Es un
**parámetro** que viaja con la respuesta, porque un conjunto de targets sin
statement de lo que se buscó es indistinguible del conjunto de todos.

### 4. Una raíz que declara su versión ES el target, y no se busca debajo

Necesita defensa, porque parece lo prohibido. No lo es:

- la preferencia **prohibida** es entre **respuestas** — dos declaraciones que
  discrepan, en las que ninguna tecnología gana—;
- esto es entre **entidades** — este repositorio y los productos que contiene—,
  que son preguntas distintas, y el default de «cuál de estos quería decir»
  cuando la respuesta está arriba es «el de arriba».

Es **estructural, no un fallback**: con un fallback, un repositorio que declara
arriba y tiene un producto abajo resolvería el de arriba sin decir nada del
otro. El resultado del escaneo viaja con la decisión, así que siempre se puede
decir qué más había.

Una raíz que declara que su versión la lleva la release ref **también** es el
target: si no, un repositorio Go en la raíz con un módulo vendorizado debajo
iría a buscar una versión de producto que acaba de decir que no tiene.

### 5. La plan registra el producto, y el outcome también

`release plan` dice `release_target`, su raíz, **todos** los candidatos y cómo
se llegó a ellos, más una nota explícita cuando el repositorio contiene más de
un producto. Sin esa nota, un plan de un monorepo es indistinguible de uno de un
repositorio con un producto.

`ReleaseOutcome` registra la **identidad** del producto, no la narración. Un
registro de release que no dice qué producto publicó no se puede auditar, y un
número de versión sin producto es la ambigüedad de ADR-0157 un nivel más arriba.
La narración se queda en el plan, porque duplicar una prosa en un registro
durable es como dos versiones de la misma historia empiezan a discrepar.

### 6. `--target` es lo que resuelve, y solo eso

No hay un flag que *dude* entre candidatos ni una bandera de «usa el primero».
O se nombra, o hay uno solo, o el rechazo dice cuántos hay y sus rutas. Un
`--target first` sería la preferencia prohibida con un nombre de flag.

---

## Consecuencias

**Positivas**

- Un monorepo con N productos deja de ser un error y pasa a ser N releases
  distintos, cada uno con su versión y su tag.
- El caso del consumidor, que era «no hay plan», pasa a ser «el plan nombra su
  producto y dice que hay más».
- La divergencia tiene **nivel**: dentro de un target es conflicto, entre
  targets no existe porque no hay un veredicto donde encontrarse.

**Negativas / costes admitidos**

- **`release plan` falla en un monorepo sin `--target`.** Es deliberado y es lo
  que el bloque pide, pero es un cambio de comportamiento: un repositorio que
  hoy publica con un tag y mañana necesita `--target` es un ruptura que hay que
  decir en el changelog, no un detalle.
- La lista de directorios omitidos es una **opinión sobre la forma del árbol**
  que vive en el adapter y no en el núcleo. Un nombre mal puesto hace un
  producto **invisible**, y un producto invisible sale como «no declaro
  versión» —un rechazo que le dice al operador que mire—, nunca como una
  respuesta equivocada. La dirección del fallo es la buena; la consecuencia
  es que un producto con nombre tomado por una regla de omitidos necesita que
  alguien quite el nombre de la lista.
- La profundidad `2` no encuentra `a/b/c/d`. Quien tenga esa forma necesita un
  llamador que pida más, y hoy el llamador no expone el parámetro — está
  declarado y es lo que falta.

---

## Criterios de aceptación, medidos uno a uno

| # | Criterio | Dónde se mide |
|---|---|---|
| T1 | Un target sin pedir se resuelve | `release_target_acceptance` |
| T2 | Cero targets no es ambigüedad | `release_target_acceptance` |
| T3 | Dos targets sin selector fallan cerrado, con los dos nombrados | `release_target_acceptance`, `release_target_conformance`, `cli.rs` |
| T4 | Un target nombrado se resuelve entre varios | los tres |
| T5 | Un nombre que no existe no cae al único que hay | `release_target_acceptance`, `cli.rs` |
| T6 | Una identidad repetida es ambigüedad, no orden | `release_target_acceptance` |
| T7 | El orden de los candidatos no decide | `release_target_acceptance` |
| T8 | Dos targets con versiones distintas no se confunden | `release_target_acceptance` |
| T9 | Un target sin versión no hereda la de su hermano | `release_target_acceptance` |
| T10 | Raíz muda con producto debajo resuelve | `release_target_conformance` |
| T11 | Una raíz que declara es el target, y no se busca debajo | `release_target_conformance` |
| T12 | Una raíz que declara la release ref también lo es | `release_target_conformance` |
| T13 | Los directorios de construcción no son productos | `release_target_conformance` |
| T14 | La profundidad es acotada, es `2`, y viaja con la respuesta | `release_target_conformance` |
| T15 | La identidad de un target es su ruta, no su nombre | `release_target_conformance` |
| T16 | El plan nombra su producto y dice que hay más | `release_cmd` (render), `cli.rs` |
| T17 | El outcome registra la identidad del producto | `release_flow` |

### Falsificación

Tres mutantes, cada uno rompiendo **una** ley, y cada uno tiene que morir **en
la fila que ejerce su ley**:

- `M1 first-target-wins` — ante varios, gana el primero.
- `M2 named-falls-back-to-the-only-one` — un nombre que no casa cae al único.
- `M3 unknown-becomes-ambiguous` — lo desconocido se degrada a ambiguo.

Los tres mueren en «dos targets sin pedir» y «un target, un nombre que no
casa», que son las filas de sus leyes.

**Una nota sobre el instrumento, porque es la parte que más enseñó.** La
primera versión de `assert_kills` exigía que el mutante se diferenciara en
**todas** las filas, lo cual es imposible — un mutante rompe una ley y en las
demás filas tiene que coincidir con el código correcto. Corregido para exigir
algo más fuerte: que muera **en la fila de su ley** y que no muera antes. Y al
corregirlo apareció que **los tres mutantes rompían más de una ley cada uno**,
así que morían en la fila equivocada. Es la segunda vez en este bloque que el
mismo error se repite, y lo que enseña no es que el falsador esté mal: es que
**exigir más de lo que se sabe produce ruido que desplaza la medición**, y que
un falsador merece el mismo escepticismo que el código que mide.
