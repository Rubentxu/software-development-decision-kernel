# Instruction source taxonomy

## Strength classes

| Source | Example | Override behavior |
|---|---|---|
| KernelInvariant | never bypass Authority | cannot be overridden |
| PolicyConstraint | protected branch requires approval | cannot be weakened by task/project/skill |
| TaskInstruction | produce verification evidence | required for task |
| ProjectInstruction | cargo fmt before Rust commit | project-scoped normative/advisory key |
| AgentRoleInstruction | reviewer challenges assumptions | role-scoped |
| SkillInstruction | threat modeling checklist | selected procedure |
| ContextualHint | likely relevant file/module | advisory only |

## Composition

Instructions should use semantic keys where possible, e.g. `git.commit.policy`, `verification.required_evidence`, instead of relying exclusively on natural-language collisions.

## Why this matters

The compiler can explain why a directive exists, detect contradiction and keep provider rendering compact. It also allows future policy engines/UI editors without rewriting prompts.
