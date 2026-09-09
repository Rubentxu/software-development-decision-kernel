# SPEC-016 — Skill Contract

## Definition

A Skill is a versioned procedural extension that teaches how to execute or reason about a class of task.

```yaml
id: core.architecture-review
version: 1
applies_when:
  task_kinds: [review.architecture]
inputs:
  - ContextCapsule
outputs:
  contract: ContributionV2
capabilities:
  required: [filesystem.read]
  optional: [command.run]
evidence_contract:
  minimum: source_refs
instruction_fragments:
  - review-checklist
```

## Required properties

- namespaced id and schema version;
- applicability predicate;
- typed input/output contract;
- required/optional capabilities (declaration only);
- evidence expectation;
- instruction fragment refs;
- compatibility range with Agent Experience contract;
- deterministic selection metadata where possible.

## Packs

Packs may register Skills through Pack SDK. They cannot use Skill loading as a backdoor to register privileged executable code outside normal Task/Capability contracts.

## Anti-patterns

A Skill MUST NOT be:

- a hidden agent identity;
- a 1,000-line monolithic prompt containing product architecture;
- a permission grant;
- a private store;
- a second workflow engine.
