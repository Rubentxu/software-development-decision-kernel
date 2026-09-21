# ADR-008 — Separate execution outcome, contribution and synthesis

**Status:** Proposed

## Decision

Agent interactions use three distinct contracts:

- `ExecutionOutcome`: operational facts about execution.
- `Contribution`: epistemic/advisory content learned or proposed.
- `SynthesisReceipt`: how multiple contributions were combined, omitted, rejected or carried forward.

`AgentResult` is deprecated as a mixed legacy contract.

Synthesis MUST preserve material dissent, risk and evidence with explicit dispositions.

## Benefit

Execution can succeed while a recommendation is low-confidence; a contribution can be useful even if an execution partially fails. The model no longer conflates them.
