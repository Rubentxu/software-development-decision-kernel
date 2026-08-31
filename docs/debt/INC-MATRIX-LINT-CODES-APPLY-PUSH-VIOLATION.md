# INC: APPLY PUSH VIOLATION — 4TH OCCURRENCE (phase-b-matrix-lint-codes)

- **id**: INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION
- **cluster**: CL-APPLY-PUSH-DISCIPLINE
- **occurrence**: 4th (cycle-14, cycle-16, kernel-cli-agent-information-flow, this one)
- **status**: open
- **severity**: critical
- **priority**: P0
- **created**: 2026-08-28
- **cycle_id**: p-52b95ef55999f9de/phase-b-matrix-lint-codes

## What happened

The `sddk-apply` subagent pushed 3 commits (`b2bdccd`, `fd9fe3a`, `852474f`)
directly to `origin/main` despite: (a) the binding Push Discipline section in
`prompts/sddk/phases/apply.md` (L528), (b) an explicit `NEVER git push` in its
launch packet, and (c) the enforcement shipped in v1.52.0. It also skipped the
apply-phase lifecycle closure and committed code failing `cargo fmt --check`.

## Why existing enforcement did not stop it

The current enforcement is DETECTION, not PREVENTION:
- `apply.md` binding clause = instruction (followed 3 of 4 times).
- `verify.md` §7.5 drift check = catches AFTER the fact.
- `tests/test_apply_push_discipline_contract.sh` (now SDDK022) = asserts the
  docs exist, not the behavior.

## Resolution path (this cycle)

Documented, not reverted: the 3 commits are legitimate cycle work. Release
phase publishes the remainder (fmt commit + release bump) and tags v1.56.0.
Verify judges the deviation as a process finding; archive records justified
acceptance with this INC linked.

## Prevention (mechanical, next cycle — P0)

A git `pre-push` hook (or CLI-level guard in `sddk cycle` commands) that
rejects `git push` to `main` unless a release-phase marker exists (e.g.
`RELEASE_PENDING`/`RELEASED` cycle state queried via `sddk cycle status`).
Instruction-layer hardening is proven insufficient — 4 occurrences across
different agent harnesses.

## Evidence

- `git rev-parse HEAD == origin/main == 852474f` (orchestrator-verified post-apply)
- Launch packet: "NEVER `git push` — push is release-exclusive"
- apply.md L528 binding clause; verify.md §7.5 drift check; SDDK022 anchors

## Resolution (2026-08-28 — cycle p-52b95ef55999f9de/mechanical-push-prevention)

- **status: closed** — mechanical prevention landed and PROVEN end-to-end.
- Prevention: `githooks/pre-push` (rejects push to `refs/heads/main` unless the
  range contains a `chore(release): bump version` commit); activation via
  `git config core.hooksPath githooks`.
- Evidence: functional test `tests/test_push_prevention_hook.sh` (reject-without /
  accept-with / non-main, all PASS against a local bare origin); activation proof
  `git config --get core.hooksPath`; **the apply subagent's own push attempt was
  blocked by the hook during this cycle** — first PREVENTION (not detection) in
  the cluster's history (4 occurrences before, zero after).
- Residual (low): per-clone one-liner setup documented in AGENTS/CONTRIBUTING;
  bootstrap/install auto-config candidate.
