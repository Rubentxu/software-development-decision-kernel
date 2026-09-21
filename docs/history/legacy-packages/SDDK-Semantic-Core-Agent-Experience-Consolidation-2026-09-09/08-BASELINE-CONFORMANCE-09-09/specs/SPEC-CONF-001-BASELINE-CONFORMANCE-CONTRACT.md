# SPEC-CONF-001 — 09/09 Baseline Conformance Contract

## Status

Normative closeout specification.

## Intent

Turn the 09/09 architecture package from a broadly delivered roadmap into a **provably closed baseline**. The target is semantic convergence, not feature expansion.

## Scope

The conformance surface includes:

- SPEC-001..018;
- ADR-001..017;
- M0..M9 deliverables and exit criteria;
- UAT-01..22;
- migration/deprecation phases;
- architecture rules ARCH-SC-001..012 and ARCH-A01..A08.

## Non-goals

This closeout MUST NOT implement 10/09-only concepts such as KMT, formal KnowledgeAssertion, Software Alignment, Verify/DebVerify, CogniCode/Chronos provider ports or Agentic Workspace.

It MAY prepare seams needed by R0/R1 only when doing so is the smallest way to satisfy an existing 09/09 contract.

## Conformance dimensions

Each baseline requirement MUST have all five dimensions:

```text
Normative requirement
  -> implementation owner
  -> reachable production path
  -> executable verification
  -> migration/deprecation disposition
  -> evidence in final receipt
```

Allowed states:

- `PASS` — requirement demonstrably satisfied;
- `PASS_WITH_COMPAT` — canonical path satisfied and compatibility is read-only, bounded and proven non-authoritative;
- `BLOCKED` — environmental dependency prevents execution but the contract is otherwise testable;
- `FAIL` — semantic or migration requirement not satisfied;
- `UNKNOWN` — insufficient evidence; never equivalent to PASS.

Final closeout permits only `PASS` and explicitly approved `PASS_WITH_COMPAT` entries. `UNKNOWN`, `BLOCKED` and `FAIL` prevent the 100% receipt.

## Canonicality rule

For any semantic concept marked authoritative by 09/09 there MUST be exactly one production write authority. A second store, enum or facade may exist only as a projection, decoder, adapter or read-only compatibility path.

## Removal rule

M9 used the verb **remove/retire**. Therefore an implementation is not conformant merely because the replacement exists. The deprecated production path MUST be unreachable or deleted.

## Compatibility exception

A compatibility path may survive closeout only if all are true:

1. it performs no canonical writes;
2. it can be deleted without losing source-of-truth data;
3. it delegates to or derives from the canonical path;
4. a parity test proves equivalence;
5. it has an owner and explicit removal trigger/date/version;
6. architecture fitness prevents new dependencies on it.

## Semantic duplication rule

If two types encode the same invariant, identity or state transition, one of these MUST happen:

- one becomes canonical and the other an adapter/view;
- both reuse a shared primitive with distinct bounded semantics;
- an ADR documents why the duplication is intentional.

“Already implemented” is not an exemption.

## Exit criteria

- all rows in `../reference/09-09-SPEC-CROSSWALK.md` are PASS/PASS_WITH_COMPAT;
- all UAT-01..22 pass;
- all closeout ratchets pass;
- no deprecated production path appears in the reachable dependency graph;
- docs/spec metadata agree with implementation;
- final conformance receipt has zero unresolved MUST findings.
