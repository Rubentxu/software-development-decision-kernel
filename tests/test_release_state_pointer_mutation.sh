#!/bin/bash
# Mutation self-test for tests/test_release_state_pointer.sh (C0 / T01).
#
# WHY THIS EXISTS
# ---------------
# T01 in docs/roadmap/UAT-MATRIX.md requires, verbatim:
#
#   "HEAD, versión Cargo, tag y release comprobados independientes;
#    inyectar puntero stale"
#
# The guard `test_release_state_pointer.sh` checked the REAL pointer and
# reported PASS/FAIL, but had NO scenario that INJECTED a stale pointer. Every
# one of its checks was therefore only ever observed in the passing state.
# A guard whose checks have never been observed failing is an unverified
# claim wearing a checkmark — the same class of defect this repo already hit
# three times in one session (LINT-PATH-1, LINT-HINT-1, WORKFLOW-CONTRACT-1).
#
# This file injects each drift mode the guard claims to detect and REQUIRES the
# guard to reject it. If the guard ever stops detecting one of these, this
# test goes red even though the real repo is perfectly consistent.
#
# Six sibling guards already establish this pattern (test_release_admission.sh,
# test_release_bump_derivation.sh, test_release_ci_staging.sh,
# test_release_pipeline_consistency.sh, test_release_public_gate.sh,
# test_release_receipt_authority.sh). The state pointer guard was the outlier.
#
# SANDBOX ONLY. Temp repos only. No network. Never mutates the real repo.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GUARD="$REPO_ROOT/tests/test_release_state_pointer.sh"

if [[ ! -f "$GUARD" ]]; then
    echo "FAIL: $GUARD missing"
    exit 1
fi

TMPROOT=$(mktemp -d)
cleanup() {
    local code=$?
    chmod -R u+rw "$TMPROOT" 2>/dev/null || true
    rm -rf "$TMPROOT"
    exit "$code"
}
trap cleanup EXIT

PASS=0
FAIL=0

# The guard does `cd "$(git rev-parse --show-toplevel)"` and then reads
# `docs/roadmap/STATE.yaml` and `Cargo.toml` relative to that root, plus
# `manifest.toml` and `Cargo.lock`. So each fixture is a self-contained repo:
# its own git history, its own origin/main (a local bare remote, no network),
# and its own copies of the four files. The guard is COPIED in because it
# resolves its own root the same way scripts/release-bump.sh does.
#
#   $1 = workspace version written to Cargo.toml / STATE.yaml
#   $2 = current_sha written into STATE.yaml ("HEAD" = the real fixture HEAD)
#   $3 = current_sha comment text ("" = no comment)
#   $4 = declared version in workspace_version_at_current
#   $5 = version written to manifest.toml and Cargo.lock (defaults to $1)
#
# Echoes the guard's RESULT line.
run_guard() {
    local ws="$1" ptr_sha="$2" ptr_comment="$3" declared="$4"
    local bundle_ver="${5:-$1}"
    local dir="$TMPROOT/case-$PASS-$FAIL-$RANDOM-$$"
    local remote="$dir.remote.git"
    mkdir -p "$dir"

    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        git init -q --bare "$remote"
        git remote add origin "$remote"

        mkdir -p tests docs/roadmap
        cp "$GUARD" tests/test_release_state_pointer.sh

        cat > Cargo.toml <<EOF
[workspace]
members = []

[workspace.package]
version = "$ws"
edition = "2021"
EOF
        printf 'version = "%s"\n' "$bundle_ver" > manifest.toml
        cat > Cargo.lock <<EOF
version = 3

[[package]]
name = "sddk-cli"
version = "$bundle_ver"
EOF

        # Commit 1: everything except the pointer, so its SHA is knowable.
        git add -A
        git commit -qm "chore: fixture base"

        local sha
        if [[ "$ptr_sha" == "HEAD" ]]; then
            sha=$(git rev-parse --verify --quiet HEAD)
        else
            sha="$ptr_sha"
        fi
        {
            echo "schema_version: 1"
            echo "source:"
            if [[ -n "$ptr_comment" ]]; then
                printf '  current_sha: "%s"  # %s\n' "$sha" "$ptr_comment"
            else
                printf '  current_sha: "%s"\n' "$sha"
            fi
            printf '  workspace_version_at_current: "%s"\n' "$declared"
        } > docs/roadmap/STATE.yaml

        # The pointer CANNOT live in commit 1: writing it requires the SHA of
        # the commit that PRECEDES it, and that SHA only exists once commit 1
        # lands. So the pointer always goes into commit 2. main is then one
        # commit ahead of the pointer — the same shape the real repo has right
        # after a docs pointer commit, which is what the punctual-tolerance
        # check is calibrated for.
        git add -A
        git commit -qm "chore: fixture head"
        # Publish so origin/main exists and contains the pointer commit: the
        # guard requires the pointer to be reachable from origin/main.
        git push -q origin HEAD:refs/heads/main 2>/dev/null

        # A third commit so main is strictly ahead of the pointer, exercising
        # the punctual-tolerance check against a real gap.
        git commit -q --allow-empty -m "chore: fixture tail"
        git push -q origin HEAD:refs/heads/main 2>/dev/null
    ) 2>/dev/null

    local out rc
    out=$(cd "$dir" && bash tests/test_release_state_pointer.sh 2>&1)
    rc=$?
    rm -rf "$dir" "$remote"
    # The guard's contract is its EXIT CODE, not a text line. When the pointer
    # SHA does not resolve at all, the guard fails closed and exits 1 WITHOUT
    # reaching its RESULT banner (it bails out of the sha block early). So the
    # verdict is derived from the exit code and, when present, the RESULT line
    # is carried along for the failure message.
    if [[ "$rc" -ne 0 ]]; then
        printf 'RESULT: FAIL'
    else
        printf '%s\n' "$out" | grep -E '^RESULT:' | head -1
    fi
}

# A coherent pointer: guard must PASS. This is the control — without it, a
# guard that failed everything would satisfy every "must FAIL" case below.
expect() {
    local name="$1" want="$2" got="$3"
    if [[ "$got" == *"$want"* ]]; then
        echo "PASS  [$want] $name"
        PASS=$((PASS + 1))
    else
        echo "FAIL  [expected $want, got ${got:-<none>}] $name"
        FAIL=$((FAIL + 1))
    fi
}

echo "== C0/T01: mutation self-test of the state pointer guard =="
echo

# --- control: a coherent pointer is accepted -------------------------------
expect "coherent pointer is accepted" "RESULT: PASS" \
    "$(run_guard 2.2.30 HEAD "" 2.2.30)"

# --- T01 proper: inject a stale pointer ------------------------------------
expect "stale pointer (SHA that is not HEAD) is rejected" "RESULT: FAIL" \
    "$(run_guard 2.2.30 0000000000000000000000000000000000000000 "" 2.2.30)"

# --- version drift: pointer narrates a version Cargo.toml does not have -----
expect "pointer version differs from Cargo.toml" "RESULT: FAIL" \
    "$(run_guard 2.2.30 HEAD "" 9.9.9)"

# --- manifest.toml drift: the field the published bundle actually carries --
# Cargo.toml says 2.2.30 but manifest.toml (and Cargo.lock) still say 2.2.29.
# This is the real shape of INC-LOCK-POINTER-GAP: the build with --locked breaks.
expect "manifest.toml and Cargo.lock behind Cargo.toml are rejected" "RESULT: FAIL" \
    "$(run_guard 2.2.30 HEAD "" 2.2.30 2.2.29)"

# --- the check added in 4cf78bfd: free-text version in the pointer comment -
expect "current_sha comment asserting a version is rejected" "RESULT: FAIL" \
    "$(run_guard 2.2.30 HEAD "workspace 2.2.99 driftado" 2.2.30)"

# --- a comment that asserts NO version stays legal ------------------------
expect "current_sha comment without a version is accepted" "RESULT: PASS" \
    "$(run_guard 2.2.30 HEAD "bump commit, see workspace_version_at_current" 2.2.30)"

echo
echo "=================================================="
echo "  PASS=$PASS  FAIL=$FAIL"
echo "=================================================="

[[ "$FAIL" -eq 0 ]] || exit 1
