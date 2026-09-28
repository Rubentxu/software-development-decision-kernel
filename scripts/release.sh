#!/usr/bin/env bash
# release.sh — Canonical end-to-end release flow for sddk-framework.
#
# Standardized after cycle-46 (install coherence) and cycle-47 (install
# consolidation). Every release MUST pass through this script — or the
# manual equivalent — so that local install stays in lockstep with what
# ships through GitHub Releases.
#
# ── PIPELINE AUTHORITY (read this before changing asset names) ────────────
#
# `.github/workflows/release.yml` is a manually dispatched distribution
# workflow. `release-automation.yml` dispatches it after pushing a release
# tag, so it is an active publisher path even though it is not the local
# SDDK release gate. This script remains the canonical local release gate.
#
# Both paths share `scripts/release-assets-contract.sh`. The local path
# publishes the canonical 9 payloads; the Actions path stages those same 9
# plus three explicitly allowlisted non-x86 unified packages. It signs all
# installable payloads before its single publish job, writes the authority
# receipt through `release-receipt.sh`, and runs the same asset contract plus
# the end-user installer smoke test. The workflow contract is pinned by
# `tests/test_release_pipeline_consistency.sh` and
# `tests/test_release_ci_contract.sh`.
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
source "$ROOT/scripts/release-assets-contract.sh"

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
# [workspace.package] version change. The commit subject may follow the
# `chore(release): bump version` convention, but the subject alone is NEVER
# the contract: an empty ceremonial marker commit is refused here, so it can
# never be mistaken for a real release.
#
# The comparison is against the LAST PUBLISHED TAG, not against HEAD^.
# This is the v2 check, and it is the default here because the v1 check
# (HEAD vs HEAD^) is wrong for any release whose bump commit is not HEAD.
# The bump legitimately sits N commits behind HEAD: docs, journal and
# pointer commits land after it, by this repo's own convention. With v1,
# HEAD and HEAD^ then carry the SAME version and the gate refuses a
# perfectly valid release with `REJECT non-monotonic 2.1.0 -> 2.1.0`.
# The v1 flaw was documented in scripts/lib/release_admission.sh but the
# publishing path never opted in, so it stayed latent until a release
# actually came due.
#
# v2 fails CLOSED on an unreachable or unauthenticated remote
# (`REJECT query-failed`) rather than degrading to the HEAD^ comparison.
#
# Single source of the invariant: `scripts/lib/release_admission.sh`.
# shellcheck source=lib/release_admission.sh
# shellcheck disable=SC1091
. "$ROOT/scripts/lib/release_admission.sh"

LAST_SUBJECT="$(git log -1 --format=%s)"
ADMISSION="$(release_admission_check_v2 HEAD)" \
    || die "release admission refused: $ADMISSION — release requires a real, monotonic [workspace.package] version bump above the last published release"
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
            tests/test_release_ci_contract.sh \
            tests/test_release_pipeline_consistency.sh \
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
             tests/test_release_ci_manifest_anchor.sh \
             tests/test_release_ci_staging.sh \
             tests/test_release_ci_contract.sh \
             tests/test_release_pipeline_consistency.sh \
             tests/falsify-ci-anchor-real.sh \
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
#
# INC-DEBT-021: el nombre del asset decía "musl" y el contenido era glibc.
# La causa era que este script compilaba con `cargo build --release` — el
# target del host — y luego empaquetaba el resultado con nombre musl.
#
# Aqui se compila con el target musl REAL. El binario resultante es
# static-pie: no necesita glibc y corre en cualquier distro, incluidos
# Alpine y Debian 12 (verificado en session-19).
#
# El target es configurable porque no todos los hosts de release tienen el
# toolchain. El default es musl porque es lo que el nombre del asset
# promete. Si se pide musl y el toolchain no esta, el script ABORTA: es
# preferible no publicar a publicar un binario con el nombre equivocado.
# Esa era exactamente la mentira que INC-021 documentaba.

step "3/14 — cargo build --release --bin sddk"

BUILD_TARGET="${SDDK_RELEASE_BUILD_TARGET:-x86_64-unknown-linux-musl}"

if [ "$BUILD_TARGET" != "x86_64-unknown-linux-musl" ]; then
    warn "SDDK_RELEASE_BUILD_TARGET=$BUILD_TARGET: el nombre del asset dice musl pero el target no es musl"
    warn "esto reintroduce INC-021. Se requiere una decision explicita del operador."
fi

if ! rustup target list --installed 2>/dev/null | grep -qx "$BUILD_TARGET"; then
    die "el target $BUILD_TARGET no esta instalado (rustup target add $BUILD_TARGET).
         release.sh publica assets musl; compilar contra otro target rompe el
         contrato del installer. Para publicar de otro modo, cambia tambien el
         nombre del asset y el contrato de install.sh. Ver INC-DEBT-021."
fi

cargo build --release --offline --bin sddk --target "$BUILD_TARGET" \
    || die "cargo build failed para target $BUILD_TARGET"

# Locate the binary via cargo metadata so we respect CARGO_TARGET_DIR.
TARGET_DIR="$(cargo metadata --format-version 1 --offline \
        | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' \
        || true)"
BIN="$TARGET_DIR/$BUILD_TARGET/release/sddk"
[ -x "$BIN" ] || die "binary not found at $BIN"

# Verificar que el binario es REALMENTE lo que el nombre del asset promete.
# Un binario dinamico en un asset llamado musl es INC-021 reincidente y
# tiene que abortar ANTES de publicar, no despues. `file` es la fuente.
#
# INC-021-FALSE-NEGATIVE (session-30): este guard solo buscaba el literal
# "statically linked", pero `file` 5.46 describe un binario static-pie como
#
#   ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped
#
# sin la palabra "statically". El guard por tanto RECHAZABA el binario musl
# correcto — el fallo era del guard, no del toolchain. Se aceptan las dos
# grafias ("statically linked" y "static-pie linked"), que son las dos
# formas de un binario sin dependencias dinamicas, y se sigue rechazando
# cualquier "dynamically linked".
if [ "$BUILD_TARGET" = "x86_64-unknown-linux-musl" ]; then
    FILE_DESC="$(file -b "$BIN")"
    if printf '%s' "$FILE_DESC" | grep -qE 'statically linked|static-pie linked'; then
        ok "binario verificado estatico: $FILE_DESC"
    else
        die "el target de build es musl pero el binario NO es estatico.
         file dice: $FILE_DESC
         Publicar esto seria reincidir en INC-021. Verificar el linker
         (musl-tools / CC=musl-gcc) antes de reintentar."
    fi
fi
ok "binary: $BIN ($("$BIN" --version)), target=$BUILD_TARGET"

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
# The manifest's OWN sha256, not the hash of its first line.
#
# `awk 'NR==1 {print $1}' MANIFEST.sha256` returned the sha256 of the first
# FILE listed in the manifest (e.g. agents/analytics-judge.md), which has
# nothing to do with the manifest itself — the field then named one bundled
# file while claiming to be the manifest digest. The consumer's own doc
# comment states the intent: "sha256 of MANIFEST.sha256 itself"
# (crates/sddk-cli/src/dev/bundle_manifest.rs:24).
#
# Nothing validated the old value (install.sh never reads it; the Rust side
# parses it as an Option and never compares), so the field was inert AND
# wrong. Fixing it changes a published value, which is safe for that
# reason. See INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE.
MANIFEST_SHA="$(sha256sum MANIFEST.sha256 | awk '{print $1}')"
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

# --- 8c. cosign signatures (authenticity) ---
#
# INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY option (a). install.sh and
# `sddk dev update` now refuse an unsigned artifact unless the operator
# opts out explicitly. This step is what makes the default path work: the
# release publishes the signatures both consumers require.
#
# Sign the three consumable artifacts: the bare binary, the unified
# tarball, and the bundle tarball. Signatures travel as `.sig` (detached)
# AND `.bundle.json` (the current cosign format), because the two
# consumers probe for both — a signature published in only one format
# would be a coin flip on which path works.
#
# KEYLESS IS THE DEFAULT and the only option that avoids a long-lived
# signing key on the release host. It needs OIDC: in CI that is
# ACTIONS_ID_TOKEN_REQUEST; locally cosign falls back to a device flow
# that requires a human to open a browser. If neither is available, the
# release ABORTS rather than shipping unsigned artifacts whose consumers
# will reject — publishing artifacts that the installer refuses is
# strictly worse than failing here.
step "8c/14 — cosign signatures"

# Pull the base64 signature out of a cosign bundle so a detached `.sig`
# can be published alongside it. The bundle is the current format; the
# detached form is what older cosign reads via `--signature`. Publishing a
# file that merely *looks* like a signature would be worse than publishing
# none: it either fails verification confusingly or, in the worst case,
# gets treated as valid.
extract_detached_sig() {
    local bundle_path="$1" out_path="$2"
    python3 - "$bundle_path" "$out_path" <<'PY'
import base64, json, sys
bundle = json.load(open(sys.argv[1]))
sig = (bundle.get("base64Signature")
       or bundle.get("signedBlob", {}).get("signature")
       or bundle.get("dsseEnvelope", {}).get("payload"))
if not sig:
    sys.exit(1)
open(sys.argv[2], "wb").write(base64.b64decode(sig))
PY
}

# The OIDC issuer the certificate MUST carry. Byte-identical to
# DEFAULT_CERT_ISSUER in crates/sddk-cli/src/cosign.rs, which is the single
# source of truth both consumers verify against. If these drift, the release
# signs with one identity and the installers demand another.
#
# Why this is a hard failure and not a warning: INC-DEBT-024. The keyless
# identity depends on WHERE you sign, and the two are mutually exclusive —
# GitHub Actions mints issuer token.actions.githubusercontent.com, the local
# device flow mints oauth2.sigstore.dev/auth with a PERSON's subject. cosign
# is installed either way, so `command -v cosign` passes locally and the
# signing step runs. The result is a release that looks correctly signed and
# that this project's own install.sh refuses, because the pin cannot match.
# Publishing that is the worst outcome available: the failure surfaces at the
# user's install, with a signature error that reads like tampering.
RELEASE_CERT_ISSUER="${SDDK_COSIGN_ISSUER:-https://token.actions.githubusercontent.com}"

# Extract the issuer from the certificate cosign just minted, and refuse to
# continue if it is not the pinned one.
#
# The certificate lives inside the bundle: v2 bundle format carries it as
# base64 in `.cert` (confirmed in cosign v2.4.3 sign_blob.go, which is what
# CI installs via cosign-installer v3.8.1's default `cosign-release`).
# Anything unparseable is a FAILURE, never a pass: a check that cannot read
# the certificate has not verified anything, and treating that as success is
# how a control silently stops controlling.
cert_issuer() {
    local bundle_path="$1"
    python3 - "$bundle_path" <<'PY'
import base64, json, re, subprocess, sys, tempfile, os

try:
    bundle = json.load(open(sys.argv[1]))
except Exception as exc:
    sys.exit(f"unreadable bundle: {exc}")

raw = None
# v2 bundle: base64 PEM in `.cert`.
cert = bundle.get("cert")
if cert:
    try:
        raw = base64.b64decode(cert)
    except Exception:
        raw = None

# New bundle format: the certificate chain is already PEM inside
# `verificationMaterial.x509CertificateChain.certificates[].rawBytes`.
if raw is None:
    certs = (bundle.get("verificationMaterial") or {}).get(
        "x509CertificateChain", {}).get("certificates", [])
    for entry in certs:
        candidate = entry.get("rawBytes")
        if candidate:
            try:
                raw = base64.b64decode(candidate)
                break
            except Exception:
                continue

if not raw:
    sys.exit("no certificate found in bundle")

with tempfile.NamedTemporaryFile("wb", suffix=".pem", delete=False) as fh:
    fh.write(raw)
    pem = fh.name
try:
    out = subprocess.run(
        ["openssl", "x509", "-noout", "-issuer"],
        input=raw, capture_output=True, check=True).stdout.decode()
finally:
    os.unlink(pem)

# `issuer=CN=...,O=...` or a URI SAN. The OIDC issuer rides in the SAN as
# a URI, so read that rather than the RFC4514 DN, which carries the Fulcio
# CA's name and not the OIDC provider.
sans = subprocess.run(
    ["openssl", "x509", "-noout", "-ext", "subjectAltName"],
    input=raw, capture_output=True).stdout.decode()
m = re.search(r"URI:(\S+)", sans)
if m:
    print(m.group(1))
else:
    print(out.strip())
PY
}

SIGN_ARTIFACTS=(
    "$(basename "$BIN")"
    "$(basename "$UNIFIED")"
    "$(basename "$BUNDLE_TARBALL")"
)

# --- Pre-check: is the project identity even reachable from here? ---
#
# INC-DEBT-024 mitigacion 2. The issuer gate below is the real defense, but
# it runs AFTER cosign has already signed, which means two bad things happen
# first when you sign from a laptop:
#
#   1. cosign falls back to the interactive device flow, which blocks on a
#      human opening a browser. In an unattended run that is a hang, not a
#      failure.
#   2. If the operator does complete it, the certificate belongs to a PERSON
#      and the release is still wrong — the gate then rejects it, but only
#      after the work and the browser round-trip.
#
# So refuse early, with a message that says where the identity does exist.
# Detect it the way GitHub sets it: `GITHUB_ACTIONS=true` on the runner.
#
# `SDDK_ALLOW_LOCAL_SIGNING=1` is NOT an escape hatch to a good release — it
# only reaches the issuer gate, which will still reject a personal identity.
# It exists so the check can be falsified without a live Actions runner.
if [ "${SDDK_SKIP_SIGNING:-0}" != "1" ] && [ "${SDDK_ALLOW_LOCAL_SIGNING:-0}" != "1" ]; then
    if [ "${GITHUB_ACTIONS:-}" != "true" ]; then
        die "the project's signing identity does not exist on this host.
               Required issuer: $RELEASE_CERT_ISSUER
               This host: not a GitHub Actions runner (GITHUB_ACTIONS != true).

               Keyless signing mints a certificate for whatever identity the
               OIDC provider sees. Inside GitHub Actions that is the workflow
               running on a tag, which is what install.sh and \`sddk dev
               update\` pin. On a workstation cosign instead asks a human to
               open a browser and mints a certificate for that PERSON — which
               the installers reject, so the release would be signed and
               uninstallable.

               Run the release from GitHub Actions (see
               .github/workflows/release-automation.yml) so the identity is
               the project's. To publish fully unsigned on purpose, set
               SDDK_SKIP_SIGNING=1 — the installers will then require
               SDDK_ALLOW_UNSIGNED=1 / SDDK_ALLOW_UNSIGNED_UPDATE=1."
    fi
    ok "signing context: GitHub Actions runner (project identity available)"
fi

SIGNED_COUNT=0
if command -v cosign >/dev/null 2>&1; then
    for artifact in "${SIGN_ARTIFACTS[@]}"; do
        src="$TMP/$artifact"
        [ -f "$src" ] || { warn "artifact missing for signing: $artifact"; continue; }
        # New bundle format is the current one. The detached `.sig` is
        # extracted FROM the bundle so it is a real signature, not a copy of
        # the artifact: consumers on older cosign read `--signature`, and a
        # file that merely looks like a signature would either fail
        # verification (confusing) or, worse, be treated as valid.
        if cosign sign-blob --yes --new-bundle-format \
             --bundle "$TMP/$artifact.bundle.json" "$src" 2>"$TMP/sign-$artifact.log"; then
            extract_detached_sig "$TMP/$artifact.bundle.json" "$TMP/$artifact.sig" \
                || warn "could not extract detached .sig from bundle for $artifact"
            ok "signed: $artifact (.sig + .bundle.json)"
            SIGNED_COUNT=$((SIGNED_COUNT + 1))
        else
            warn "cosign could not sign $artifact:"
            sed 's/^/    /' "$TMP/sign-$artifact.log" | head -5
        fi
    done

    # Identity gate (INC-DEBT-024). Signing succeeding is NOT the same as
    # signing with the identity this project pins. Check the issuer of the
    # certificate that was actually minted, before anything is uploaded.
    #
    # One artifact is enough to establish it: every signature in this release
    # comes from the same `cosign sign-blob` invocation shape in the same
    # loop, so they share the identity. Checking all three would be theatre.
    if [ "$SIGNED_COUNT" -gt 0 ]; then
        first_artifact="${SIGN_ARTIFACTS[0]}"
        if emitted_issuer=$(cert_issuer "$TMP/$first_artifact.bundle.json"); then
            if [ "$emitted_issuer" = "$RELEASE_CERT_ISSUER" ]; then
                ok "signing identity verified: issuer $emitted_issuer"
            else
                die "signed with the WRONG identity.
               expected issuer: $RELEASE_CERT_ISSUER
               actual issuer:   $emitted_issuer

               This release would be published signed, but unusable: install.sh
               and \`sddk dev update\` pin the issuer above and would refuse every
               artifact, with a signature error that reads like tampering.

               The keyless identity depends on WHERE you sign. GitHub Actions
               mints a certificate for the workflow ($RELEASE_CERT_ISSUER);
               the local device flow mints one for a PERSON. cosign being
               installed is not evidence that the right identity was used.

               Refusing to publish. To sign with the project identity, run the
               release from GitHub Actions (release-automation.yml). To publish
               unsigned on purpose, set SDDK_SKIP_SIGNING=1."
            fi
        else
            die "could not read the signing identity from $first_artifact.bundle.json.

               A check that cannot read the certificate has verified nothing,
               and reporting success here is how a control silently stops
               controlling. Refusing to publish."
        fi
    fi

    # All-or-nothing. A partially signed release is worse than an unsigned
    # one: the binary verifies while the bundle tarball does not, so the
    # failure lands on the user at install time instead of here. The count
    # is compared against the size of SIGN_ARTIFACTS, not against zero.
    if [ "$SIGNED_COUNT" -ne "${#SIGN_ARTIFACTS[@]}" ]; then
        die "signed $SIGNED_COUNT of ${#SIGN_ARTIFACTS[@]} artifacts. Refusing to publish a partial set: a release whose bundle tarball has no signature is a release that install.sh refuses, and the user finds out instead of us.

         To sign, provide an OIDC identity. In GitHub Actions it is automatic
         (id-token: write). Locally, cosign needs the browser device flow:
           cosign sign-blob --yes --bundle <file>.bundle.json <file>
         Or set SDDK_SKIP_SIGNING=1 to publish fully unsigned on purpose — the
         installers will then require SDDK_ALLOW_UNSIGNED=1 / SDDK_ALLOW_UNSIGNED_UPDATE=1."
    fi
    ok "$SIGNED_COUNT/${#SIGN_ARTIFACTS[@]} artifacts signed"
else
    if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ]; then
        warn "SDDK_SKIP_SIGNING=1 — publishing UNSIGNED artifacts. Both installers"
        warn "will refuse this release unless the operator opts out explicitly."
    else
        die "cosign is not installed and SDDK_SKIP_SIGNING is not set. Both
         installers now require a signature, so an unsigned release would be
         uninstallable. Install cosign, or set SDDK_SKIP_SIGNING=1 to accept
         that knowingly. See INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY."
    fi
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

# Signature assets. Both consumers (install.sh and `sddk dev update`) look
# for `.bundle.json` first and fall back to `.sig`, so both must ship. They
# are additive to the 9-asset canonical contract, not part of it: the
# canonical list below is a public-release gate and its shape is defined
# elsewhere (tests/test_release_public_gate.sh).
for sig_artifact in "${SIGN_ARTIFACTS[@]}"; do
    # .bundle.json already carries the certificate, so the bundle path works
    # on its own. The .pem is published anyway so the DETACHED path works
    # too: the CI signs detached, and both consumers now require a
    # certificate before accepting a detached signature. Shipping only the
    # bundle would make the two signing paths produce releases that are
    # mutually un-verifiable depending on which consumer fetched what.
    for ext in .sig .bundle.json .pem; do
        if [ -f "$TMP/$sig_artifact$ext" ]; then
            ASSETS+=("$TMP/$sig_artifact$ext")
        fi
    done
done

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

    # 3. Asset contract. Keep the production rule in one place and exercise
    #    the exact same function from tests/lib_public_release_gate.sh.
    validate_release_asset_contract "$RELEASE_JSON" "${TAG#v}"
    CANONICAL_ASSETS=("${RELEASE_CANONICAL_ASSETS[@]}")


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
