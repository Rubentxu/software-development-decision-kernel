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

# --- cycle-c: release_admission_check_v2 (SCOPE-CONTRACT §3.3) ----------
#
# The v2 variant uses `last_published_version()` over `git ls-remote --tags`
# against an origin-like local bare repo created in the test. Each v2 case
# sets up a local non-bare repo, optionally creates a tag named `v<last_pub>`
# at the relevant commit, mirrors the result to a local bare "origin", and
# points `SDDK_RELEASE_ADMISSION_REMOTE` at that path. That way
# `last_published_version()` reads from the bare repo (not GitHub).
# Setup functions are responsible for creating the right state.

case_v2_run() {
    local name="$1" expect="$2" setup="$3"
    local dir="$TMPROOT/$RANDOM-$$-v2"
    local remote_dir="$TMPROOT/$RANDOM-$$-remote"
    mkdir -p "$dir"
    # The remote needs to be a bare repo before we can push to it.
    git init -q --bare "$remote_dir"
    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        # Build the local repo with the chosen scenario.
        "$setup"
        # Mirror the resulting repo as a bare remote so that
        # `git ls-remote --tags` has something to look at.
        git remote add fake_origin "$remote_dir" 2>/dev/null || true
        # Push branches first, then tags (--all + --tags are mutually
        # exclusive). Lightweight empty repos have only a master/main
        # branch by default; current branch is enough for the test.
        local current_branch
        current_branch="$(git branch --show-current 2>/dev/null || git rev-parse --abbrev-ref HEAD)"
        [[ -n "$current_branch" ]] && \
            git push fake_origin "$current_branch" 2>/dev/null || true
        git push fake_origin --tags 2>/dev/null || true
    )
    # Temporarily route last_published_version at the fake bare remote.
    local out got rc
    out=$(SDDK_RELEASE_ADMISSION_REMOTE="$remote_dir" \
          SDDK_RELEASE_ADMISSION_MODE=v2 \
          bash -c "cd '$dir' && . '$LIB' && release_admission_check_v2 HEAD")
    rc=$?
    got="ACCEPT"
    [[ $rc -ne 0 ]] && got="REJECT"
    if [[ "$got" == "$expect" ]]; then
        echo "PASS  [$expect] v2: $name  ($out)"
        PASS=$((PASS + 1))
    else
        echo "FAIL  [expected $expect, got $got] v2: $name  ($out)"
        FAIL=$((FAIL + 1))
    fi
    chmod -R u+rw "$dir" "$remote_dir" 2>/dev/null || true
    rm -rf "$dir" "$remote_dir"
}

# Each v2 setup leaves HEAD at a chosen version on top of a baseline
# "v1.169.122" tagged commit (or just an empty tag set, for bootstrap cases).
v2_setup_seed_base() { seed . 1.169.122; git add Cargo.toml; git commit -qm "chore: base"; }
v2_setup_tag_base() {
    v2_setup_seed_base
    git tag v1.169.122 HEAD
}

# v2-1: bootstrap (no published tags), head > parent_version → ACCEPT
v2_setup_bootstrap_accept() {
    v2_setup_seed_base
    seed . 1.169.123; git add Cargo.toml; git commit -qm "chore(release): bump"
}
v2_setup_bootstrap_tie() {
    v2_setup_seed_base
    # Add a non-bump commit (just touch Cargo.toml without changing version).
    echo "# touch" >> Cargo.toml; git add Cargo.toml; git commit -qm "docs: update CURRENT"
    # HEAD still has version 1.169.122 (parent version); bootstrap REJECT
}

# v2-4: published = 1.169.122, head > last → ACCEPT
v2_setup_published_above() {
    v2_setup_tag_base
    seed . 1.169.123; git add Cargo.toml; git commit -qm "chore(release): bump"
}
# v2-5: published = 1.169.122, head == last → REJECT already-published
v2_setup_published_equal() {
    v2_setup_tag_base
    # leave head at 1.169.122 (no bump commit); docs-only commit added
    echo "# touch" >> Cargo.toml; git add Cargo.toml; git commit -qm "docs: update CURRENT"
    # HEAD has same Cargo.toml version (1.169.122) as the tag
    # Since the rules forbid marking HEAD at exactly the published version
    # unless it had a bump commit, we add an in-range bump branch:
    # Actually no — in this scenario, the contract says "any commit
    # following the last publish must be > last publish". A docs-only
    # commit on top of v1.169.122 still has Cargo.toml at 1.169.122. So
    # head == last → REJECT already-published.
}
# v2-6: published = 1.169.122, head < last → REJECT not-above-last-publish
v2_setup_published_lower() {
    v2_setup_tag_base
    # bump DOWN to 1.169.121
    seed . 1.169.121; git add Cargo.toml; git commit -qm "chore(release): rewind (invalid)"
}
# v2-7: published-tie-via-docs-only — same as published_equal but we force
# the test to validate docs-only commits are allowed via v2 path (just
# like published_equal, this must REJECT — no exception for docs-only
# because the invariant is "head version must exceed last").
v2_setup_published_tie_docs() {
    v2_setup_published_equal
}
# v2-8: head-not-on-tag-sha — local HEAD has v1.169.123 in Cargo.toml, but
# the tag v1.169.122 is on a different commit (we keep it on the base
# commit). v2 admission compares versions, not SHAs, so this case is
# equivalent to "head > last_published" → ACCEPT.
v2_setup_head_not_on_tag_sha() {
    v2_setup_tag_base
    seed . 1.169.123; git add Cargo.toml; git commit -qm "chore(release): bump"
    # The tag v1.169.122 stays on the base commit; HEAD is one commit ahead
    # with a bumped version. v2 should ACCEPT.
}

case_v2_run "bootstrap monotonic accept"           ACCEPT v2_setup_bootstrap_accept
case_v2_run "bootstrap tie reject"                 REJECT v2_setup_bootstrap_tie
case_v2_run "published above accept"               ACCEPT v2_setup_published_above
case_v2_run "published equal reject"               REJECT v2_setup_published_equal
case_v2_run "published lower reject"               REJECT v2_setup_published_lower
case_v2_run "published tie via docs only"          REJECT v2_setup_published_tie_docs
case_v2_run "head not on tag sha (still accept)"    ACCEPT v2_setup_head_not_on_tag_sha

echo ""
echo "=== matrix result: PASS=$PASS FAIL=$FAIL ==="
[[ $FAIL -eq 0 ]] || exit 1
exit 0
