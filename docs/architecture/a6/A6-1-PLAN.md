# A6-1 — `framework_bundle` migration to `AdmissionTicketBus`

> Cycle: `p-63676b11dc0ef88f/a6-1-framework-bundle-migration`
> Status: **planning only**
> Standing on: A6-0 (v1.169.76 — primitive `AdmissionTicketBus` shipped).
> Single change budget: **wrap the `framework_bundle` write path with
> ticket issue + consume**, leaving every other surface to A6-2 / future.

## Why this cycle

A6-0 closed R4-B at the *primitive level* (`TESTED_BOUNDARY`). The
incident (`docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md`) has
four closure requirements; the receipt §7 honestly marks the migration
to `framework_bundle` and `github_releases` as deferred. This cycle
delivers the first half: **`framework_bundle` is the first High-band
unguarded surface to opt into the ticket wrapper**.

## Scope budget (one cycle)

1. **Helper** — `with_framework_bundle_ticket` in
   `crates/sddk-cli/src/dev/install.rs` (and an analogous
   counterpart if `sddk dev update` writes bundles):
   - Take the existing `AuthorityContext::for_cli(...)` + `auth.validate(...)`
     as the **admit** call (already fails closed via R4-A).
   - Build a `PolicySnapshot` matching the registered policy at the
     call site; issue an `AuthorityAdmissionTicket` against the live
     `DefaultAuthorityEngine` registered in the workspace.
   - Run the existing write code inside `bus.consume(now)` so the
     side effect is re-validated at effect time.
   - On consume error → typed refusal; the install aborts **before**
     any `atomic_write` / `copy_tree` / `fs::copy` call.
2. **Integration test** `crates/sddk-cli/tests/a6_1_framework_bundle_migration.rs`:
   - T1: ticket is issued and survives the happy-path install (install completes; ticket is consumed).
   - T2: policy is mutated *between* issue and consume → consume returns `PolicyChanged`; install aborts before any prefix write.
   - T3: a second install on the same bus uses a fresh `ticket_id` (monotonic fence / per-issue identity preserved).
3. **ADR-0131** — `MIGRATION-PATTERN-FOR-WRITABLE-SURFACES` (short ADR): documents how a surface opts into the ticket wrapper. Future A6-2 (`github_releases`) follows the same pattern.
4. **Receipt** — `docs/architecture/a6/A6-1-RECEIPT.md` mirroring A6-0 shape.

## Out of scope (named explicitly)

- **No `github_releases` migration in this cycle.** A6-2 owns it. The
  pattern in ADR-0131 is identical for any other surface; A6-2 will
  consume ADR-0131 verbatim, not re-design.
- **No Medium/Low band migration.** Same as A6-0.
- **No ledger-schema migration.** Tied to A6-0's ticket primitive; no
  new state classes introduced.
- **No change to `AuthorityContext::for_cli` or `AuthorityEngine`.**
  The wrapper is additive at the call site.

## Hard constraints (re-affirmed from A6-0)

| Constraint | Honoured by |
|---|---|
| Strangler over flag-day | `auth.validate(...)` remains; ticket wraps. |
| No new authority | Reuses `AdmissionTicketBus` from A6-0. |
| Falsify before fix | Tests pinned against pre-fix code first (RED). |
| Stop on semantic defect | If A5-C gate evidence requires more, STOP and recertify. |

## Risk gate (initial, 2026-09-18)

- `cargo test --workspace` green at A6-0 close (`9b83046`).
- The current `install.rs` path runs `auth.validate(WritableSurface::FrameworkBundle)`
  before any writes — that gives us R4-A. Adding the ticket wrapper on
  top does not weaken R4-A; it adds the R4-B re-validation.
- The `sddk dev update` path also writes `framework_bundle` (verified
  by `dev update --prune-only` and the install CLI); both call sites
  are owned by A6-1.

## Roadmap Delta (at A6-1 close)

```text
A4-CLOSEOUT     CLOSED v1.169.68
A5-1..A5-3      CLOSED (v1.169.71, v1.169.74, v1.169.75)
A6-0            CLOSED v1.169.76 (R4-B TESTED_BOUNDARY)
A6-1            NEXT (this cycle) — framework_bundle migration     ← YOU ARE HERE
A6-2            framework_bundle DONE -> github_releases
A5-4..A5-5      unblocked at A5-3
A5-C            unblocked at A6-0; A6-1 unblocks FULLY_MIGRATED path
```

## See also

- `docs/architecture/adrs/ADR-0130-FENCED-ADMISSION-TICKETS.md`
- `docs/architecture/a6/A6-0-RECEIPT.md` §6 (deferred migration list)
- `crates/sddk-cli/src/dev/install.rs` (call site)
- `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` (closure criteria, partially met)
