---
id: INC-DEBT-063-FU-PLANNING-DOCUMENTS
title: "Los documentos de intencion del directorio de recibos declaran el mismo cycle_id inexistente que ya se corrigio en los de resultado"
status: open
severity: low
priority: P3
revalidated_at: 2026-10-06
revalidated_in_session: session-89
fingerprint: "planning_documents_declare_nonexistent_cycle_id"
fingerprint_aliases: []
cluster_id: CL-DOCS
created: 2026-10-06
created_by: MiniMax Code (mvs_d352999db1f44ac38b455be1d93abb09)
detected_at: 2026-10-06
detected_in_session: session-89
component: docs
surface: docs/roadmap/receipts/**/PRE-FLIGHT.md, docs/roadmap/receipts/**/SCOPE-CONTRACT.md
related: [INC-DEBT-063]
references:
  - docs/debt/INC-DEBT-063-CYCLE-RECEIPTS-DECLARE-NONEXISTENT-CYCLE-ID.md
---

## Qué es

Cuando se corrigió INC-DEBT-063 (2026-10-06) el alcance se acoto a los
documentos que **afirman resultado** — `RECEIPT*.md` y `CLOSURE.md` — porque en
ellos un `cycle_id` falso deja al lector sin nada que comprobar. Los documentos
de **intención** del mismo directorio afirman lo mismo y **no se tocaron**.

Medido el 2026-10-06 sobre el ledger real de `p-63676b11dc0ef88f`:

| Clase | Ficheros | Aserciones |
|---|---|---|
| `PRE-FLIGHT.md` con un `cycle_id` inexistente | **16** | 16 |
| `SCOPE-CONTRACT.md` con un `cycle_id` inexistente | **11** | 11 |
| `**Cycle:**`/`**Ciclo:**` con un hito del roadmap (`C3e`, `C2a`, …) en vez de un id | — | **31** en 26 textos distintos |
| Ficheros que declaran un `cycle_id` **existente** | 35 | 35 |

Para contraste, en el alcance ya corregido eran **11 ids fabricados + 2 campos mal
etiquetados**. La cifra de INC-DEBT-063 ("tres recibos") estaba casi tres veces
por debajo de lo que la medición da.

## Por qué NO se corrigieron aquí

Es una decisión de alcance, no un límite de medición, y la razón es la misma que
la deuda madre da para no crear los ciclos: **reescribir historia**.

Un `PRE-FLIGHT.md` que nombra `p-.../identity-alias` afirma lo que el autor
pretendía al escribir el plan. No afirma que el trabajo ocurriera bajo ese ciclo
—el trabajo ocurrió, con commits, y el plan es un documento de lo que se iba a
hacer. Un `RECEIPT.md` afirma lo contrario: que lo que se hizo pasó por ese ciclo,
y eso sí deja al lector sin nada que comprobar.

Y el coste de corregirlo no es solo de honestidad: son 27 cabeceras de documentos
de planificación reescritas a posteriori, en un repo cuya política de traslados y
reescrituras (`docs/history/README.md`) exige mapa de enlaces y gates
corresponding. Eso es un bloque con su propio SCOPE, no un efeito colateral de
este.

## La salida, y hay dos

1. **Barrido mecánico** de las 27 cabeceras, con mapa de enlaces y un guard
   ampliado a `PRE-FLIGHT.md` y `SCOPE-CONTRACT.md`. Es lo que hará que
   `tests/test_receipt_cycle_id_authority.sh` pueda mirarlos sin recortarse.
2. **Declararlo en el propio documento**, como se hizo con los de resultado: que
   el campo diga lo que es y que el lector sepa que ese ciclo no se emitió.

La opción 2 es la coherente con lo ya hecho y es la más barata. La 1 es la que
cierra la clase entera.

## Gravedad

**low/P3**, y es una decisión: el daño es documental, no afecta al runtime, y el
guard que sí existe (`test_receipt_cycle_id_authority.sh`) **no** cubre estos
ficheros — lo dice en su cabecera, con estos números, precisamente para que el
hueco sea visible y no heredado.

Cerrado por: —
Cierre previsto: sweep con su propio SCOPE.
