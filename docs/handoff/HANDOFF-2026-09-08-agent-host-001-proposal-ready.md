# AGENT-HOST-001 — Agent Host Substrate (handoff propose-only)

**Cycle ID:** `p-63676b11dc0ef88f/agent-host-001-substrate`
**Horizon:** H4 (Agent Host) — **change of horizon from H3 Decision Plane**
**Status:** PROPOSED (proposal + spec + ADR complete; **apply pending user sign-off**)
**Spine order:** 230 (depends on DEC-PLANE-004, SHIPPED in v1.93.0)

## Goal

Introduce the agent host that owns lease/fencing/retry/identity so
cycles and runs execute through one substrate instead of per-call
wiring.

## Why now

- DEC-PLANE-001 to -004 closed the Decision Plane (typed verdicts,
  provenance, CLI parity). Every cycle / run surface can now produce a
  `DecisionRecord`.
- What is missing is the **executor**: lease ownership, fencing,
  retry, identity. Today each call site wires these manually:
  - `crates/sddk-storage/src/lib.rs::acquire_cycle_lease` is invoked
    directly.
  - `crates/sddk-engine/src/retry.rs::RetryPolicy` exists but is
    unwired.
  - No `AgentIdentity` struct; agents are `String` owners.

## Phase status

| Phase | Status | Notes |
|---|---|---|
| Propose | Done | This document |
| Spec | Done | `~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentHost.md` |
| ADR | Done (proposed) | `~/.sddk-knowledge/sddk-framework/adrs/ADR-079-AGENT-HOST-SUBSTRATE.md` |
| Tasks | **Pending** | User sign-off on P8 + open questions below |
| Apply | **Pending** | `crates/sddk-engine/src/agent_host.rs` (new, ~250 lines) + tests |
| Verify | **Pending** | new tests + workspace gates |
| Debt-verify | **Pending** | |
| Release | **Pending** | Target v1.94.0 |
| Archive | **Pending** | |

## Decisions

- **P8 (substrate shape) = P8-a (recommended):** `AgentHost` struct +
  `LeaseHandle` RAII + `DecisionReceipt`. RAII guarantees no lease
  leaks even on panic paths. `DecisionReceipt` ties provenance (from
  DEC-PLANE-003) to lease + retry outcomes — full audit trail.
- **P8-b (rejected):** free functions. Loses RAII; callers must
  remember to release.
- **P8-c (rejected):** actor model. Overkill for bounded scope.

## Scope (apply phase, bounded)

1. Add `crates/sddk-engine/src/agent_host.rs` (~250 lines):
   - `AgentIdentity { id, display_name, kind }`
   - `AgentKind { Human, Auto, External }`
   - `AgentHost { identity, storage, clock, retry_policies }`
   - `LeaseHandle` (RAII with `Drop`)
   - `DecisionReceipt`
   - `LeaseError`, `ExecuteError<E>`, `ExecuteDecisionError`
2. Wire into `crates/sddk-engine/src/lib.rs` (re-exports).
3. New tests in `crates/sddk-engine/tests/agent_host_tests.rs` (≥6 scenarios).
4. Workspace gates: fmt, clippy, test, no regressions.
5. Bump to `v1.94.0` and release.
6. Spine reconcile + archive-manifest.

## Out of scope (deferred to later cycles)

- Migrating `sddk cycle transition` to consume `AgentHost::execute_decision`
  (will be AGENT-HOST-002).
- Wiring the CLI to construct an `AgentIdentity` from the local
  environment (e.g. `SDDK_AGENT_ID` env var).
- Async / actor model support.
- Per-cycle custom retry policies (current scope uses defaults).

## Files expected to change

- `crates/sddk-engine/src/agent_host.rs` (new, ~250 lines)
- `crates/sddk-engine/src/lib.rs` (re-exports)
- `crates/sddk-engine/tests/agent_host_tests.rs` (new, ≥6 tests)
- `Cargo.toml` + `Cargo.lock` (version bump)
- `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml`
- `.sddk/cycles/p-63676b11dc0ef88f/agent-host-001-substrate/archive-manifest.md`

## Open questions for user sign-off

1. **P8 substrate shape:** confirm P8-a (struct + RAII) is the locked
   decision, or pick P8-b / P8-c. P8-a is recommended.
2. **Default retry policies:** confirm the defaults in the spec:
   `Retry: max_attempts=3, backoff_ms=1000`; everything else
   `max_attempts=1`. Alternative: make `Retry` default to
   `max_attempts=5` (some CI bots expect aggressive retry).
3. **AgentKind mapping:** the spec defines `Human`, `Auto`,
   `External`. Should we add `Service` for long-running services
   (different lease TTL semantics)?
4. **Lease TTL default:** what should `acquire_lease` default to when
   the caller omits `ttl_ms`? Spec assumes caller passes it explicitly;
   alternative is a 60_000 ms (60 s) default.
5. **Release version:** v1.94.0 (incremental minor) is recommended for
   new substrate. Patch (v1.93.1) is also viable since the new types
   are additive — your call.

## Next action

Wait for user to:
- Confirm P8-a vs alternatives
- Resolve the 4 open questions above
- Greenlight apply phase

Then execute: tasks → apply → verify → debt-verify → release v1.94.0 → archive.
