# Handoff — REL-1 Public-Release Gate (FU-A4-4A-REL-1) + v1.169.53 release

- **Cycle:** `p-63676b11dc0ef88f-rel-1-public-release-gate`
- **Closed:** 2026-09-16 (single budget; gate infrastructure only).
- **Release shipped:** v1.169.53 (REL-1 active).
- **Predecessor handoff:** `docs/handoff/HANDOFF-2026-09-16-a4-4a-intent-universal-concern-foundation-v1.169.52.md`
  (now with a correction addendum citing the SHA `9785288` instead of "tracks main").

## Goal (single budget)

Close `FU-A4-4A-REL-1` (raised at the close-out of v1.169.52 / A4-4a because
v1.169.52 was tagged and the GH release published as `isDraft=false` BUT
without a `PublicReleaseGate` between publish and local install: a
silent-draft bug existed in concept). Add a fail-closed gate between step 9
(`gh release create`) and step 10 (`install --version $TAG`) of
`scripts/release.sh` so:

1. **A draft release cannot be installed from.** Step 9b re-checks
   `gh release view` on the live API: `isDraft=false`, `isPrerelease=false`.
2. **A release with the wrong tag SHA cannot be installed from.** The tag's
   `git ls-remote origin $TAG` SHA must equal the local HEAD (tag SHA
   anchoring, not `origin/main`).
3. **A release with the wrong 9-asset contract cannot be installed from.**
   The asset set must be exactly the 9 canonical names; missing or extra
   assets fail closed.
4. **A release whose assets are not yet HTTP 200 from the public CDN cannot
   be installed from.** Each asset URL is probed (6 × 10 s budget per
   asset, ~9 min worst-case).

## Out of scope (anti-encroachment)

No engine/CLI/lens/paradigm_lens/architectural_contract changes. Single
budget = release-hygiene infrastructure. See
`.sddk/cycles/p-63676b11dc0ef88f-rel-1-public-release-gate/spec.md`
(MUST_NOT §7 — names the forbidden surfaces explicitly).

## What changed (ground-truth diff)

| File | Δ | Purpose |
|---|---|---|
| `scripts/release.sh` | +108 / -0 | New step 9b gate block (between step 9 and step 10); `jq` added to step 0 preflight; markers `# >>> REL-1 public-release gate begin >>>` / `# <<< REL-1 public-release gate end <<<` for test extraction |
| `tests/lib_public_release_gate.sh` | new, executable | Extracted gate logic as `run_public_release_gate()` with the same helpers (`step/ok/warn/die/require`) — kept in sync with the script via the markers |
| `tests/test_release_public_gate.sh` | new, executable | 10 contract tests using mocked `gh`/`git`/`curl` in `$tmp` on PATH; 1 static-check on dry-run/skip-install guards |
| `docs/RELEASING.md` | +39 / -0 | New "Public-release gate" paragraph at top of doc; step 7b inserted in the manual fallback matching the script |
| `AGENTS.md` | +21 / -9 | Pipeline now 14 pasos (was 13); step 9b row added with references; flags block updated |
| `Cargo.toml`, `Cargo.lock` | 1.169.52 → 1.169.53 | version bump (chore(release) commit) |

Total: ~5 file additions, ~170 line insertions.

## Achievements (what we now have that we did not before)

1. **Step 9b — `PublicReleaseGate`.** Tag SHA anchored via
   `git ls-remote origin $TAG` (NOT `origin/main`), `isDraft=false`,
   `isPrerelease=false`, 9-asset contract, per-asset HTTP probe up to
   60 s budget each. Sourced from `tests/lib_public_release_gate.sh`
   for testability; mirrors the script exactly.
2. **Contract tests.** `tests/test_release_public_gate.sh` pins the
   gate against 10 acceptance anchors from the spec (PASS=11/FAIL=0
   on the current workspace). All scenarios are mocked; the real
   acceptance is the release run itself (scenario 9 by construction).
3. **Dry-run/skip-install guards.** Step 9b is short-circuited when
   `DRY_RUN=1` or `SKIP_INSTALL=1` (no point probing a release that
   doesn't exist, or skipping the install that the gate guards).
4. **Docs.** `docs/RELEASING.md` and `AGENTS.md` updated; step counter
   bumped from 13 to 14; manual fallback lists the gate inline.
5. **Correction addendum on A4-4a handoff.** Pins the v1.169.52 SHA
   `9785288` explicitly instead of "tracks main", per the 6-rule
   SHA discipline.

## Pinned reference points (SHA discipline)

| Identifier | Value |
|---|---|
| released_baseline | v1.169.53 |
| released_baseline_commit (REL-1) | `ccecdc723355b0ecc7e037f2266f465165dc59dc` (chore(release): bump version 1.169.52 -> 1.169.53) |
| released_baseline_origin_main_HEAD | `ccecdc723355b0ecc7e037f2266f465165dc59dc` (matches) |
| released_baseline_tag | `v1.169.53` → `ccecdc723355b0ecc7e037f2266f465165dc59dc` (per `git ls-remote origin v1.169.53`) |
| workspace_version_pre_REL-1 | 1.169.53 |
| previous_released_baseline | v1.169.52 (HEAD `9785288`) |
| tag_drift_anchor | `git ls-remote origin $TAG` (NOT `origin/main`) |

> **Backfill (A4-4aR chore(reelease) — 2026-09-16).** Placeholder rows in
> this handoff were backfilled onto origin when v1.169.54's release commit
> landed. See `.sddk/followups/a4-followups.md` line `FU-REL-1-BACKFILL`
> disposition: **closed 2026-09-16 (backfilled onto origin as part of the
> v1.169.54 chore(release) commit).** Durable local receipts (gitignored)
> remain at `.sddk/cycles/p-63676b11dc0ef88f-rel-1-public-release-gate/{release-receipt.json,live-receipts.txt,archive-manifest.md}`.

> **Why deferred, not at chore(release) 1.169.53.** At chore(release)
> 1.169.53 the release had not yet shipped; the live SHA `ccecdc7…` was
> not knowable until `bash scripts/release.sh` ran. The pre-push hook
> (`githooks/pre-push`) blocks doc-only follow-up commits (no
> `chore(release)` subject, no Cargo.toml bump), so the backfill had to
> wait for the next legitimate `chore(release)` — which A4-4aR provided
> via the 1.169.53 → 1.169.54 bump.

## Verification

Distinct from re-running the acceptance suite: each check below targets a
specific property the gate MUST hold. A green bar here means **that
property was attempted to be falsified and held**, not that the existing
suite happened to pass.

| Property the gate MUST hold | Check that falsifies it | Result |
|---|---|---|
| Bash source lints clean under `-x` | `shellcheck -x scripts/release.sh` | clean |
| Heredoc embedding `\`gh release view ... --json ...\`` in tests does not invoke real `gh` via backtick command substitution | `shellcheck -x tests/test_release_public_gate.sh tests/lib_public_release_gate.sh` | 2 cosmetic warnings only (`GATE_END` unused, loop var `i` unused); no SC issued for the heredoc backtick risk |
| All 7 failure paths close the gate (not silently PASS) | `bash tests/test_release_public_gate.sh` scenarios 2-7 | each die'd with the precise reason; PASS=11 FAIL=0; exit 0 |
| HTTP 404 from any public asset fails closed (not retried forever) | `bash tests/test_release_public_gate.sh` scenario 8 with curl mock returning 404 | `public URL probes failed after 60s-per-asset budget: …`, gate rc=1 |
| Dry-run / skip-install skip the gate (no point probing a release that doesn't exist) | `bash tests/test_release_public_gate.sh` scenario 10 grep on `DRY_RUN`/`SKIP_INSTALL` strings in release.sh | both `"skipping step 9b"` strings present |
| Tag SHA anchoring does NOT silently pass when HEAD and tag diverge | `bash tests/test_release_public_gate.sh` scenario 7 (`expected_sha=abc123` vs `tag_remote_sha=deadbeef`) | `tag v1.169.53 SHA drift: HEAD=abc123 tag=deadbeef — refusing to install`, gate rc=1 |
| The 9-asset contract is exact (no missing, no extra) | `bash tests/test_release_public_gate.sh` scenarios 4 (missing) and 5 (extra) | both die closed; comm-based diff catches both directions |
| Live step 9b on the real GH API does what it claims | `bash scripts/release.sh` real run, captured in `/tmp/release-1.169.53.log` | 6/6 assertions PASS with `ccecdc7…` SHA; receipt in archive-manifest.md |

**Scope-statement fact (not verified by re-running the suite):** scenario 9
asserts `grep -q 'public-release gate PASS' scripts/release.sh`. That
substring is a textual marker — it confirms the gate *block* exists in
the script. It does NOT confirm the gate ran and passed for v1.169.53; that
property is verified by the live `/tmp/release-1.169.53.log` excerpt in
the archive-manifest, captured separately.

## Receipt (post `bash scripts/release.sh`)

Live release ran end-to-end. Tag created at `ccecdc7…`, step 9b ran on
the real GH API and passed all six assertions, install + doctor reported
green.

| Property | Source of truth | Snapshot |
|---|---|---|
| Tag `v1.169.53` SHA | `git ls-remote origin v1.169.53` | `ccecdc723355b0ecc7e037f2266f465165dc59dc` |
| HEAD == origin/main | `git rev-parse HEAD` and `git rev-parse origin/main` | both `ccecdc7…` |
| Release isDraft | `gh release view v1.169.53 --json isDraft` | `false` |
| Release isPrerelease | `gh release view v1.169.53 --json isPrerelease` | `false` |
| Asset count | `gh release view v1.169.53 --json assets --jq '.assets \| length'` | `9` |
| Asset names | same call, `assets[].name` | 9 canonical names listed in archive-manifest |
| Local binary | `sddk --version` | `sddk 1.169.53` |
| Bundle coherence | `sddk dev doctor --prefix /home/rubentxu/.local/bin` | `binary.bundle_coherence: present` + `all_present: true` |
| Live step 9b output | `/tmp/release-1.169.53.log` (matches `==> 9b/14`) | 7 lines of `✓` then `public-release gate PASS` |
| Release URL | `gh release view v1.169.53 --json url` | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.53 |

The SHA + live-receipt rows above are deferred backfill candidates for
the next A4-4b `chore(release)` commit (pre-push hook blocks doc-only
follow-ups; durable ground truth already lives in
`.sddk/cycles/p-63676b11dc0ef88f-rel-1-public-release-gate/archive-manifest.md`).

## Followups (no longer open)

- **FU-A4-4A-REL-1** — closed by this cycle (gate infrastructure in place).

## Next step (NOT auto-started — STOP per 6-rule protocol)

The pre-REL-1 user direction queued a follow-on cycle: **A4-4b — generic
`AlignmentLens` kernel** (`AlignmentLens` abstraction/kernel; `paradigm_lens`
migration stays A4-4M). The user must explicitly green-light A4-4b before it
opens. Do not auto-open.

## Relevant files (durable references)

- `scripts/release.sh` — gate block between step 9 and step 10 (markers `# >>> REL-1 … >>>` / `# <<< … <<<`)
- `tests/lib_public_release_gate.sh` — extracted gate function (kept in sync with the script)
- `tests/test_release_public_gate.sh` — 10-scenario contract suite
- `docs/RELEASING.md` — Public-release gate paragraph + manual fallback step 7b
- `AGENTS.md` §8 — Pipeline 14 pasos; step 9b row
- `.sddk/cycles/p-63676b11dc0ef88f-rel-1-public-release-gate/spec.md` — scope contract
- `.sddk/cycles/p-63676b11dc0ef88f-rel-1-public-release-gate/archive-manifest.md` — durable close-out (filled in post-release)
- `.sddk/followups/a4-followups.md` — FU-A4-4A-REL-1 disposition: closed
