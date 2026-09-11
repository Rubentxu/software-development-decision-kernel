---
name: core.release-planning
description: "Trigger: planning or supervising a release. Use when deciding the next version bump, drafting release notes, sequencing the 13-step scripts/release.sh flow, or assessing whether a release is safe to publish."
disable-model-invocation: true
user-invocable: false
license: Apache-2.0
metadata:
  author: SDDK Team
  version: "1"
  delegate_only: true
---

## Activation Contract

Activated automatically by the `sddk release` CommandSpec via
`with_required_skill("core.release-planning@v1")`. The runtime
admission gate requires this skill to be present on disk; without
it, the gate emits a `MissingPlaceholderSkill` warning.

## When this skill applies

- Choosing the next version bump (major / minor / patch) based on
  conventional commits since the last tag.
- Drafting release notes (one-line summary, scope, evidence refs).
- Sequencing the 13-step `scripts/release.sh` flow and assessing
  which steps the current cycle actually needs.
- Promoting a draft GH Release to published (post-burst cleanup).
- Planning a self-install + `sddk dev doctor` + prune round-trip
  to verify local coherence.

## Evidence it expects

- `docs/RELEASING.md` (the canonical release contract; AGENTS.md §8
  references this)
- `scripts/release.sh` (the 13-step pipeline; the live authority)
- `scripts/release-bump.sh` (semver rules)
- `AGENTS.md` §8 (release flow auto-install local)
- `AGENTS.md` §5 (verify profile gates)
- The `gh release list --repo …` view for the latest published tags

## Pattern of release planning

1. **Bump decision** — run `bash scripts/release-bump.sh --dry-run`
   to see what semver the conventional commits since the last tag
   imply. Override only with explicit rationale in the commit body.
2. **Pre-flight gates** (per AGENTS.md §5):
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `cargo build --release -p sddk-cli`
   - `shellcheck tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh`
3. **Bump commit** — `chore(release): bump version to vX.Y.Z` MUST
   appear in the push range or the pre-push hook will reject
   (INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION).
4. **Run the canonical flow** — `bash scripts/release.sh` (with
   `--dry-run` first if uncertain). All 13 steps must succeed;
   each step's gate is the previous step's success.
5. **Post-release coherence** — `sddk dev doctor --prefix
   $SDDK_PREFIX` must report `binary.bundle_coherence: present`
   and `all_present: true`. `sddk dev update --prune-only
   --keep 1` cleans stale framework versions.
6. **Draft cleanup** — if the burst produced any `isDraft=true`
   GH releases, promote them with `gh release edit --draft=false`
   in chronological order (each gh release's CDN needs ~5 min
   to refresh before the next install from URL can be safe).

## Hard rules

- Skill ≠ Capability (SPEC-016): this skill plans releases; it
  does not grant permission to push without the pre-push gate
  passing.
- Never release with a `isDraft=true` GH release left over from a
  prior cycle. Promoted GH Releases are the user-visible artifact.
- Never skip the CDN sha256 poll (release.sh step 10). The CDN
  may serve stale assets for ~5 minutes after upload.
- Never delete or force-push a tag without an explicit cycle
  decision (and a documented INC if the original release was
  published externally).
