# SPEC-037 — Dynamic Workflow Compiler

**Status:** Proposed

## Purpose
Compile compact workflow intent into a validated, durable, provider-neutral execution plan while allowing bounded runtime expansion.

## Inputs
- `WorkflowTemplate`;
- user goal / structured inputs;
- Capability Registry snapshot;
- project/context summary;
- risk/uncertainty assessment;
- budgets and policy profile;
- optional historical workflow statistics.

## Output
`WorkflowIR` with:
- operators;
- capability references;
- schemas;
- guards/conditions;
- budgets/concurrency;
- expansion permissions;
- required invariants/verifiers;
- provenance (`generated_by`, model/prompt/policy hashes if cognitive planning used).

## Example

```yaml
workflow_ir: 1
sequence:
  - task: change.shape
  - map:
      source: $.change_contract.work_units
      concurrency: adaptive
      task: code.implement
  - loop:
      max_iterations: 3
      until: $.convergence.verdict == "PASS"
      body:
        - task: change.verify
        - choice:
            when: $.convergence.has_gaps
            then: code.remediate
  - task: change.integrate
```

## Compiler stages
1. Parse/normalize template.
2. Resolve capabilities, not concrete agents/models.
3. Select pattern composition.
4. Produce IR candidate.
5. Static validation.
6. Governance validation.
7. Budget/conflict/worktree validation.
8. Hash/sign/store IR.

## Runtime expansion
IR can authorize specific expansion points (`map`, discovery, replan). Runtime receives typed `ExpansionProposal`; the same validator rules apply to deltas.

## Security
- no raw arbitrary script execution;
- no hidden capability escalation;
- generated plan cannot remove mandatory invariants/policies;
- destructive effects require governed capabilities/gates;
- max graph depth/nodes/concurrency mandatory.

## Acceptance criteria
- identical deterministic inputs can compile without LLM for canonical templates;
- generated and canonical IR use same runtime;
- invalid capability or cycle is rejected;
- every IR/expansion has reproducible provenance/hash.
