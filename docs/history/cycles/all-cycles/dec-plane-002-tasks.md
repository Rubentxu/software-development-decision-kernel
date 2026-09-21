# DEC-PLANE-002 — Tasks Breakdown

**Cycle ID:** `p-63676b11dc0ef88f/dec-plane-002-next-action-derivation`
**Spine order:** 200
**Path:** A-lite (propose → spec → tasks → apply → verify → debt-verify → release → archive)
**P3 mapping:** P3-a (frontier projection with fallback) — confirmed in auto-mode per ADR-076 recommendation.

## Tasks (apply-ready)

### T1. RED tests — `crates/sddk-engine/tests/next_action_derivation_tests.rs`

One scenario per mapping rule (7 scenarios) + 2 cross-cutting scenarios:

- [ ] T1.1 `frontier_projection_empty_only_abort` — empty frontier → `[Abort]`
- [ ] T1.2 `frontier_projection_resume_from_running` — single Running ready entry → `[Resume, Abort]`
- [ ] T1.3 `frontier_projection_start_from_pending` — Pending entry → `[Start, Abort]`
- [ ] T1.4 `frontier_projection_approve_only` — entry with has_approval_gate → `[Approve, Abort]`
- [ ] T1.5 `frontier_projection_escalate_only` — entry with has_escalation_gate → `[Escalate, Abort]`
- [ ] T1.6 `frontier_projection_retry_failed` — Failed entry with retry_policy → `[Retry, Abort]`
- [ ] T1.7 `frontier_projection_reconcile_drifted` — entry with to_status=Drifted, requires_met → `[Reconcile, Abort]`
- [ ] T1.8 `frontier_projection_multi_action_coexist` — Approve+Escalate entries → both present
- [ ] T1.9 `frontier_projection_determinism` — same inputs across two builds → byte-equal Vec
- [ ] T1.10 `frontier_projection_supersedes_heuristic` — pending_decisions has approval-X but frontier has none → no Approve

### T2. Frontier projection types — `crates/sddk-engine/src/run_view.rs`

- [ ] T2.1 Add `FrontierProjection` struct (entries, declared_transitions)
- [ ] T2.2 Add `DeclaredTransitionRef` struct (id, from_status, to_status, from_phase, has_approval_gate, has_escalation_gate, has_retry_policy, paths)
- [ ] T2.3 Add `FrontierEntry` ref struct (transition_id, requires_met)
- [ ] T2.4 Add `empty_projection()` constructor for fallback

### T3. Builder signature — `crates/sddk-engine/src/run_view.rs`

- [ ] T3.1 Add `build_action_surface_view_with_frontier(state, policy, frontier)` returning `Result<ActionSurfaceView, ViewError>`
- [ ] T3.2 Re-implement `preconditions_hold` as `project_frontier_to_actions(frontier, state)` returning the same `Vec<ActionKind>`
- [ ] T3.3 Add `#[deprecated(note = "...")]` to `build_action_surface_view` (no-frontier variant); body delegates to `*_with_frontier(state, policy, &empty_projection())`
- [ ] T3.4 Ensure sorted-by-discriminant output for determinism

### T4. Module wiring — `crates/sddk-engine/src/lib.rs`

- [ ] T4.1 Re-export `FrontierProjection`, `DeclaredTransitionRef`, `FrontierEntryRef`, `build_action_surface_view_with_frontier`, `empty_projection`
- [ ] T4.2 Confirm clippy clean

### T5. CLI — `crates/sddk-cli/src/run_view.rs`

- [ ] T5.1 Load workflow manifest from CLI runtime when present (best-effort)
- [ ] T5.2 Build `FrontierProjection` from the manifest + ledger (delegated to engine helper)
- [ ] T5.3 Pass projection to `build_action_surface_view_with_frontier`
- [ ] T5.4 When manifest is missing, fall back to deprecated path with a stderr warning

### T6. CLI tests — `crates/sddk-cli/tests/run_view_cli.rs`

- [ ] T6.1 Add scenario: `run_view_with_manifest_uses_frontier`
- [ ] T6.2 Add scenario: `run_view_without_manifest_falls_back_with_warning`

### T7. Verification

- [ ] T7.1 `cargo fmt --check`
- [ ] T7.2 `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] T7.3 `cargo test --workspace --offline`

### T8. Release

- [ ] T8.1 Bump `Cargo.toml` to `1.91.0`
- [ ] T8.2 Run `scripts/release.sh`
- [ ] T8.3 Move tag, mark `Latest=true`

### T9. Archive

- [ ] T9.1 Reconcile `EXECUTION-SPINE.yaml` `DEC-PLANE-002 PROPOSED → SHIPPED`
- [ ] T9.2 Write `archive-manifest.md` in `.sddk/cycles/p-63676b11dc0ef88f/dec-plane-002-next-action-derivation/`
- [ ] T9.3 Write handoff doc

## Out of scope (deferred to later cycles)

- Generalizing `sddk cycle next` to consume the same projection (DEC-PLANE-004).
- Persisted-frontier-aware approval/escalation workflow (DEC-PLANE-003).
- Multi-frontier parallel workflows (diamond topology).

## Acceptance criteria

1. All 10 new engine tests green.
2. 2 new CLI tests green.
3. Existing 14 tests still pass (fallback path keeps them green).
4. Workspace `cargo test --workspace --offline` → 0 failures.
5. Clippy clean with `-D warnings`.
6. Release v1.91.0 published + installed.
7. Spine reconciled.
