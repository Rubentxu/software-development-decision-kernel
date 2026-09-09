# Skills as procedural modules

## Skill composition

Prefer narrow composable skills:

```text
core.code-review
core.architecture-review
core.dependency-analysis
rust.clippy-remediation
uat.browser-validation
security.threat-model
```

rather than one “senior-engineer-agent” prompt containing all domains.

## Selection

Selection can use Task kind, project technology facts, Pack availability, context tags and explicit user/project configuration. Selection decisions should be visible through `context explain --instructions`.

## Capability declaration

Skills declare what capabilities they may need so the Task/Authority layer can reason about feasibility, but they never acquire authority themselves.
