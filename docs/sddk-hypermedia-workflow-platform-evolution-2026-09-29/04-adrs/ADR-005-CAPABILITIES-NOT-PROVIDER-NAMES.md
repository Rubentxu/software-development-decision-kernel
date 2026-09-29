# ADR-005 — Workflows request capabilities, not provider products

**Status:** Proposed

## Context

Hardcoding CogniCode/Chronos in workflows couples authored logic to current products and makes user workflows responsible for integration details.

## Decision

StepDefinitions and WorkflowDefinitions refer to semantic capabilities (`code.dependencies`, `runtime.trace`, etc.). ProviderRegistry maps negotiated capabilities to concrete providers. A provider override is allowed only as an explicit execution policy/debug/reproducibility choice.

## Consequences

- CogniCode/Chronos become replaceable implementations;
- new providers require adapter + capability mapping rather than workflow edits;
- provider absence semantics remain REQUIRED/PREFERRED/OPTIONAL;
- receipts must record resolved provider basis.

## Rejected

- `use_cognicode: true` fields;
- provider version as capability proof;
- provider selection hidden in prompt prose.
