# SDDK — Semantic Versioning & Conventional Commits Discipline

> **Authority:** This document is the operational reference for SemVer and
> Conventional Commits application in SDDK. It is **complementary** to
> `AGENTS.md §2.1` (commits) and `§8` (release flow) and **does not
> override them**. AGENTS.md remains the authoritative source for
> pre-push hook contract and release gates; this document explains the
> semantics of version bumps in the SDDK development model.
>
> **Scope:** This applies to the **workspace `[workspace.package] version`**
> (in `Cargo.toml`) which tracks **development state** on `main`. It is
> distinct from **release tags** (e.g. `v1.169.122`) which are produced
> by `bash scripts/release.sh` and ARE SemVer-compliant.

## 1. Two distinct version concepts

SDDK has **two** version values, often confused:

| Concept | Where | SemVer-strict? | Created by | Released to users? |
|---|---|---|---|---|
| **Workspace version** | `[workspace.package] version` in `Cargo.toml` | **No** — pointer | `chore(release): bump version` (pre-push hook contract) | No (development only) |
| **Release tag** | Git tag `vX.Y.Z` | **Yes** | `bash scripts/release.sh` (operator-authorized) | Yes (GH Releases) |

The **workspace version** is bumped **per push range** to keep the
pre-push hook satisfied (it requires a real version change when source
is touched). It does NOT follow SemVer — it follows the project's own
"every-push-needs-a-bump" contract.

The **release tag** is created at publish time by `scripts/release.sh`,
which calls `cargo_ws_version_at` to read the workspace version, decides
the SemVer bump based on accumulated commits in the pushed range, and
publishes a single GH Release. **That** tag IS SemVer-compliant.

## 2. Conventional Commits in SDDK

Per AGENTS.md §2.1, every commit subject follows Conventional Commits in
Spanish, with a scope:

| Subject prefix | When | SemVer component (release tag) |
|---|---|---|
| `feat(<scope>):` | new feature, new test, new behavior | MINOR (`+0.1.0`) |
| `fix(<scope>):` | bug fix, behavior correction | PATCH (`+0.0.1`) |
| `docs(<scope>):` | documentation-only | no bump (excluded from release tag calculation) |
| `test(<scope>):` | tests only, no behavior change | no bump |
| `chore(release):` | workspace version bump only | depends on what's in the range |
| `chore(<scope>):` | tooling, non-feature work | no bump |

The scope is **specific** (e.g. `c3a`, `c3g`, `cli`, `roadmap`,
`journal`, `c2a-msgfix`), not generic.

### 2.1. Body and rationale

Per AGENTS.md §2.1: "Una concernencia por commit. Si un cambio toca
docs + código, un solo commit con la concernencia explicada en el
body."

The commit body should briefly justify the change, especially:
- Why this scope (which WorkItem or sub-cycle).
- What evidence the commit carries (tests, receipts).
- Any surprising finding or STOP condition encountered.

## 3. Workspace version bumping rules

### 3.1. When to bump

Per `githooks/pre-push`:
- **(A)** Push range contains a real `[workspace.package] version` change
  (old != new).
- **(B)** Push range is non-empty and EVERY changed path lies inside the
  documentation-only allowlist (`docs/**`, `.sddk/followups/**`,
  `tests/cycle-artifacts/p-*/*/{SCOPE-CONTRACT,DISCOVERY,RECEIPT}.md`).
- **(C)** Push range is non-empty and ONLY contains generated-derived
  metadata (MANIFEST.sha256) plus paths from (B).

If you touch any path outside (B), you MUST bump. The bump magnitude
(PATCH vs MINOR) does not matter for the hook — only the change does.

### 3.2. What magnitude to bump

The **workspace bump magnitude** is **informal**, not SemVer-strict.
SDDK practice (historically):
- One PATCH bump per push range that ends at a non-release commit.
- MINOR bump ONLY if the operator decides a release is imminent.

This keeps the hook satisfied without inflating the version into
non-SemVer territory. The actual SemVer bump happens at release time
in `scripts/release.sh`.

### 3.3. What NOT to do

- **Do not retroactively re-bump session-11 commits** — would be
  ceremonial work without value. The next release tag will reconcile
  the range into one SemVer-compliant bump.
- **Do not use a workspace bump to "advertise" SemVer intent** — the
  release tag is the only authoritative SemVer signal.
- **Do not bypass the hook** — `git push --no-verify` is forbidden by
  AGENTS.md §2.5.
- **Do not use `--skip-tests` or `--skip-install`** to avoid a bump —
  these are release-time flags, not commit-time gates.

## 4. Release tag SemVer discipline

At release time, `scripts/release.sh` reads the workspace version
(`Cargo.toml [workspace.package] version`) and uses it directly as the
release tag prefix (`v` + workspace version). The operator decides
**what workspace version to use** before invoking release.sh.

### 4.1. Choosing the release workspace version

Per ROADMAP.md §C4 and `docs/RELEASING.md`:

- The workspace version SHOULD be **bumped to the target SemVer**
  before running `scripts/release.sh`. The bump is `chore(release): bump
  version OLD → NEW`.
- The SemVer decision (MAJOR/MINOR/PATCH) is made by the operator based
  on the accumulated commits in the range since the last release tag.
- The `chore(release): bump version` commit is a **separate** commit
  from the feature commits, per AGENTS.md §8 ("Por qué dos commits").

### 4.2. Mapping commit prefixes to SemVer

For deciding the next release tag:

| Accumulated range contains | SemVer bump |
|---|---|
| Only `docs:`, `chore:`, `test:` commits | **PATCH** (`+0.0.1`) |
| Any `feat(<scope>):` (with backwards-compatible behavior) | **MINOR** (`+0.1.0`) |
| Any `feat(<scope>):` with breaking change documented in body | **MAJOR** (`+1.0.0`) |
| Any `fix(<scope>):` of a real bug | **PATCH** (could be combined with MINOR if feat also present) |

The operator makes the final call. AGENTS.md §2.1 and CERTIFICATIONS.md
§3 require that any release be honest: `PASS_BY_CODE_READING` does not
exist, and `NOT_EVALUATED` cannot be silently flipped to PASS.

### 4.3. Mechanized calculation via `scripts/release-bump.sh`

The release script reads `scripts/release-bump.sh` to compute the next
SemVer tag. This is **not** a heuristic — it is the actual algorithm
that the script applies when called from `scripts/release.sh` (or
manually via `bash scripts/release-bump.sh --dry-run`).

The rules (verbatim from `release-bump.sh` lines 51-63):

| Detected pattern in `git log $last_tag..HEAD` | Resulting bump |
|---|---|
| `BREAKING CHANGE` in any commit body, OR `<type>!:` prefix | **MAJOR** (`X+1.0.0`) |
| Any commit matching `^[a-f0-9]+ feat` | **MINOR** (`X.Y+1.0`) |
| Any commit matching `^[a-f0-9]+ (fix\|refactor\|perf\|docs\|ci\|chore\|style\|test\|build)` (and no feat) | **PATCH** (`X.Y.Z+1`) |
| None of the above | "no release-worthy commits" — no release |

Commits with prefix `chore(release):` are **filtered out** before the
detection (line 47: `grep -vE 'chore\(release\)'`). This is why our
ceremonial bumps in `main` do not skew the calculation.

**Worked example (session-11, observed 2026-09-22T10:51Z):**

```bash
$ bash scripts/release-bump.sh --dry-run
release bump: v1.169.122 -> v1.170.0 (minor)
new tag: v1.170.0
```

Why **MINOR** and not PATCH? The script counts every commit since
`v1.169.122` (the last public release tag) and detects:

```
b562f5d feat(c3f): T28 — fix C3e-F1 via ADR-0141 + pre_flight_check + InconsistentMigrationState
4dc2a08 feat(c3e): T27 — schema resilience tests (8 added) + finding C3e-F1 (DEFERRED_FIX)
d039457 feat(c3d): T26 — storage performance baseline (append, cas, lease) opt-in benches
775ec93 feat(c3c): T23+T24+T25 — capability receipts, cycle leases, schema_guard boundary
7a5388a feat(c3b): T21+T22 — CAS corruption detection, event idempotency, reopen, IMMEDIATE contention
...
```

Six `feat:` commits (C3b/c/d/e/f + ADR-0141) push MINOR. PATCH would
be wrong here — that is a feature-carrying release. The workspace
version `1.169.152` is **misleading** because it incremented per
ceremonial bump, but the **release tag** `v1.170.0` will correctly
reflect SemVer.

**Honest correction to §4.2 table:** A range with only `docs:`,
`chore:`, `test:` commits yields PATCH. But `feat(c3*):` commits are
**features** (new measurable behaviors: schema resilience, performance
baseline, capability receipts, etc.), not "tests". The `test:` prefix
is reserved for unit-test-only changes. The convention in SDDK is that
a new measurable capability gets `feat:` even when its primary
artifact is a test file (e.g. `tests/perf_budget_base.rs`).

**Operator override:** `scripts/release-bump.sh --force-version X.Y.Z`
bypasses the calculation and pins an explicit version. Use this when
the auto-calculation would be misleading (e.g. hidden breaking changes
not flagged in commit bodies).

## 5. Concrete examples

### 5.1. Workspace version drift example (session-11)

```
session-10 close:   workspace v1.169.142 (last public release v1.169.122)
session-11 C3a:     3 feat(c3a): commits → workspace bumped to v1.169.143
session-11 C3b:     1 feat(c3b): commit   → workspace bumped to v1.169.144
session-11 C3c:     1 feat(c3c): commit   → workspace bumped to v1.169.145
session-11 C3d:     1 feat(c3d): commit   → workspace bumped to v1.169.146
session-11 C3e:     1 feat(c3e): commit   → workspace bumped to v1.169.147
session-11 C3f:     1 feat(c3f): commit   → workspace bumped to v1.169.148
session-11 housekeep:docs commits (no bump, allowlist (B))
session-11 C2 re-investigation: docs commits (no bump)
session-11 C2a-MsgFix: 1 fix(cli): + 1 chore(release): → v1.169.150
session-11 C3g:    1 test(c3g): + 1 chore(release): → v1.169.151
```

This produces a workspace version that has grown by **+0.0.9** since the
last public release, but the actual SemVer-compliant release that would
publish these 27 commits as a single tag is determined by the
**operator** at release time, looking at the entire range:
- 9 `feat:` commits (c3a, c3b, c3c, c3d, c3e, c3f) → MINOR at least.
- 1 `fix:` commit (cli) → PATCH.
- 1 `test:` commit (c3g) → PATCH.
- Net: **MINOR bump** at release.

So the next release tag would be **`v1.169.123`** (PATCH on the last
public `v1.169.122`) if the operator treats the range as PATCH-only
(documentation + bug fixes), or **`v1.169.123`** MINOR if the operator
counts the new C3 sub-cycles as features.

In either case, the workspace `1.169.151` is the **input** to the
release, not the output.

### 5.2. Correct flow

```
1. Develop on main: feat/fix/test/docs commits accumulate, workspace
   bumps per push range per pre-push hook.
2. When ready to release:
   a. Operator audits the range since last release tag.
   b. Operator decides SemVer bump (PATCH / MINOR / MAJOR).
   c. Operator runs `chore(release): bump version OLD → NEW` where NEW
      reflects the SemVer decision.
   d. Operator runs `bash scripts/release.sh` which:
      - Reads NEW workspace version
      - Creates git tag vNEW
      - Builds, tests, bundles, publishes to GH Releases
      - Updates the runtime install
3. Workspace version and release tag are now in sync (workspace vNEW,
   release tag vNEW).
```

## 6. Common mistakes

### 6.1. Treating workspace version as SemVer

**Wrong**: "Workspace is 1.169.151, that means MINOR bump over 1.169.122,
so we shipped 29 new features."

**Right**: "Workspace 1.169.151 reflects per-push bumps over 27 commits
since the last release. The actual SemVer component will be decided by
the operator at release time based on the commit types in the range."

### 6.2. Skipping the bump because "no behavior changed"

**Wrong**: "This commit only adds docs, so no bump."

**Right**: Docs-only commits in `docs/**` are allowlisted; they do NOT
require a bump. Commits in any other path require a bump per the
pre-push hook contract.

### 6.3. Massive MINOR bumps per push range

**Wrong**: "I added 5 new tests in one commit, so MINOR bump."

**Right**: Tests alone don't trigger a MINOR bump in the workspace
version (workspace bumps are not SemVer-strict). The MINOR component
is decided at release time based on whether the range adds user-facing
features.

## 7. Related documents

- AGENTS.md §2.1 — Conventional Commits requirement (this document
  operationalizes it).
- AGENTS.md §2.5 — CI local-first, no bypass, no skipped tests.
- AGENTS.md §8 — Release flow (`scripts/release.sh`).
- `docs/RELEASING.md` — Step-by-step release procedure.
- `githooks/pre-push` — Hook source of truth for bump requirement.
- `scripts/lib/release_admission.sh` — Release-time admission (v1/v2).
- ROADMAP.md §C4 — Release and certification cycle.

## 8. Change history

- 2026-09-22T10:13Z — Initial version. Decision taken in response to
  operator question about SemVer discipline (session-11 close). Document
  separates workspace-version semantics from release-tag SemVer,
  preserving the existing pre-push hook contract without modifying it
  (AGENTS.md §2.5 authority).
