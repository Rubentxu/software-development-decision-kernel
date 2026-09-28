#!/bin/bash
# Contract test for scripts/release-bump.sh version DERIVATION.
#
# Session-28 defect: `next_version` derived from the last TAG, ignoring the
# version the workspace is actually at. A workspace at 2.1.1 with last tag
# v2.0.1 derived v2.1.0 — LOWER than the workspace. The CI release branch
# ("Open release PR when a bump is pending") runs this script and
# auto-merges the result, so CI would have landed a version ROLLBACK on main.
#
# Reproduced in an isolated clone before fixing; see the journal.
#
# Temp repos only. No network. Never mutates the real repo.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUMP="$REPO_ROOT/scripts/release-bump.sh"

if [[ ! -f "$BUMP" ]]; then
    echo "FAIL: $BUMP missing"
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

# Build a repo with a given workspace version and a given last tag, then ask
# the tool to derive. Echoes the "new tag: vX.Y.Z" it reports.
#
# The script does `cd "$ROOT"` where ROOT is its OWN repo root, so invoking it
# from another directory silently operates on this repo instead of the temp
# one. Each fixture therefore gets its own COPY of the script, exactly as CI
# does (CI checks out the repo, so $ROOT == the fixture).
#
#   $1 = workspace version
#   $2 = last tag version
#   $3 = commit subject used for the level detection
derive() {
    local ws="$1" tag="$2" subject="$3"
    local dir="$TMPROOT/$RANDOM-$$"
    mkdir -p "$dir/scripts"
    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        cp "$BUMP" scripts/release-bump.sh
        mkdir -p crates/fake
        cat > Cargo.toml <<EOF
[workspace]
members = ["crates/fake"]

[workspace.package]
version = "$ws"
edition = "2021"
EOF
        cat > crates/fake/Cargo.toml <<EOF
[package]
name = "fake"
version.workspace = true
EOF
        printf 'version = "%s"\n' "$ws" > manifest.toml
        git add -A
        git commit -qm "chore: base"
        git tag "v$tag"
        git commit -q --allow-empty -m "$subject"
    )
    local out
    out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run 2>&1)
    rm -rf "$dir"
    echo "$out" | grep -oE '^new tag: v[0-9]+\.[0-9]+\.[0-9]+$' | awk '{print $3}'
}

check() {
    local name="$1" expect="$2" got="$3"
    if [[ "$got" == "$expect" ]]; then
        echo "PASS  [$expect] $name"
        PASS=$((PASS + 1))
    else
        echo "FAIL  [expected $expect, got ${got:-<none>}] $name"
        FAIL=$((FAIL + 1))
    fi
}

# --- The regression: workspace ahead of the tag -----------------------------
#
# A manual `--force-version` bump (or a pending ceremonial release) leaves
# the workspace ABOVE the last tag. Deriving from the tag would produce a
# version LOWER than the current workspace — a rollback.
check "workspace ahead of tag, minor commit does not roll back" \
    "v2.2.0" "$(derive 2.1.1 2.0.1 'feat: something')"

check "workspace ahead of tag, patch commit does not roll back" \
    "v2.1.2" "$(derive 2.1.1 2.0.1 'fix: something')"

check "workspace ahead of tag, major commit bumps from workspace" \
    "v3.0.0" "$(derive 2.1.1 2.0.1 'feat!: breaking change')"

# --- The normal case: workspace == tag -------------------------------------
#
# Nothing pending. The tag is the base, as before.
check "workspace equals tag, minor commit" \
    "v2.1.0" "$(derive 2.0.1 2.0.1 'feat: something')"

check "workspace equals tag, patch commit" \
    "v2.0.2" "$(derive 2.0.1 2.0.1 'fix: something')"

# --- Workspace BELOW the tag is impossible in a healthy repo, but if it
# --- happened the tag must still win (never derive downward from nothing).
check "workspace behind tag keeps tag as base" \
    "v2.0.2" "$(derive 2.0.0 2.0.1 'fix: something')"

echo ""
echo "=== matrix result: PASS=$PASS FAIL=$FAIL ==="
[[ $FAIL -eq 0 ]] || exit 1
exit 0
