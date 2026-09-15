---
id: ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a4-0-verification-provenance-foundation
implementation_evidence:
  - "docs/architecture/specs/arch-spec-043-generic-verify.md (contract-ready)"
  - "docs/architecture/specs/arch-spec-044-generic-debverify.md (contract-ready)"
  - "docs/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/02-BOUNDED-CONTEXTS/verification/DEB-VERIFY.md (historical source)"
superseded_by: []
related_adrs:
  - "ADR-0122-EVIDENCE-OBSERVES-SOFTWARE"
  - "ADR-0117-DEBVERIFY-GLOBAL-ARCHITECTURE-AUDIT"
stale_after: 2027-03-15
---

# ADR-0123 — Verify and DebVerify are distinct, and both are kernels

## Context

A3 delivered AC4 (architecture Verify) and AC5 (architecture DebVerify) early.
Both work, and both are **architecture-specific**. A4 must generalise them without
duplicating their receipts, evidence or statuses, and without collapsing them into
one function with a flag.

The historical verification context already states the distinction correctly:
DebVerify "can discover stale/debt in files not changed recently. It is not
`verify --all`."

## Decision

**1. The two are distinct by *scope*, not by degree.**

```text
Verify     delta-scoped:      what changed → what can be affected → what must be refreshed
DebVerify  baseline-challenge: what is wrong with the baseline, including where nothing changed
```

`verify --all` is **not** DebVerify. Running Verify over everything inherits
Verify's change-shaped reasoning and therefore misses the staleness DebVerify
exists to find.

**2. Both are kernels, not per-domain engines.**

```text
Generic Verify    ← ArchitectureVerificationDomain (AC4 becomes a specialization)
Generic DebVerify ← Architecture challenge          (AC5's detectors become one strategy)
```

There must not be an "architecture verify engine", a "security verify engine" and
a "performance verify engine", each minting its own receipts, evidence and statuses.

**3. Closed vocabularies, no booleans.**

Verify results: `Verified | Contradicted | Unknown(gap) | Stale(basis) |
NotApplicable`. Not `passed: bool, warning: bool, reason: Option<String>`.

**4. A4-0 implements neither.** This ADR is the decision that keeps ADR-0122's
substrate consumer-neutral; A4-1 and A4-2 implement against it. Implementing either
kernel before the observation substrate exists would force a provisional evidence
representation that would then have to be converged.

## Consequences

- AC4/AC5 are **adapted, never rewritten**.
- `architecture receipt` and `architecture findings` keep their current behaviour.
- DebVerify's strategies are extensible without touching the kernel.

## Alternatives considered

- **One function with a `scope` flag.** Rejected: the two have different inputs
  (a change set versus a baseline), different minimality reasoning and different
  outputs; a flag would hide that in every consumer.
- **Generalise AC4/AC5 by widening their types in place.** Rejected: it would make
  an architecture-specific receipt carry fields it cannot populate, and break the
  released receipt vocabulary.
- **Implement Verify in A4-0 alongside the substrate.** Rejected: the substrate's
  shape would then be dictated by one consumer, which is how the missing leg was
  lost in the first place.
