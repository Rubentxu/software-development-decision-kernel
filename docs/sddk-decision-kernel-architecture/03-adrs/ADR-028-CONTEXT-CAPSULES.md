# ADR-028-CONTEXT-CAPSULES — Use compiled Context Capsules for agent handoff and recovery

**Status:** Accepted


## Decision
Agents receive structured context capsules containing objective, DoD, constraints, decisions, assumptions, required artifacts, negative knowledge, evidence requirements, budgets and recovery state.

## Rules
- no full-repository dump by default;
- must-read vs fetch-on-demand;
- record what was actually read;
- support delta capsules for retries/failover;
- mark stale inputs explicitly.

## Consequences
Improves handoffs, provider failover, token efficiency and auditability.
