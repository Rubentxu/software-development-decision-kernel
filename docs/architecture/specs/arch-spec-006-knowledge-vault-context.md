---
id: arch-spec-006-knowledge-vault-context
package_local_id: SPEC-006
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-006-KNOWLEDGE-VAULT-CONTEXT.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-006 — KNOWLEDGE-VAULT-CONTEXT

> **Mirror of package SPEC `SPEC-006`.** Repository-native identifier is `arch-spec-006-knowledge-vault-context` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-006` |
| Repository native | `arch-spec-006-knowledge-vault-context` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-006-KNOWLEDGE-VAULT-CONTEXT.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

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
