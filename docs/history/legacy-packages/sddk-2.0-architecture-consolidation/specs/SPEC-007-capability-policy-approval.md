# SPEC-007 — Capability, Proposal, Policy and Approval Model

**Status:** Proposed

## 1. Security boundary

Reactive behaviors and agents never gain authority merely because they are registered. They may propose actions. The deterministic SDDK capability path evaluates authority.

```text
Agent/Behavior
    -> Proposal
    -> Validate
    -> Policy
    -> optional Human Approval
    -> Capability Gateway
    -> External Effect
    -> Postcondition Verification
    -> Receipt
```

## 2. Proposal

A proposal MUST capture:

- proposer actor identity;
- actor definition/policy hashes;
- intended capability/action class;
- arguments with secret references, not secret values;
- reason/goal;
- evidence references;
- observed target version where applicable;
- expiry;
- idempotency key.

## 3. Two-phase governed changes

Audit-critical changes SHOULD use:

```text
proposed -> approved|denied -> executed|failed -> verified|verification_failed
```

A denial is a domain event, not an exception.

## 4. Optimistic concurrency

When proposals target mutable SDDK entities, the proposal SHOULD include observed version/hash. Apply must fail closed if the target drifted unless policy explicitly allows rebase/re-evaluation.

## 5. Human approval

Human approval events MUST include actor identity and SHOULD include reason/note. Approval may never silently mutate the original proposal; amendments create a new proposal or an explicit amendment event.

## 6. Capability classes

Capabilities SHOULD have risk/action classes such as:

- R0 read-only/local observation;
- R1 reversible local mutation;
- R2 repository mutation;
- R3 remote/published mutation;
- R4 destructive/privileged action.

The exact taxonomy must be aligned with current SDDK permissions rather than duplicated.

## 7. Receipts

Every governed execution MUST produce a receipt binding:

- proposal;
- policy decision;
- capability version;
- actor identity hashes;
- inputs hashes;
- outputs/evidence hashes;
- postcondition result.
