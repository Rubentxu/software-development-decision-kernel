# SPEC-036 — Moldable View Contract

**Status:** Proposed

## Goal
Let kernel and packs contribute task-specific views over the same persisted projections/graph without coupling domain logic to one UI.

## View descriptor

```yaml
id: view://routing/failover-trace
applies_to:
  - provider_failure
  - attempt
requires:
  graph_query: query://failover-causal-neighborhood
renderer: graph
interactions:
  - drill_to_event
  - open_attempt
  - compare_route
```

## Renderer families
- table;
- journal/timeline;
- graph;
- metric/cards;
- dependency/tree;
- evidence chain;
- C4/UML/custom pack renderer metadata.

## Separation
A view may query projections but never mutate operational state directly. Actions from a view invoke application commands that are governed normally.

## Static Cockpit
View descriptors compile into the static snapshot when supported. Unsupported rich renderers degrade to a table/links representation rather than requiring a server.

## Pack extension
Packs may ship views with their manifest, enabling UAT, incident, security and supply-chain lenses without bloating core UI code.
