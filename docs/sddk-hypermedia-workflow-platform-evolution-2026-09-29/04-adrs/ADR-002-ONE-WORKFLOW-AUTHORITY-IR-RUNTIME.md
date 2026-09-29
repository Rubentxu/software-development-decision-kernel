# ADR-002 — One workflow execution authority: Definition → IR → Runtime

**Status:** Proposed

## Context

SDDK currently contains the legacy lifecycle manifest, a rich WorkflowIR/WorkflowRuntime, and prompt-driven dynamic workflow generation. Keeping three control-flow authorities makes behavior hard to reason about and prevents reusable custom workflows from becoming a product primitive.

## Decision

Establish one chain:

```text
WorkflowDefinition → normalized template → WorkflowIR → WorkflowRuntime
```

Legacy manifests become compatibility input/projection during migration. LLM-generated workflows produce WorkflowDefinition proposals that must compile; the LLM never directly executes invented phase YAML.

## Consequences

- compiler becomes critical type/policy gate;
- WorkflowRuntime must complete missing operator semantics incrementally;
- old lifecycle can be strangled rather than rewritten at once;
- replay identity gains a clear authored→compiled lineage.

## Rejected

- maintain separate “simple” and “dynamic” runtimes permanently;
- make prompts the dynamic runtime;
- compile WorkflowIR back to legacy phases for execution indefinitely.

## Exit criterion

All default software workflows execute through WorkflowRuntime with equivalence/failure/restart UAT and no duplicate side effects.
