# SPEC-023 — Knowledge Dependency Overlay

Typed SemanticGraph relations connect observations to derived knowledge:

- ASSERTION_DERIVED_FROM_EVIDENCE
- ASSERTION_DEPENDS_ON_ASSERTION
- DECISION_ASSUMES_ASSERTION
- TRADEOFF_REVISIT_WHEN
- INVARIANT_APPLIES_TO
- FINDING_AFFECTS
- UNIT_DEPENDS_ON_UNIT
- RUNTIME_OBSERVES_EDGE

Impact propagates `POSSIBLY_STALE`, not automatic falsity. Propagation is bounded by relation whitelist, risk, depth and budget and always records a reason path.
