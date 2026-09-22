---
id: ADR-0142-RELEASE-SCRIPT-SEMVER-CORRECTNESS
status: accepted
supersedes_history: false
adopted_at: 2026-09-22
adoption_cycle: p-63676b11dc0ef88f/c4-release-cut
package_local_id: null
package_source: null
accepted_at: 2026-09-22
accepted_by_cycle: p-63676b11dc0ef88f/c4-release-cut
superseded_by: []
related_adrs:
  - ADR-0097-COMMON-REVISION-SUBSTRATE
stale_after: 2027-09-22
---

# ADR-0142 — Release script SemVer-correctness via `release-bump.sh` integration

- **Status:** accepted
- **Date:** 2026-09-22T11:40:00Z
- **Cycle:** C4 closure (post-publish)
- **Supersedes:** — (none — this is a new decision)
- **Refs:**
  - `docs/roadmap/receipts/c4-release-pending/SEMVER-MISMATCH.md` (incident)
  - `scripts/release.sh` step 2.5 (the new code path)
  - `scripts/release.sh` step 9b asset contract (the fix)
  - `scripts/release-bump.sh` (existing helper)
  - `docs/architecture/CONTRIBUTING-SEMVER.md` §4.3 (mechanics)

## Context

SDDK has two version concepts:

| Concept | Where | SemVer-strict? |
|---|---|---|
| Workspace version | `[workspace.package] version` in `Cargo.toml` | **No** (ceremonial per-push pointer, bumped by pre-push hook) |
| Release tag | Git tag `vX.Y.Z` on GH Releases | **Yes** |

The pre-push hook (AGENTS.md §2.5 contract) requires every source-touching
commit to bump the workspace version, so the workspace version drifts
roughly +1 per push range and is **not a SemVer indicator**.

`scripts/release-bump.sh` (existing) correctly computes the next SemVer
release tag from the conventional commits accumulated since the last
published tag (lines 51-63: feat → minor, fix → patch, BREAKING →
major). Its `--dry-run` reports:

```
release bump: v1.169.122 -> v1.170.0 (minor)
```

`scripts/release.sh` (the canonical release pipeline) **reads the workspace
version verbatim** from `Cargo.toml` and uses it as the release tag (line
274: `TAG="v$VERSION"`). So the released tag would be e.g. `v1.169.152`
instead of the SemVer-correct `v1.170.0`.

### Incident (2026-09-22T11:00Z)

Operator authorized `bash scripts/release.sh` after session-11 had
accumulated six `feat(c3*):` commits (C3b-f) plus one ADR-0141
production-code commit. The script published `v1.169.152` (workspace
literal), marking it as Latest on GH Releases. The correct tag was
`v1.170.0` (minor).

### Decision

The release pipeline **must compute the tag from the conventional commits
since the last published tag**, not from the workspace version. The
workspace version remains authoritative for the **release admission
check** (`scripts/lib/release_admission.sh`), which compares `[workspace.
package] version` HEAD vs HEAD^; that check is independent of the tag.

Two changes are required:

1. **Step 2.5** — invoke `scripts/release-bump.sh --dry-run`, parse the
   `new tag:` line, override the local `TAG` variable if the computed
   tag differs from the workspace-derived tag. The workspace `VERSION`
   remains authoritative for admission + BUNDLE.toml anchor.

2. **Step 9b asset contract** — the canonical asset basenames use
   `sddk-${TAG}-...`, not `sddk-v$VERSION-...`. Before this ADR, the
   contract check (lines 643-653) hardcoded `v$VERSION` while the actual
   uploaded assets used `$TAG`. After step 2.5, `VERSION` (e.g.
   `1.169.154`) and `TAG` (e.g. `v1.170.0`) may diverge; both must use
   `$TAG` so the 9-asset contract check passes.

### Operator override

`scripts/release-bump.sh --force-version X.Y.Z` continues to bypass the
calculation. Use when the auto-calculation is wrong (e.g. hidden
breaking changes not flagged in commit bodies). This override is
**honest**: it bypasses the auto-bump but the script reports
`release bump: <last> -> <forced> (forced)` so the operator decision is
visible in the log.

## Consequences

### Positive

- Released tags now correctly match the SemVer of the commits they
  include. Users consuming tags see `v1.170.0` (minor bump) for a range
  with 6 `feat():` commits, not `v1.169.152` (the ceremonial
  per-push-counter).
- The two-version model documented in `CONTRIBUTING-SEMVER.md` now has
  mechanical support: workspace version is a push-range pointer, release
  tag is the SemVer-bumped outcome.
- Future releases require no manual SemVer reasoning — the script does
  it deterministically from conventional commits.

### Negative

- A new dependency on `scripts/release-bump.sh` inside
  `scripts/release.sh`. If release-bump.sh changes its output format
  (e.g. drops the `new tag:` line), step 2.5 will fall back to the
  workspace-derived TAG with a warning. This is the correct
  fail-soft behavior.
- Workspace version (`1.169.155`, `1.169.156`, ...) and release tag
  (`v1.170.0`) are now **explicitly divergent in the same release
  artifact**. Users reading `sddk --version` see the workspace version
  (which is the build identity); users reading the GH tag see the
  SemVer-bumped identity. Both are correct, just different roles. The
  drift is documented in
  `docs/roadmap/receipts/c4-release-pending/SNAPSHOT.md`.

### Neutral

- The release admission contract is unchanged: a real workspace-version
  bump is still required for `release_admission_check` to accept.
- `MANIFEST.sha256` regeneration and `BUNDLE.toml` injection continue to
  use the workspace version for the `bundle.version` field. This is
  intentional: the bundle is the build artifact identity, and the
  build identity is the workspace version.

## Validation

The fix was validated end-to-end by:

1. Yanking the wrongly-tagged `v1.169.152` release (`gh release delete`
   + `git push origin :refs/tags/v1.169.152`).
2. Committing the step-2.5 + step-9b fixes (commits `1537adc` and
   `df5295e`).
3. Re-running `bash scripts/release.sh --skip-tests` → published
   `v1.170.0` (the SemVer-correct minor bump). All 14 pipeline steps
   PASS. Marked as Latest on GH Releases.
4. `cargo test --workspace --lib` (2931 pass, 0 fail) and
   `cargo clippy --workspace --all-targets -- -D warnings` (clean)
   were verified at the release SHA.

## References

- `scripts/release.sh` lines 347-373 (step 2.5) and 643-653 (step 9b)
- `scripts/release-bump.sh` lines 51-63 (SemVer detection)
- `scripts/lib/release_admission.sh` (admission contract — unchanged)
- `githooks/pre-push` (hook contract — unchanged; still requires real
  workspace-version bump)
- `docs/architecture/CONTRIBUTING-SEMVER.md` §4.3 (mechanics)
- `docs/roadmap/receipts/c4-release-pending/SEMVER-MISMATCH.md` (incident)
- `docs/roadmap/receipts/c4-release-pending/SNAPSHOT.md` (drift binary)
