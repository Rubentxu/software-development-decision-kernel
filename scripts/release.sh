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

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run)      DRY_RUN=1; shift ;;
        --skip-tests)   SKIP_TESTS=1; shift ;;
        --skip-install) SKIP_INSTALL=1; shift ;;
        --force)        FORCE=1; shift ;;
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

# Confirm the last commit matches chore(release): bump version — without it
# the pre-push hook will reject the next push. We refuse early to keep the
# loop tight.
LAST_SUBJECT="$(git log -1 --format=%s)"
if ! echo "$LAST_SUBJECT" | grep -qE '^chore\(release\): bump version'; then
    die "HEAD is not a chore(release) commit: $LAST_SUBJECT"
fi
ok "on main, clean tree, HEAD is a release commit"

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
            tests/test_release_receipt_authority.sh \
            tests/test_authority_helper_lockstep.sh \
            tests/test_adr_promotion_format.sh \
            || die "shellcheck failed"
        ok "shellcheck clean (scope: release-receipt + 3 cross-crate/M9+ tests)"
    else
        warn "shellcheck not installed — skipping static gate (install shellcheck for full coverage)"
    fi
    for t in tests/test_release_receipt_authority.sh \
             tests/test_authority_helper_lockstep.sh \
             tests/test_adr_promotion_format.sh; do
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

# --- 2. version ---

step "2/14 — read version"
VERSION="$(awk '/^\[workspace\.package\]/{flag=1; next} flag && /^version = /{print $3; exit}' Cargo.toml \
    | tr -d '\"')"
TAG="v$VERSION"
[ -n "$VERSION" ] || die "could not parse version from Cargo.toml"
ok "version: $VERSION → tag: $TAG"

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
