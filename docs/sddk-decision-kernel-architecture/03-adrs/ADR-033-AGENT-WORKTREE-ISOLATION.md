# ADR-033-AGENT-WORKTREE-ISOLATION — Isolate mutating agent executions using Git worktrees/branches

**Status:** Accepted


## Decision
Concurrent agents that mutate code receive isolated worktrees or equivalent workspace snapshots. Promotion to the target branch occurs only after verification/governance.

## Consequences
Prevents race conditions, accidental overwrites and ambiguous provenance; requires worktree lifecycle cleanup and disk-budget management.
