# SPEC-022 — Knowledge Merkle Tree

## Scope hierarchy

`SYSTEM -> BOUNDED_CONTEXT -> PACKAGE/CRATE -> MODULE/DIRECTORY -> FILE -> SYMBOL(optional)`.

## Fingerprint vector

```text
source_hash
structure_hash
symbols_hash
dependency_hash
behavior_hash?
knowledge_hash
intent_hash
decision_refs_hash
analyzer_set_hash
```

A root is an auditable `KnowledgeBasis`, not merely one opaque digest.

## Semantics

- hierarchical parent invalidation is cheap;
- a child change does not imply every parent assertion is false;
- semantic cross-links are handled by SemanticGraph overlay;
- analyzer/policy/decision changes can invalidate knowledge even when source does not change.

## Freshness

`CURRENT | INVALIDATED | POSSIBLY_STALE | STALE | RECONCILING`.
