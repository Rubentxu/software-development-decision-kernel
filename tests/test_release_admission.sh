#!/bin/bash
# Contract test for release admission — A5-1 §6 / §10.
#
# Distinguishes:
#   - push admission  (githooks/pre-push): any REAL version change OR docs-only
#   - release admission (scripts/lib/release_admission.sh): REAL *monotonic*
#     version change vs HEAD's first parent
#
# Temp repos only. No network. No mutation of the real repo.

# shellcheck disable=SC2329  # functions are invoked indirectly (by name)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB="$REPO_ROOT/scripts/lib/release_admission.sh"

if [[ ! -f "$LIB" ]]; then
    echo "FAIL: $LIB missing"
    exit 1
fi

# shellcheck source=/dev/null
# shellcheck disable=SC1091
. "$LIB"

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

seed() { # $1 = dir, $2 = version
    mkdir -p "$1"
    cat > "$1/Cargo.toml" <<EOF
[workspace]
members = []

[workspace.package]
version = "$2"
edition = "2021"
EOF
}

# case <name> <expect: ACCEPT|REJECT> <fn>
case_run() {
    local name="$1" expect="$2" fn="$3"
    local dir="$TMPROOT/$RANDOM-$$"
    mkdir -p "$dir"
    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        "$fn"
    )
    local out
    out=$(cd "$dir" && release_admission_check HEAD)
    local rc=$?
    local got="ACCEPT"
    [[ $rc -ne 0 ]] && got="REJECT"
    if [[ "$got" == "$expect" ]]; then
        echo "PASS  [$expect] $name  ($out)"
        PASS=$((PASS + 1))
    else
        echo "FAIL  [expected $expect, got $got] $name  ($out)"
        FAIL=$((FAIL + 1))
    fi
    chmod -R u+rw "$dir" 2>/dev/null || true
    rm -rf "$dir"
}

c_real_bump() {
    seed . 1.169.70; git add Cargo.toml; git commit -qm "chore: base"
    seed . 1.169.71; git add Cargo.toml; git commit -qm "chore(release): bump version 1.169.70 -> 1.169.71"
}
c_real_bump_non_conventional_subject() {
    seed . 1.169.70; git add Cargo.toml; git commit -qm "chore: base"
    seed . 1.169.71; git add Cargo.toml; git commit -qm "feat: some change that also bumps"
}
c_empty_marker() {
    seed . 1.169.70; git add Cargo.toml; git commit -qm "chore: base"
    git commit -q --allow-empty -m "chore(release): bump version (cycle close marker)"
}
c_no_change() {
    seed . 1.169.70; git add Cargo.toml; git commit -qm "chore: base"
    echo "# note" >> Cargo.toml; git add Cargo.toml; git commit -qm "chore(release): bump version 1.169.70 -> 1.169.71"
}
c_decrease() {
    seed . 1.169.70; git add Cargo.toml; git commit -qm "chore: base"
    seed . 1.169.69; git add Cargo.toml; git commit -qm "chore(release): bump version 1.169.70 -> 1.169.69"
}
c_same_version() {
    seed . 1.169.70; git add Cargo.toml; git commit -qm "chore: base"
    echo "# touched" >> Cargo.toml; git add Cargo.toml
    git commit -qm "chore(release): bump version 1.169.70 -> 1.169.70"
}
c_patch_bump() {
    seed . 1.169.70; git add Cargo.toml; git commit -qm "chore: base"
    seed . 1.169.71; echo "fn main(){}" > x.txt; git add Cargo.toml x.txt
    git commit -qm "chore(release): bump version 1.169.70 -> 1.169.71"
}

echo "=== A5-1 release-admission matrix ==="
case_run "real monotonic bump"                     ACCEPT c_real_bump
case_run "real monotonic bump, non-conventional subject" ACCEPT c_real_bump_non_conventional_subject
case_run "patch bump with other changes"           ACCEPT c_patch_bump
case_run "empty chore(release) marker"             REJECT c_empty_marker
case_run "Cargo.toml touched, version unchanged"   REJECT c_no_change
case_run "version decreases"                       REJECT c_decrease
case_run "same version"                            REJECT c_same_version

echo ""
echo "=== matrix result: PASS=$PASS FAIL=$FAIL ==="
[[ $FAIL -eq 0 ]] || exit 1
exit 0
