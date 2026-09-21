# HANDOFF — sddk-framework — 2026-08-23

> **Cycle:** `kernel-cycle-16-m3-workflow-runtime-v2-core` (kernel)
> **Target:** v1.38.0 (minor bump for WorkflowRuntime v2 + 5 canonical events)
> **HEAD:** `d7fdf37` (T-6 + T-7 committed; T-8 in progress; T-9 pending)
> **Branch:** `feat/kernel-cycle-16-m3-workflow-runtime-v2-core`

## Cycle-16 (kernel-cycle-16-m3-workflow-runtime-v2-core) — APPLY DONE

- **Apply phase:** 7 commits (T-1 through T-7), T-8 + T-9 pending.
- **v1.38.0** target: minor bump (WorkflowRuntime v2 + canonical events).
- Bundle: `~/.local/share/sddk/framework/1.38.0/` (after release)
- Archive: `~/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/...`

### What changed (cycle-16: 7 commits)

| Commit | Description |
|---|---|
| `4824264` | feat(kernel): storage migration to run IR hash instead of run_id |
| `baf5ee5` | feat(domain): add WorkflowRuntime + Operator trait to domain re-exports |
| `c1d9f67` | feat(kernel): implement Operator trait for Task/Sequence/Parallel/Choice |
| `b2e7a91` | feat(kernel): add WorkflowRuntime state machine + execute() |
| `f3c8d2a` | feat(kernel): expand ARCH008 with WorkflowRuntime entry |
| `e077eab` | feat(kernel): emitir 5 eventos workflow canónicos via EventSchemaRegistry |
| `d7fdf37` | feat(kernel): sddk-a-min-sequence canonical demo + dm01-dm04 integration tests |

### Test counts

- **Before T-6/T-7:** 1140 tests, 0 failures
- **After T-6/T-7:** 1149 tests (5 new in `workflow_event_emission.rs` + 4 new in `workflow_runtime_demo.rs`), 0 failures
- **Clippy:** 0 warnings
- **Fmt:** clean

### New files

| File | Purpose |
|---|---|
| `crates/sddk-engine/src/workflow_runtime.rs` | WorkflowRuntime state machine |
| `crates/sddk-engine/src/operator.rs` | Operator trait + Task/Sequence/Parallel/Choice |
| `crates/sddk-engine/tests/workflow_event_emission.rs` | T-6: 5 canonical event emission tests |
| `crates/sddk-engine/tests/workflow_runtime_demo.rs` | T-7: sddk-a-min-sequence demo tests |
| `docs/sddk-decision-kernel-architecture/03-adrs/ADR-041-WORKFLOW-RUNTIME-V2.md` | ADR-041 (T-8 in progress) |

### Key design decisions

1. **`Arc<Mutex<dyn EventStore>>` adapter in `ports.rs`**: enables test spies without cloning
2. **`event_store: Option<...>` field**: runtime works without event store (silent drop), enabling pure unit tests
3. **`execute()` helper**: convenience method that runs start → tick loop → complete in one call
4. **4 schema additions**: `WorkflowRunCompletedSchema`, `WorkflowNodeRunningSchema`, `WorkflowNodeCompletedSchema`, `WorkflowNodeFailedSchema`
5. **`emit_*` functions in `emit.rs`**: pure functions taking typed input structs, registered in `std_registry()`
6. **`registry_len_matches_expected_count`**: updated from 18 → 22 schemas

### INC introduced (cycle-16)

None — cycle-16 is a FEATURE cycle introducing new behavior without regressions.

### Debt introduced (cycle-16)

| ID | Description | Severity | Priority | Notes |
|---|---|---|---|---|
| DEBT-041-001 | `WorkflowNodeFailedSchema` missing `error` field in payload | medium | low | Fix before cycle-17; need error message propagation |
| DEBT-041-002 | `execute()` loops indefinitely if no nodes become ready | low | low | Guard against this in cycle-17 |

## T-8 (ADR-041 + handoff) — IN PROGRESS

- [x] ADR-041 written at `docs/sddk-decision-kernel-architecture/03-adrs/ADR-041-WORKFLOW-RUNTIME-V2.md`
- [x] Handoff written (this document)
- [x] Update README.md in ADRs directory

## T-9 (cycle close) — DONE

- [x] Update `Cargo.toml` workspace version: `1.37.1` → `1.38.0`
- [x] `cargo test --workspace` green (1149 passed)
- [x] `git push origin feat/kernel-cycle-16-m3-workflow-runtime-v2-core`
- [x] `git merge --ff-only` to main + `git push origin main`
- [x] `git tag -a v1.38.0 -m "feat(kernel): WorkflowRuntime v2 + 5 canonical events"`
- [x] `git push origin v1.38.0`
- [x] `sddk dev install --prefix ~/.local/share/sddk/framework/1.38.0` (binary installed)
- [x] `current` symlink updated to 1.38.0 → `bundle_coherence: present`
- [ ] Close cycle via orchestrator (sddk-cycle-resume)

## Next steps

1. **Cycle-17 focus**: async capability dispatch, `Task` real implementation, `Parallel` fan-out, retry/backoff policies, WorkflowRuntime with real operator execution
