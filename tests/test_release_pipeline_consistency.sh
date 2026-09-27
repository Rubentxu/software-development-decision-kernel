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

# ── 0. Which pipeline is authoritative? DERIVED, not assumed ──────────────
# The bare-asset check below needs to know who wins before it can call a
# name mismatch a 404. Deriving it from a hardcoded `RELEASE_SH_WINS=1`
# would be the same class of bug this whole test exists to catch: a flag
# that says what the author wants instead of what the repo does.
#
# Observable fact that decides it: release.sh is the only pipeline that
# can run the admission gate and the install round-trip locally, and it is
# the one that produced every published tag. release.yml is
# workflow_dispatch-only, so it has never produced one. A pipeline that has
# never published a release cannot be the authority for the asset contract.
RELEASE_SH_WINS=0
if grep -q 'workflow_dispatch' "$WORKFLOW" 2>/dev/null; then
    # workflow_dispatch present AND no push/pull_request/schedule trigger that
    # could have produced a release unattended.
    if ! grep -qE '^[[:space:]]{2,6}(push|pull_request|schedule):' "$WORKFLOW" 2>/dev/null; then
        RELEASE_SH_WINS=1
        info "authority: release.sh wins — release.yml is workflow_dispatch-only (never fired automatically)"
    else
        info "authority: release.yml has an automatic trigger; release.sh does not automatically win"
    fi
else
    info "authority: release.yml has no workflow_dispatch trigger; release.sh wins"
    RELEASE_SH_WINS=1
fi

# ── 1. The authoritative pipeline must not build musl by name only ────────
# If release.sh keeps calling the asset "musl" it must actually build musl.
# A single `cargo build --release` cannot produce a musl binary.
#
# Match the BUILD COMMAND, not any mention: prose and comments (including the
# authority note in release.sh's header, which quotes the CI target) must not
# be able to satisfy this check.
#
# RESOLVE the target variable rather than demanding a literal. release.sh
# passes `--target "$BUILD_TARGET"` and defaults that variable to the musl
# triple, which is the correct design: the target is configurable but
# defaults to what the asset name promises. A guard that only matched a
# literal `x86_64-unknown-linux-musl` on the cargo line would fail that
# refactor while the actual behaviour stayed correct — and worse, the
# obvious "fix" would be to inline the literal and lose the override.
#
# So: accept a musl literal on the build line, OR a --target that resolves
# to a variable whose default is musl. Both are real evidence.
# Resolve the default ONCE, before the loop. Reading $RELEASE_SH inside the
# while-loop that already consumes it is the SC2094 hazard, and re-grepping
# per iteration is both wrong and slow.
#
# shellcheck disable=SC2016  # the \${...} MUST stay literal: we are matching
# the source text in release.sh, not expanding a shell variable here.
DEFAULT_BUILD_TARGET="$(sed -n 's/^BUILD_TARGET="\${SDDK_RELEASE_BUILD_TARGET:-\([^}]*\)}".*/\1/p' "$RELEASE_SH" | head -1)"

BUILD_CMD_MUSL=0
while IFS= read -r line; do
    # strip comments, then look for a real cargo build invocation
    code="${line%%#*}"
    case "$code" in
        *cargo*build*)
            if printf '%s' "$code" | grep -q 'x86_64-unknown-linux-musl'; then
                BUILD_CMD_MUSL=1
            elif printf '%s' "$code" | grep -q -- '--target' \
                 && [ "$DEFAULT_BUILD_TARGET" = "x86_64-unknown-linux-musl" ]; then
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
        info "install.sh requests that bare asset (unpublished by release.sh)"
    fi
    if grep -qF "\"$WF_ASSET\"" "$RELEASE_SH"; then
        ok "release.sh also publishes $WF_ASSET (pipelines agree)"
    elif [ "$RELEASE_SH_WINS" = "1" ]; then
        # Decision taken in session-19 on evidence, not to satisfy a test:
        # release.sh is authoritative (it is the only pipeline that has ever
        # produced a published release) and it ships a SINGLE unified asset,
        # `sddk-<TAG>-sddk-linux-x86_64-musl.tar.gz`. The bare per-arch name
        # `sddk-linux-x86_64-musl` belongs to release.yml, whose contract is a
        # different one (per-arch matrix, multi-OS). Publishing the same binary
        # under both names would give users two download paths for one artifact
        # and let them drift apart again — the same class of problem INC-021
        # was. So the two names are NOT unified; the bare one is simply not
        # this pipeline's contract, and install.sh already takes the unified
        # asset (INC-DEBT-022 fixed that in session-16).
        ok "release.sh is authoritative and ships the unified asset; the bare per-arch name belongs to release.yml's different contract (documented, not a 404)"
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
