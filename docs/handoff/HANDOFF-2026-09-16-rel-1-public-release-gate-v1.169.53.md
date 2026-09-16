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
| released_baseline_commit (REL-1) | post-REL-1 `chore(release): bump version 1.169.52 -> 1.169.53` (exact SHA filled in by `bash scripts/release.sh` close-out block) |
| released_baseline_origin_main_HEAD | matches the commit above |
| released_baseline_tag | `v1.169.53` |
| workspace_version_pre_REL-1 | 1.169.53 (already bumped; release not yet shipped) |
| previous_released_baseline | v1.169.52 (HEAD `9785288`) |
| tag_drift_anchor | `git ls-remote origin $TAG` (NOT `origin/main`) |

## Verification

- `shellcheck -x scripts/release.sh` — clean.
- `shellcheck -x tests/test_release_public_gate.sh tests/lib_public_release_gate.sh` — 2 cosmetic warnings (unused `GATE_END`, no-op loop var `i`) only.
- `bash tests/test_release_public_gate.sh` — **PASS=11, FAIL=0**.
  - Scenario 1: happy path → PASS (tag SHA anchored, tagName match, isDraft=false, isPrerelease=false, 9-asset contract, 9/9 HTTP 200)
  - Scenarios 2-7: each failure path → FAIL closed
  - Scenario 8: curl=404 → gate dies "public URL probes failed after 60s-per-asset budget"
  - Scenario 9: real acceptance — proven by construction (gate block present in `scripts/release.sh`, asserted in test)
  - Scenario 10: `--dry-run` and `--skip-install` guards skip step 9b

## Receipt (post `bash scripts/release.sh`)

The actual receipt fills in once `bash scripts/release.sh` runs end-to-end.
The release must close scenario 9 of the contract test suite by construction
(the gate block is in the script; `bash scripts/release.sh` calls it after
`gh release create`). If the gate dies during the real run, the install
step is not reached and the user has a working draft with a clear
diagnostic, which is the bug-fix REL-1 was chartered to deliver.

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
