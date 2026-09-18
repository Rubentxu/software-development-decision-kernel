# A6-2 RECEIPT — `github_releases` wrapped by AdmissionTicketBus

## Scope

`run_release_apply` (Forge route) — the chain
`plan_release(input, &forge)` → `apply_release(gateway, plan, forge, version_lockstep_passed)`
runs inside a single `AdmissionTicketBus` issue + consume gate.

## Why this surface

`apply_release(...)` performs three forge side effects in series:
1. `CreatePr` (HTTP POST `/repos/{owner}/{repo}/pulls`)
2. `MergePr` (HTTP PUT `/repos/{owner}/{repo}/pulls/{n}/merge`)
3. `CreateRelease` (HTTP POST `/repos/{owner}/{repo}/releases`)

These three steps belong to the High band (forge-external surface). They are
not retried automatically and they mutate state outside our control. Without
R4-B, a decide-then-act race (decide at `plan_release` time → act later after
the policy was tightened or the actor's lease expired) was open. The A6-2
ticket closes that window for the forge path the same way A6-1 closes it for
the local install path.

## Changes

### `crates/sddk-cli/src/dev/github_releases_ticket.rs` (new, 230 LOC)

- `with_github_releases_ticket(actor, target_id, body)` — wraps `body()` so
  it runs ONLY after a successful ticket issue + consume. Deny ⇒ no ticket,
  body never runs. Returns `GithubReleasesTicketError` (typed).
- `github_releases_policy()` — low-risk `PolicySnapshot` permitting
  `ActionKind::CliRelease` for a `System` actor with `cli.execute`
  capability. Engine-flavoured `Actor` (not domain), so the helper accepts
  the actor as-is and does not enrich capabilities.
- 4 inline `#[test]` verdicts:
  - `fence_t1_happy_path_runs_body_and_consumes_ticket` (R4-B body runs
    iff ticket consumed)
  - `fence_t4_one_shot_ticket_does_not_reissue_within_a_bus`
  - `fence_t5_deny_yields_no_ticket_and_body_does_not_run`
  - `fence_t6_ticket_with_swapped_policy_is_rejected_at_consume`

### `crates/sddk-cli/src/dev/mod.rs`

- `+pub(super) mod github_releases_ticket;` (line 21a)

### `crates/sddk-cli/src/release_cmd.rs`

- New imports at line 21:
  `use crate::dev::github_releases_ticket::{with_github_releases_ticket, GithubReleasesTicketError};`
- Forge branch of `run_release_apply` (line ~840) wraps the body that calls
  `apply_release(...)` in the helper. The `actor` passed is a `System` actor
  with `service="sddk-cli/release-apply"` and capability `cli.execute`.
- Errors mapped:
  - `Denied(msg)` → `anyhow::anyhow!("github_releases ticket denied: {msg}")`
  - `Ticket(err)` → `anyhow::anyhow!("github_releases ticket error: {err:?}")`
  - `Apply(err)` → propagated as-is.

## Honest limits (carried from ADR-0131)

1. `current_seq = 0` is hardcoded at the consume site. The primitive-level
   T1–T5 fence matrix is pinned in
   `crates/sddk-engine/tests/a6_0_admission_tickets.rs`. Wiring the live
   monotonic seq from `AuthorityEngineRunner` is **A6-3** work.
2. `policy_digest` consumed is the policy snapshot's own digest (no
   override / swap step inside the call site). T2 of the FENCE matrix is
   pinned at primitive level; the call site accepts whatever digest the
   policy has.
3. The helper does NOT enrich the actor's capabilities — the caller is
   responsible. This was deliberate (allows the test to construct an
   actor that will be Denied).

## Tests run (gate profile for the change scope)

```text
cargo build -p sddk-cli                      # ok, no warnings
cargo test -p sddk-cli --lib github_releases_ticket
  → 4 passed; 0 failed
cargo test -p sddk-engine --lib authority_admission_ticket
  → 8 passed; 0 failed  (regression: A6-0 primitive)
cargo test -p sddk-cli --lib framework_bundle
  → 5 passed; 0 failed  (regression: A6-1 sibling)
cargo clippy -p sddk-cli --all-targets -- -D warnings
  → exit 0
cargo fmt --all -- --check
  → exit 0
```

`bash tests/test_vault_adr_mirror_coverage.sh` → 39 ADRs mirrored,
idempotent.

## Out of scope (deferred)

- Wiring `current_seq` from live `AuthorityEngineRunner` — A6-3.
- Wiring live `PolicySnapshot` from `AuthorityEngineRunner` (vs. the local
  helper policy) — A6-3. Honest disclosure at the call site.
- Other Low / Medium band unguarded writable surfaces (R4 back-of-envelope:
  `policy_snapshot`, `gate_receipt`, `cycle_lock`). These are scoped in a
  later A6-* wave and are NOT part of `BASE_PRODUCTION_READY`.

## Verification

`apply_release(...)` body inside `run_release_apply` is now gated by an
admission ticket. If `admit` denies or the ticket fails to consume, no
HTTP calls reach GitHub. The forge path remains an unsafe-by-default
fail-closed action — it has been formally tagged High band by
INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY and is now ticket-protected.

## Linkage

- ADR-0132 (`GITHUB-RELEASES-MIGRATION-PATTERN`)
- ADR-0131 (`MIGRATION-PATTERN-FOR-WRITABLE-SURFACES`) — sibling pattern.
- ADR-0130 (`FENCED-ADMISSION-TICKETS`) — A6-0 primitive authority.
- INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY — back-of-envelope (closing
  toward `TESTED_BOUNDARY`).
