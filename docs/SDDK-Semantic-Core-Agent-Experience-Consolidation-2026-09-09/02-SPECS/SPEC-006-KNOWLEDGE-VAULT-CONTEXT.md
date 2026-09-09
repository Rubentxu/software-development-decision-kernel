# SPEC-006 — Knowledge sources, Vault and Context Compiler

## Boundaries

- KnowledgeSource: material available for retrieval (code/docs/vault/external connectors).
- Vault: curated human-readable KnowledgeSource.
- Memory: durable typed knowledge acquired/approved by SDDK.
- SemanticGraph: relational projection/index.
- ContextCapsule: bounded ephemeral selection for one task/agent.

## Context Compiler pipeline

```text
candidate sources
→ scope filtering
→ validity/staleness filtering
→ exact/graph retrieval
→ optional semantic ranking
→ authority/evidence/recency scoring
→ dedup/diversity
→ token-budget packing
→ ContextCapsule + RetrievalReceipt
```

## Retrieval tiers

- HOT: invariants, objective, current decisions, blockers.
- WARM: relevant memory/evidence/code topology.
- COLD: historical branches/details, fetch on demand.

## Explainability

`sddk context explain --for <target>` reports why each item was included/excluded, authority class, staleness, score/budget cost and source provenance.

Embeddings/vector stores may rank candidates but never become Memory Authority.
