#!/usr/bin/env bash
# tests/test_release_pipeline_consistency.sh
#
# The repository has TWO release pipelines and they do not agree:
#
#   A. scripts/release.sh          — AUTHORITATIVE. Runs locally, produces
#                                     the real releases. Compiles ONE binary
#                                     with `cargo build --release` (host glibc)
#                                     and publishes it as
#                                     `sddk-<tag>-sddk-linux-x86_64-musl.tar.gz`.
#   B. .github/workflows/release.yml — manual-only (workflow_dispatch). Builds
#                                     a real static musl binary and publishes
#                                     `sddk-linux-x86_64-musl`.
#
# B is never triggered by a release ("SDDK never depends on CI/CD"). So the
# artifact that the name promises (musl) is never produced by the pipeline
# that actually runs, and the asset name the installer expects comes from
# the pipeline that never runs.
#
# This test pins the three-way contract so the drift is loud:
#
#   install.sh  --requests-->  ?  release.sh  --publishes-->
#   release.yml --builds--------------------^
#
# It does NOT decide which pipeline should win (that is an architecture
# decision, see INC-DEBT-021). It fails when the pipelines disagree in a way
# that breaks users, and it reports the musl claim explicitly.
#
# Exit 0 = consistent. Non-zero = pipelines drifted.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_SH="$ROOT/scripts/install.sh"
RELEASE_SH="$ROOT/scripts/release.sh"
WORKFLOW="$ROOT/.github/workflows/release.yml"

failures=0
ok()   { printf '  ok   %s\n' "$1"; }
fail() { printf '  FAIL %s\n' "$1"; failures=$((failures + 1)); }
info() { printf '  ..   %s\n' "$1"; }

for f in "$INSTALL_SH" "$RELEASE_SH"; do
    [ -f "$f" ] || { echo "missing $f" >&2; exit 2; }
done

echo "=== release pipeline consistency ==="

# ── 1. The authoritative pipeline must not build musl by name only ────────
# If release.sh keeps calling the asset "musl" it must actually build musl.
# A single `cargo build --release` cannot produce a musl binary.
#
# Match the BUILD COMMAND, not any mention: prose and comments (including the
# authority note in release.sh's header, which quotes the CI target) must not
# be able to satisfy this check.
BUILD_CMD_MUSL=0
while IFS= read -r line; do
    # strip comments, then look for a real cargo build invocation
    code="${line%%#*}"
    case "$code" in
        *cargo*build*)
            if printf '%s' "$code" | grep -q 'musl'; then
                BUILD_CMD_MUSL=1
            fi
            ;;
    esac
done < "$RELEASE_SH"

if grep -q 'musl' "$RELEASE_SH"; then
    if [ "$BUILD_CMD_MUSL" = "1" ]; then
        ok "release.sh builds an explicit musl target in a cargo build command"
        BUILD_IS_MUSL=1
    else
        fail "release.sh names a 'musl' asset but no cargo build command uses a musl target"
        BUILD_IS_MUSL=0
    fi
else
    ok "release.sh does not claim a musl asset"
    BUILD_IS_MUSL=0
fi

# ── 2. The two pipelines must agree on the bare-binary asset name ─────────
# release.yml publishes `sddk-linux-x86_64-musl`. If install.sh is going to
# ask for a bare per-arch asset, that asset must be published by whichever
# pipeline is authoritative.
WF_ASSET="$(grep -oE 'asset:[[:space:]]*sddk-[a-z0-9_-]+' "$WORKFLOW" 2>/dev/null | head -1 | awk '{print $2}')"
if [ -n "$WF_ASSET" ]; then
    info "release.yml publishes bare asset: $WF_ASSET"
    if grep -qF "release_url \"$WF_ASSET\"" "$INSTALL_SH"; then
        info "install.sh requests that bare asset (currently unpublished by release.sh)"
    fi
    if grep -qF "\"$WF_ASSET\"" "$RELEASE_SH"; then
        ok "release.sh also publishes $WF_ASSET (pipelines agree)"
    else
        fail "release.yml publishes $WF_ASSET but release.sh does not; installers requesting it get 404"
    fi
else
    info "release.yml bare-asset name not parseable; skipping cross-pipeline name check"
fi

# ── 3. The workflow must not be the only place the contract is defined ───
# If the authoritative pipeline and the CI pipeline can disagree, that is a
# structural hazard. At minimum, the release script must reference the CI
# workflow, so a reader knows both exist and which one wins.
if grep -qF 'release.yml' "$RELEASE_SH"; then
    ok "release.sh acknowledges the CI release workflow"
else
    fail "release.sh never mentions .github/workflows/release.yml — two pipelines, no declared authority"
fi

# ── 4. Report, do not enforce, the musl/glibc reality ───────────────────
# This is a fact about the artifact, not a policy. Print it so it is visible.
if [ "$BUILD_IS_MUSL" = "0" ] && grep -q 'musl' "$RELEASE_SH"; then
    info "REALITY: the published 'musl' asset contains a host-glibc binary."
    info "         Users on glibc < build host cannot run it. See INC-DEBT-021."
fi

echo
if [ "$failures" -ne 0 ]; then
    echo "release pipeline consistency: $failures check(s) FAILED"
    exit 1
fi
echo "release pipeline consistency: all checks passed"
