# Migration Plan — Current SDDK → Decision Kernel with Dynamic Workflows

## Principle
Strangle the legacy architecture; do not rewrite all packs/agents at once.

## Step 1 — Architectural seam
Remove concrete storage creation from core, introduce focused ports and contract tests. Preserve current CLI/agent behavior.

## Step 2 — Event compatibility
Map current cycle/phase/agent results into canonical events without changing user-visible SDD behavior.

## Step 3 — Workflow v2 compatibility compiler
Translate current `CyclePath/Phase` paths into `WorkflowTemplate/WorkflowIR` while keeping existing agents/artifacts.

```text
AFull legacy manifest → LegacySddCompiler → WorkflowIR
```

## Step 4 — Introduce dynamic operators without changing SDD default
Implement Map/Join/Loop/ExecutionGraphRevision and prove them with synthetic workflows/research/incident use cases.

## Step 5 — OpenCode + resilience
Add AgentHost adapter and provider failover on the new Attempt model.

## Step 6 — Introduce ChangeContract alongside current artifacts
During current A-full, populate ChangeContract from explore/propose/spec/design/tasks. This proves schema completeness before removing phase boundaries.

## Step 7 — Add `sdd-adaptive` as experimental
Run SHAPE/BUILD/CONVERGE/INTEGRATE. Generate legacy Markdown artifacts as projections where useful.

## Step 8 — Workflow Laboratory
Compare A-full and adaptive. Run ablation tests merging/removing phase boundaries. Measure quality first, efficiency second.

## Step 9 — Promote cautiously
If evidence supports it:
- adaptive becomes default for eligible changes;
- A-full remains an explicit high-ceremony/reference preset;
- `CyclePath` compatibility remains at boundary until deprecation window ends.

## Step 10 — Remove kernel SDD coupling
Only after packs and migration adapters cover persisted legacy state, remove `Phase/CyclePath` from generic domain runtime.

## No-delete rule in this refinement
No existing pack/spec needs deletion now. The new architecture is additive and migration-oriented.

---

## Test-Tooling Boundary Migration (per ADR-042)

> Phased migration of test-tooling ownership boundaries per [ADR-042-TEST-TOOLING-BOUNDARY.md](../03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md). Ownership policy: [ADR-0069](../../adr/ADR-0069-test-tooling-ownership.md) (accepted).

### Phase A — Completed audit

Completed through v1.58.0: the audit of the shell contract tests established the ownership boundary in ADR-0069.

- Audited the shell contract tests from commit `643180a` against ADR-0069 ownership cells.
- Classified binary behavior as Rust-owned and system orchestration as Shell-owned.
- Recorded findings in `TEST-TOOLING-EVIDENCE-AUDIT.md` §5 + §6.

### Phase B — Completed ownership migration

Completed through v1.58.0: Rust ownership is represented by tests SDDK015-SDDK032. `tests/test_push_prevention_hook.sh` is the only remaining Shell-owned test.

### Phase C — Next work (gated on parity evidence)

**Scope:** Pending lint, decision, and cleanup work. Do not begin cleanup until parity is verified.

- Add `shellcheck` to local CI gate for `tests/test_*.sh`.
- Add Python linter (`ruff`) to local CI gate for `scripts/`.
- Evaluate ADR-0022 (sddk-testkit, proposed) for adoption or supersession.
- Consolidate or delete remaining misowned tests after parity evidence.
- Remove superseded scaffolding after one stable release cycle.
