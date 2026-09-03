# ADR-045 — Agents express goals; the Decision Kernel resolves deterministic procedures

**Status:** Proposed

## Context

The CLI currently exposes a rich set of deterministic commands. Agents often call them directly in repeated sequences and sometimes rediscover syntax through help/retry behavior.

That is useful for debugging but inefficient as the primary cognitive interface.

A repeatable sequence such as:

```text
state
→ lease
→ gate
→ transition
→ state
```

should not require the LLM to act as a shell-level workflow engine when Rust already owns the authoritative state machine.

## Decision

Introduce a canonical **Agent-First Goal Surface**.

Agents SHOULD primarily express:

- semantic goal/desired state;
- scope;
- typed cognitive output;
- evidence submission.

The Decision Kernel resolves:

- current state;
- operation graph;
- dependency order;
- locks/fencing;
- policy/approval;
- capability implementation;
- idempotency/retry;
- report/artifact persistence;
- postconditions;
- events/receipts.

## Compatibility invariant

**Simplifying the interaction must never reduce workflow functionality or reporting.**

A high-level goal may replace multiple calls only when it preserves or strengthens:

```text
mandatory gates
validation semantics
detailed reports
evidence
receipts
metrics
audit events
human approvals
knowledge updates
safety/recovery behavior
```

## Low-level surface

Existing low-level commands remain supported for:

- expert use;
- debugging;
- recovery;
- testing;
- implementation of migration tooling.

They are not the preferred normal LLM surface.

## Capability refinement

A semantic capability should not require a normal agent to supply executable details that provider routing can resolve.

Preferred:

```text
testing.execute(TestRequest)
```

rather than:

```text
capability=testing.execute
program=cargo
args=[test,...]
```

Explicit-runner mode may remain for advanced/debug scenarios.

## No prompt-owned deterministic procedure

A stable repeatable sequence representable by state, operation contracts and policies must not remain solely encoded as prompt prose.

## One application core

CLI, AgentHost, stdio/MCP and future transports are adapters over the same application services.

No adapter may reimplement the workflow/gate/report rules.

## Goal execution

A goal is reconciled toward desired invariants, not expanded into a static shell macro.

## Consequences

- lower agent call count;
- smaller prompts;
- fewer invalid calls;
- better support for smaller/local models;
- stronger deterministic semantics;
- measurable interface quality;
- migration complexity and temporary dual surfaces.

## Rejected alternatives

### Better CLI cheat sheet only
Useful but does not fix granularity.

### Expose all CLI commands via MCP
Changes transport, not semantics.

### One giant "do everything" command
Too opaque and non-composable.

### Remove low-level commands
Breaks valuable recovery/debug functionality.

### Return only a concise goal summary
Rejected because it would lose reports/evidence consumed by existing workflows.
