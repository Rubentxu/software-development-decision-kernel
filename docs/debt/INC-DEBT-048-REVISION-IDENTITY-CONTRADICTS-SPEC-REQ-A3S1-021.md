---
id: INC-DEBT-048-REVISION-IDENTITY-CONTRADICTS-SPEC-REQ-A3S1-021
title: la identidad de revisión de KnowledgeBasis contradice REQ-A3S1-021, y el UAT que la gobierna cita un ADR que no existe
status: open
severity: high
priority: P1
detected_at: 2026-10-01
detected_in_session: session-62
component: architecture-contracts
surface: docs/architecture/specs/arch-spec-A3-S1-knowledge-substrate.md
references:
  - crates/sddk-engine/src/knowledge.rs
  - docs/architecture/specs/arch-spec-A3-S1-knowledge-substrate.md
  - docs/roadmap/UAT-MATRIX.md
  - tests/cycle-artifacts/p-63676b11dc0ef88f/session62-c3m2-knowledge-basis-revise-identity/RECEIPT.md
fingerprint: "basis_revision_identity_contradicts_req_a3s1_021_and_uat_cites_nonexistent_adr"
---

## Qué es

Al cerrar C3m.2.session-62 se encontró, al verificar los límites de la propia
entrega, que **el arreglo choca con la especificación vigente**:

`docs/architecture/specs/arch-spec-A3-S1-knowledge-substrate.md`
(`status: proposed`), **REQ-A3S1-021**:

> `KnowledgeBasis::basis_hash` SHALL be deterministically derived from the
> sorted `(id, inner_basis_hash)` pairs (test asserts insertion-order
> independence).

La derivación, según la spec, es función **sólo del conjunto de assertions**.
La corrección de C3m.2 hace que `revised_at` participe del digest
(`derive_basis_hash_at`), de modo que **código y spec ahora discrepan**.

## El matiz que hay que preservar

Antes de session-62 el diagnóstico era "`revise` con docs/código discrepantes".
Medido, la realidad es más matizada y menos봬 triumphante:

- El **doc de `revise`** afirmaba *"a new basis hash (because the `revised_at`
  participates in the hash)"*. Eso **era falso**: `derive_basis_hash` sólo
  recibía `assertions`. El doc mentía y se corrigió.
- El **código era consistente con REQ-A3S1-021**. No había descuadre
  docs/código sobre la derivación: los tres —doc, implementación, spec—
  describían el mismo digest por assertions, salvo el doc de `revise`.

El defecto de **comportamiento** que sí sobrevive a la spec es otro, y es
independiente del doc: `KMT::evaluate` compara hashes **antes** que
timestamps, así que con la spec vigente **una revisión puramente temporal es
invisible al freshness** (devuelve `Fresh` sin mirar `revised_at`). Eso
contradice REQ-A3S1-033, que gobierna `Fresh`/`Stale`/`Unknown`.

## Por qué queda abierta en vez de cerrada

Entregar el código sin más dejaría el repo con spec y código en desacuerdo,
que es exactamente la deriva que este roadmap existe para eliminar. Pero
**cambiar REQ-A3S1-021 es un acto normativo** que no corresponde a una slice
que repara un doc de código, y `status: proposed` no significa "aceptado para
modificar sinmás".

La decisión que corresponde a la autoridad normativa es binaria:

- **(a) Actualizar REQ-A3S1-021** para que la derivación incluya `revised_at`.
  Entonces el código de session-62 pasa a ser la implementación de la spec, y
  el digest `v2` queda justificado por ella.
- **(b) Revertir el cambio** y reescribir el doc de `revise` para que describa
  la semántica real (hash sólo por assertions). Entonces **hay que resolver
  además** el problema de `KMT::evaluate`: o la spec de freshness cambia, o
  `KMT::evaluate` pasa a comparar `revised_at` antes que el hash.

Ninguna de las dos está implementada. La opción por defecto que **no** se toma
es fingir que el conflicto no existe.

## El UAT que gobierna esto cita una autoridad inexistente

`AT-UAT-019` (C3m.2) dice:

> revise con mismo contenido / distinto tiempo ⇒ comportamiento coincide con
> **el ADR de identidad**

**Ese ADR no existe.** Verificado: ni `docs/adr/` ni
`docs/architecture/adrs/` contienen un ADR de identidad de `KnowledgeBasis`; lo
más cercano es ADR-0147 (seam del intelligence loop), que no habla de esto.

Un criterio de aceptación que remite a un documento inexistente no puede
cumplirse: no hay forma de pasar ni de fallar honestamente contra él. Por eso
C3m.2 se reporta **PASS PARCIAL** y no PASS:

- **Sí:** el comportamiento es coherente consigo mismo y falsificable (4 tests,
  falsificadores F19 y F20 OBSERVED).
- **No:** no está verificado contra una especificación normativa, porque la que
  existe dice lo contrario y la que el UAT cita no está.

## Impacto en datos: medido, no temido

El cambio de dominio `v1 → v2` no invalida ninguna identidad almacenada:

- `grep basis_hash crates/sddk-storage/src/` → **0 resultados** (no hay columna
  en el schema SQLite).
- `grep KnowledgeBasis crates/sddk-storage/` y rutas de persistencia del engine
  → **0 resultados**.

`KnowledgeBasis` **no se persiste**: vive en memoria. El tag `v2` sigue siendo
correcto (los dominios no colisionan) pero hoy es una propiedad preventiva, no
un riesgo operativo.

## Criterio de cierre

Esta deuda se cierra cuando **una** de estas se cumple:

- (a) `arch-spec-A3-S1-knowledge-substrate.md` pasa a `status: accepted` con
  REQ-A3S1-021 reescrita para incluir `revised_at`, **y** `AT-UAT-019` se
  reescribe para citar la spec real en vez de un ADR inexistente; o
- (b) el commit de session-62 se revierte y el doc de `revise` se corrige para
  describir la semántica vigente, **y** se resuelve por separado el defecto de
  `KMT::evaluate` que compara hash antes que tiempo.

En cualquiera de los dos casos, `AT-UAT-019` debe apuntar a una autoridad que
exista. Un UAT que cita un ADR inexistente debería fallar por sí mismo.
