#!/usr/bin/env bash
# release.sh — Canonical end-to-end release flow for sddk-framework.
#
# Standardized after cycle-46 (install coherence) and cycle-47 (install
# consolidation). Every release MUST pass through this script — or the
# manual equivalent — so that local install stays in lockstep with what
# ships through GitHub Releases.
#
# Pipeline (each step is gated on the previous one succeeding):
#   1. Preflight  — workspace green: fmt, clippy -D warnings, tests
#   2. Version    — read current version from Cargo.toml
#   3. Build      — cargo build --release --bin sddk
#   4. Manifest   — regenerate MANIFEST.sha256 from the bundle surface
#   5. Bundle     — tar agents/ skills/ prompts/sddk/ assets/ MANIFEST.sha256
#   6. BUNDLE.toml — inject schema_version=2 + manifest_sha256 into the bundle
#   7. Unified    — repack bin/sddk + framework/ as sddk-<TAG>-<ASSET>.tar.gz
#                   with chmod 0755 on the binary (defensive against CDN cache)
#   8. Checksums  — sha256 + CHECKSUMS + sbom.json for the binary
#   9. Publish    — gh release create with all assets (--clobber if --force)
#  10. Install    — bash scripts/install.sh --version <TAG> against the real
#                   GitHub URL (no SDDK_BASE_URL override)
#  11. Verify     — sddk dev doctor --prefix <P> reports binary.bundle_coherence
#  12. Prune      — sddk dev update --prune-only --keep 1 to clean stale dirs
#  13. Manifest   — print final state (binary version, bundle version,
#                   framework layout, doctor result)
#
# Usage:
#   bash scripts/release.sh                 # full release flow
#   bash scripts/release.sh --dry-run       # walk steps 1-8 only (no publish)
#   bash scripts/release.sh --skip-tests    # skip step 1 (you just ran them)
#   bash scripts/release.sh --skip-install  # steps 1-9 only (no local install)
#   bash scripts/release.sh --force         # overwrite existing GH release
#
# Tag format: vX.Y.Z (semver). The script reads the version from Cargo.toml's
# workspace.package.version and prepends "v" — never accepts a --version
# override (the version is the source of truth; bump it via
# scripts/release-bump.sh or by hand before invoking release.sh).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$ROOT"
REPO="${SDDK_REPO:-Rubentxu/software-development-decision-kernel}"
SDDK_PREFIX="${SDDK_PREFIX:-$HOME/.local/bin}"
SDDK_FRAMEWORK_DIR="${SDDK_FRAMEWORK_DIR:-$HOME/.local/share/sddk/framework}"
cd "$ROOT"

# Isolate TMPDIR for the whole release run so the test gate is deterministic
# regardless of the ambient TMPDIR. The scratch MUST live OUTSIDE the repo
# tree and OUTSIDE a symlinked home prefix:
#   * Under `$ROOT/target` (inside the repo) tests create temp dirs that walk
#     up and hit the repo's own git/sddk markers, breaking cycle/manifest
#     walk-up tests.
#   * Under a symlinked home (e.g. `/home -> /var/home`) the real path differs
#     from the reported one, breaking XDG canonicalization in integration
#     tests.
# Earlier flakiness blamed the ambient scratch for `PermissionDenied` under
# parallel load; the real cause was two test-isolation races in sddk-cli
# (a global `chdir` in a cycle test and a `copy_tree` test chmod-ing the shared
# temp root to read-only), now fixed in the test suite. We keep the isolation
# for determinism but place it on a real, disk-backed path outside the repo.
mkdir -p "${CARGO_TARGET_DIR:-$HOME}" 2>/dev/null || true
SCRATCH_ROOT="$(readlink -f "${CARGO_TARGET_DIR:-$HOME}" 2>/dev/null || echo "$ROOT")"
mkdir -p "$SCRATCH_ROOT"
RELEASE_SCRATCH="$(mktemp -d "$SCRATCH_ROOT/sddk-release-tmp.XXXXXX")"
export TMPDIR="$RELEASE_SCRATCH"
cleanup_release_scratch() { rm -rf "$RELEASE_SCRATCH"; }
trap cleanup_release_scratch EXIT

# --- args ---

DRY_RUN=0
SKIP_TESTS=0
SKIP_INSTALL=0
FORCE=0
FORCE_VERSION=""

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run)         DRY_RUN=1; shift ;;
        --skip-tests)      SKIP_TESTS=1; shift ;;
        --skip-install)    SKIP_INSTALL=1; shift ;;
        --force)           FORCE=1; shift ;;
        --force-version)   FORCE_VERSION="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,/^[^#]/p' "$0" | head -50
            exit 0
            ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

# --- helpers ---

step() { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m  !\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m  ✗\033[0m %s\n' "$*" >&2; exit 1; }

require() {
    command -v "$1" >/dev/null 2>&1 \
        || die "required command not found: $1"
}

# --- 0. preflight: tooling + git state ---

step "0/14 — preflight"
require cargo
require git
require gh
require tar
require sha256sum
require curl
require jq

gh auth status >/dev/null 2>&1 \
    || die "gh CLI not authenticated — run: gh auth login"

# Branch check: must be on main, clean working tree (release-bump script
# already stages the version bump; we expect that to be in HEAD or HEAD~1).
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[ "$BRANCH" = "main" ] \
    || die "must be on main (currently on $BRANCH)"

if ! git diff --quiet || ! git diff --cached --quiet; then
    die "working tree is dirty — commit or stash before releasing"
fi

# A5-1 §6 — release admission is semantic, not textual.
#
# A release HEAD MUST carry a REAL (and monotonically increasing)
# [workspace.package] version change relative to its first parent. The commit
# subject may follow the `chore(release): bump version` convention, but the
# subject alone is NEVER the contract: an empty ceremonial marker commit is
# refused here, so it can never be mistaken for a real release.
#
# Single source of the invariant: `scripts/lib/release_admission.sh`.
# shellcheck source=lib/release_admission.sh
# shellcheck disable=SC1091
. "$ROOT/scripts/lib/release_admission.sh"

LAST_SUBJECT="$(git log -1 --format=%s)"
ADMISSION="$(release_admission_check HEAD)" \
    || die "release admission refused: $ADMISSION — release requires a real, monotonic [workspace.package] version bump"
if ! echo "$LAST_SUBJECT" | grep -qE '^chore\(release\): bump version'; then
    warn "HEAD subject does not follow the 'chore(release): bump version' convention: $LAST_SUBJECT"
fi
ok "on main, clean tree, release admission: $ADMISSION"

# --- 1. tests ---

if [ "$SKIP_TESTS" = "0" ]; then
    step "1/14 — cargo fmt + clippy + test (workspace)"
    cargo fmt --all -- --check || die "cargo fmt failed"
    cargo clippy --workspace --offline --all-targets -- -D warnings \
        || die "cargo clippy failed"
    cargo test --workspace --offline \
        || die "cargo test --workspace failed"
    ok "workspace green"

    step "1b/14 — shell contract tests (tests/test_*.sh)"
    # The shell contract tests pin invariants that cargo cannot cover
    # (githooks/pre-push behavior, release-receipt authority gate, the
    # cross-crate lockstep between scripts/release-receipt.sh and the
    # engine's infer_actor_kind, the ADR-0001 §3.4 frontmatter
    # convention). They run sequentially; each is expected to exit 0
    # with its own banner. shellcheck is run first as a static gate;
    # the dynamic tests follow.
    if command -v shellcheck >/dev/null 2>&1; then
        # Scope the static gate to scripts/tests authored or extended by
        # this repository's M9+ contracts; legacy tests in
        # tests/test_vault_coherence_alignment.sh have pre-existing
        # SC2034/SC2329 warnings outside our gate (they're exercised by
        # their own dynamic tests, not by shellcheck).
        shellcheck --severity=warning scripts/release-receipt.sh \
            scripts/lib/release_admission.sh \
            githooks/pre-push \
            tests/test_release_admission.sh \
            tests/test_push_prevention_hook.sh \
            tests/test_release_receipt_authority.sh \
            tests/test_authority_helper_lockstep.sh \
            tests/test_adr_promotion_format.sh \
            tests/test_advisory_lint_explanations.sh \
            tests/test_deny_lint_zero_hits.sh \
            tests/test_vault_adr_mirror_coverage.sh \
            tests/test_release_tag_anchoring.sh \
            tests/test_vault_mirror_auto.sh \
            || die "shellcheck failed"
        ok "shellcheck clean (scope: release-receipt + release/push admission + 8 cross-crate/M9+ tests)"
    else
        warn "shellcheck not installed — skipping static gate (install shellcheck for full coverage)"
    fi
    for t in tests/test_push_prevention_hook.sh \
             tests/test_release_admission.sh \
             tests/test_release_receipt_authority.sh \
             tests/test_authority_helper_lockstep.sh \
             tests/test_adr_promotion_format.sh \
             tests/test_advisory_lint_explanations.sh \
             tests/test_deny_lint_zero_hits.sh \
             tests/test_vault_adr_mirror_coverage.sh \
             tests/test_release_tag_anchoring.sh \
             tests/test_vault_mirror_auto.sh; do
        if [ -x "$t" ]; then
            bash "$t" >/dev/null \
                || die "shell test failed: $t (run manually for details)"
            ok "shell test: $(basename "$t")"
        else
            warn "shell test not executable, skipping: $t"
        fi
    done
    ok "shell contract tests green"
else
    warn "skipping step 1 (tests) — assumed already run"
fi

# --- 1c. publish sync: ensure origin/main is at HEAD before step 9 ---
#
# INC-RELEASE-TAG-FIX: step 9 (gh release create --target main) resolves
# `main` to the commit at the tip of origin/main. The release flow's
# previous commits (feat + ceremonial + bump) are local-only and must be
# pushed before publish, otherwise the tag points to a stale commit and
# requires manual repointing. This step pushes HEAD to origin/main
# non-interactively (the pre-push hook enforces the bump predicate) and
# fails closed if origin/main has advanced concurrently — the operator
# must merge before re-running.
#
# Behaviour:
#   1. git fetch origin main → see the current remote tip.
#   2. If origin/main == HEAD → nothing to do, fast path.
#   3. If origin/main behind HEAD → git push origin main (pre-push hook
#      enforces bump-commit + version-bump predicate; no need to repeat
#      that check here).
#   4. If origin/main ahead of HEAD → refuse to release, instruct the
#      operator to merge origin/main into HEAD and re-run.
#
# The --skip-tests flag does not skip this step: pushing is part of the
# release contract, not the test gate.

step "1c/14 — sync HEAD to origin/main (closes INC-RELEASE-TAG-FIX)"
git fetch origin main --quiet \
    || die "git fetch origin main failed — cannot verify remote state"
LOCAL_HEAD="$(git rev-parse HEAD)"
REMOTE_MAIN="$(git rev-parse origin/main)"

if [ "$LOCAL_HEAD" = "$REMOTE_MAIN" ]; then
    ok "HEAD already at origin/main ($LOCAL_HEAD) — no push needed"
elif git merge-base --is-ancestor "$REMOTE_MAIN" "$LOCAL_HEAD"; then
    # origin/main is behind HEAD → fast-forward push.
    # The pre-push hook (githooks/pre-push) is automatically invoked by
    # `git push` and rejects non-release commits to main. We rely on
    # that gate; do not duplicate the predicate here.
    if git push origin main >/dev/null 2>&1; then
        ok "pushed HEAD to origin/main: $LOCAL_HEAD"
    else
        die "git push origin main failed — pre-push hook rejected the push; ensure HEAD carries a real [workspace.package] version bump or a docs-only range"
    fi
else
    # origin/main is ahead of HEAD → concurrent advance. Fail closed.
    die "origin/main ($REMOTE_MAIN) is ahead of HEAD ($LOCAL_HEAD); merge origin/main into HEAD and re-run"
fi

# --- 1d. EXT auto-activation (closes FU-A6-EXT-AUTO) ---
#
# When the operator exports $COGNICODE_MCP_BIN and/or $CHRONOS_MCP_BIN
# before invoking release.sh, run the previously-#[ignore] EXT tests
# against the real provider binaries, capture pass/fail per provider, and
# write an EXT-RECEIPT.md to the cycle artifacts. Without the env vars,
# the step is a no-op (the EXT tests stay #[ignore] and the release
# proceeds normally — operators without the binaries are not blocked).
#
# This step MUST run before version reading because the receipt dir
# embeds the version string. We read VERSION early here.
step "1d/14 — EXT auto-activation (cognicode-mcp / chronos-mcp, opt-in)"
VERSION="$(awk '/^\[workspace\.package\]/{flag=1; next} flag && /^version = /{print $3; exit}' Cargo.toml \
    | tr -d '\"')"
TAG="v$VERSION"
[ -n "$VERSION" ] || die "could not parse version from Cargo.toml"

EXT_RECEIPT_DIR=""
EXT_FAIL=0
if [ -n "${COGNICODE_MCP_BIN:-}" ] || [ -n "${CHRONOS_MCP_BIN:-}" ]; then
    require jq
    EXT_RECEIPT_DIR="tests/cycle-artifacts/p-63676b11dc0ef88f/ext-auto-activation-$VERSION"
    mkdir -p "$EXT_RECEIPT_DIR"
    : > "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
    {
        echo "# EXT-RECEIPT — $TAG (release-time auto-activation)"
        echo
        echo "- HEAD: $LOCAL_HEAD"
        echo "- COGNICODE_MCP_BIN: ${COGNICODE_MCP_BIN:-NOT SET}"
        echo "- CHRONOS_MCP_BIN: ${CHRONOS_MCP_BIN:-NOT SET}"
        echo "- date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo
    } >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"

    if [ -n "$COGNICODE_MCP_BIN" ]; then
        echo "## CogniCode EXT (COGNICODE_MCP_BIN=$COGNICODE_MCP_BIN)" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        if COGNICODE_MCP_BIN="$COGNICODE_MCP_BIN" \
            cargo test --workspace --offline -- \
                --include-ignored \
                a6_cc_s1 aiw_s1_cognicode_real 2>&1 \
                | tee "$EXT_RECEIPT_DIR/cognicode-ext.log" >/dev/null; then
            echo "- result: PASS" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        else
            echo "- result: FAIL" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
            EXT_FAIL=1
        fi
        # Extract test counts from the log.
        grep -E 'test result:' "$EXT_RECEIPT_DIR/cognicode-ext.log" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md" || true
    fi

    if [ -n "$CHRONOS_MCP_BIN" ]; then
        echo "## Chronos EXT (CHRONOS_MCP_BIN=$CHRONOS_MCP_BIN)" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        if CHRONOS_MCP_BIN="$CHRONOS_MCP_BIN" \
            cargo test --workspace --offline -- \
                --include-ignored \
                aiw_s5_chronos_real a7_s1_runtime_uat 2>&1 \
                | tee "$EXT_RECEIPT_DIR/chronos-ext.log" >/dev/null; then
            echo "- result: PASS" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        else
            echo "- result: FAIL" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
            EXT_FAIL=1
        fi
        grep -E 'test result:' "$EXT_RECEIPT_DIR/chronos-ext.log" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md" || true
    fi

    ok "EXT receipt written to $EXT_RECEIPT_DIR/EXT-RECEIPT.md"
    if [ "$EXT_FAIL" = "1" ]; then
        die "EXT tests failed against real binaries — refusing to release (see $EXT_RECEIPT_DIR/EXT-RECEIPT.md)"
    fi
else
    ok "no EXT env vars set — skipping (EXT tests stay #[ignore] in this release)"
fi

# --- 2. version ---

# VERSION/TAG were already read in step 1d above. Re-read defensively in
# case step 1d was added as an insert (the read is idempotent).
VERSION="${VERSION:-$(awk '/^\[workspace\.package\]/{flag=1; next} flag && /^version = /{print $3; exit}' Cargo.toml \
    | tr -d '\"')}"
TAG="${TAG:-v$VERSION}"
[ -n "$VERSION" ] || die "could not parse version from Cargo.toml"
ok "version: $VERSION → tag: $TAG"

# --- 2.5 semver-correct tag (cycle-c2 bug fix) ---
#
# The workspace version above is a CEREMONIAL per-push pointer (incremented by
# the pre-push hook for every source-touching commit, not SemVer-strict).
# `scripts/release-bump.sh` computes the actual SemVer-correct next tag from
# the conventional commits accumulated since the last published release tag.
# Without this step the released tag would be the workspace version literal
# (e.g. v1.169.152) instead of the SemVer-bumped tag (e.g. v1.170.0).
#
# Invocation: invoke release-bump.sh in dry-run mode and parse its output.
# Honors the operator's --force-version flag (passed through if set).
BUMP_ARGS=(--dry-run)
if [ -n "$FORCE_VERSION" ]; then
    BUMP_ARGS+=(--force-version "$FORCE_VERSION")
    warn "operator forced version override: $FORCE_VERSION (SemVer algorithm bypassed)"
fi
STEP2P5_OUTPUT="$(bash "$ROOT/scripts/release-bump.sh" "${BUMP_ARGS[@]}" 2>&1)" \
    || die "scripts/release-bump.sh failed (cannot compute SemVer tag)"
SEMVER_TAG="$(echo "$STEP2P5_OUTPUT" \
    | awk '/^new tag: / {print $3; exit}')"
if [ -z "$SEMVER_TAG" ]; then
    # release-bump.sh exits 0 with "no commits since <tag>" message — keep TAG.
    warn "release-bump.sh did not produce a tag; keeping workspace-derived TAG=$TAG"
else
    if [ "$SEMVER_TAG" != "$TAG" ]; then
        warn "semver tag overrides workspace-derived tag: $TAG → $SEMVER_TAG"
        TAG="$SEMVER_TAG"
    else
        ok "semver tag matches workspace tag: $TAG"
    fi
fi
ok "final tag: $TAG"

# --- 3. build ---

step "3/14 — cargo build --release --bin sddk"
cargo build --release --offline --bin sddk \
    || die "cargo build failed"
# Locate the binary via cargo metadata so we respect CARGO_TARGET_DIR.
BIN="$(cargo metadata --format-version 1 --offline \
        | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' \
        || true)/release/sddk"
[ -x "$BIN" ] || die "binary not found at $BIN"
ok "binary: $BIN ($("$BIN" --version))"

# --- 4. manifest ---

step "4/14 — regenerate MANIFEST.sha256"
"$BIN" dev manifest --root . --format text \
    || die "sddk dev manifest failed"
"$BIN" dev manifest --verify --root . --format text \
    || die "manifest verification failed (RDI)"
ok "MANIFEST.sha256 regenerated and verified"

# --- 5. bundle tarball ---

step "5/14 — bundle tarball"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP" "$RELEASE_SCRATCH"' EXIT

BUNDLE_TARBALL="$TMP/software-development-decision-kernel.tar.gz"
tar czf "$BUNDLE_TARBALL" \
    --xform "s|^|software-development-decision-kernel/|" \
    -C . agents skills prompts/sddk assets MANIFEST.sha256
sha256sum "$BUNDLE_TARBALL" | awk '{print $1}' > "$BUNDLE_TARBALL.sha256"
ok "bundle: $(basename "$BUNDLE_TARBALL") ($(stat -c%s "$BUNDLE_TARBALL") bytes)"

# --- 6. BUNDLE.toml ---

step "6/14 — inject BUNDLE.toml (schema v2)"
BUNDLE_DIR="$TMP/bundle"
mkdir -p "$BUNDLE_DIR"
tar xzf "$BUNDLE_TARBALL" -C "$BUNDLE_DIR"
MANIFEST_SHA="$(awk 'NR==1 {print $1}' MANIFEST.sha256)"
FW_DIR="$BUNDLE_DIR/software-development-decision-kernel"
printf '%s\n' \
    '[bundle]' 'schema_version = 2' \
    "version = \"$VERSION\"" \
    "binary_min_version = \"$VERSION\"" \
    "binary_max_version = \"$VERSION\"" \
    '' '[contents]' "manifest_sha256 = \"$MANIFEST_SHA\"" \
    > "$FW_DIR/BUNDLE.toml"
ok "BUNDLE.toml written (manifest_sha256=$MANIFEST_SHA)"

# --- 7. unified tarball ---

step "7/14 — unified tarball (bin/sddk + framework/)"
UNIFIED="$TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz"
PACK="$TMP/pack"
rm -rf "$PACK"
mkdir -p "$PACK/bin" "$PACK/framework"
chmod 0755 "$BIN"
cp "$BIN" "$PACK/bin/sddk"
# Copy the CONTENTS of FW_DIR (which is wrapped under
# software-development-decision-kernel/) directly into pack/framework/, so
# install.sh finds BUNDLE.toml / MANIFEST.sha256 at framework/BUNDLE.toml
# (its expected location) instead of framework/software-development-decision-kernel/BUNDLE.toml.
cp -r "$FW_DIR/." "$PACK/framework/"
tar -C "$PACK" -czf "$UNIFIED" bin framework
chmod 0755 "$PACK/bin/sddk"  # post-extract defensive chmod (already 0755)
# Verify the exec bit survives in the archive AND that BUNDLE.toml lives
# at the install.sh-expected path (not nested under software-development-decision-kernel/).
# CRITICAL: do NOT use `grep -q` here. With `set -o pipefail`, `grep -q`
# exits on the first match and closes stdin, which causes `tar` to receive
# SIGPIPE (exit 141) and the pipeline to be reported as failed even though
# grep found the match. Capture tar output to a variable and grep it
# afterwards.
TAR_LISTING="$(tar tvzf "$UNIFIED")"
if ! grep -q -- '-rwxr-xr-x.* bin/sddk' <<<"$TAR_LISTING"; then
    die "unified tarball lost the exec bit on bin/sddk — refusing to ship"
fi
if ! grep -q -- 'framework/BUNDLE.toml$' <<<"$TAR_LISTING"; then
    die "unified tarball lacks framework/BUNDLE.toml at the install.sh-expected path"
fi
sha256sum "$UNIFIED" | awk '{print $1}' > "$UNIFIED.sha256"
ok "unified: $(basename "$UNIFIED") ($(stat -c%s "$UNIFIED") bytes, exec bit + BUNDLE.toml OK)"

# --- 8. checksums + sbom ---

step "8/14 — sha256 + CHECKSUMS + sbom.json"
BIN_SHA="$(sha256sum "$BIN" | awk '{print $1}')"
echo "$BIN_SHA  $(basename "$BIN")" > "$TMP/$(basename "$BIN").sha256"
( cd "$TMP" && sha256sum "$(basename "$UNIFIED")" "$(basename "$BUNDLE_TARBALL")" ) \
    > "$TMP/CHECKSUMS"
cat > "$TMP/sbom.json" <<EOF
{"bomFormat":"CycloneDX","specVersion":"1.5","version":1,"components":[{"type":"application","name":"sddk","version":"$VERSION","purl":"pkg:generic/sddk@$VERSION"}]}
EOF
ok "checksums + sbom ready (binary sha256: ${BIN_SHA:0:16}…)"

# --- 8b. vault ADR mirror sync (best-effort, fail-soft) ---
#
# INC-VAULT-MIRROR-AUTO: vault mirrors at
# ~/.sddk-knowledge/sddk-framework/adrs/ must stay in sync with the
# accepted ADRs in docs/architecture/adrs/. Without this step the
# operator must invoke `python3 scripts/mirror_adrs_to_vault.py` manually
# after each release — easy to forget, drifts the human-knowledge source
# from the runtime authority.
#
# The mirror script is idempotent (skips existing mirrors) and the
# vault is human knowledge per AGENTS §2.7 (the repo ADR remains
# canonical). Therefore this step is best-effort:
#   - exit 0 → log counts as ok
#   - non-zero exit → log as warn, do NOT abort the release; the bump
#     commit is the canonical record and is already published
#
# Always executed (also under --skip-tests and --dry-run) — vault sync
# is part of the release contract, not the test gate.

step "8b/14 — vault ADR mirror sync (closes INC-VAULT-MIRROR-AUTO)"
if MIRROR_OUT="$(python3 "$ROOT/scripts/mirror_adrs_to_vault.py" 2>&1)"; then
    ok "vault mirror sync: $(echo "$MIRROR_OUT" | tr '\n' ' ')"
else
    warn "vault mirror sync failed (non-fatal — repo ADR is canonical): $MIRROR_OUT"
fi

if [ "$DRY_RUN" = "1" ]; then
    ok "dry-run: stopping before gh release create"
    echo
    echo "Assets staged in $TMP:"
    ls -la "$TMP"
    exit 0
fi

# --- 9. publish ---

step "9/14 — gh release create $TAG"
# Anchor the release to the current branch (not the tag SHA). `gh release
# create --target` accepts a branch name or tag name; passing the raw SHA
# of HEAD fails with HTTP 422 ("Release.target_commitish is invalid")
# because that SHA has no ref yet. Using the current branch lets `gh`
# resolve to HEAD automatically and avoids the manual `gh release edit
# --target` repoint that v1.89.7 needed.
RELEASE_TARGET="$(git rev-parse --abbrev-ref HEAD)"
ok "release target: $RELEASE_TARGET"

# ARCH-HEX-001 slice 3: GitHub Releases is a System-only writable surface
# (ADR-069 §3, row 7). Emit an actor-kind authority receipt via
# scripts/release-receipt.sh and fail-closed if the actor is not System.
# The helper applies the locked v1.81.x prefix heuristic
# (crates/sddk-engine/src/authority.rs::infer_actor_kind) and writes a
# JSON receipt that downstream tooling can consume. Same shape as the
# engine-side checks introduced in v1.168.22 (apply_cycle_start) and
# v1.168.24 (sddk dev install).
RECEIPT_PATH="$TMP/gh-release-receipt.json"
RECEIPT_ACTOR="${SDDK_ACTOR:-system}"
bash "$ROOT/scripts/release-receipt.sh" \
    --actor-id "$RECEIPT_ACTOR" \
    --tag "$TAG" \
    --out "$RECEIPT_PATH" \
    || die "github_releases authority check failed (actor_kind must be System); refusing to publish"
ok "github_releases receipt: $RECEIPT_PATH (actor=$RECEIPT_ACTOR)"

RECEIPT_SUMMARY="$(python3 -c '
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
ak = data["actor_kind"]
aid = data["actor_id"]
sv = data["schema_version"]
print("actor_kind=" + ak + " actor_id=" + aid + " schema_version=" + str(sv))
' "$RECEIPT_PATH")"

RELEASE_ARGS=(
    "$TAG"
    --repo "$REPO"
    --target "$RELEASE_TARGET"
    --title "sddk $TAG"
    --notes "Release $TAG — published by scripts/release.sh.

ARCH-HEX-001 receipt: ${RECEIPT_SUMMARY}"
)
# Note: --clobber is NOT supported on `gh release create` in gh <2.99 (only on `upload`).
# We pass --clobber to `gh release upload` below, which is the path that actually needs it.
# Forcing re-create of an existing release is handled by deleting first.

ASSETS=(
    "$BIN"
    "$TMP/$(basename "$BIN").sha256"
    "$TMP/CHECKSUMS"
    "$TMP/sbom.json"
    "$UNIFIED"
    "$UNIFIED.sha256"
    "$BUNDLE_TARBALL"
    "$BUNDLE_TARBALL.sha256"
    "$RECEIPT_PATH"
)
# If step 1d ran, include the EXT-RECEIPT.md as a release asset so the
# provider-exercised evidence ships alongside the binary. The release
# gate (step 9b) does not poll this asset — it is evidence-of-record,
# not part of the 9-asset public-release contract — but adding it here
# keeps the EXT profile auditable from the GitHub Release page itself.
if [ -n "$EXT_RECEIPT_DIR" ] && [ -f "$EXT_RECEIPT_DIR/EXT-RECEIPT.md" ]; then
    ASSETS+=("$EXT_RECEIPT_DIR/EXT-RECEIPT.md")
fi

if gh release view "$TAG" --repo "$REPO" \
        >/dev/null 2>&1; then
    if [ "$FORCE" = "1" ]; then
        gh release upload "$TAG" --repo "$REPO" \
            --clobber "${ASSETS[@]}" \
            || die "gh release upload --clobber failed"
    else
        die "release $TAG already exists — pass --force to overwrite"
    fi
else
    gh release create "${RELEASE_ARGS[@]}" "${ASSETS[@]}" \
        || die "gh release create failed"
fi
ok "release $TAG published"

# --- 9b. public-release gate ---
# >>> REL-1 public-release gate begin >>>
# Closes FU-A4-4A-REL-1. Before the install step, assert the GitHub
# Release is publicly distributable. Fail closed on any mismatch.
#
# Skipped under --dry-run (no publish happened) and --skip-install
# (no install will run, so the gate is moot).
if [ "$DRY_RUN" = "1" ]; then
    warn "skipping step 9b (--dry-run)"
elif [ "$SKIP_INSTALL" = "1" ]; then
    warn "skipping step 9b (--skip-install)"
else
    step "9b/14 — public-release gate for $TAG"
    require jq

    # 1. Tag SHA anchoring: refs/tags/$TAG must equal the release commit
    #    we just published. `main` will keep moving; the tag is durable.
    EXPECTED_RELEASE_SHA="$(git rev-parse HEAD)"
    ACTUAL_TAG_SHA="$(git ls-remote origin "$TAG" | awk '{print $1}')"
    if [ -z "$ACTUAL_TAG_SHA" ]; then
        die "tag $TAG not found on origin — refusing to install"
    fi
    if [ "$EXPECTED_RELEASE_SHA" != "$ACTUAL_TAG_SHA" ]; then
        die "tag $TAG SHA drift: HEAD=$EXPECTED_RELEASE_SHA tag=$ACTUAL_TAG_SHA — refusing to install"
    fi
    ok "tag SHA anchored: $ACTUAL_TAG_SHA"

    # 2. Release metadata: query gh release view, parse JSON, assert state.
    #    isDraft=false, isPrerelease=false, tagName=expected, asset set
    #    equals the canonical 9-asset contract.
    RELEASE_JSON="$(gh release view "$TAG" --repo "$REPO" --json tagName,isDraft,isPrerelease,assets 2>/dev/null)" \
        || die "gh release view $TAG failed — refusing to install"
    GOTTEN_TAG="$(echo "$RELEASE_JSON" | jq -r '.tagName')"
    if [ "$GOTTEN_TAG" != "$TAG" ]; then
        die "tagName drift: expected $TAG got $GOTTEN_TAG"
    fi
    ok "tagName match: $TAG"

    IS_DRAFT="$(echo "$RELEASE_JSON" | jq -r '.isDraft')"
    if [ "$IS_DRAFT" != "false" ]; then
        die "release $TAG is in draft state (isDraft=$IS_DRAFT) — run: gh release edit $TAG --draft=false"
    fi
    ok "isDraft=false"

    IS_PRERELEASE="$(echo "$RELEASE_JSON" | jq -r '.isPrerelease')"
    if [ "$IS_PRERELEASE" != "false" ]; then
        die "release $TAG is a prerelease (isPrerelease=$IS_PRERELEASE) — Base releases must be non-prerelease"
    fi
    ok "isPrerelease=false"

    # 3. Asset contract: exactly the 9 canonical assets by basename.
    CANONICAL_ASSETS=(
        "sddk"
        "sddk.sha256"
        "sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz"
        "sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz.sha256"
        "CHECKSUMS"
        "sbom.json"
        "gh-release-receipt.json"
        "software-development-decision-kernel.tar.gz"
        "software-development-decision-kernel.tar.gz.sha256"
    )
    ACTUAL_ASSETS="$(echo "$RELEASE_JSON" | jq -r '.assets[].name' | sort -u)"
    EXPECTED_ASSETS_SORTED="$(printf '%s\n' "${CANONICAL_ASSETS[@]}" | sort -u)"
    MISSING_ASSETS="$(comm -23 <(echo "$EXPECTED_ASSETS_SORTED") <(echo "$ACTUAL_ASSETS"))"
    EXTRA_ASSETS="$(comm -13 <(echo "$EXPECTED_ASSETS_SORTED") <(echo "$ACTUAL_ASSETS"))"
    if [ -n "$MISSING_ASSETS" ]; then
        die "missing canonical assets: $(echo "$MISSING_ASSETS" | tr '\n' ' ')"
    fi
    if [ -n "$EXTRA_ASSETS" ]; then
        die "unexpected assets replacing canonical ones: $(echo "$EXTRA_ASSETS" | tr '\n' ' ')"
    fi
    ok "asset set matches 9-asset contract"

    # 4. Public URL HTTP probes: each canonical asset must respond
    #    200 from the public releases/download/$TAG/<asset> URL.
    #    CDN refresh can lag a few minutes after `gh release create`.
    #    Per-asset budget: 60s (6 attempts × 10s). Across 9 assets
    #    the worst case is ~9 minutes if every asset is stale. In
    #    practice the CDN catches up within seconds; this is a safety
    #    net, not a retry loop.
    URL_FAILS=""
    for asset in "${CANONICAL_ASSETS[@]}"; do
        URL="https://github.com/$REPO/releases/download/$TAG/$asset"
        ok_remote=0
        last_rc="000"
        for i in $(seq 1 6); do
            rc="$(curl -fsSL -o /dev/null -w '%{http_code}' "$URL" 2>/dev/null || echo "000")"
            last_rc="$rc"
            if [ "$rc" = "200" ]; then
                ok_remote=1
                break
            fi
            sleep 10
        done
        if [ "$ok_remote" = "0" ]; then
            URL_FAILS="$URL_FAILS $asset(HTTP $last_rc)"
        fi
    done
    if [ -n "$URL_FAILS" ]; then
        die "public URL probes failed after 60s-per-asset budget:$URL_FAILS"
    fi
    ok "9/9 canonical assets reachable from public CDN (HTTP 200)"
    ok "public-release gate PASS"
fi
# <<< REL-1 public-release gate end <<<

if [ "$SKIP_INSTALL" = "1" ]; then
    warn "skipping step 10-13 (--skip-install)"
    exit 0
fi

# --- 10. install from real GH URL ---

step "10/14 — install from GitHub Release URL"
# Defense against GH CDN caching: the URL may serve a stale tarball for
# up to a few minutes after upload. We poll the binary sha256 until it
# matches what we just uploaded, with a 5-minute budget.
EXPECTED_SHA="$(sha256sum "$BIN" | awk '{print $1}')"
URL_BIN="https://github.com/$REPO/releases/download/$TAG/$(basename "$BIN")"
ATTEMPTS=30
SLEEP_SECS=10
for i in $(seq 1 "$ATTEMPTS"); do
    ACTUAL="$(curl -fsSL "$URL_BIN" 2>/dev/null | sha256sum | awk '{print $1}' || true)"
    if [ "$ACTUAL" = "$EXPECTED_SHA" ]; then
        ok "CDN served correct binary sha256 after $((i * SLEEP_SECS))s"
        break
    fi
    if [ "$i" = "$ATTEMPTS" ]; then
        die "CDN still serving stale binary after $((ATTEMPTS * SLEEP_SECS))s — refusing to install"
    fi
    warn "CDN stale (got ${ACTUAL:-empty}, want ${EXPECTED_SHA:0:16}…) — retry $i/$ATTEMPTS"
    sleep "$SLEEP_SECS"
done

unset SDDK_BASE_URL SDDK_VERSION
export SDDK_PREFIX="/home/rubentxu/.local/bin"
export SDDK_FRAMEWORK_DIR="/home/rubentxu/.local/share/sddk/framework"
export SDDK_EDITOR="all"
bash scripts/install.sh --version "$TAG" --editor all \
    || die "install.sh failed"

# --- 11. doctor ---

step "11/14 — sddk dev doctor --prefix $SDDK_PREFIX"
DOCTOR_OUT="$("$SDDK_PREFIX/sddk" dev doctor --prefix "$SDDK_PREFIX" --format text)"
echo "$DOCTOR_OUT" | grep -E "binary\.bundle_coherence|^all_present" \
    || die "doctor output missing expected checks"
echo "$DOCTOR_OUT" | grep -q "binary\.bundle_coherence: present" \
    || die "binary.bundle_coherence not present"
echo "$DOCTOR_OUT" | grep -q "all_present: true" \
    || die "all_present is not true"
ok "binary.bundle_coherence: present, all_present: true"

# --- 12. prune ---

step "12/14 — sddk dev update --prune-only --keep 1"
"$SDDK_PREFIX/sddk" dev update --prune-only --keep 1 \
    --root "$SDDK_FRAMEWORK_DIR" --format text \
    || die "prune failed"
ok "stale bundles pruned"

# --- 13. re-install from distrib (smoke test the published artefact) ---

step "13/14 — re-install from URL (distrib smoke test)"
# After step 11 (install from real URL) and step 12 (prune stale bundles),
# re-run install.sh --editor none against the same GH URL to confirm the
# published artefact round-trips through `sddk dev install` + `dev use`
# + `dev link` without any housekeeping needed. This is the same code
# path that an external user would take after `gh release create`. We
# pass --editor none to avoid relinking editors every release; the
# doctor check at the end confirms the install was successful.
SDDK_REPO="$REPO" \
SDDK_VERSION="$TAG" \
SDDK_BASE_URL="${SDDK_BASE_URL:-https://github.com/$REPO/releases}" \
SDDK_EDITOR="none" \
bash "$REPO_ROOT/scripts/install.sh" \
    --version "$TAG" \
    --prefix "$SDDK_PREFIX" \
    --editor none \
    || die "re-install from distrib failed; the published artefact is broken"
ok "distrib round-trip OK (binary + bundle coherent after prune)"

# --- 14. final state ---

step "14/14 — final state"
echo
BIN_VER="$("$SDDK_PREFIX/sddk" --version 2>&1 | head -1)"
BUNDLE_VER="$("$SDDK_PREFIX/sddk" dev doctor --prefix "$SDDK_PREFIX" --format json 2>/dev/null \
    | python3 -c '
import json, sys
data = json.load(sys.stdin)
# binary.bundle_coherence lives under checks[]; the bundle version itself
# comes from the receipt (sddk-install.json).
print(json.loads(open("'"$SDDK_PREFIX"'/sddk-install.json").read()).get("bundle_version", "?"))
' 2>/dev/null || echo "?")"
CURRENT_VER="$(basename "$(readlink "$SDDK_FRAMEWORK_DIR/current" 2>/dev/null || echo "?")")"
echo "  binary:        $BIN_VER"
echo "  bundle:        $BUNDLE_VER"
echo "  current:       $CURRENT_VER"
echo "  framework/:"
find "$SDDK_FRAMEWORK_DIR" -mindepth 1 -maxdepth 1 -printf '    %f\n' | sort
echo
ok "release $TAG shipped and installed locally"
