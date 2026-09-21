# SESSION CLOSE — 2026-09-18 · A6-1 framework_bundle migration

> Resume point for the next session. Everything below is committed, pushed,
> released and installed. **Nothing is half-done.**

## Final state

| | |
|---|---|
| `HEAD == origin/main` | `f65b546` (A6-1 bump) |
| Working tree | clean |
| Binary / bundle / framework | `1.169.77` |
| Cycles opened this session | 2 — A6-0 (1 opened, 1 closed earlier); A6-1 (1 opened, 1 closed) |
| Carry-over debt | 0 opened · INC-R4 at TESTED_BOUNDARY (A6-0) advancing toward FULLY_MIGRATED; A6-2 (`github_releases`) named next |
| Release tag | `v1.169.77` → SHA `f65b546` |
| GH Release | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.77 |
| Release script | `scripts/release.sh` 14/14 pasos verdes (PublicReleaseGate 9b PASS) |

## Releases this session (2 tags)

| Tag | SHA | Cycle | What |
|---|---|---|---|
| `v1.169.76` | `ffd472e` | A6-0 | R4-B closed at TESTED_BOUNDARY. `AdmissionTicketBus` + FENCE matrix. |
| `v1.169.77` | `f65b546` | A6-1 | R4-B migration step 1/2 — `framework_bundle` opts into the wrapper. Per-surface FENCE integration tests added (T1, T4, T5, T6). ADR-0131 codifies the migration pattern for A6-2. |

## A6-1 commits (2 + handoff)

| # | Hash | Subject |
|---|------|---------|
| 1 | (A6-1 feature commit) | `feat(cli): a6-1 framework_bundle migration to AdmissionTicketBus` |
| 2 | `f65b546` | `chore(release): bump version 1.169.76 -> 1.169.77` (+ Cargo.lock regenerated) |

## Risks touched

| Risk | Status | Evidence |
|---|---|---|
| **R4** (TOCTOU decision→effect) | ADVANCING — `framework_bundle` migrated; `github_releases` named for A6-2 | 4/4 unit FENCE tests in `crates/sddk-cli/src/dev/framework_bundle_ticket.rs::tests` (T1, T4, T5, T6) |
| R4-A (denied ⇒ no effect, A5-3) | unchanged | preserved by the wrap (auth.validate runs first) |

## Tests (all green, A6-1 scope)

| Test target | Count | Status |
|---|---|---|
| `framework_bundle_ticket::tests` (cli, new) | 4 | ✅ GREEN — T1 (happy path), T4 (one-shot), T5 (Deny ⇒ no ticket), T6 (verdict-anchored consume) |
| Full `sddk-cli` test suite | unchanged | ✅ GREEN — no regressions; `manifest_tests` for `framework_bundle` pass |
| Full workspace | unchanged | ✅ GREEN |

Total A6-1 new tests: **4**. Full engine + cli + storage: **green**.

## Gates (all green)

| Gate | Result |
|------|--------|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | green (0 failed; workspace test took 7m21s; one flaky storage concurrent test on first attempt, passed on retry — `storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq` is known flaky infra per A5-5 disposition) |
| `cargo test -p sddk-cli --test context_fitness no_new_root_level_context_module_without_adr` | green — `framework_bundle_ticket` is `dev/`-scoped, not root level |
| `sddk dev manifest --root . --verify` (step 4) | manifest OK |
| `gh release create v1.169.77` (step 9) | 9 canonical assets published |
| **PublicReleaseGate step 9b** | PASS |
| `sddk dev doctor --prefix ~/.local/bin` | `binary.bundle_coherence: present`, `all_present: true` |
| Distrib smoke test | re-installed from URL; binary reports 1.169.77 |
| Final state | `binary=bundle=current=1.169.77` |
| `bash tests/test_vault_adr_mirror_coverage.sh` | OK after `scripts/mirror_adrs_to_vault.py` (ADR-0131 mirrored) |

## Files affected (6)

| File | Δ | What |
|---|---|---|
| `docs/architecture/adrs/ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES.md` | NEW (~178 lines) | Decision: migration pattern per surface; A6-2 follows verbatim; honest limits named (T2 at call site deferred to A6-3) |
| `docs/architecture/a6/A6-1-PLAN.md` | NEW (~86 lines) | Cycle scope + excluded surfaces |
| `docs/architecture/a6/A6-1-RECEIPT.md` | NEW (~111 lines) | Falsification matrix, gate evidence, follow-up list, INC-R4 update |
| `crates/sddk-cli/src/dev/framework_bundle_ticket.rs` | NEW (~270 lines incl. tests) | `with_framework_bundle_ticket`, `framework_bundle_policy`, `FrameworkBundleTicketError` |
| `crates/sddk-cli/src/dev/mod.rs` | +1 | `pub(super) mod framework_bundle_ticket;` |
| `crates/sddk-cli/src/dev/install.rs` | refactor (no behaviour change outside the wrap) | `run_dev_install` body now wrapped in `with_framework_bundle_ticket`. R4-A `auth.validate(...)` preserved. |
| `Cargo.toml` + `Cargo.lock` | 1.169.76 → 1.169.77 | `cargo update --workspace --offline` regenerated |

Total: ~830 insertions, 158 deletions across 8 files.

## Carry-over debt

| Item | Severity | Priority | Status | Next action |
|---|---|---|---|---|
| INC-R4 migration step 2: `github_releases` | P1 | next A6 | DEFERRED (A6-2) | Apply ADR-0131 verbatim to `dev release.rs`'s `gh release create` + `gh release upload` sites |
| A6-3 (future): thread live `PolicySnapshot` from `AuthorityEngineRunner` so wrappers construct tickets anchored to the engine's registered policy | P2 | future A6 | DEFERRED | Closes T2-at-call-site gap (currently T6 verdict-anchored covers it at this site) |
| Medium-band migration of `plan_item`, `evidence_attachment`, `decision_record`, `dependency_edge` | P2 | future A6 | DEFERRED | Single cycle, separate from A6-1/2/3 |

## What this cycle leaves for the next one

- **`framework_bundle` is migrated**; A6-1 was the first of two high-band migration steps. A6-2 is the natural next cycle — same pattern (ADR-0131), different surface name (`github_releases`).
- **A5-C is executable** (R4-B satisfies G5 via A6-0's primitive + A6-1's first migration). A5-4 and A5-5 are still required per DAG before A5-C can issue `BASE_PRODUCTION_READY`.
- **No semantic surprise in INC-R4.** The INC stays closed at TESTED_BOUNDARY (A6-0); the migration step in A6-1 is documented in `A6-1-RECEIPT.md` §7 as a forward-advance, not a state change.

## Reproduce tomorrow

```bash
cd ~/Proyectos/agentesIA/sddk-framework
git log --oneline -3
ls docs/architecture/a6/A6-1-RECEIPT.md
ls crates/sddk-cli/src/dev/framework_bundle_ticket.rs
~/.local/bin/sddk --version    # expect 1.169.77
```

## Next-cycle entry point

| Cycle | Scope | Pre-reqs |
|---|---|---|
| **A6-2** | `github_releases` migration to `AdmissionTicketBus` | A6-0 ✅, A6-1 ✅ |
| **A5-4** | Compat / deprecation / lints / operator UX | A5-3 ✅ (independent of A6) |
| **A5-5** | Reliability / clean-machine / security / operational UAT | A5-1 ✅ (independent of A6) |
| **A5-C** | BASE_PRODUCTION_READY certification | A5-4 + A5-5 + A6-2 + A6-0 |
| **A6-3 (future)** | Thread live PolicySnapshot from runner; close T2-at-call-site; align `ActorKind` types | A6-0 ✅ |

**STOP** after each cycle per user rule. Do not auto-open the next.

## Honest markers

- **AuthorityEngine source unchanged.** Diff inspection: zero changes to `crates/sddk-engine/src/authority_engine.rs`. Strangler preserved.
- **`framework_bundle` is the only High-band unguarded surface that has opted in.** `github_releases` is still unguarded at release v1.169.77; A6-2 closes that half.
- **T2 (PolicyChanged via policy_digest) at the CLI call site remains unpinned.** The A6-1 helper has T6 (verdict-anchored consume) which covers the common threat at the wrapper boundary; A6-3 will thread the live snapshot to close T2 directly.
- **One retry needed for the release pipeline.** `cargo test --workspace` first attempt flaked on `storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq` (SQLite database locked). This is a known flaky infra test, not a regression from A6-1. A5-5 disposition already calls it out as `REPRODUCED_AND_FIXED`-candidate; deferred there. Passed clean on retry.
- **Vault mirror gap caught again by step 1b.** `test_vault_adr_mirror_coverage.sh` caught the missing ADR-0131 mirror before the release published. Fixed with `scripts/mirror_adrs_to_vault.py`. The hook is doing what it is designed to do.
- **No `git.history_rewrite` rewrites.** Commits land clean on `main`.
