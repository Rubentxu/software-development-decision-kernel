# Mutation, Counterfactual Verification and Proof-Carrying Changes

## Architecture mutation testing

A fitness rule existing is weaker evidence than proving it detects the forbidden change. SDDK SHOULD support sandboxed architecture mutations such as:

```text
inject domain → tonic dependency
inject Alignment → AuthorityEngine dependency
inject Knowledge → CogniCode SDK type
inject Workbook → canonical write path
inject second event writer
```

Expected result:

```text
MutationProbe
  mutation_applied: yes
  expected_guard: arch.contract.X
  detected: yes
  evidence: ...
```

Mutation never lands in the working tree. It executes in a disposable sandbox/worktree.

## Counterfactual architecture

Before applying a refactor, SDDK MAY evaluate a proposed graph delta:

```text
CurrentArchitectureProjection
        + ProposedChange
        ↓
CandidateProjection
        ↓
contract evaluation
```

Questions:

- does the move introduce a forbidden dependency?
- does it create multiple owners?
- which facades are required?
- which contracts become UNKNOWN?
- which UAT/fitness tests become affected?

Counterfactual results are advisory and ephemeral until the actual change occurs.

## Proof-carrying changes

A change MAY carry a structured verification attachment:

```text
ChangeConformanceEnvelope {
  change_basis,
  affected_contracts,
  verification_receipt,
  evidence_refs,
  unknowns,
  compatibility_delta,
}
```

This does not authorize merge/release. Governance may optionally require such a receipt for selected actions.

## Architecture time travel

Because decisions, revisions, evidence and graph projections are basis-addressed, SDDK SHOULD eventually answer:

```text
sddk diff architecture REV_A..REV_B
```

with semantic changes such as:

- owner changed;
- authority added/removed;
- compatibility path introduced/retired;
- paradigm intent changed;
- contract moved VERIFIED→UNKNOWN;
- provider evidence changed the basis.

## Shadow authority detection

Static/runtime graph evidence MAY produce `ShadowAuthorityCandidate` when two components appear to decide/write the same conceptual truth. It remains a hypothesis until Verify/DebVerify classifies it.

## Safety

- LLM-proposed mutations are never applied outside sandbox by this subsystem.
- counterfactual analysis never grants capability;
- receipts cannot self-approve;
- provider observations are evidence, not authority.
