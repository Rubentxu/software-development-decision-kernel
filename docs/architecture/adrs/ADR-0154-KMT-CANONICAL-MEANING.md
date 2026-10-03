---
id: ADR-0154-KMT-CANONICAL-MEANING
title: KMT means Knowledge Merkle Tree, and the freshness evaluator that currently holds the acronym is renamed, because three concepts compete for three letters
status: accepted
proposed_at: 2026-10-03
accepted_at: 2026-10-03
cycle: p-63676b11dc0ef88f/c3m-kmt-canonical
accepted_by_cycle: p-63676b11dc0ef88f/c3m-kmt-canonical
supersedes: null
superseded_by: null
component: knowledge
surface: crates/sddk-engine/src/knowledge.rs
closes: []
---

# ADR-0154 — Qué significa KMT, y por qué el evaluador de frescura tiene que soltarlo

**Status:** accepted (2026-10-03)
**Date:** 2026-10-03
**Cycle:** `p-63676b11dc0ef88f/c3m-kmt-canonical` (C3m.0)
**Closes:** nada por sí mismo. Deja C3m.1 condicionado.
**Relates:** ADR-022 (la invalidación la posee el KMT), REQ-A3S1-035,
`arch-spec-A3-S1-knowledge-substrate.md`, AGENTS.md §2.7 (una autoridad
canónica por concepto), `docs/roadmap/ROADMAP.md` §C3m.0

---

## Context

`ROADMAP.md` §C3m.0 pide «una sola definición; **rename solo tras aceptar el
ADR**». Este documento es ese ADR. Antes de escribirlo se midió qué compite
hoy por las tres letras, porque decidir el nombre sin medir quién lo usa es
justo el error que este ADR viene a cerrar.

**Medición, sobre `HEAD` de 2026-10-03:**

| Siglas | Dónde vive | Qué es | Ocurrencias |
|---|---|---|---|
| `KMT` | `crates/sddk-engine/src/knowledge.rs:615` | `pub struct KMT` — **evaluador de frescura**, función pura sobre `(basis, expected, now)` | 17 llamadas `KMT::`, 36 menciones |
| `KmtIndex` | `crates/sddk-engine/src/reactive_verify.rs:103` | índice de **unidades por namespace**, para acotar la verificación reactiva | 4, todas en ese fichero |
| «KMT» | `reactive_verify.rs:157`, `card.rs:56,64,67,225` | usado en prosa y comentarios **como si fuera el árbol** | — |

**Los dos son tipos distintos que comparten el mismo nombre.** `KMT` es un
evaluador; `KmtIndex` es un índice de árbol. No es un doc desalineado: es una
colisión de símbolos en el mismo crate, y la prosa de `reactive_verify.rs`
refiere al árbol con las siglas del evaluador.

**La tercera acepción ya está reservada.** `ROADMAP-ACCEPTANCE-TRUTHFULNESS.md`
§C3m.0 lista tres candidatos — *Knowledge Merkle Tree*, *Knowledge Management
Tiers*, *Knowledge-Machine Topology* — y declara que solo uno puede ser el
canónico. Los tres existen hoy en documentos distintos, y el tercero aparece
únicamente en el paquete de evolutivo, sin ninguna superficie de código.

## La decisión, y por qué este orden

**`KMT` = Knowledge Merkle Tree.** Es el nombre que el roadmap ya reserva para el
trabajo de C3m.1, el que describe la **estructura** — un árbol de fingerprints
por unidad, con `build`/`diff`/`invalidate`/`affected_units` — y el único de
los tres que describe un objeto con una forma.

**El evaluador de frescura se renombra a `KnowledgeFreshness`.** Es la decisión
con coste, así que se justifica con su mérito y no con la comodidad del
nombre. Lo que `pub struct KMT` hace hoy es exactamente lo que dice el nombre
propuesto: evaluar si una base de conocimiento está fresca respecto a la
esperada. Y deja de ser un nombre que el roadmap necesita para otra cosa.

**`KmtIndex` pasa a `KmtUnitIndex`.** Un `Index` que además nombra qué indexa
dice más, y el conflicto de este ADR es precisamente que un nombre corto
promete una cosa y entrega otra.

**«Knowledge Management Tiers» se retira del vocabulario.** No describe nada
que el código tenga: no hay tiers, ni hay niveles, ni hay una jerarquía que
`KMT` evalúe. Es un nombre que se coló en `knowledge.rs:1` y `:611` y en la
spec, y que describe una intención que nunca se implementó. Retirarlo no
contradice ninguna conducta: no hay código que lo cumpla.

## Por qué esto tiene que ser un ADR y no un rename

Porque el rename **rompe un contrato normativo escrito**, y hacerlo en
silencio sería exactamente la clase de cambio que este repo ha rechazado
antes.

`arch-spec-A3-S1-knowledge-substrate.md` fija en **REQ-A3S1-035** que
«`KMT::evaluate(basis, expected, now)` is the canonical entry point», y su
§Intent (línea 21) nombra explícitamente las tres letras como *Knowledge
Management Tiers*. El rename:

1. cambia un símbolo que una spec `accepted` declara canónico;
2. obliga a revisar `AT-UAT-019`, que cita «el ADR de identidad» — y que
   según `ROADMAP.md` §C3m.2 **no existe**, así que esa cita nunca fue
   verificable;
3. afecta a 17 llamadas y 36 menciones en `sddk-engine`, y a tests de
   integración que importan el símbolo.

**Renombrar sin este ADR sería un cambio de comportamiento contractual
declarado en un commit.** Con el ADR, es una ejecución de una decisión
tomada, y la spec se actualiza en el mismo movimiento o se declara
explícitamente qué pasa con ella.

## Consecuencias

**Aceptado obliga a, y en este orden:**

1. **Actualizar `arch-spec-A3-S1`** — REQ-A3S1-035 pasa a nombrar
   `KnowledgeFreshness::evaluate`, y el §Intent retira *Management Tiers*. Sin
   este paso el repo tiene spec y código en desacuerdo, que es peor que
   tenerlo coherente y con un nombre viejo.
2. **Renombrar** `KMT` → `KnowledgeFreshness` y `KmtIndex` → `KmtUnitIndex`,
   con `cargo fix`/sed y el perfil completo del workspace.
3. **Añadir `KmtNode`/`KmtTree`** para C3m.1, que es donde el nombre KMT
   recupera su significado propio. Hasta entonces `KMT` **no designa ningún
   tipo**, y eso se dice en el código, no se infiere.
4. **Cerrar la cita rota de `AT-UAT-019`**, que promete un ADR de identidad
   que no existe.

**Riesgo declarado:** el rename es puramente nominal y no cambia ninguna
conducta, pero toca 17+ símbolos en el crate que sostiene la capa semántica.
El guard de este ADR debe comprobar que **después** del rename no queda ninguna
de las dosExpansionaciones retiradas, y que el símbolo nuevo existe — un
refactor que se aplica a medias deja el peor de los dos mundos: el nombre viejo
para el código viejo y el nuevo sin uso.

**Lo que este ADR NO hace:** no construye el árbol. C3m.1 es eso, y depende de
que este ADR esté aceptado. Tampoco toca `KnowledgeBasis`/`revise`, que es
C3m.2 y tiene su propia deuda abierta (INC-DEBT-048) con una decisión binaria
distinta: actualizar la spec o revertir el código. **Mezclar ambas aquí sería
meter dos decisiones incompatibles en un solo documento aceptado**, que es
como C3m.2 empezó a contradecir a REQ-A3S1-021 en primer lugar.

## Aceptación: siete criterios, medidos uno a uno

Una aceptación no se declara por suma, así que cada criterio se ejecutó por
separado y se registra su veredicto. El criterio 4 es el que casi no se puede
cumplir y es el que más información dio.

| # | Criterio | Veredicto |
|---|---|---|
| 1 | El símbolo canónico del árbol se reserva y no colisiona | **cumple** — tras el rename queda **1** struct `KMT*` (`KmtUnitIndex`) |
| 2 | *Knowledge Management Tiers* desaparece del código | **cumple** — 0 apariciones en `crates/`, donde había 2 |
| 3 | El rename no rompe ningún enlace intradoc | **cumple** — 21 antes, **21 después**; el rename no añadió ni uno |
| 4 | El rename rompe un contrato normativo **escrito** | **cumple, y es el motivo del ADR** — REQ-A3S1-035 Updated en la spec, en el mismo movimiento |
| 5 | El rename no se aplica a medias | **cumple** — `KnowledgeFreshness` y `KMT` no coexisten |
| 6 | La librería y sus tests siguen verdes | **cumple** — `sddk-engine` compila, 1403+ tests de lib en verde |
| 7 | El guard que vigila esta decisión tiene dientes | **cumple** — 5 mutaciones, 5 detectadas, 0 sobrevividas |

**El criterio 3 no se pudo medir a la primera, y por cómo falló importa.** La
primera ejecución dio **0** enlaces rotos, lo que parecía una mejora enorme. Era
falso: `cargo doc` no llegó a generar documentación porque la librería no
compilaba, y un `cargo doc` que no genera nada no informa de enlaces. El
baseline real era **21**, y el correcto es «21 después» — no «0». **Medir el
cambio y medir el fallo del instrumento por el mismo número son cosas
distintas**, que es la misma lección que ya se pagó dos veces con el escáner de
contaminación.

**Un criterio que el propio guard dijo.** El guard decía «ADR-0154 está en
`proposed`, el rename sigue bloqueado» y pasaba en verde **después** de que el
rename estuviera aplicado: afirmaba un estado sin comprobarlo. Corregido para
que exija que la historia del ADR y la del código **cuadren entre sí** — rename
aplicado con ADR `proposed` es alguien saltándose el orden, y ADR `accepted`
con el rename sin hacer es una decisión no ejecutada.

## Alternativas descartadas

- **Dejar `KMT` como está y renombrar el árbol.** Descartada: el roadmap
  reserva las tres letras para C3m.1, y el nombre de un evaluador de frescura
  no describe un árbol de fingerprints.
- **No renombrar nada y documentar la ambigüedad.** Descartada por medición:
  la prosa de `reactive_verify.rs` **ya** usa «KMT» para el árbol, luego la
  ambigüedad no es hipotética, está escrita.
- **Introducir un alias `pub use KnowledgeFreshness as KMT`.** Descartada: es
  exactamente la forma que este ADR viene a cerrar — dos nombres para un
  concepto — y un alias de compatibilidad que nadie retira es una segunda
  autoridad con permiso de escritura.
