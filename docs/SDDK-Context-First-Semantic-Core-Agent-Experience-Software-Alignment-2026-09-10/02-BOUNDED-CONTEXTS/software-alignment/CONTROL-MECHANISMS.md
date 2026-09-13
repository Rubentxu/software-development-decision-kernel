# Control mechanisms — advisory first

## Attention model

Usar componentes visibles:

```text
risk
impact radius
change frequency
uncertainty
finding persistence
architectural centrality
knowledge age
```

El resultado es `AttentionPriority`, no `QualityScore`.

## Ratchet

Alignment sólo calcula el delta y lo presenta. Governance decide si existe una policy `no_new_contract_violations`.

## Budget

Puede existir budget de observación (`existing tensions`, `stale critical knowledge`, etc.), pero sólo se vuelve gate si Governance lo declara.

## Watchlist

Lista consultiva de áreas que merecen reevaluación incluso sin threshold.

## Revisit trigger

Un TradeoffRecord de Decision Memory puede referenciar una assertion/metric condition. Alignment detecta la condición y emite `REVIEW_DUE`; no cambia la decisión.

## Contradiction detector

Presenta simultáneamente declared/decided/observed/projected. No elige automáticamente quién tiene razón.
