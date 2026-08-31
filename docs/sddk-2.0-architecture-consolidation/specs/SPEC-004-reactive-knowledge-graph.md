# SPEC-004 — Reactive Knowledge and Evidence Graph

**Status:** Proposed

## 1. Goal

Turn SDDK knowledge from passive documentation into a living, queryable and reactive world model while preserving the deterministic kernel as authority.

## 2. Source model

```text
Ledger events -> GraphProjection -> GraphStore
```

The graph MUST be rebuildable from ledger events. GraphStore is an adapter, not authority.

## 3. Graph primitives

- typed nodes;
- typed relations;
- provenance on every derived node/relation;
- content/version identity;
- pattern queries;
- bounded views/scopes;
- relation behaviors;
- staleness metadata.

## 4. Behaviors

A behavior has:

- trigger event types and/or graph pattern;
- bounded read view;
- deterministic or agentic execution class;
- declared outputs;
- declared capability needs;
- budgets;
- evidence requirements.

Behaviors SHOULD be registered from Behavioral IR/pack declarations rather than ad hoc global code.

## 5. Pattern subscriptions

Initial patterns SHOULD cover high-value engineering shapes without implementing full Cypher:

- node type and property predicates;
- directed relation chains;
- `EXISTS` / `NOT EXISTS` subpatterns;
- simple temporal/version predicates;
- bounded path depth.

Examples:

```text
Requirement -> implemented_by -> Commit
AND NOT EXISTS Commit -> verified_by -> Test
```

```text
Feature(risk=critical)
AND NOT EXISTS Feature -> accepted_by -> HumanAcceptance
```

## 6. Relation behaviors

Coordination that semantically belongs to an edge MAY be attached to the relation type.

Examples:

- `depends_on`: block/unblock dependent work;
- `verifies`: mark verification stale when the verified subject changes;
- `governs`: mark ADR/document potentially stale after governed component drift;
- `contradicts`: create a review proposal when evidence conflicts.

Relation behaviors MUST obey the same no-direct-side-effect rule.

## 7. Read views

Behaviors SHOULD receive bounded graph views rather than unrestricted graph access. Views can filter by:

- node/relation types;
- hop depth;
- project/cycle/frame;
- event recency;
- provenance source;
- pack namespace.

This enables context minimization and context-read tracing.

## 8. Graph diff

The graph layer SHOULD support structural and semantic diffs across:

- releases;
- cycles;
- forks;
- architecture snapshots;
- UAT states.

A semantic diff may add metrics and interpretation but must retain a structural base diff.

## 9. Proposed CLI

```text
sddk graph query <pattern>
sddk graph why <entity>
sddk graph diff <a> <b>
sddk graph stale
sddk graph rebuild
```
