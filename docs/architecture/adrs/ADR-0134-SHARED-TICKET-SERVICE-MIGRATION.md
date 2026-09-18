---
id: ADR-0134-SHARED-TICKET-SERVICE-MIGRATION
status: accepted
supersedes_history: false
adopted_at: 2026-09-18
adoption_cycle: p-63676b11dc0ef88f/a6-4-shared-ticket-service-migration
package_local_id: null
package_source: null
accepted_at: 2026-09-18
accepted_by_cycle: p-63676b11dc0ef88f/a6-4-shared-ticket-service-migration
superseded_by: []
related_adrs:
  - ADR-0130-FENCED-ADMISSION-TICKETS
  - ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES
  - ADR-0132-GITHUB-RELEASES-MIGRATION-PATTERN
  - ADR-0133-AUTHORITY-TICKET-SERVICE
  - ADR-0102-UNIFIED-AUTHORITY-ENGINE
---

# ADR-0134 — Shared `AuthorityTicketService` migration of High-band call sites

## Status

Accepted — A6-4 / v1.169.80

## Context

A6-0 shipped the `AdmissionTicketBus` primitive (ADR-0130, v1.169.76).
A6-1 (ADR-0131) and A6-2 (ADR-0132) migrated `framework_bundle` and
`github_releases` to ticket-protected apply chains using **per-helper**
local buses (v1.169.77, v1.169.78). A6-3 (ADR-0133) introduced
`AuthorityTicketService` as the shared admission+ticket facade with a
process-wide monotonic seq (v1.169.79). The honest limits of A6-1 and
A6-2 (named in their respective receipts) remained:

1. `current_seq = 0` was hardcoded at each consume site (the bus never
   saw a monotonic seq advance).
2. The local helper built its own `DefaultAuthorityEngine` and its own
   `AdmissionTicketBus`, so two High-band surfaces in the same process
   had two independent buses / fences.

`INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY` documented the gap as
`TESTED_BOUNDARY`. The migration of the two High-band surfaces to a
shared service was the named follow-up.

## Decision

Migrate both `framework_bundle` and `github_releases` call sites to the
shared `AuthorityTicketService` (`sddk_engine::authority_ticket_service`).

1. **Process ownership.** The service lives at
   `sddk_engine::authority_ticket_service::process_service()`, a
   `OnceLock<AuthorityTicketService>` constructed lazily on first
   access. Call sites MUST go through `process_service()`; constructing
   one directly is reserved for tests via the documented
   `set_process_service_for_tests` seam.

2. **Helper disposition.** Both
   `with_framework_bundle_ticket(actor, target_id, body)` and
   `with_github_releases_ticket(actor, target_id, body)` are now
   `THIN_COMPAT_FACADE`s over `AuthorityTicketService::process_service()`.
   Their public-internal signatures are unchanged so the call sites in
   `run_dev_install` (framework_bundle) and `run_release_apply`
   (github_releases) do not have to change.

3. **Test seam.** A `*_on(svc, ...)` variant on each facade accepts an
   explicit `&AuthorityTicketService`. The inline tests use the variants
   with a fresh local service to keep parallel test runs isolated; the
   cross-surface shared-state proof uses `process_service()` directly.

4. **Shared-state invariant.** The service's `last_policy_digest` is
   restored by each facade before issue, so a re-entry of either
   facade sees its own canonical digest (avoiding cross-surface
   `PolicyChanged` regressions when one surface is invoked after the
   other).

5. **Live seq.** `current_seq = 0` is **gone** from production call
   sites. `AuthorityTicketService::next_seq()` advances on every
   successful issue within a process.

## Honest limits (carried forward)

1. The seq is still **service-local**, not the canonical event-log
   seq. Wiring the real event-log seq remains a future ADR (out of
   `BASE_PRODUCTION_READY`).
2. `register_policy` still advances the fence unconditionally. A6-4
   did not touch this; if a future cycle finds that this prevents
   atomicity guarantees on the High-band surfaces, a corrective
   cycle must be opened.
3. `last_policy_digest` is single-slot. Two concurrent issues from
   different facades could race on the digest restore. In practice
   the CLI process is single-threaded for High-band writes, so this
   is not exercised.

## What A6-4 does NOT do

- Does NOT migrate Medium / Low-band surfaces (`policy_snapshot`,
  `gate_receipt`, `cycle_lock`, etc.). They remain out of scope for
  `BASE_PRODUCTION_READY`.
- Does NOT wire the canonical event-log seq.
- Does NOT modify the engine's policy matrix.
- Does NOT touch `AuthorityEngineRunner`. Runner and service coexist
  (runner is admit-only, service is admit + ticket).

## Linkage

- ADR-0130 — A6-0 primitive authority.
- ADR-0131 — A6-1 migration pattern.
- ADR-0132 — A6-2 call-site migration.
- ADR-0133 — A6-3 shared facade.
- ADR-0102 — Unified Authority Engine (parent architecture).
- INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY — closed as
  `FULLY_MIGRATED_FOR_REQUIRED_HIGH_BAND_SURFACES`.

## Implementation

- `crates/sddk-engine/src/authority_ticket_service.rs` — added
  `process_service()` singleton + `set_last_policy_digest(...)` seam.
- `crates/sddk-cli/src/dev/framework_bundle_ticket.rs` —
  `THIN_COMPAT_FACADE` over the service; `*_on(svc, ...)` test seam.
- `crates/sddk-cli/src/dev/github_releases_ticket.rs` — same.
- `crates/sddk-cli/src/lib.rs` — `mod dev;` → `pub mod dev;` so the
  integration test can import the facades.
- `crates/sddk-cli/tests/a6_4_shared_ticket_service.rs` — 5 cross-surface
  tests (singleton identity, monotonic seq, fence / policy transition,
  deny ⇒ zero side effects, facade-level shared instance).
- `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` —
  disposition updated to `FULLY_MIGRATED_FOR_REQUIRED_HIGH_BAND_SURFACES`.
