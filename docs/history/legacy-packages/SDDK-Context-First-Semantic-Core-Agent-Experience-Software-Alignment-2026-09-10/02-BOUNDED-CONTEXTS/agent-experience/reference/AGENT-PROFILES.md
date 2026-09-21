# Agent profiles

## Role examples

```yaml
id: core.reviewer
version: 1
responsibilities:
  - identify correctness and architectural risks
admissible_task_kinds:
  - review.*
  - verify.analysis
authority_ceiling: propose
context_requirements:
  - relevant_code
  - active_decisions
  - evidence
preferred_skills:
  - core.code-review
output_contract: ContributionV2
```

Profiles describe semantic roles, not model brands. Model/provider selection belongs to resolved configuration/routing.

## Lateral benefit

Because profiles are provider-independent, they become an evaluation axis: SDDK can compare provider/model behavior under the same role/task/context contract without conflating “agent definition” with “prompt for vendor X”.
