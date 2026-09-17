---
id: ADR-0127-VERIFIEDBY-TARGETS-EVIDENCEREF
status: accepted
supersedes_history: false
proposed_at: 2026-09-17
accepted_at: 2026-09-17
accepted_by_cycle: p-63676b11dc0ef88f/a4-s15r-verifiedby-evidence-provenance
implementation_evidence:
  - "crates/sddk-engine/src/architecture_graph/types.rs (EvidenceRef node kind)"
  - "crates/sddk-engine/src/architecture_graph/overlay.rs (VerifiedBy repointing)"
  - "crates/sddk-engine/tests/a4_s15r_verifiedby_evidence_provenance.rs (21 pins)"
  - ".sddk/cycles/p-63676b11dc0ef88f-a4-s15r-verifiedby-evidence-provenance/spec.md"
superseded_by: []
---

# ADR-0127 — VerifiedBy Targets EvidenceRef (Provenance Model Correction)

> Status: **accepted** (A4-S15R shipped v1.169.65; acceptance closed by A4-5C)
> Cycle: `p-63676b11dc0ef88f/a4-s15r-verifiedby-evidence-provenance`
> Spec:  `arch-spec-016` part-2, `arch-spec-017` part-2
> Companion ADRs: ADR-0121 (Architecture WHY traversal — explicitly
> deferred this repointing to its own cycle),
>                 ADR-0095 (ArchitectureGraphOverlay is PROJECTION).

## Context

The architecture overlay (AC2) emits `VerifiedBy` from a contract anchor
to a destination node. After A3-S15, the destination was a `spec:` node,
with `EvidenceRef`s attached as relation metadata (`relation.evidence`).

The shape is wrong on three axes:

1. **Provenance confusion.** `SpecifiedBy` (declared intent) and
   `VerifiedBy` (verification evidence) both targeted the spec node.
   `SpecifiedBy` always emitted; `VerifiedBy` only emitted when evidence
   was attached. The two relations became indistinguishable at the
   destination.
2. **No first-class evidence node.** The overlay never projected an
   `EvidenceRef` as a node. Evidence was an attached list on a relation.
   WHY traversal (ADR-0121 §4) reads evidence from the claim's typed
   `evidence_refs`, not from the graph — exactly because the graph had
   no evidence node.
3. **Reachability dead end.** No producer passed non-empty `verified_by`
   to `add_contract_metadata`, so the `VerifiedBy → spec` edge was
   unreachable in practice. The code path existed; the semantics did not.

A3-S15 (ADR-0121 §4) explicitly deferred the repointing to its own
cycle:

> "The `VerifiedBy → spec` relation is left as it is. It is not on the
> WHY path (evidence is read from the claim's typed `evidence_refs`),
> and repointing it is an AC2 semantics change with its own cycle."

That cycle is A4-S15R. A4-0 introduced a universal, typed, deterministic
`EvidenceRef` (kind + locator + cas, `ordering_key` from sha256). That
removes the precondition A3-S15 could not satisfy.

## Decision

### 1. Provenance axes are distinct

```text
contract --SpecifiedBy--> spec         (declared intent; always emitted)
contract --VerifiedBy-->  evidence      (verification evidence; 0/1..N edges)
```

`SpecifiedBy` and `VerifiedBy` are **separate provenance axes**:

- `SpecifiedBy`: contract → `OverlayNodeRef::Spec` (always exactly 1).
- `VerifiedBy`:  contract → `OverlayNodeRef::Evidence(EvidenceRef)` (0
  or 1 per typed-unique `EvidenceRef`).

### 2. Evidence is a first-class PROJECTION node

Add `ArchitectureOverlayNodeKind::EvidenceRef` and
`OverlayNodeRef::Evidence(EvidenceRef)`. The evidence node:

- Is a **PROJECTION** (rebuildable from canonical inputs).
- Has identity = `EvidenceRef` itself (kind + locator + cas).
- Carries the typed identity in `props_inline` so a future WHY traversal
  can recover the `EvidenceRef` without re-parsing the locator.

There is no new authority, no new store, no new CAS, no second semantic
graph. The graph surface stays unified (REQ-AC2-007).

### 3. Identity is typed, not string-inferred

`EvidenceRef::ordering_key()` (sha256 over `kind | locator | cas`) is
both the dedup key for `EvidenceBundle` and the basis of the evidence
node locator (`evidence:{ordering_key}`). Two typed-equal `EvidenceRef`s
produce the same `NodeId`. Two `EvidenceRef`s that differ in any of
kind/locator/cas produce distinct `NodeId`s.

The producer MUST NOT do string-level inference on `locator`
(`starts_with`, `contains`, `split`). The graph consumes types, never
strings.

### 4. Cardinality

- 0 evidence: no `VerifiedBy` edge.
- 1..N evidence: exactly one `VerifiedBy` edge per typed-unique
  `EvidenceRef`. Dedup via `EvidenceBundle::from_refs` (`BTreeSet`
  over `EvidenceRef` Ord, which is sha256-derived).
- Different insertion order: same canonical bytes (digest stable).

### 5. No inverse relation

No `Verifies`, no `EvidenceFor`, no `Proves`, no `SupportsContract`.
The single semantic relation is `VerifiedBy`. Future WHY traversal
walks `VerifiedBy` in reverse if it needs `evidence → contract`.

### 5b. Relation metadata (`relation.evidence`) is not the target

`ArchitectureOverlayRelation.evidence` still exists as a field (used by
the generic `add_relation` path and by `attach_claim_to_unit` for
`ArchitectureClaimedBy`/`ContradictsBy`, per REQ-AC2-017). It is
**annotative only** and MUST NOT be used to reconstruct the `VerifiedBy`
target.

Critically, the new `add_contract_metadata` producer does **not** attach
`evidence` to the `VerifiedBy` edges it emits: the evidence is the
edge's **target** (a typed node), not duplicated metadata. §0 preflight
confirmed no real consumer reads `relation.evidence` for the contract
metadata path (all six real callsites historically passed empty
`verified_by`), so there is no compatibility obligation to keep the
duplicate.

- Annotation status: **not authority**.
- Removal trigger for the generic `evidence` field: a future cycle that
  retires REQ-AC2-017's relation-metadata shape in favour of node
  targets everywhere. Out of scope for A4-S15R.
- The target is never derived from the annotation.

### 6. WHY boundary

A4-S15R does NOT implement WHY. It demonstrates that the projection
contains sufficient provenance for a future traversal:

```text
contract
  ├─ SpecifiedBy → spec
  └─ VerifiedBy  → evidence
```

A single reachability test (`s19_provenance_reachability_contract_to_evidence`)
demonstrates that `contract → evidence` is a one-edge traversal. No
`WhyEngine`, no `why` CLI command, no explanation vocabulary. A4-5b
will decide how to reuse existing WHY surfaces.

### 7. Anti-encroachment

The cycle MUST NOT introduce any dependency on:

- `AdvisoryContext`.
- `ContextCompiler`.
- `InstructionCompiler`.
- `AuthorityEngine`.
- `Capability`.
- `Governance`.
- `Alignment` reducer.
- `IntelligenceLoop` composition.
- Providers (CogniCode, Chronos, JCode).

Pinned by `s20_no_advisory_context_dependency_in_overlay` (grep-time
+ compile-time).

## Consequences

Positive:

- `VerifiedBy` is now distinguishable from `SpecifiedBy` at the
  destination, by graph shape.
- Evidence nodes are typed identity in the overlay, opening the door to
  future traversals (`evidence → contract`, `evidence → spec`, etc.)
  without further schema changes.
- Two rebuilds over the same evidence set produce identical canonical
  bytes — the projection is fully deterministic.
- A4-0 substrate (`EvidenceRef` universal) is now graph-visible.

Neutral:

- `ArchitectureOverlayNodeKind::ALL` grew from 8 to 9. The count pin
  in `architecture_graph/tests.rs` is updated (`bonus_overlay_node_kinds_all_nine_have_unique_tags`).
- `OverlayNodeRef::Evidence` is a new variant. `sddk-cli`'s
  `node_tag` helper renders it as `evidence:<kind>:<locator>`.

Negative:

- None within the cycle budget. A4-5b is the cycle that decides how to
  surface WHY over the new provenance graph; this ADR scopes it out.

## Compliance evidence

- `crates/sddk-engine/src/architecture_graph/types.rs:158-188` — variant + tags.
- `crates/sddk-engine/src/architecture_graph/types.rs:303-321` — `OverlayNodeRef::Evidence` + helper.
- `crates/sddk-engine/src/architecture_graph/overlay.rs:148-265` — repointed emission.
- `crates/sddk-engine/src/architecture_graph/overlay.rs:412-447` — node kind tag + locator.
- `crates/sddk-engine/src/architecture_graph/tests.rs:467-471` — count pin updated to 9.
- `crates/sddk-engine/tests/a4_s15r_verifiedby_evidence_provenance.rs` — 21 pins.
- `crates/sddk-cli/src/architecture_cmd.rs:411-413` — CLI render.

## Cycle closure

- `FU-A3-S15-3` (blocks A4-5b) → CLOSED with §0 evidence.
- A3-S15 receipt: NOT rewritten. Addendum at
  `docs/handoff/HANDOFF-2026-09-15-session-close.md` (or successor
  handoff) records A4-S15R's resolution of the deferred item.
- A4-5b unblocks after A4-S15R closes + `FU-A3-S15-3` resolves. **A4-5b
  does NOT auto-open.**
