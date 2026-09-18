---
id: ADR-0133-AUTHORITY-TICKET-SERVICE
status: accepted
supersedes_history: false
adopted_at: 2026-09-18
adoption_cycle: p-63676b11dc0ef88f/a6-3-authority-ticket-service
package_local_id: null
package_source: null
accepted_at: 2026-09-18
accepted_by_cycle: p-63676b11dc0ef88f/a6-3-authority-ticket-service
superseded_by: []
related_adrs:
  - ADR-0130-FENCED-ADMISSION-TICKETS
  - ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES
  - ADR-0132-GITHUB-RELEASES-MIGRATION-PATTERN
  - ADR-0102-UNIFIED-AUTHORITY-ENGINE
---

# ADR-0133 — `AuthorityTicketService`: shared admission+ticket facade

## Status

Accepted — A6-3 / v1.169.79

## Context

A6-0 shipped the `AdmissionTicketBus` primitive (issue + consume with
FENCE matrix T1–T5). A6-1 (`framework_bundle`) and A6-2 (`github_releases`)
each wrapped a side effect with their own **local** `AdmissionTicketBus`
created inside the helper.

Two honest limits were named in A6-1-RECEIPT.md and A6-2-RECEIPT.md:

1. `current_seq = 0` hardcoded at the consume site — the bus never sees
   a monotonic seq advance.
2. `policy_digest` is the helper's local `PolicySnapshot` digest; the
   helper does NOT take a live policy from `AuthorityEngineRunner`.

Both limits are pinned at primitive level by
`crates/sddk-engine/tests/a6_0_admission_tickets.rs`. They are NOT pinned
at the A6-1 / A6-2 call sites.

`AuthorityEngineRunner` (the engine's high-level admission facade) does
NOT expose seq or a ticket bus. It just runs `engine.admit()` and returns
a `RunnerVerdict`. So "wire the live seq / live policy into A6-1 / A6-2"
presupposed a source that does not exist.

## Decision

Create a new module `sddk_engine::authority_ticket_service` that combines:

- `DefaultAuthorityEngine` (R4-A admit step)
- `AdmissionTicketBus` (R4-B issue + consume, A6-0 primitive)

into a single `AuthorityTicketService`. The service:

- holds a **process-wide monotonic `AtomicU64` seq** that advances on
  every successful issue;
- owns a **shared `AdmissionTicketBus`** constructed once at service
  construction (so different call sites inside one CLI process see the
  same fence / consumed-set);
- accepts **live `PolicySnapshot` registration** via
  `register_policy(policy)`. Each registration advances the fence so any
  in-flight ticket from the previous policy is invalidated at consume
  time (the existing `PolicyChanged` / `FenceExpired` paths handle this);
- exposes `current_fence()`, `next_seq()`, `authority_now(policy)` for
  receipts and observability;
- exposes a one-shot helper `issue_and_consume(...)` that mirrors the
  wrapper shape used by A6-1 / A6-2 but routes through the shared
  service.

The service is `Clone` (cheap, all internal state is `Arc`/`Atomic`).

## Scope of this cycle

A6-3 ships **the service + its evidence only**. It does NOT yet migrate
A6-1 / A6-2 call sites to the service. That migration is A6-4.

**Why not now?**

A6-1 / A6-2 are now in production (`v1.169.77` and `v1.169.78`). Wiring
the service into them changes the helper signature and may interact with
how the callers construct the actor (the engine-flavoured `Actor` vs.
domain `Actor` mapping at the `run_dev_install` boundary). The blast
radius of "migrate both" exceeds what a single cycle should commit. The
strangler pattern (ADR-0131) says: introduce the new authority first,
then migrate.

## Honest limits (carried forward)

1. The seq is **service-local**. It is NOT the seq from the canonical
   event log. Wiring the real event-log seq is a future ADR (out of
   scope for `BASE_PRODUCTION_READY`).
2. A6-1 / A6-2 **still use the local helper** with `current_seq = 0`.
   Until A6-4 migrates them, the `seq=0` honest limit is still in force
   at the call sites, even though the service now exposes a real seq.
3. `last_policy_digest` is the digest of the **last registered** policy;
   if the caller passes a *different* `PolicySnapshot` to
   `consume_at_live_now`, the live digest still wins (the `policy` arg
   to `consume_at_live_now` is a fallback when no policy was ever
   registered).
4. The service's `body` callback returns `Result<T, String>` to avoid
   pulling `anyhow` into `sddk-engine`. Callers that already use
   `anyhow::Error` map with `.map_err(|e| e.to_string())` at the
   boundary.

## Linkage

- ADR-0130 — `FENCED-ADMISSION-TICKETS` (A6-0 primitive authority)
- ADR-0131 — `MIGRATION-PATTERN-FOR-WRITABLE-SURFACES` (sibling pattern)
- ADR-0132 — `GITHUB-RELEASES-MIGRATION-PATTERN` (A6-2 call site)
- INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY (closing toward
  `TESTED_BOUNDARY` → `CLOSED`)
- A6-1-RECEIPT.md §"Honest limits" — currently in force at A6-1 call site
- A6-2-RECEIPT.md §"Honest limits" — currently in force at A6-2 call site

## Implementation

The service is implemented in `crates/sddk-engine/src/authority_ticket_service.rs`
and re-exported as `sddk_engine::authority_ticket_service` (top-level
`pub mod` in `crates/sddk-engine/src/lib.rs`). The five FENCE-matrix tests
are inline in the file (`fence_t1` … `fence_t5`).
