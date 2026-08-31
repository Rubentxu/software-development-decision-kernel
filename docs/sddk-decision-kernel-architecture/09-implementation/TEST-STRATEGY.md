# Test Strategy

## Pyramid

### Domain/property tests
- state transitions;
- event validation;
- circuit breaker;
- budgets;
- route filtering/scoring invariants;
- workflow DAG validation.

### Port contract tests
One suite executed against in-memory and real adapters where feasible.

### Event fixture tests
Store representative host/provider events and expected canonical mappings. Essential for OpenCode API evolution.

### Replay tests
Golden event logs -> exact projection snapshots.

### Scenario tests
Examples:
- provider quota failover;
- human approval wait/resume;
- context overflow recompilation;
- task verification failure vs infrastructure failure;
- parallel agent worktrees and join;
- UAT defect/retest/signoff.

### Chaos/fault injection
Inject timeout, 429, 503, host crash, partial tool response, corrupted projection cache and process restart.

### Architecture tests
Run `check-arch` in CI.

## Determinism boundary
Do not require LLM calls for most CI. Supervisor/agent behavior is tested using:
- recorded outputs;
- fake model gateway;
- schema validation;
- golden tasks in separate evaluation jobs.

## Dynamic workflow tests

- property tests for graph expansion validity;
- Map with 0/1/N/large N items;
- exactly-once Join readiness under retries/replay;
- bounded loop/no-progress behavior;
- expansion budget rejection;
- conflicting worktree/resource scheduling;
- deterministic graph digest after process restart and replay;
- malicious/invalid Supervisor expansion proposal rejection.

## SDD Adaptive non-regression tests
Use the same evaluation contract for A-full and adaptive on representative tasks. Assert required invariant/evidence coverage independently of which Markdown artifacts or agent boundaries were used.

## Test-Tooling Ownership (per ADR-0069)

The test pyramid layers are owned by language per [ADR-0069](../../adr/ADR-0069-test-tooling-ownership.md):

| Pyramid layer | Owner | Rationale |
|---|---|---|
| Domain/property tests | Rust | State transitions, event validation, circuit breaker — binary behavior |
| Port contract tests | Rust | Binary API contracts |
| Event fixture tests | Rust | Binary behavior and event schema |
| Replay tests | Rust | Binary behavior and event log projection |
| Scenario tests | Rust or Shell | Rust for binary behavior; Shell for system-level orchestration |
| Chaos/fault injection | Rust or Shell | Rust for binary behavior; Shell for system-level fault injection |
| Architecture tests | Rust | Binary behavior (check-arch CLI) |
| Shell smoke tests | Shell | Pre-binary bootstrap, installer, Podman, TUI |
| Python golden/evaluation | Python | External tooling for SPEC-024/SPEC-040 |

Full inventory and evidence: [TEST-TOOLING-EVIDENCE-AUDIT.md](./TEST-TOOLING-EVIDENCE-AUDIT.md).

Migration plan: [ADR-042-TEST-TOOLING-BOUNDARY.md](../03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md).
