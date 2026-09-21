# Handoff A6-2 session close — 2026-09-18

## Result

A6-2 SHIPPED — `run_release_apply` (Forge route) is now wrapped by an
`AdmissionTicketBus` issue + consume gate. `apply_release(...)` runs
only after the ticket is consumed. R4-B is closed for the second High-band
unguarded writable surface.

## State

- Binary installed: `sddk 1.169.78` (`/home/rubentxu/.local/bin/sddk`).
- HEAD: `15d4b95` (clean, pushed to `origin/main`).
- Tag: `v1.169.78` published on GitHub Releases (10-asset contract, public
  release gate 10/10 GREEN).

## Commits this cycle

```text
1551ea4 feat(cli): A6-2 wrap github_releases apply chain under AdmissionTicket
15d4b95 chore(release): bump version to 1.169.78
```

## What landed

- `crates/sddk-cli/src/dev/github_releases_ticket.rs` (new) — helper
  `with_github_releases_ticket(actor, target_id, body)` + typed
  `GithubReleasesTicketError` + 4 unit tests (T1/T4/T5/T6).
- `crates/sddk-cli/src/dev/mod.rs` — `pub(super) mod github_releases_ticket;`.
- `crates/sddk-cli/src/release_cmd.rs` — Forge branch of `run_release_apply`
  wraps `apply_release(...)` body in the helper. Errors mapped through
  three variants (`Denied` / `Ticket` / `Apply`).
- ADR-0132 (`GITHUB-RELEASES-MIGRATION-PATTERN`) accepted.
- `docs/architecture/a6/A6-2-PLAN.md` + `A6-2-RECEIPT.md`.

## Gates run (scope = change-scoped, scoped profile)

```text
cargo build -p sddk-cli                                   # ok
cargo test -p sddk-cli --lib github_releases_ticket       # 4 passed
cargo test -p sddk-engine --lib authority_admission_ticket
                                                          # 8 passed (regression)
cargo test -p sddk-cli --lib framework_bundle             # 5 passed (regression)
cargo clippy -p sddk-cli --all-targets -- -D warnings    # exit 0
cargo fmt --all -- --check                                # exit 0
bash tests/test_vault_adr_mirror_coverage.sh              # 39 ADRs mirrored
bash scripts/release.sh                                   # 14/14 GREEN
```

## Honest limits (named, not skipped)

1. `current_seq = 0` is hardcoded at the consume site. Live monotonic seq
   from `AuthorityEngineRunner` is A6-3.
2. `policy_digest` consumed is the helper's own `PolicySnapshot` digest
   (no live override step). T2 of the FENCE matrix is pinned at primitive
   level (`a6_0_admission_tickets.rs`); call site does not pin it directly.
3. Helper does NOT enrich capabilities — the caller passes an `Actor` already
   constructed. This was a deliberate design choice so tests can construct
   actors whose capability set will be denied.
4. R4-B is now closed for **two of two** High-band unguarded writable surfaces
   (`framework_bundle` and `github_releases`). The Low/Medium band back-of-envelope
   surfaces (`policy_snapshot`, `gate_receipt`, `cycle_lock`) are NOT yet
   ticket-protected. They are out of scope for `BASE_PRODUCTION_READY` and
   will be tackled in a later A6-* wave.

## Next steps

- A6-3: thread the live `PolicySnapshot` from `AuthorityEngineRunner` and the
  monotonic seq into the consume sites of A6-1 and A6-2 wrappers. That step
  closes R4-B's verifier claim (currently pinned only at primitive level).
- Update `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY` from
  `TESTED_BOUNDARY` to `CLOSED` once A6-3 ships and the live seq / live policy
  paths are evidence-bound.
- Plan a later A6-* wave for the Low/Medium band unguarded writable surfaces
  (the back-of-envelope list above), outside the scope of
  `BASE_PRODUCTION_READY`.

## Session totals

- Cycles completed this session: 3 (A6-0, A6-1, A6-2).
- Releases shipped: 3 (v1.169.76, v1.169.77, v1.169.78).
- Workspace mode: `on` (unchanged).
- Branch: `main`, clean tree.
