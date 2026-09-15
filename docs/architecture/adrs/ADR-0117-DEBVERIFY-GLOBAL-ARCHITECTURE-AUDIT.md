---
id: ADR-0117-DEBVERIFY-GLOBAL-ARCHITECTURE-AUDIT
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-7-ac5-debverify-audit
implementation_evidence:
  - "crates/sddk-engine/src/architecture_debverify/mod.rs (public surface; state-class + exit-criterion doc)"
  - "crates/sddk-engine/src/architecture_debverify/types.rs (DebVerifyFindingKind 5-closed; FindingSeverity 3-closed with a total kind->severity mapping; DebVerifyFinding; DebVerifyAudit)"
  - "crates/sddk-engine/src/architecture_debverify/detectors.rs (shadow_authority; missing_owner; authority_bypass; stale_compatibility; contradiction)"
  - "crates/sddk-engine/src/architecture_debverify/audit.rs (run_debverify_audit(overlay, contracts, now) + deterministic digest)"
  - "crates/sddk-engine/src/architecture_debverify/tests.rs (28 tests incl. AC-UAT-009)"
  - "crates/sddk-engine/tests/ac5_not_verify_full.rs (AC4 delta empty vs AC5 Critical finding, same overlay)"
  - "docs/architecture/specs/arch-spec-A3-S7-ac5-debverify-audit.md (cycle-bounded spec, 22 REQs)"
superseded_by: []
related_adrs:
  - "ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS"
  - "ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY"
  - "ADR-0116-ARCHITECTURE-MUTATION-AND-COUNTERFACTUAL-VERIFICATION"
stale_after: 2027-03-14
---

# ADR-0117 — DebVerify Is a Global Architecture Audit, Not a Verify Mode

## Context

Change-scoped verification (Verify) answers one question well: *given this
change, which contracts are affected and do they still hold?* It is structurally
blind to incoherence that no recent change caused — two contracts that have
claimed the same authority since they were written, an owner that was never
declared in the graph, a compatibility window that quietly elapsed.

Upstream `arch-spec-034` AC-034-002 states the requirement:

> DebVerify independently challenges global assumptions and is not
> `verify --full`.

The risk is that "DebVerify" degenerates into "run Verify with no scope filter",
which is `verify --full`: the same traversal, the same inputs, just wider. That
is not an independent challenge; it inherits Verify's blind spots.

## Decision

Introduce DebVerify as a **separate, global capability** with its own inputs,
vocabulary and output type — not a mode or a scope flag of Verify.

1. **Inputs.** `run_debverify_audit(overlay, contracts, now)` takes the whole
   contract set and the semantic graph. It takes **no change basis**: there is
   no parameter through which a delta, a changed-unit list, or an
   `ArchitectureConformanceDelta` could enter.
2. **Output.** A distinct `DebVerifyAudit` receipt. There is no conversion
   between `DebVerifyAudit` and Verify's `ArchitectureConformanceDelta`, and the
   module does not import the Verify module at all.
3. **Vocabulary.** Five challenge classes, one per roadmap noun:
   `ShadowAuthority`, `MissingOwner`, `AuthorityBypass`, `StaleCompatibility`,
   `Contradiction`. Each has a deterministic severity and every detector has a
   negative control.
4. **Independence.** DebVerify does not read Verify's state and is unaffected by
   whether any verification ran.

The distinction is enforced structurally (missing parameter, absent import,
distinct type) and behaviourally (a test where Verify's delta is empty and
DebVerify still reports a Critical finding — AC-UAT-009).

## Consequences

- The case Verify cannot see becomes the headline behaviour: global incoherence
  is found with no change basis at all.
- Two receipts now describe architecture health: a change-scoped delta and a
  global audit. They answer different questions and are deliberately not merged.
- DebVerify's identifier matching (owner-present-in-graph, forbidden-edge-
  present) is a documented substring heuristic over graph node locators. A
  typed-edge implementation (AC10 / CogniCode) is the natural upgrade and does
  not change this decision.
- Adding DebVerify as a new root-level engine module requires this ADR under the
  repository's root-module fitness rule; that rule and this ADR agree that a new
  root-level capability is an architectural decision, not a refactor.

## Alternatives considered

- **A `--full` flag on Verify.** Rejected: same inputs and traversal, so it
  inherits Verify's blindness and makes the "independent challenge" claim false.
- **DebVerify consuming Verify's delta as input.** Rejected: it would make the
  global pass contingent on a change having occurred, which is precisely the
  case AC-UAT-009 exercises.
- **Folding DebVerify into the mutation-probe module (ADR-0116).** Rejected:
  mutation proves a guard fires; the audit looks for incoherence without any
  injected violation. Different evidence, different lifecycle.
