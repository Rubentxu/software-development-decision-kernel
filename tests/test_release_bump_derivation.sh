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
# shellcheck disable=SC2329  # se referencia via trap EXIT (no captado por shellcheck)
cleanup() {
    local code=$?
    chmod -R u+rw "$TMPROOT" 2>/dev/null || true
    rm -rf "$TMPROOT" >/dev/null 2>&1
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
    local out derived
    out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run 2>&1)
    # El valor de retorno de esta funcion ES lo que el caller lee. Cualquier
    # salida en stdout del cleanup se concatenaria al tag y haria fallar la
    # comparacion, asi que se silencia explicitamente en vez de confiar en que
    # `rm` no imprime nada.
    rm -rf "$dir" >/dev/null 2>&1
    derived=$(printf '%s\n' "$out" | grep -oE '^new tag: v[0-9]+\.[0-9]+\.[0-9]+$' | awk '{print $3}')
    printf '%s\n' "$derived"
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

# Same fixture as `derive`, but passes an explicit --force-version.
#
#   $1 = workspace version
#   $2 = last tag version
#   $3 = commit subject used for the level detection
#   $4 = version to force
derive_forced() {
    local ws="$1" tag="$2" subject="$3" force="$4"
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
    local out derived
    out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run --force-version "$force" 2>&1)
    # Mismo motivo que en derive(): el retorno se lee como valor, no como
    # salida acumulada.
    rm -rf "$dir" >/dev/null 2>&1
    derived=$(printf '%s\n' "$out" | grep -oE '^new tag: v[0-9]+\.[0-9]+\.[0-9]+$' | awk '{print $3}')
    printf '%s\n' "$derived"
}

# --- The regression: workspace ahead of the tag -----------------------------
#
# A manual `--force-version` bump (or a pending ceremonial release) leaves
# the workspace ABOVE the last tag. AGENTS.md §2.3 makes the workspace
# version the *ceremonial release pointer*: it declares the version that is
# going to appear as the tag. So the release IS the workspace version and
# there is nothing left to derive.
#
# Deriving from the tag alone rolls the workspace BACK (session-28).
# Deriving max(workspace, tag) and then bumping SKIPS the declared release
# (session-29: 2.2.0 -> v2.3.0, publishing a version nobody asked for).
# Both are wrong; the answer is "no pending bump".
check "workspace ahead of tag, minor commit derives no bump" \
    "" "$(derive 2.1.1 2.0.1 'feat: something')"

check "workspace ahead of tag, patch commit derives no bump" \
    "" "$(derive 2.1.1 2.0.1 'fix: something')"

check "workspace ahead of tag, breaking commit derives no bump" \
    "" "$(derive 2.1.1 2.0.1 'feat!: breaking change')"

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

# --- --force-version still wins over the short-circuit ----------------------
#
# The short-circuit above stops the accidental double bump. But the operator
# must keep a way to say "actually make it this", even with the workspace
# already ahead of the tag. If --force-version were swallowed by the
# short-circuit, the escape hatch documented in AGENTS.md §2.3 would be dead.
check "--force-version overrides the declared-release short-circuit" \
    "v2.5.0" "$(derive_forced 2.1.1 2.0.1 'feat: something' 2.5.0)"

echo ""
echo "=== matrix result: PASS=$PASS FAIL=$FAIL ==="
[[ $FAIL -eq 0 ]] || exit 1
exit 0
