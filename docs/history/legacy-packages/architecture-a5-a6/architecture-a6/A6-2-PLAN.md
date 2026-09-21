# A6-2 — `github_releases` migration to `AdmissionTicketBus`

> Cycle: `p-63676b11dc0ef88f/a6-2-github-releases-migration`
> Status: **planning only**
> Standing on: A6-0 (v1.169.76), A6-1 (v1.169.77).
> Single change budget: **wrap the `github_releases` (Forge route) entry
> point with the same `AdmissionTicketBus` helper pattern as A6-1**.

## Why this cycle

A6-1 migrated `framework_bundle`. This cycle migrates `github_releases`,
the second High-band unguarded surface named in
`docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md`. The migration
pattern from ADR-0131 is applied verbatim; this cycle adds the second
data point that validates the pattern.

## Scope budget (one cycle)

1. **Helper** — `with_github_releases_ticket` in
   `crates/sddk-cli/src/dev/github_releases_ticket.rs`:
   - Take the existing `authorize_release(...)` + the **Forge route**
     decision as the admit-time check (R4-A preserved).
   - Construct an `AuthorityAdmissionTicket` against an
     `AuthorityEngineRunner`-aligned policy.
   - Run `apply_release(...)` inside `bus.consume(now)`; the ticket
     **covers the entire release apply chain** (CreatePr, MergePr,
     CreateRelease all under the same ticket; any of them seeing a
     stale decision causes the consume to refuse and the chain aborts).
   - On consume error → typed refusal; the apply chain aborts.
2. **Integration tests** in
   `crates/sddk-cli/src/dev/github_releases_ticket.rs::tests`:
   - **T1**: ticket survives a happy-path apply (MockForge records the
     expected PR + release operations; ticket is consumed at end).
   - **T4**: ticket consumed once; a second `apply_release` on the
     same bus instance is refused with `TicketAlreadyConsumed`.
   - **T5**: a forged ticket (not from `with_...`) cannot drive the
     Forge apply chain — because the wrapper constructs the ticket
     itself, no ticket-less call site can invoke the protected path.
   - **T6 (verdict-anchored)**: forged ticket with swapped
     `policy_digest` is refused at consume.
3. **ADR-0132** — `GITHUB-RELEASES-MIGRATION-PATTERN` (very short
   reference ADR that says "follows ADR-0131 verbatim, applied to the
   Forge route of `run_release_apply`").
4. **Receipt** — `docs/architecture/a6/A6-2-RECEIPT.md`.

## What this cycle covers (more than A6-1)

The A6-1 helper wraps `framework_bundle` writes. The A6-2 helper wraps
**the entire `apply_release(...)` chain**, which is the only call site
that touches `github_releases`, `pr.create`, `pr.merge`, and
`release.create` together. If any one of these is gated by a policy
flip, the whole chain fails closed.

## Out of scope (named explicitly)

- **No Medium-band migration** (named in A6-0 §6 follow-up #3).
- **No `Local` route ticket** (the Local route touches `git.push` /
  `git.tag` only, not GitHub Releases; that's a separate consideration).
- **No A4 semantic change.**
- **No `AuthorityEngine::admit` change.**

## Hard constraints (re-affirmed)

| Constraint | Honoured by |
|---|---|
| Strangler over flag-day | `authorize_release(...)` remains; ticket wraps. |
| No new authority | Reuses `AdmissionTicketBus` from A6-0. |
| Falsify before fix | Tests pinned against pre-fix code first (RED). |
| Stop on semantic defect | If A5-C gate evidence requires more, STOP and recertify. |

## Risk gate (initial, 2026-09-18)

- `cargo test --workspace` green at A6-1 close (`f65b546`).
- `apply_release(...)` is the single canonical entry point for the
  Forge route; the helper routes ALL its work through the consume
  point. MockForge tests will prove the protection at the type level.
- The `Local` route (no `--repo`) does not touch `github_releases`;
  this cycle does not modify Local.

## Roadmap Delta (at A6-2 close)

```text
A4-CLOSEOUT     CLOSED v1.169.68
A5-1..A5-3      CLOSED (v1.169.71, v1.169.74, v1.169.75)
A6-0            CLOSED v1.169.76 (R4-B TESTED_BOUNDARY primitive)
A6-1            CLOSED v1.169.77 (framework_bundle migration)
A6-2            NEXT — github_releases migration                  ← YOU ARE HERE
A5-4..A5-5      unblocked (A6 independent)
A5-C            unblocked at A6-0; A6-1 + A6-2 advance FULLY_MIGRATED
```

## See also

- `docs/architecture/adrs/ADR-0130-FENCED-ADMISSION-TICKETS.md`
- `docs/architecture/adrs/ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES.md`
- `docs/architecture/a6/A6-0-RECEIPT.md` §6 (deferred migration list)
- `docs/architecture/a6/A6-1-RECEIPT.md` (the previous migration step)
- `crates/sddk-cli/src/release_cmd.rs::run_release_apply` (call site)
- `crates/sddk-gateway/src/release.rs::apply_release` (downstream)
