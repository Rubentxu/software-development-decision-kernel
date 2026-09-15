---
id: ADR-0119-ARCHITECTURE-CONFORMANCE-RECEIPT
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-9-ac8-self-audit-receipt
implementation_evidence:
  - "crates/sddk-engine/src/architecture_receipt/mod.rs (public surface; Base-mode + aggregator-only doc)"
  - "crates/sddk-engine/src/architecture_receipt/types.rs (ReceiptVerdict 3-closed; HistoricalClass 5-closed; ReceiptBasis; ClaimResult/MutationResult/LensResult/CompatibilityEntry/UnresolvedFinding/ClassCoverage; ArchitectureConformanceReceipt; ReceiptId)"
  - "crates/sddk-engine/src/architecture_receipt/compose.rs (compose_receipt; verdict rules AC-041-003)"
  - "crates/sddk-engine/src/architecture_receipt/self_audit.rs (evaluate_class_coverage for AC-041-002)"
  - "crates/sddk-engine/src/architecture_receipt/tests.rs (27 tests incl. AC-UAT-016)"
  - "docs/architecture/specs/arch-spec-A3-S9-ac8-self-audit.md (cycle-bounded spec, 24 REQs)"
superseded_by: []
related_adrs:
  - "ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS"
  - "ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY"
  - "ADR-0116-ARCHITECTURE-MUTATION-AND-COUNTERFACTUAL-VERIFICATION"
  - "ADR-0117-DEBVERIFY-GLOBAL-ARCHITECTURE-AUDIT"
  - "ADR-0118-PARADIGM-LENS-EVALUATION"
stale_after: 2027-03-14
---

# ADR-0119 — One Aggregating Architecture-Conformance Receipt

## Context

AC1–AC7 produced five capabilities that each answer a narrow question:

| Capability | Question |
|---|---|
| AC2 graph overlay | what shape is the architecture? |
| AC4 Verify | did *this change* break a contract? |
| AC5 DebVerify | is the architecture *itself* coherent? |
| AC6 mutation probes | do the guards actually fire? |
| AC7 paradigm lenses | does the code match its declared paradigm? |

`10-UAT.md` §Production acceptance makes the Base gate concrete:

> Base cannot claim the AC capability production-ready until AC-UAT-001..016 pass
> at a named commit and `ARCHITECTURE-CONFORMANCE-RECEIPT` lists **zero
> unresolved MUST findings**.

So a single named artefact must exist that a reviewer can read to answer "is
this revision conformant, and on what basis?" Without one, each capability's
output is a separate receipt and no decision can be made from them together.

## Decision

**1. Add one aggregating receipt capability.** `architecture_receipt` composes
the outputs of AC2/AC4/AC5/AC6/AC7 into `ArchitectureConformanceReceipt`.

**2. It aggregates; it does not re-derive.** Every value in the receipt comes
from an input. The module contains no observation vocabulary, no probe and no
detector; it imports the other capabilities' *types* only to interpret their
outputs. Re-deriving would create a second authority for the same facts
(ADR-0095).

**3. The verdict is a closed 3-variant judgement, not a score.**
`Pass | PassWithWaivers | Blocked`, derived from unresolved MUST findings:

- AC4 `unknown` and `contradiction` → MUST;
- AC5 findings of severity `Critical` or `High` → MUST; `Medium` is recorded but
  advisory;
- AC6 probes that were **not** detected → MUST (a negative-evidence gap);
- any MUST finding not covered by a supplied waiver → `Blocked`.

No numeric weight or aggregate quality number exists.

**4. Waivers are Governance's, not ours.** The composer records the waivers it
is given and matches them explicitly; it never invents, stores or expires one.

**5. The receipt is a PROJECTION.** It has a derived identity, no persistence
authority, and performs no IO. `now` is a parameter.

**6. Base mode is first-class.** `provider_basis` is empty by construction and
the module imports no provider/CogniCode/Chronos surface. Provider entries are
additive evidence: the verdict rules are provably identical with and without
them (AC-041-006).

**7. The historical-class coverage is explicit.** `HistoricalClass` (5-closed)
records whether the native analysis reproduced the A0/A1 classes named in
`01-VISION` §A0/A1 closeout, each with concrete evidence. AC-UAT-016 is the
assertion that all five are reproduced on a representative revision.

## Consequences

- One named artefact answers the Base gate question; the five capabilities stay
  narrowly scoped and independently testable.
- The receipt cannot silently strengthen a claim: it carries the exact revision,
  contract-set digest, semantic-graph digest and verification-plan digest it was
  computed from (AC-041-001).
- `BASE_PRODUCTION_READY` remains a *separate* decision: AC8 emits the receipt,
  the production gate consumes it. This ADR deliberately does not flip the flag.
- A new root-level engine module requires an ADR under the repository's
  root-module fitness rule.

## Alternatives considered

- **No receipt; each capability reports separately.** Rejected: no single
  artefact can satisfy `10-UAT.md`'s "zero unresolved MUST findings" line, and
  reviewers would have to reconcile five outputs by hand.
- **Put the receipt in AC4's module.** Rejected: it would make the change-scoped
  verifier the owner of a global judgement, repeating the AC5/`verify --full`
  confusion ADR-0117 forbids.
- **Emit a numeric conformance score for easy gating.** Rejected:
  `11-FITNESS-RECEIPTS.md` explicitly says not to compress architecture into a
  meaningless score, and AC-UAT-043 forbids it.
- **Let the receipt store its own waivers.** Rejected: waivers are governance
  objects with owners and expiry; a projection duplicating them would create a
  second authority.
