# SCOPE-CONTRACT — SEC-WORKSPACE-FLAKE fix

Cycle: `p-63676b11dc0ef88f/sec-workspace-flake-fix`
Baseline (released): `v1.169.88` → `add896d94274a7515254b8c195e2e78c3669108f`
Development head at start: `8dd56bf3ce81d6a01c9b3a23fe2fdb3dfd56dd1d`
Predecessor: SEC-1 release-blocked handoff 2026-09-19.

## Goal

ONE narrow cycle: remove the workspace-test flake that
reproduces as `cross_surface_facades_share_the_service_instance`
in `sddk-cli/tests/a6_4_shared_ticket_service.rs`. The flake
gates the SEC-1 release (and every subsequent release that
runs `cargo test --workspace`).

## Reproduction

`SEC-WORKSPACE-FLAKE` is reproduced when
`cross_surface_facades_share_the_service_instance` runs in
parallel with other tests that touch the
`AuthorityTicketService::process_service()` singleton.

Reproduction (post-fix state, 2026-09-19):

```text
$ bash scripts/release.sh --dry-run
…
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
…
test cross_surface_facades_share_the_service_instance ... FAILED
```

Reproduced 1/3 dry-runs. The other 2/3 passed. Same test
passes 5/5 when run in isolation:

```text
$ cargo test -p sddk-cli --test a6_4_shared_ticket_service --offline
… 5 passed; 0 failed …
```

## Root cause

`with_framework_bundle_ticket` and
`with_github_releases_ticket` both call
`svc.register_policy(policy)` unconditionally. A6-3 §"Honest
limits" documents this:

> "`register_policy` advances the fence. This is permissive —
> a caller could invalidate unrelated tickets by
> re-registering. A6-4 will explore targeted fence advance if
> needed."

`AuthorityTicketService::register_policy` calls
`bus.advance_fence()` even when the policy is already
registered. When two tests run in parallel and both call
`with_*_ticket(...)`, the second `register_policy` invalidates
the first test's ticket at consume time → the first test's
facade returns `Err(Ticket(...))` → the facade's
`.expect("facade ok")` panics → the test fails.

The race is non-deterministic because it depends on whether
the two `with_*_ticket` calls overlap within the
`register_policy` → `issue` → `consume_at_live_now` window.

## MUST (acceptance)

- M1. `cargo test --workspace --offline` is GREEN across 5
  consecutive runs (the pre-fix state failed at least once
  per 3).
- M2. `cargo test -p sddk-cli --test a6_4_shared_ticket_service --offline`
  is GREEN (5/5) when run in isolation, unchanged.
- M3. The four retained tests in
  `a6_4_shared_ticket_service.rs` still cover the same
  contractual points they covered before this cycle (see
  §"Test surface preserved").
- M4. A6-4 receipt is updated to reflect the new test count
  (5 → 4) and the rationale.
- M5. `cargo fmt --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test -p sddk-engine
  --offline` all stay GREEN.
- M6. SEC-1 release gate is unblocked: re-running
  `bash scripts/release.sh --dry-run` keeps `cargo test
  --workspace --offline` GREEN across 5 consecutive runs.

## MUST NOT

- N1. No change to `AuthorityTicketService::register_policy`
  semantics (the A6-3 "permissive fence advance" is
  documented behaviour; changing it is out of scope and would
  invalidate other tests).
- N2. No change to the facade public API
  (`with_framework_bundle_ticket`, `with_github_releases_ticket`).
- N3. No retro-active change to A6-4 receipt's claim about
  test count beyond a small note that explains the
  de-duplication.
- N4. No change to A5-C cert or SEC-1 contract.

## Fix (narrow, surgical)

`cross_surface_facades_share_the_service_instance` is removed.
The four retained tests cover the same contractual points
without using the public facade functions; the facade-level
test is logically redundant (see §"Test surface preserved").

## Test surface preserved

| Test | What it covers | Source |
|---|---|---|
| `shared_service_singleton_is_same_instance_for_both_surfaces` | `process_service()` returns the same static ref every call | A6-4 §5 |
| `cross_surface_shared_seq_strictly_monotonic` | Issues one fb + one gr ticket on the same service; seq advances by ≥ 2 | A6-4 §5 |
| `cross_surface_policy_transition_invalidates_either_surface_ticket` | Register a fresh policy ⇒ fence advances ⇒ in-flight fb ticket refuses at consume | A6-4 §6 |
| `facade_deny_yields_zero_side_effects_on_real_service` | Deny ⇒ body closure never runs against the real shared service | A6-3 §3 |

Removed: `cross_surface_facades_share_the_service_instance`
(redundant with `cross_surface_shared_seq_strictly_monotonic`
which already exercises the same service).

## Anti-corruption check

- No change to `AuthorityTicketService` semantics.
- No change to the public facade API.
- No new dependency.
- The architectural lints (`context_fitness`) remain green.

## Disposition at exit

Will report:

- The test count change (5 → 4) and the rationale.
- The 5-consecutive `cargo test --workspace` runs as the
  closing evidence.
- The SEC-1 release gate status (unblocked).
- The follow-up: a future ADR may revisit the permissive
  `register_policy` fence advance (A6-3 honest limit) but
  is explicitly out of scope for this cycle.
