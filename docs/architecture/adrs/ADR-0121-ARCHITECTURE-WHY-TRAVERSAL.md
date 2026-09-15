---
id: ADR-0121-ARCHITECTURE-WHY-TRAVERSAL
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-15-why-architecture
implementation_evidence:
  - "crates/sddk-engine/src/architecture_why/mod.rs (public surface; why-not-graph-why doc)"
  - "crates/sddk-engine/src/architecture_why/types.rs (ArchitectureWhy, WhyContract, WhyAssessment, WhyIntent, WhyEvidence, WhyUnresolved, WhyNotReason)"
  - "crates/sddk-engine/src/architecture_why/traverse.rs (explain; reads only precomputed AC1/AC2/AC5 state)"
  - "crates/sddk-engine/src/architecture_debverify/finding_id.rs (FindingId, FindingBasis)"
  - "crates/sddk-engine/src/architecture_graph/overlay.rs (SpecifiedBy emitted unconditionally)"
  - "crates/sddk-cli/src/why_cmd.rs (sddk why architecture; explicit namespace resolution)"
  - "crates/sddk-cli/tests/architecture_why_cli_e2e.rs (e2e suite)"
  - "docs/architecture/specs/arch-spec-A3-S15-why-architecture.md (cycle-bounded spec, 19 REQs)"
superseded_by: []
related_adrs:
  - "ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS"
  - "ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY"
  - "ADR-0117-DEBVERIFY-GLOBAL-ARCHITECTURE-AUDIT"
  - "ADR-0119-ARCHITECTURE-CONFORMANCE-RECEIPT"
  - "ADR-0120-DECLARATIVE-ARCHITECTURE-CONTRACTS"
stale_after: 2027-03-14
---

# ADR-0121 — Architecture provenance traversal is a read surface with a stable finding identity

## Context

A3-S14 made AC5's findings inspectable, which exposed two holes.

**First, a finding had no identity.** `DebVerifyFinding` is `{kind, severity,
subjects, contract_ids, message}`. A3-S14 had already measured that `subject` is
not an identity: `comp:dup` produces both a `shadow_authority` and a
`contradiction`. Without an identity there is no way to ask "explain *this*
finding".

**Second, provenance was not traversable.** The intended chain is

```text
finding → claim assessment → contract → decision/spec → evidence → software relation
```

and two links were missing or unmodelled. Measuring the AC2 vocabulary (8 node
kinds, 14 relation kinds) against it showed:

- `contract → spec` had a node but no stable edge. `add_contract_metadata`
  creates the `spec:` node unconditionally but attaches no relation unless
  `verified_by` is non-empty — and then emits `VerifiedBy` pointing at the *spec*
  node, because there is no evidence node kind.
- `evidence → software relation` does not exist at all. Evidence is a
  `(provider, reference)` tuple attached as **relation metadata**; there is no
  evidence node and no edge from an evidence reference to a software relation.

There was also a naming question: `sddk graph why` already exists as the
canonical WHY surface for the **event-ledger** reactive graph.

## Decision

**1. A finding gets a deterministic id, and its basis is deliberately not the
graph digest.**

```
FindingId = sha256(domain | revision | knowledge_basis | contract_set_digest
                   | kind | subjects… | contract_ids…)
```

`message` is excluded because it can change for a redaction alone. `severity` is
excluded because it is a pure function of `kind`. `now` and
`semantic_graph_digest` are excluded because they are **clock-dependent** —
measured: the overlay embeds each linkage claim's `evaluated_at`, so that digest
moves with the clock, while `contract_set_digest` does not.

That exclusion is load-bearing, not tidiness. The workflow is two invocations:

```text
sddk architecture findings            → shadow_authority <finding-id>
sddk why architecture <finding-id>     → explanation
```

With the default wall clock those are different `now` values, so a
clock-dependent basis would make the id impossible to feed back and the feature
unusable in its primary use case.

**2. `SpecifiedBy` joins the AC2 relation vocabulary**, emitted unconditionally
by `add_contract_metadata` (contract anchor → spec node). The vocabulary pin
moves 14 → 15. This is a vocabulary extension, not a new graph: the overlay
remains a rebuildable projection of the declaration.

**3. The `evidence → software relation` leg is reported as UNKNOWN, never
inferred.** `ArchitectureWhy::unresolved_edges` always carries it, with a reason
stated in terms of the substrate. Closing it is a substrate change with its own
blast radius; making the gap visible is this decision.

**4. The architecture WHY is a new surface (`sddk why architecture`), and it is
not an alias of `graph why`.** The two answer different questions over different
substrates: `graph why` reads the event-ledger graph; `why architecture` reads
the AC2 overlay plus AC1 claims plus AC5's audit. `sddk architecture why` is not
created (a second surface for one question), and `sddk why graph` is not created
(that would be an alias of `graph why`).

**5. The traversal evaluates nothing.** `explain` reads only state the caller
already computed — the declared contracts, the AC1 claims the overlay registered,
and the audit. No `ContractEvaluation::evaluate`, no second audit, no graph
mutation, no canonical append. `receipt`, `findings` and `why` share one
`ArchitectureContext` seam so they cannot prepare three different universes; the
AC4 delta stays caller-specific, which preserves A3-S14's observation that
`findings` reports where `receipt` errors out.

**6. A missing leg is an error, not an empty answer.** An audit failure is exit 2,
never `0 findings`; an id matching both namespaces is an ambiguity error, never a
silent choice.

## Consequences

- Finding ids are stable across clocks and re-runnable: an id printed by
  `findings` can be handed to `why`.
- Two findings on one subject stay distinct, because `kind` participates in the id.
- A finding named by a contract that is not in the validated set, and a contract
  whose subject unit is not declared, both produce an explicit
  `not_evaluated` assessment plus a `why_not` reason rather than a blank leg.
- `WhyNotReason` exists as a six-variant ADT populated from the claims'
  `missing_evidence` and from the unresolved legs. **No `why-not` surface is
  created**; the ADT is the prepared seam.
- The `VerifiedBy → spec` relation is left as it is. It is not on the WHY path
  (evidence is read from the claim's typed `evidence_refs`), and repointing it is
  an AC2 semantics change with its own cycle.

## Alternatives considered

- **Use the audit digest as the finding basis.** Rejected: it ingests the
  clock-dependent `graph_digest`, so ids would move with `now` and could not be
  fed back between two CLI invocations.
- **Use `subject` as the identity.** Rejected: measured in A3-S14, one subject
  produces two findings.
- **Resolve `CONTRACT_OR_FINDING` by shape** ("64 hex means a finding").
  Rejected: a declaration may legally carry a 64-hex contract id, so shape cannot
  decide a namespace. Resolution is namespace-based and fails closed on ambiguity.
- **Extend `sddk graph why`.** Rejected: the architecture substrate is not in the
  event graph, and materialising it there would make the declaration a graph
  authority, against ADR-0120.
- **Fabricate an `evidence → observes → software` edge by treating the attached
  relation as the target.** Rejected: it would produce provenance that is not in
  the substrate — the one thing an explanation must not do.
- **Surface `why-not` now.** Rejected as scope creep: the brief asks for the ADT
  to be prepared, not for a second surface.
