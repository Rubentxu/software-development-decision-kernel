# SPEC-031 — Enhanced Modes and Capability Negotiation

Profiles:

- BASE
- STATIC_ENHANCED
- RUNTIME_ENHANCED
- FULLY_ENHANCED

Provider states:

`UNAVAILABLE | DORMANT | STARTING | READY | BUSY | INCOMPATIBLE | FAILED`.

Requirement:

`OPTIONAL | PREFERRED | REQUIRED`.

Unavailable optional/preferred evidence is `NOT_EVALUATED`/EvidenceGap. Required evidence may block the calling use case according to policy. SDDK itself remains functional in BASE.
