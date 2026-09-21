# ADR-016 — Agent profiles are provider-independent roles

**Status:** Proposed

## Decision

Define stable `AgentProfile` objects around role semantics: responsibilities, admissible task kinds, context requirements, authority ceiling, preferred skills, output contract and quality constraints.

Provider/model concerns live in `ProviderAdapter` + `ModelDescriptor` configuration.

An agent profile is not a giant provider prompt.

## Consequences

The same reviewer/researcher/implementer role can run through Codex, Claude, OpenCode or local models. Provider-specific prompt/message formatting is replaceable and cannot redefine SDDK semantics.
