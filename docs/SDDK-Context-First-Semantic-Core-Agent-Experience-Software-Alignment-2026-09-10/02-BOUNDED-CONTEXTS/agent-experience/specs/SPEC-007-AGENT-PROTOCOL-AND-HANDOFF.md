# SPEC-007 — Agent execution, delegation and synthesis protocol

This spec defines **what crosses agent/handoff boundaries**. Agent assembly/instructions/skills/CLI knowledge are specified by SPEC-013..018.

## Core contracts

```text
ExecutionRequest
ExecutionOutcome
Contribution
DelegationRequest
SynthesisReceipt
ActionProposal
```

`ExecutionRequest` references, rather than embeds as untracked text, the AgentProfile, ContextCapsule hash, EffectiveInstructions hash, selected Skill set, AgentCommandSurface hash, TaskContract and policy/config snapshots.

## Contribution

Contains findings, alternatives, claims, assumptions, uncertainty, risks, questions, Evidence refs, Artifact refs, recommendation and confidence. It does not claim side-effect success.

## ExecutionOutcome

Contains factual operational result: completed/failed/interrupted status, errors, Artifacts, Evidence, usage and side-effect Receipts.

## Synthesis disposition invariant

Every material contribution item gets exactly one disposition:

```text
CONSUMED
REJECTED(reason)
DEFERRED(reason)
SUPERSEDED(by)
OMITTED(justification)
```

Material High/Critical risk and required Evidence MUST NOT be silently omitted.

## Dissent

Dissent is first-class with stable id, kind/claim ref, source, materiality, Evidence refs, status, resolution and revisit trigger.

## Delegation

Delegation creates a new typed ExecutionRequest with an explicit context delta, profile/task contract and authority ceiling. It does not forward an opaque transcript as the durable contract.

## Metrics

- handoff information-loss rate;
- dissent retention rate;
- stale-context handoff rate;
- evidence carry-forward completeness;
- instruction/command contract mismatch rate;
- provider normalization failure rate.

Mandatory risk/evidence information-loss target is zero.
