# A6-3 RECEIPT — `AuthorityTicketService` (shared admission+ticket facade)

## Scope

A new module `sddk_engine::authority_ticket_service::AuthorityTicketService`.
It does NOT modify any call site (A6-1 / A6-2 wrappers remain on the
local helper with `current_seq = 0`).

## Why this surface

`AuthorityEngineRunner` is the engine's high-level admission facade but
it does NOT expose seq or a ticket bus. The A6-1 / A6-2 wrappers each
construct their own local `AdmissionTicketBus`, so:

- seq never advances (`current_seq = 0` is hardcoded at consume),
- fence / consumed-set is **per-helper**, not per-process,
- swapping policies between issue and consume is not detected unless
  the caller manually calls `bus.advance_fence()`.

A6-3 introduces a shared facade that the migration cycle (A6-4) can wire
into the wrappers without re-architecting.

## Changes

### `crates/sddk-engine/src/authority_ticket_service.rs` (new, 401 LOC)

- `AuthorityTicketService` (`Clone`, all internal state is `Arc`/`Atomic`).
  Construct **once** per CLI process.
- Internal state:
  - `engine: Mutex<DefaultAuthorityEngine>` — shared admit engine.
  - `bus: AdmissionTicketBus` — shared A6-0 primitive.
  - `seq: AtomicU64` — monotonic seq counter.
  - `last_policy_digest: Mutex<Option<DigestSha256>>` — the digest of
    the most recently registered policy (consumed at the live `now`).
- Methods:
  - `new() -> Self`
  - `register_policy(PolicySnapshot) -> Result<_, _>` — also advances
    the fence so any in-flight ticket from the previous policy is
    invalidated at consume time (`PolicyChanged` / `FenceExpired`
    paths).
  - `next_seq() -> u64` (observability)
  - `current_fence() -> u64`
  - `authority_now(&PolicySnapshot) -> AuthorityNow`
  - `issue(&Actor, &ActionProposal, &PolicySnapshot, &Facts)` — bumps
    `seq` on success.
  - `consume_at_live_now(&AuthorityAdmissionTicket, &PolicySnapshot)`
  - `issue_and_consume(...)` — one-shot helper mirroring the A6-1 /
    A6-2 wrapper shape, returning the issued ticket for receipts.
- Typed `AuthorityTicketServiceError` (`Denied` / `Ticket` /
  `UnknownSurface` / `LockPoisoned`).
- 5 inline `#[test]` verdicts:
  - `fence_t1_admit_issue_consume_returns_ticket_and_value` (happy path)
  - `fence_t2_swapped_policy_is_rejected_at_consume` (PolicyChanged)
  - `fence_t3_advance_fence_via_register_policy_invalidates_in_flight`
    (PolicyChanged OR FenceExpired)
  - `fence_t4_shared_bus_across_two_issues_distinct_seq` (process-wide
    monotonic seq, two issues share the consumed-set)
  - `fence_t5_deny_yields_no_ticket` (deny ⇒ seq NOT bumped)

### `crates/sddk-engine/src/lib.rs`

- `+pub mod authority_ticket_service;` (line 31)

### `docs/architecture/adrs/ADR-0133-AUTHORITY-TICKET-SERVICE.md` (new, 104 LOC)

Accepted. Frontmatter mirrors ADR-0132's canonical schema. Links
ADR-0130, ADR-0131, ADR-0132, ADR-0102.

## Honest limits (carried forward, expanded)

1. The seq is **service-local**. It is NOT the seq from the canonical
   event log. Wiring the real event-log seq is out of scope for
   `BASE_PRODUCTION_READY` (a future ADR).
2. A6-1 / A6-2 still use the local helper with `current_seq = 0`.
   Until A6-4 migrates them, that limit is still in force at the call
   sites. **The seq=0 honest limit is NOT yet closed.**
3. The service `body` callback returns `Result<T, String>` to avoid
   pulling `anyhow` into `sddk-engine`. Any caller that already uses
   `anyhow::Error` maps at the boundary.
4. `register_policy` advances the fence. This is **permissive** — a
   caller could invalidate unrelated tickets by re-registering. A6-4
   will explore targeted fence advance if needed.

## Tests run (scope = service + regressions)

```text
cargo test -p sddk-engine --lib authority_ticket_service
  → 5 passed; 0 failed
cargo test -p sddk-engine --lib
  → 1294 passed; 0 failed; 1 ignored (full engine regression)
cargo test -p sddk-cli --lib github_releases_ticket
  → 4 passed; 0 failed  (A6-2 sibling regression)
cargo test -p sddk-cli --lib framework_bundle_ticket
  → 4 passed; 0 failed  (A6-1 sibling regression)
cargo test -p sddk-cli --test context_fitness
  → 7 passed; 0 failed  (architectural lint including
                          no_new_root_level_context_module_without_adr)
cargo clippy -p sddk-engine --all-targets -- -D warnings
  → exit 0
cargo fmt -p sddk-engine
  → applied
bash scripts/mirror_adrs_to_vault.py
  → created: 1, skipped: 39  (ADR-0133 mirrored)
```

## What this cycle does NOT do

- Does NOT migrate A6-1 / A6-2 to the service (A6-4 work).
- Does NOT wire the real event-log seq (out of BASE_PRODUCTION_READY).
- Does NOT modify the engine event log semantics.
- Does NOT modify `AuthorityEngineRunner`. The runner and the service
  coexist: runner is the **admit-only** facade, service is the
  **admit + ticket** facade.

## Linkage

- ADR-0133 (`AUTHORITY-TICKET-SERVICE`) — accepted this cycle.
- ADR-0132 (`GITHUB-RELEASES-MIGRATION-PATTERN`) — sibling, A6-2 call
  site (still uses local helper).
- ADR-0131 (`MIGRATION-PATTERN-FOR-WRITABLE-SURFACES`) — sibling pattern.
- ADR-0130 (`FENCED-ADMISSION-TICKETS`) — A6-0 primitive authority.
- ADR-0102 (`UNIFIED-AUTHORITY-ENGINE`) — parent architecture.
- INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY — back-of-envelope (still at
  `TESTED_BOUNDARY`; not closed because A6-4 still has to migrate the
  call sites).

## Next step

A6-4 (next session, per "un ciclo por sesión" rule):

1. Migrate `framework_bundle_ticket` to the service.
2. Migrate `github_releases_ticket` to the service.
3. Re-run the FENCE matrix at the new call sites.
4. Update INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY to `CLOSED` if the
   migration holds.
