# ADR-015 — Skill and Capability are orthogonal contracts

**Status:** Proposed

## Decision

A `SkillDefinition` describes procedural knowledge and an expected task/output contract. A `Capability` describes an executable effect surface governed by Authority.

```text
Skill: core.architecture-review
  may require: filesystem.read, command.run

Capability: filesystem.write
  admission: AuthorityEngine
```

A Skill may declare required/recommended capabilities but cannot grant, acquire or bypass them.

## Consequences

- reusable skills remain safe across provider adapters;
- enabling a skill never silently increases authority;
- Packs can contribute skills without gaining privileged access;
- tests can independently validate skill selection and capability admission.
