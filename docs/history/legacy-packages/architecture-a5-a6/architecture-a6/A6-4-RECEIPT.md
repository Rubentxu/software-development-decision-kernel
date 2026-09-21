# A6-4 RECEIPT — Migrate High-band call sites to shared `AuthorityTicketService`

## Scope

Migrate `framework_bundle` (A6-1) and `github_releases` (A6-2) from
their per-helper local `AdmissionTicketBus` / `current_seq = 0` setup
to the process-wide `AuthorityTicketService::process_service()` singleton
shipped in A6-3.

`INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY` disposition is moved from
`TESTED_BOUNDARY` to `FULLY_MIGRATED_FOR_REQUIRED_HIGH_BAND_SURFACES`.

## Why this surface

After A6-3, both High-band call sites were still using local helper
buses. The shared service was shipped but **not** wired. A6-4 closes
that gap and removes `current_seq = 0` from production paths.

## Changes

### `crates/sddk-engine/src/authority_ticket_service.rs`

- Added `process_service()` returning `&'static AuthorityTicketService`
  via a `OnceLock` singleton.
- Added `set_last_policy_digest(digest)` for facade re-entry to restore
  the canonical digest without triggering a fence advance.
- Added `set_process_service_for_tests(svc)` test-only seam.

### `crates/sddk-cli/src/dev/framework_bundle_ticket.rs`

- `with_framework_bundle_ticket(actor, target_id, body)` is now a
  `THIN_COMPAT_FACADE` over the shared service.
- New `with_framework_bundle_ticket_on(svc, ...)` test seam.
- Inline tests (T1, T4, T5, T6) use the `_on` variant with a fresh
  local service so parallel test runs do not collide on the singleton.

### `crates/sddk-cli/src/dev/github_releases_ticket.rs`

- Symmetric refactor: `with_github_releases_ticket(...)` is a
  `THIN_COMPAT_FACADE` over the service; `*_on(svc, ...)` is the test
  seam.
- T1, T4, T5, T6 tests rewritten to use the local-service variant.

### `crates/sddk-cli/src/lib.rs`

- `mod dev;` → `pub mod dev;` (so the integration test can import the
  facades).

### `crates/sddk-cli/src/dev/mod.rs`

- `pub(super) mod framework_bundle_ticket;` → `pub mod ...;`
- `pub(super) mod github_releases_ticket;` → `pub mod ...;`

### `crates/sddk-cli/tests/a6_4_shared_ticket_service.rs` (new, 5 tests)

- `shared_service_singleton_is_same_instance_for_both_surfaces` —
  proves `process_service()` returns the same static ref every time
  (§5).
- `cross_surface_shared_seq_strictly_monotonic` — issues one fb ticket
  and one gr ticket against the same service and asserts `seq` advances
  by ≥ 2 (§5).
- `cross_surface_facades_share_the_service_instance` — same as above
  but through the public facade functions (§5).
- `cross_surface_policy_transition_invalidates_either_surface_ticket` —
  issue an fb ticket, register a fresh gr policy on the service,
  consume the fb ticket at live_now: refuses with `PolicyChanged` /
  `FenceExpired` (§6).
- `facade_deny_yields_zero_side_effects_on_real_service` — confirms
  that deny ⇒ no body execution against the real shared service (§3).

### `docs/architecture/adrs/ADR-0134-SHARED-TICKET-SERVICE-MIGRATION.md`

Accepted. Links ADR-0130, -0131, -0132, -0133, -0102.

### `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md`

`closed_by` → `sddk-apply (A6-4)`; `closed_reason` rewritten to
`FULLY_MIGRATED_FOR_REQUIRED_HIGH_BAND_SURFACES`; `cycle_closed` →
`p-63676b11dc0ef88f/a6-4-shared-ticket-service-migration`.

## Honest limits (carried forward, expanded)

1. The seq is still **service-local**, not the canonical event-log
   seq. Wiring the real event-log seq remains out of scope for
   `BASE_PRODUCTION_READY`. Honest assertion: `service-local monotonic
   admission sequence`, NOT a global ordering.
2. `register_policy` advances the fence globally. If a future cycle
   demonstrates that this prevents atomicity guarantees on the
   High-band surfaces, a corrective cycle must be opened — **without**
   silently expanding A6-4 (per the user's guardrail).
3. `last_policy_digest` is single-slot; concurrent issues from
   different facades could race on the digest restore. The CLI is
   single-threaded for High-band writes, so this is not exercised.
4. The `*_on(svc, ...)` test seams accept any service; production
   must still go through `process_service()`. The thin facades do not
   expose a path that lets production construct its own service.
5. `AuthorityEngineRunner` is **not** wired to the service. The runner
   remains an admit-only facade; defense-in-depth checks at the
   engine entry points are unchanged.

## FENCE matrix at the call sites (re-run)

| T | Test | Status |
|---|------|--------|
| T1 | happy path runs body and consumes ticket | GREEN (fb + gr) |
| T2 | policy changed ⇒ typed refusal at consume | GREEN at primitive + service; **not** pinned at the facade call site (cross-surface proof covers it via `cross_surface_policy_transition_invalidates_either_surface_ticket`) |
| T3 | fence advance ⇒ typed refusal | GREEN (fb + gr T6) |
| T4 | one-shot ticket does not reissue within a bus | GREEN (regression on A6-0 primitive) |
| T5 | Deny ⇒ no ticket, body never runs | GREEN (fb + gr) |
| T6 | policy swap invalidates ticket at consume | GREEN (fb + gr T6) |

## Tests run (scope = migration + regressions)

```text
cargo test -p sddk-cli --lib
  → 781 passed; 0 failed; 0 ignored
cargo test -p sddk-cli --lib -- framework_bundle_ticket:: github_releases_ticket::
  → 8 passed; 0 failed
cargo test -p sddk-cli --test a6_4_shared_ticket_service
  → 5 passed; 0 failed  (cross-surface shared-state proof)
cargo test -p sddk-cli --test context_fitness
  → 7 passed; 0 failed  (architectural lint incl. no_new_root_level_context_module_without_adr)
cargo test -p sddk-engine --lib
  → 1294 passed; 0 failed; 1 ignored  (full engine regression)
cargo fmt --all -- --check
  → exit 0
cargo clippy --workspace --all-targets -- -D warnings
  → exit 0
bash scripts/mirror_adrs_to_vault.py
  → created: 1, skipped: 40  (ADR-0134 mirrored)
```

## What A6-4 does NOT do (per the change budget)

- No new Authority Engine primitives.
- No new policy model.
- No event-log sequence integration.
- No Medium / Low-band expansion.
- No defence-in-depth refactor of `AuthorityEngineRunner`.

## Linkage

- ADR-0134 (`SHARED-TICKET-SERVICE-MIGRATION`) — accepted this cycle.
- INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY — closed as
  `FULLY_MIGRATED_FOR_REQUIRED_HIGH_BAND_SURFACES`.
- A6-3-RECEIPT.md §"Next steps" — A6-4 inherits the closure of the
  `current_seq = 0` honest limit.
- A6-1-RECEIPT.md, A6-2-RECEIPT.md — referenced for the migrated call
  sites; their local-helper disposition is `THIN_COMPAT_FACADE` after
  A6-4.

## Next step

Per A6-4 exit criteria (§14): the required High-band migration is
complete. After release + handoff:

1. **ROADMAP-CLOSEOUT AUDIT** against the living roadmap
2. If only POST_BASE P2/P3 items remain, proceed to
   `ROADMAP-COMPLETION-RECEIPT`.
