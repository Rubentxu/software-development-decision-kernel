#!/bin/bash
# Contract test for githooks/pre-push — A5-1 push-admission matrix.
#
# Cycle: p-63676b11dc0ef88f/a5-1-release-distribution-version-governance
# INC:   INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION (CL-APPLY-PUSH-DISCIPLINE)
#        INC-A5-PUSH-RELEASE-MARKER-FRICTION (ceremonial empty marker)
#
# Contract (A5-1 §1-§4):
#
#   push to refs/heads/main is ACCEPTED iff
#     (A) the pushed range contains a REAL [workspace.package] version change
#   OR
#     (B) the pushed range is NON-EMPTY and every changed path is inside the
#         closed documentation-only allowlist (docs/**, .sddk/followups/**).
#
#   The commit subject is NOT authority. An empty `chore(release): bump version`
#   marker MUST be rejected.
#
# Isolation: fresh temp origin + file:// remote per case. No network.
# No mutation of the real repo's main.

# shellcheck disable=SC2329  # functions are invoked indirectly (by name)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK_PATH="$REPO_ROOT/githooks/pre-push"

if [[ ! -f "$HOOK_PATH" ]]; then
    echo "RED phase: githooks/pre-push does not exist"
    exit 1
fi
if [[ ! -x "$HOOK_PATH" ]]; then
    echo "FAIL: hook not executable"
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

PASS_COUNT=0
FAIL_COUNT=0

# Seed Cargo.toml with a [workspace.package] version.
seed_cargo() { # $1 = version
    cat > Cargo.toml <<EOF
[workspace]
members = []

[workspace.package]
version = "$1"
edition = "2021"
EOF
}

# run_case <name> <expect: ACCEPT|REJECT> <setup-fn> [pre-fn]
# The setup function runs inside a fresh clone whose origin already has an
# accepted release tip. It must leave the desired local commits on `main`.
# An optional `pre-fn` runs first and is pushed (it must itself be admissible);
# this establishes remote state so deletions/renames have a real base.
run_case() {
    local name="$1" expect="$2" setup="$3" pre="${4:-}"
    local dir="$TMPROOT/case-$RANDOM-$$"
    local origin="$dir/origin"
    local clone="$dir/clone"
    mkdir -p "$origin"

    git init --bare "$origin" >/dev/null 2>&1
    git clone "file://$origin" "$clone" >/dev/null 2>&1
    (
        cd "$clone" || exit 2
        git config user.email "t@example.com"
        git config user.name "T"
        git config core.hooksPath "$REPO_ROOT/githooks"
        git checkout -b main >/dev/null 2>&1 || git branch -M main

        # Seed: two commits so the root-commit case has no bump, and the
        # second is a REAL bump (0.0.0 -> 1.0.0). Initial push must be accepted.
        seed_cargo "0.0.0"
        git add Cargo.toml
        git commit -qm "chore: init" >/dev/null

        seed_cargo "1.0.0"
        git add Cargo.toml
        git commit -qm "chore(release): bump version 0.0.0 -> 1.0.0" >/dev/null

        if ! git push -q origin main 2>/dev/null; then
            echo "  (seed push rejected — fixture broken)"
            return 2
        fi

        # Optional pre-phase: establish remote state (must be admissible).
        if [[ -n "$pre" ]]; then
            "$pre"
            if ! git push -q origin main 2>/dev/null; then
                echo "  (pre push rejected — fixture broken)"
                return 2
            fi
        fi

        # Case-specific changes (the range under test).
        "$setup"

        local out
        out=$(git push origin main 2>&1)
        local code=$?
        local got="ACCEPT"
        [[ $code -ne 0 ]] && got="REJECT"

        if [[ "$got" == "$expect" ]]; then
            echo "PASS  [$expect] $name"
            return 0
        fi
        echo "FAIL  [expected $expect, got $got] $name"
        # shellcheck disable=SC2001
        echo "$out" | sed 's/^/        /'
        return 1
    )
    local rc=$?
    case $rc in
        0) PASS_COUNT=$((PASS_COUNT + 1)) ;;
        2) FAIL_COUNT=$((FAIL_COUNT + 1)); echo "FAIL  fixture error: $name" ;;
        *) FAIL_COUNT=$((FAIL_COUNT + 1)) ;;
    esac
    chmod -R u+rw "$dir" 2>/dev/null || true
    rm -rf "$dir"
}

# ── setup functions ─────────────────────────────────────────────────────────

s_bump() { # real version bump
    seed_cargo "1.1.0"; git add Cargo.toml
    git commit -qm "feat: bump" >/dev/null
}
s_bump_plus_runtime() { # real bump AND runtime changes -> accepted
    seed_cargo "1.1.0"; git add Cargo.toml
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat(engine): real bump with code" >/dev/null
}
s_docs_only() {
    mkdir -p docs; echo hi > docs/a.md; git add docs/a.md
    git commit -qm "docs: note" >/dev/null
}
# Pre-phases: admissible (real bump) and establish remote state.
p_docs_add() {
    seed_cargo "1.0.1"; git add Cargo.toml
    mkdir -p docs; echo hi > docs/a.md; git add docs/a.md
    git commit -qm "docs: add under a bump" >/dev/null
}
p_runtime_add() {
    seed_cargo "1.0.1"; git add Cargo.toml
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat: add runtime under a bump" >/dev/null
}
s_docs_delete() {
    git rm -q docs/a.md; git commit -qm "docs: delete" >/dev/null
}
s_followups_only() {
    mkdir -p .sddk/followups; echo hi > .sddk/followups/f.md; git add .sddk/followups/f.md
    git commit -qm "docs(followups): note" >/dev/null
}
s_docs_rename_docs() {
    git mv docs/a.md docs/b.md; git commit -qm "docs: rename" >/dev/null
}
s_empty_marker() {
    git commit -q --allow-empty -m "chore(release): bump version (cycle close marker)" >/dev/null
}
s_empty_plain() {
    git commit -q --allow-empty -m "chore: nothing" >/dev/null
}
s_crates_only() {
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat(engine): code" >/dev/null
}
s_scripts_only() {
    mkdir -p scripts; echo a > scripts/a.sh; git add scripts/a.sh
    git commit -qm "chore(scripts): tool" >/dev/null
}
s_githooks_only() {
    mkdir -p githooks; echo a > githooks/x; git add githooks/x
    git commit -qm "chore(githooks): x" >/dev/null
}
s_agents_md_only() {
    echo a > AGENTS.md; git add AGENTS.md
    git commit -qm "docs(agents): x" >/dev/null
}
s_docs_plus_crates() {
    mkdir -p docs crates/x; echo a > docs/a.md; echo b > crates/x/b.rs
    git add docs/a.md crates/x/b.rs
    git commit -qm "docs+code" >/dev/null
}
s_docs_plus_scripts() {
    mkdir -p docs scripts; echo a > docs/a.md; echo b > scripts/b.sh
    git add docs/a.md scripts/b.sh
    git commit -qm "docs+scripts" >/dev/null
}
s_fake_subject_runtime() {
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "chore(release): bump version 1.0.0 -> 1.1.0" >/dev/null
}
s_cargo_touched_no_change() {
    echo "# comment" >> Cargo.toml; git add Cargo.toml
    git commit -qm "chore: touch cargo" >/dev/null
}
s_rename_runtime_to_docs() {
    mkdir -p docs
    git mv crates/x/a.rs docs/a.rs; git commit -qm "move runtime to docs" >/dev/null
}
s_rename_docs_to_runtime() {
    mkdir -p crates/x
    git mv docs/a.md crates/x/a.md; git commit -qm "move docs to runtime" >/dev/null
}
s_space_path_runtime() {
    mkdir -p crates/x; echo a > "crates/x/a b.rs"; git add "crates/x/a b.rs"
    git commit -qm "feat: spaced" >/dev/null
}
s_space_path_docs() {
    mkdir -p docs; echo a > "docs/a b.md"; git add "docs/a b.md"
    git commit -qm "docs: spaced" >/dev/null
}

# ── matrix ──────────────────────────────────────────────────────────────────

echo "=== A5-1 push-admission matrix ==="

run_case "real [workspace.package] version bump"                ACCEPT s_bump
run_case "real bump + runtime changes"                          ACCEPT s_bump_plus_runtime
run_case "docs/** only (non-empty)"                             ACCEPT s_docs_only
run_case "docs file delete"                                     ACCEPT s_docs_delete p_docs_add
run_case ".sddk/followups/** only (non-empty)"                  ACCEPT s_followups_only
run_case "rename docs -> docs"                                  ACCEPT s_docs_rename_docs p_docs_add
run_case "path with spaces under docs"                          ACCEPT s_space_path_docs

run_case "empty chore(release) marker"                          REJECT s_empty_marker
run_case "plain empty commit"                                   REJECT s_empty_plain
run_case "crates/** only"                                       REJECT s_crates_only
run_case "scripts/** only"                                      REJECT s_scripts_only
run_case "githooks/** only"                                     REJECT s_githooks_only
run_case "AGENTS.md only"                                       REJECT s_agents_md_only
run_case "docs + crates"                                        REJECT s_docs_plus_crates
run_case "docs + scripts"                                       REJECT s_docs_plus_scripts
run_case "fake release subject + runtime change"                REJECT s_fake_subject_runtime
run_case "Cargo.toml touched but version unchanged"             REJECT s_cargo_touched_no_change
run_case "rename runtime -> docs"                               REJECT s_rename_runtime_to_docs p_runtime_add
run_case "rename docs -> runtime"                               REJECT s_rename_docs_to_runtime p_docs_add
run_case "path with spaces under crates"                        REJECT s_space_path_runtime

echo ""
echo "=== matrix result: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
if [[ $FAIL_COUNT -ne 0 ]]; then
    exit 1
fi
exit 0
