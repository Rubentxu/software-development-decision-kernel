# Knowledge domain model

## Aggregates / concepts

```text
KnowledgeAssertion
KnowledgeAssertionRevision
KnowledgeBasis
SoftwareUnitIdentity
SoftwareUnitFingerprint
KnowledgeNode
StalenessRecord
EvidenceCoverage
KnowledgeCardView (projection)
```

## KnowledgeAssertion

```text
subject
predicate
value
status: OBSERVED | DECLARED | INFERRED | DECIDED | VERIFIED | CONTRADICTED | STALE | SUPERSEDED | UNKNOWN
basis
producer
EvidenceRefs[]
valid_from
valid_until?
supersedes[]
invalidation_dependencies[]
```

`INFERRED` nunca se confunde con `OBSERVED`. `DECIDED` exige DecisionRef. `VERIFIED` exige verificador/evidence reproducible.

## KnowledgeBasis

```text
source_root
structure_root?
dependency_root?
behavior_root?
knowledge_root
decision_memory_head
alignment_lens_set_hash
policy_hash?
analyzer_set_hash
```

El basis permite responder exactamente sobre qué versión del mundo se calculó una evaluación.
