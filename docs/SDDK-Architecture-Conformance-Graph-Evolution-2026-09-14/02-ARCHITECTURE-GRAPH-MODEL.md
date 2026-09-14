# Architecture Graph Model

## One graph, multiple typed overlays

SDDK already requires one rebuildable `SemanticGraphProjection`. Architecture intelligence SHALL extend that graph through typed nodes/relations and views. It SHALL NOT introduce `ArchitectureGraph` as a competing persistence authority.

## Suggested node kinds

The list is deliberately extensible and domain-scoped rather than a universal ontology:

```text
SoftwareUnit          module / crate / package / class / function / endpoint
BoundedContext
ArchitecturalConcept  authority, evidence, run, decision, ...
DecisionRef           ADR / DecisionMemory ref
SpecRef
ContractRef
TestRef
UatRef
StorageSurface
EventType
ProjectionType
ProviderRef
CompatibilityPath
ParadigmProfile
DslSurface
VerificationReceiptRef
```

## Core relations

Raw/observed relations:

```text
OWNS
DEPENDS_ON
CALLS
READS
WRITES
EMITS
CONSUMES
PROJECTS
DERIVES_FROM
EXPOSES
IMPLEMENTS
REQUIRES
INTEGRATES_WITH
```

Decision/traceability relations:

```text
DECIDED_BY
SPECIFIED_BY
VERIFIED_BY
SUPPORTS
CONTRADICTS
SUPERSEDES
COMPATIBLE_WITH
REPLACES
PRODUCED_BY
```

Alignment results SHOULD be represented as assessment objects connected to evidence rather than raw `VIOLATES` edges masquerading as facts.

## Authority map as query

`AuthorityMap` is a view:

```text
Concept ─DECIDED_BY→ Decision
Concept ─IMPLEMENTED_BY→ SoftwareUnit
SoftwareUnit ─WRITES→ StorageSurface
Contract ─VERIFIED_BY→ Test/UAT/Receipt
```

A shadow-authority candidate exists when graph evidence suggests overlapping write/decision responsibility for the same concept. It is an assessment candidate, not automatically a defect.

## Ownership map as query

```text
BoundedContext ─OWNS→ ArchitecturalConcept
SoftwareUnit ─IMPLEMENTS→ ArchitecturalConcept
SoftwareUnit ─DEPENDS_ON→ SoftwareUnit
```

This permits deterministic questions:

- does a concept have zero, one or multiple owners?
- does a dependency cross a forbidden boundary?
- is a host/provider type leaking inward?
- is derived state being treated as canonical?

## Behavior maps

For event/reactive flows SDDK SHOULD expose a generated behavior map:

```text
EventType → ReactiveBehavior → Object/Relation/Event → next behavior
```

This is documentation/projection, not scheduler authority. It is especially useful for JCode reactive Verify and provider-driven evidence updates.

## Graph identity and provenance

Every graph assertion SHOULD record enough basis to reproduce or invalidate it:

```text
source_basis
revision_basis
analyzer_basis
provider_basis?
evidence_refs[]
observed_at
freshness/status
```

KMT identity/freshness controls invalidation; SemanticGraph supplies cross-tree navigation.

## Critical invariant

**The graph explains and connects authority; it never becomes authority merely because it is convenient to query.**
