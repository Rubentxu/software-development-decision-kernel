# Knowledge bounded context

## Owns

KnowledgeAssertion, KnowledgeBasis, Knowledge Merkle Tree, staleness, semantic graph projection, knowledge cards y evidence diversity.

## Explicitly does not own

No decide si un diseño es bueno ni impone un paradigma.

## Source layout

Domain types: `crates/sddk-domain/src/knowledge/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/knowledge/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
