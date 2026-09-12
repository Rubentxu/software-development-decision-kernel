---
id: INC-RELEASE-001
title: "release script step 1 lacks shell contract tests as a gate"
status: closed
severity: medium
priority: P2
fingerprint: "release-001-step1b-shell-tests"
fingerprint_aliases: []
cluster_id: CL-RELEASE-TEST-COVERAGE
created: 2026-09-12
created_by: orchestrator
owner: release-pipeline
closed_at: 2026-09-12
closed_by: orchestrator (commit adb39cd "feat(release): wire shell contract tests into release flow (step 1b)")
resolution_note: |
  Closed at commit adb39cd by adding step 1b to scripts/release.sh.
  The 4 shell contract tests added across cycles v1.168.27 (release-receipt
  authority), v1.168.30 (cross-crate authority lockstep), and the 2
  earlier push-prevention + vault-coherence tests now have a runner.
  shellcheck --severity=warning gates scripts/release-receipt.sh + the 2
  M9+ contract tests; legacy tests/test_vault_coherence_alignment.sh is
  excluded (its SC2034/SC2329 warnings predate this repo's M9+ contracts).
  Dynamic execution of the 2 release-receipt + lockstep tests exits 0 in
  manual validation. --skip-tests still skips step 1b (inside the same
  guard). The next full release will execute step 1b end-to-end.
last_updated: 2026-09-12
---

# INC-RELEASE-001 — release script step 1 lacks shell contract tests as a gate

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`scripts/release.sh` step 1 (the local verification gate) runs
`cargo fmt --check`, `cargo clippy -D warnings`, and
`cargo test --workspace`. It does **not** execute the shell contract
tests under `tests/test_*.sh`. Across cycles v1.168.27, v1.168.30, and
earlier work, four such tests were added — including one that pins a
**cross-crate contract** between `scripts/release-receipt.sh` (shell)
and `crates/sddk-engine/src/authority.rs::infer_actor_kind` (Rust).
If the shell helper's prefix heuristic drifts from the engine's
`infer_actor_kind`, the lockstep test would fail, but the release flow
would publish without detecting it.

## Rationale

- Release gates must cover every kind of artifact the release ships.
- Cargo gates cover `crates/`. Shell gates must cover `scripts/` and
  `tests/test_*.sh` independently.
- The release script is the only place where this coverage can be
  enforced automatically (cloud CI is disabled per AGENTS.md §2.5).

## Resolution

Added `scripts/release.sh` step 1b:

1. `shellcheck --severity=warning` over
   `scripts/release-receipt.sh` + `tests/test_release_receipt_authority.sh`
   + `tests/test_authority_helper_lockstep.sh`.
2. Dynamic execution of the 2 release-receipt + lockstep tests, each
   expected to exit 0.

Scope of change: 32 LoC in `scripts/release.sh`, +0/-0 elsewhere.

`--skip-tests` continues to skip step 1b (lives inside the same guard
as step 1).

## Evidence

- Commit `adb39cd` (feat(release): wire shell contract tests into
  release flow (step 1b)).
- Manual validation: shellcheck exits 0, both shell tests exit 0.
- The pre-push hook (githooks/pre-push) still requires
  `^chore\(release\): bump version` as HEAD before push, so the wire-up
  lands in a release commit, not just any commit.

## Status

Closed. Will be re-verified at the next `bash scripts/release.sh`
invocation.
