---
id: ADR-0156-EVIDENCE-STATE-REPLACES-CONFIDENCE
title: G01 exige refs, no confianza - el estado evidencial sustituye a la magnitud
status: accepted
proposed_at: 2026-10-03
accepted_at: 2026-10-03
cycle: p-63676b11dc0ef88f/c3m4-evidence-states
accepted_by_cycle: p-63676b11dc0ef88f/c3m4-evidence-states
supersedes: null
superseded_by: null
component: evidence
surface: crates/sddk-domain/src
closes:
  - INC-DEBT-066
---

# ADR-0156 — G01 exige refs, no confianza: el estado evidencial sustituye a la magnitud

- **Status:** `accepted`
- **Date:** 2026-10-03
- **Ciclo:** `p-63676b11dc0ef88f/c3m4-evidence-states`
- **Cierra:** INC-DEBT-066
- **Ruta de roadmap:** C3m.4 — *estados evidenciales en lugar de confidence mágica*
- **Relaciona:** ADR-0151 (ancla de firma), ADR-0155 (el mismo patrón de
  homonimia que ya corrigió KMT)

---

## Contexto

C3m.4 pedía eliminar la «confidence mágica» de Snapshot L1. Lo medido (y
documentado en el `SCOPE-CONTRACT` del ciclo) es que el número de
`storage_snapshot_l1_consumer.rs` no era mágico por capricho: **no lo exigía
nadie**.

La cadena medida fichero a fichero:

| Referencia | Qué dice |
|---|---|
| Fila canónica de G01 (`UAT-MATRIX.md:42`) | criterio «snapshot Planning reconciliado, A bloquea B»; aceptación «Agenda indica candidato/causa y refs; NO autorización de ejecución por `project_next`» — **0 menciones de `confidence`** |
| SCOPE que transcribe esa fila (`aiw-s7-secretary-attention/SCOPE-CONTRACT.md:43`) | G01 = «snapshot Planning reconciliado, A bloquea B» — **0 menciones en todo el fichero** |
| `RECEIPT.md:42` del ciclo `aiw-s7b` | «G01 \| **PASS** \| evidence ref = …; **confidence 0.95/0.5 by head**» |
| `UAT-EVIDENCE.yaml:11` | «… **confidence tracks durability (0.95 with head > 0, 0.5 otherwise)** » |

Y el número, medido sobre el repo:

- **9** lecturas de un campo `confidence` en código de producto, **ninguna** de un
  `SecretaryProposal`.
- Los **únicos** consumidores de la confianza de una propuesta eran **los dos
  tests que comprobaban que valía 0.95 o 0.5**.
- El `//! Spec:` del módulo (**`:6`**) apuntaba a un `SCOPE-CONTRACT.md` que
  **nunca se commiteó**.

## Decisión

**Parte 1 — qué exige G01.** G01 exige lo que su fila dice: que la agenda
indique candidato, causa y refs, y que **no** autorice ejecución. **La cláusula
de la confianza no formaba parte del criterio**, y se retira de los dos
artefactos que la habían introducido. La fila canónica **no se reescribe**: el
error estaba en la evidencia que la sostenía, no en el criterio.

**Parte 2 — qué es la confianza en una emisión.** `SecretaryProposal.confidence:
f64` pasa a `SecretaryProposal.evidence: EvidenceState`, con los estados que el
propio roadmap nombra para C3m.4: `Missing`, `Observed`, `Empty`, `Stale`,
`Conflicted`.

El consumidor de snapshot mapea lo que **de verdad** observa:

```rust
let evidence = if snapshot.log_head > 0 { EvidenceState::Observed }
               else { EvidenceState::Empty };
```

La frase que el código viejo no podía sostener —«an empty fact log is reported
at half confidence»— desaparece porque era falsa: **un log vacío no es media
observación, es la ausencia de una**, y 0.5 frente a 0.95 no expresa esa
diferencia, la disfraza de una cuestión de magnitud. `Empty` y `Observed` la
expresan, y `Stale`/`Conflicted` son distinciones que un número **no puede**
hacer en absoluto.

**Parte 3 — el segundo lugar donde el número no discriminaba.** `ExpansionTrigger`
llevaba `confidence: f64` **dentro de su identidad content-addressed**, lo que lo
hacía parecer un discriminante real. Medido: `ExpansionTrigger` tiene **un solo
punto de construcción en todo el repo**, y es un helper de test con `0.9`
constante — el componente del hash **nunca ha discriminado nada**. Pasa a
`evidence: EvidenceState`.

> **Una afirmación mía que quedó falsificada por la medición, y que por eso va
> escrita.** El `SCOPE-CONTRACT` afirmaba «un sitio que escribe, ninguno que
> lee» sobre `confidence`. Al implementar apareció un **segundo** escritor en
> producción, `dynamic_expansion.rs:415`, que pasa `trigger.confidence`. El
> grep que hice antes buscaba el *literal* y no vio la *variable*. Es la
> **quinta** vez en esta sesión que se mide mención donde se iba a medir uso, y
> la primera que falsea algo escrito en un documento publicado.

**Parte 4 — la validación desaparece con el número.** `propose()` tenía una
comprobación `0.0..=1.0` y un error `InvalidConfidence` que solo existían para
decidir si un número arbitrario caía en un intervalo. Un enum no tiene rango que
incumplir, así que **la comprobación se va con el número**, y `InvalidConfidence`
deja de ser alcanzable desde `SecretaryL1Engine::propose`.

**Parte 5 — la cita rota.** El `//!` del módulo pasa a citar **las dos rutas que
existen**: la fila canónica y el SCOPE de `aiw-s7-secretary-attention` §S7-STOP-2,
declarando además que la cláusula de confianza no estaba en ninguna de las dos.

## Criterios de aceptación, medidos uno a uno

| # | Criterio | Resultado |
|---|---|---|
| 1 | `SecretaryProposal` ya no lleva `confidence: f64` | verificado |
| 2 | `EvidenceState` tiene los cinco estados que nombra C3m.4 | verificado |
| 3 | El consumidor de snapshot mapea `log_head > 0` → `Observed`, si no `Empty` | verificado |
| 4 | `ExpansionTrigger` ya no lleva `confidence: f64` | verificado |
| 5 | `InvalidConfidence` ya no es alcanzable desde `propose()` | verificado |
| 6 | La fila canónica de G01 **no** se modificó | verificado — el error estaba en la evidencia |
| 7 | `RECEIPT.md` ya no declara la cláusula de confianza en G01 | verificado |
| 8 | `UAT-EVIDENCE.yaml` ya no la declara | verificado |
| 9 | Los dos tests que fijaban 0.95/0.5 afirman el estado, no la magnitud | verificado |
| 10 | El `//!` del módulo cita ficheros que existen | verificado con `test -f`, no leyendo la línea |
| 11 | Workspace verde | verificado |

## Lo que este ADR NO decide

- **No toca `UatOracleAssessment.confidence`**, que **sí se discrimina**
  (`LowAiConfidence: mejor confidence < 0.7`, `uat.rs:540`) y el CLI la muestra.
  Es un tipo distinto con consumidores reales, y borrarlo sería tirar una
  propiedad que existe. Lo que este ADR hace es dejar escrito **dónde termina el
  alcance de «confidence sin consumidor»**: en el campo que nadie leía.
- **No da consumidor de producto a `dynamic_expansion`.** Sigue siendo uno de los
  10 módulos «solo tests» de INC-DEBT-065, y eso es un inventario con su propia
  entrada, no una consecuencia de este ADR.
- **No reescribe la fila de G01.** Reconcilia la evidencia que la sostenía.
- **No reabre el caso de las certifying UAT de AIW-S7b** entero: eso es C3n.2.
