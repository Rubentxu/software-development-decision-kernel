---
id: ADR-0122-EVIDENCE-OBSERVES-SOFTWARE
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a4-0-verification-provenance-foundation
implementation_evidence:
  - "crates/sddk-engine/src/observation/mod.rs (public surface)"
  - "crates/sddk-engine/src/observation/types.rs (SoftwareRelation, RelationId, SoftwareObservation, ObservationOrigin, ObservationBasis)"
  - "crates/sddk-engine/src/observation/resolution.rs (EvidenceResolution; contradictions as first-class)"
  - "docs/architecture/specs/arch-spec-042-evidence-observation-provenance.md"
superseded_by: []
related_adrs:
  - "ADR-0094-ONE-CANONICAL-FACT-LOG"
  - "ADR-0098-ONE-SEMANTIC-GRAPH"
  - "ADR-0100-UNIVERSAL-EVIDENCE"
  - "ADR-0121-ARCHITECTURE-WHY-TRAVERSAL"
stale_after: 2027-03-15
---

# ADR-0122 — Evidence observes software; the producer observes, SDDK decides

## Context

A3's closeout recorded a structural gap: the provenance chain

```text
Finding → Contract → Decision/Spec → Evidence -X→ SoftwareRelation
```

has no last leg. Evidence is a `(provider, reference)` tuple attached as relation
metadata; there is no evidence node, no edge to a software relation, and no
`SoftwareRelation` ADT at all. Relations are only expressible as graph edges or as
rendered strings (`"comp:domain -> comp:provider"`).

WHY reports the gap as an `unresolved_edge` rather than inventing the leg. That was
the correct local call, but it means every future consumer would have to reinvent
the same missing representation.

## Decision

**1. A software relation is an ADT with a deterministic identity, never a rendered
string.**

```text
SoftwareRelation { from: SoftwareEntityRef, kind: CoreRelationKind, to: SoftwareEntityRef }
RelationId        = sha256("sddk.software_relation.id.v1|" | from | kind | to)
```

Timestamps, messages, severities and renderer text are excluded from identity. This
is the lesson A3 applied to `FindingId`, applied again.

**2. An observation carries provenance, and its identity excludes the volatile.**

```text
SoftwareObservation { id, subject, evidence, origin, basis, freshness, producer }
ObservationId = sha256(namespace | subject | evidence | origin | basis)
```

`revision` and `knowledge_basis` are inputs because they are clock-stable; A3
measured that `semantic_graph_digest` is not, which is why the substrate does not
use it for identity.

**3. The producer observes; SDDK interprets.**

`ObservationOrigin` (`DeterministicLocal | StaticProvider | RuntimeProvider |
HumanDeclared | Inferred`) answers *who and how*. It is a different axis from the
existing provenance classes, so it is a different enum — and crucially:

```text
Evidence ≠ truth
Provider ≠ authority
Observation ≠ decision
```

Any producer (a deterministic local analyzer, CogniCode's static observations,
Chronos's runtime traces) can emit observations **without becoming authority**.

**4. No third evidence type.** The substrate references the existing universal
`evidence_ref::EvidenceRef`. AC1's `(provider, reference)` tuple stays as AC1's
concern, bridged by the existing converter.

**5. Contradictions are first-class. Never latest-wins.**

Two incompatible observations of one relation coexist; `EvidenceResolution` carries
`Supported | Contradicted | Conflicted { supporting, contradicting } |
Insufficient(gap)` so no consumer is forced to collapse them into a number.

**6. One projection.** Observations project into the single
`SemanticGraphProjection`. No `EvidenceGraph`, no `ObservationGraph`, no second DB.

## Consequences

- `why architecture` can close `evidence → observes → software relation` when real
  evidence exists, and keeps its `unresolved_edge` when it does not. The truthful
  distinction survives; nothing is fabricated to make the chain look complete.
- Generic Verify (A4-1) and DebVerify (A4-2) get a subject and an evidence channel
  instead of minting their own.
- Missing data is an explicit gap (`Unknown`/`Insufficient`), never a pass.

## Alternatives considered

- **Identify a relation by its rendered string.** Rejected: two different relations
  can render alike, and a renderer change would move every identity.
- **Attach observations as evidence on existing graph relations.** Rejected: it
  keeps evidence as anonymous metadata, so "what observed this, on what basis, with
  what freshness" stays unanswerable — the exact question A4-0 exists to answer.
- **Let observations overwrite each other.** Rejected: last-write-wins destroys the
  static/runtime disagreement that CogniCode and Chronos will produce, which is
  precisely the information Alignment needs.
- **A new `ObservationGraph`.** Rejected: a second graph is a second authority
  (ADR-0098).
- **Extend `EvidenceKind` with producer classes.** Rejected: `EvidenceKind`
  describes where evidence lives, not who produced an observation; conflating the
  two axes would force a future producer to declare a storage location it does not
  have.
