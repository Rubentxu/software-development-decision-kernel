# DEC-PLANE-002 — Next-Action Derivation from Persisted Frontier

**Cycle ID:** `p-63676b11dc0ef88f/dec-plane-002-next-action-derivation`
**Horizon:** H3
**Status:** PROPOSED (proposal + spec complete; **apply pending user sign-off**)
**Spine order:** 200 (depends on DEC-PLANE-001, now `SHIPPED`)
**Spine dependency:** DEC-PLANE-001 (v1.90.0, `e8ec46f`)

## Goal

Replace the heuristic `preconditions_hold` match arm in
`crates/sddk-engine/src/run_view.rs` with a derivation from the persisted
frontier of the workflow run. Exit gate: "Next legal actions derive from
persisted frontier, not hard-coded CLI sequences."

## Why now

- DEC-PLANE-001 (v1.90.0) shipped `ActionSurfaceView` with three
  `ActionKind` variants returning `false` ("deferred"): `Start`, `Retry`,
  `Reconcile`.
- The other four variants (`Resume`, `Approve`, `Escalate`, plus the
  unconditional `Abort`) use coarse heuristics on `pending_decisions` and
  `frontier.is_empty()` that are not durable or derivable.
- The same frontier derivation already powers `sddk cycle next`
  (`frontier_for_state` in `crates/sddk-engine/src/lib.rs:1603`).
  Generalizing it to workflow runs is the natural next step.

## Phase status

| Phase | Status | Notes |
|---|---|---|
| Explore | n/a | Skipped (A-lite path; clear precedent in `frontier_for_state`) |
| Propose | Done | This document |
| Spec | Done | `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-NextActionDerivation.md` |
| ADR | Done (proposed) | `~/.sddk-knowledge/sddk-framework/adrs/ADR-076-NEXT-ACTION-DERIVATION.md` |
| Tasks | **Pending** | User sign-off on P3 mapping (P3-a recommended) |
| Apply | **Pending** | Will execute after tasks approval |
| Verify | **Pending** | New tests + workspace gates |
| Debt-verify | **Pending** | |
| Release | **Pending** | Target v1.91.0 |
| Archive | **Pending** | |

## Decisions

- **P3 (mapping) = P3-a (recommended):** builder accepts an optional
  `FrontierProjection`. With projection → authoritative (heuristic ignored).
  Without projection → legacy path runs with `#[deprecated]` warning.
- **P3-b (rejected):** hard-fail without projection. Breaks 14 existing tests.
- **P3-c (rejected):** lazy projection. Contradicts ADR-075 P2=eager.

## Scope (apply phase, bounded)

1. Add `FrontierProjection` + `DeclaredTransitionRef` to `run_view.rs` (or a
   sibling module).
2. Add `build_action_surface_view_with_frontier(state, policy, frontier)`.
3. Annotate `build_action_surface_view` with `#[deprecated]`.
4. Re-implement `preconditions_hold` as a projection of the frontier onto
   the closed taxonomy. `Abort` remains unconditional.
5. Update `sddk run-view` CLI to load the workflow manifest when present
   and pass the projection.
6. Add `crates/sddk-engine/tests/next_action_derivation_tests.rs` with
   one scenario per mapping rule (7 scenarios minimum).

## Out of scope (deferred)

- Generalizing `sddk cycle next` to consume the same projection (cycle CLI
  uses different lifecycle states; DEC-PLANE-004 will reconcile both
  surfaces).
- Persisted-frontier-aware approval/escalation workflow (DEC-PLANE-003
  scope).
- Multi-frontier parallel workflows (diamond topology); deferred until a
  user-visible need emerges.

## Files expected to change

- `crates/sddk-engine/src/run_view.rs` (main refactor)
- `crates/sddk-engine/src/lib.rs` (re-exports)
- `crates/sddk-cli/src/run_view.rs` (CLI loads manifest)
- `crates/sddk-cli/src/main.rs` (passes new args)
- `crates/sddk-engine/tests/next_action_derivation_tests.rs` (new)
- `crates/sddk-engine/tests/run_state_view_tests.rs` (legacy path still passes)
- `crates/sddk-cli/tests/run_view_cli.rs` (extend with frontier scenarios)
- `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`
  (reconciled at archive)

## Open questions for user sign-off

1. **P3 mapping:** confirm P3-a (frontier + fallback) is the locked
   decision, or pick P3-b (frontier-only, hard-fail). P3-a is
   recommended.
2. **Module placement:** add `FrontierProjection` inside `run_view.rs` or
   split into `frontier_projection.rs`? Single-module is consistent with
   the current layout.
3. **CLI manifest loading:** when `sddk run-view` is invoked on a run that
   has no associated workflow manifest, should it error or fall back to
   heuristic? Current heuristic fallback would emit a deprecation warning
   — is that acceptable, or should we hard-error in CLI but keep the
   fallback in the engine API?
4. **Release version:** v1.91.0 (incremental) vs v1.90.1 (patch)? Both
   are additive; minor is recommended for new API surface.

## Next action

Wait for user to:
- Confirm P3-a mapping (or pick alternative)
- Approve the spec scenarios as written
- Greenlight apply phase

Then execute the A-lite sequence: tasks → apply → verify → debt-verify →
release → archive.
