# SPIKE SP-07 — CLI target ergonomics

> **Status:** completed 2026-09-11 (v1.168.10 workspace). P0 fix applied
> in the same cycle: honest stub reporting shipped (`has_body` field +
> `not_implemented`/`degraded` statuses).
> **Question (per SPIKES.md):** prototype `change` and `verify` Targets over
> existing commands. Compare number of commands/flags and error recovery
> versus current manual flow.
> **Method:** empirical probing of the live `sddk 1.168.10` CLI in a
> non-adopted scratch workspace, counting surface area (commands, flags,
> failure modes) for both flows.

## Current state of the two flows

### Manual flow: "create a change in a cycle"

| step | command | flags required |
|---|---|---|
| 1 | `sddk cycle start` | `--name` (+ inference defaults; `--root`/`--scope` if `--no-infer`) |
| 2 | `sddk plan work-item create` | `--cycle-id`, `--title`, `--description`, `--actor-id` |

**Total: 2 commands, ≥5 flags**, plus the cognitive burden of knowing the
`--cycle-id` produced by step 1 must be threaded into step 2 (no pipe or
shared session context). Error recovery: `plan work-item create` against a
missing cycle exits 1 with a clear error; the user must re-run discovery
(`cycle list`/`plan work-item list`) manually.

### Target flow: `sddk target run --name change`

**Total: 1 command, 1 flag** (`--name change`; optional `--actor`,
`--dry-run`, `--allow-high-band`).

### Manual flow: "verify"

`sddk verify` — 1 command, 1 optional flag (`--format`). The Target
`verify` resolves to `ledger.verify` → `capability.status` (2 tasks), which
the single facade command already covers.

## Measured comparison

| dimension | manual (`change`) | Target (`change`) |
|---|---|---|
| commands | 2 | 1 |
| flags | ≥5 | 1 (+3 optional) |
| cross-command state threading | yes (`--cycle-id`) | no |
| error surface | per-command, granular | per-DAG, coarse |
| **actually performs the work** | **yes** | **NO — stubs** |

## Key finding: false-success ergonomics (blocking defect)

`sddk target run --name change` in a **non-adopted workspace** reports
`"status": "succeeded"` with both tasks `"executed"`:

```json
{ "target": "change", "status": "succeeded", "dry_run": false,
  "tasks": [ { "task_id": "context.resolve", "status": "executed" },
             { "task_id": "plan.workitem.create", "status": "executed" } ] }
```

**No work item is created.** The task bodies are M6.2 declaration stubs
(`builtin.rs`: "All tasks declared here are *stubs* (no-op task bodies)"),
but the executor reports `executed` and `succeeded` instead of
`not_implemented`. From an ergonomics standpoint this is worse than a
higher flag count: the agent/human trusts a receipt that asserts work that
never happened. It violates the SDDK honesty invariant ("never fabricate
evidence") at the tooling level.

## Ergonomics verdict

1. **The Target surface wins on mechanics**: 1 command vs 2, no state
   threading, uniform flags. Once task bodies are real, `change`/`verify`
   Targets are the better UX by a wide margin.
2. **`verify` is already target-equivalent**: the manual `sddk verify`
   facade and the Target resolve to the same work; no ergonomic gap.
3. **`change` has a real gap and a false-success hazard**: the Target
   prototype cannot replace the manual flow until (a) task bodies are
   wired to the real `plan work-item create` path and (b) the executor
   reports stub execution honestly.

## Recommendations (feed M6.3)

1. **P0 — honest stub reporting**: `target run` on a task with a no-op
   body must emit `status: "not_implemented"` and a target-level status of
   `degraded` (or refuse execution outside `--dry-run`). Success receipts
   for unimplemented work are a trust defect, not a missing feature.
2. **P1 — wire `change` task bodies** to the existing
   `plan work-item create` + `cycle start` internals, threading
   `--cycle-id` internally (the Target DAG already declares the
   dependency `context.resolve → plan.workitem.create`).
3. **P2 — keep `verify` as facade**; the Target adds nothing until
   verification gains multi-step gates the facade doesn't cover.
4. **P3 — error recovery parity**: preserve the manual flow's granular
   per-command errors as task-level failure notes in the Target receipt.

## P0 fix applied (this cycle)

The false-success hazard is fixed, not just documented:

- `Task.has_body: bool` added to the Task contract (`target_task::mod`).
  Built-in targets mark all tasks `has_body: false`.
- `TaskStatus::NotImplemented` and `ExecutionStatus::Degraded` added
  (`target_task::outcome`).
- `DagExecutor::walk` now reports `not_implemented` per stub task and
  degrades the target status to `degraded` instead of `succeeded`
  (`target_task::executor`).
- Regression tests: `stub_tasks_report_not_implemented_and_degrade_target`,
  `wired_task_with_real_body_reports_executed` (a `has_body: true` task
  still produces a normal `executed` receipt).
- Verified end-to-end: `sddk target run --name change` now reports
  `"status": "degraded"` with both tasks `not_implemented`.

M6.3 (wiring real task bodies) now has a mechanical path: implement the
body, flip `has_body: true`, and the executor restores `executed`/
`succeeded` reporting automatically.

## Scope note

Flag counts for `cycle start` assume context inference (the default);
`--no-infer` workspaces add `--root`/`--scope`. The probe ran on sddk
1.168.10 in `/tmp` (non-adopted); adopted-workspace flows may differ in
inference but not in the stub-execution finding.
