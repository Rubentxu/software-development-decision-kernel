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
# ── INC-AIWS1-RECEIPT-PUSH-BLOCK: cycle-artifacts documentary files ─────────

mk_cycle_dir() {
    mkdir -p "tests/cycle-artifacts/p-example/some-cycle"
}

s_cycle_receipt_only() {
    mk_cycle_dir
    echo "# receipt" > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "docs(cycle): receipt" >/dev/null
}
s_cycle_receipt_plus_followup() {
    mk_cycle_dir
    mkdir -p .sddk/followups
    echo "# receipt" > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    echo hi > .sddk/followups/f.md
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md .sddk/followups/f.md
    git commit -qm "docs(cycle)+followup" >/dev/null
}
s_cycle_scope_contract_only() {
    mk_cycle_dir
    echo "# scope" > tests/cycle-artifacts/p-example/some-cycle/SCOPE-CONTRACT.md
    git add tests/cycle-artifacts/p-example/some-cycle/SCOPE-CONTRACT.md
    git commit -qm "docs(cycle): scope" >/dev/null
}
s_cycle_discovery_only() {
    mk_cycle_dir
    echo "# discovery" > tests/cycle-artifacts/p-example/some-cycle/DISCOVERY.md
    git add tests/cycle-artifacts/p-example/some-cycle/DISCOVERY.md
    git commit -qm "docs(cycle): discovery" >/dev/null
}
s_cycle_receipt_plus_crates() {
    mk_cycle_dir
    mkdir -p crates/x
    echo "# receipt" > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    echo a > crates/x/a.rs
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md crates/x/a.rs
    git commit -qm "docs+code" >/dev/null
}
s_cycle_receipt_plus_githooks() {
    mk_cycle_dir
    mkdir -p githooks
    echo "# receipt" > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    echo x > githooks/x
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md githooks/x
    git commit -qm "docs+githooks" >/dev/null
}
s_cycle_receipt_with_secret() {
    mk_cycle_dir
    cat > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md <<'MDEOF'
# receipt

token: ghp_16Charactersxx99
aws_key: AKIAIOSFODNN7EXAMPLE
MDEOF
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "docs(cycle): receipt (leaks secrets)" >/dev/null
}
s_cycle_non_documentary_file() {
    mk_cycle_dir
    echo "raw log output" > tests/cycle-artifacts/p-example/some-cycle/raw-output.log
    git add tests/cycle-artifacts/p-example/some-cycle/raw-output.log
    git commit -qm "chore(cycle): raw log" >/dev/null
}
s_cycle_stray_top_level_file() {
    mkdir -p tests/cycle-artifacts
    echo "notes" > tests/cycle-artifacts/notas.md
    git add tests/cycle-artifacts/notas.md
    git commit -qm "docs(cycle): stray note" >/dev/null
}
s_cycle_rename_runtime_to_cycle() {
    mk_cycle_dir
    git mv crates/x/a.rs tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "hide runtime as cycle receipt" >/dev/null
}
s_cycle_rename_receipt_to_docs() {
    mk_cycle_dir
    echo "# receipt" > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "add receipt" >/dev/null
}

# ── INC-AIWS1-RECEIPT-PUSH-BLOCK canary purref (cycle inc-push-hook-canary-purref)
# Documenting test canaries in RECEIPT.md must not trigger the secret screen.
# Real secrets still must. Distinguish: redactable placeholders (<…>, {…},
# <redacted:N>, ellipsis …) vs literal high-confidence credentials.

s_cycle_receipt_with_test_canary() {
    mk_cycle_dir
    cat > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md <<'MDEOF'
# Receipt

| Test | Canary shape | Before | After |
|------|--------------|--------|-------|
| t1 | `api_key=CANARY_…` | RED | GREEN |
| t2 | `token={CANARY_TOKEN}` | RED | GREEN |
| t3 | `password=<USER_PROVIDED>` | RED | GREEN |
MDEOF
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "docs(cycle): receipt with test canaries" >/dev/null
}
s_cycle_receipt_with_redacted_marker() {
    mk_cycle_dir
    cat > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md <<'MDEOF'
# Receipt

config: api_key=<redacted:23>
other: token="<redacted:42>"
MDEOF
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "docs(cycle): receipt with redacted markers" >/dev/null
}
s_cycle_receipt_with_literal_high_confidence_secret() {
    mk_cycle_dir
    cat > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md <<'MDEOF'
# Receipt

This MUST be rejected by the screen:

ghp_aaaaaaaaaaaaaaaaaaaa
github_pat_xxxxxxxxxxxxxxxxxxxxxxxxxxxx
AKIAIOSFODNN7EXAMPLE
xoxb-1234567890-abcdef
sk-aaaaaaaaaaaaaaaaaaaaaaaaaaaa
-----BEGIN RSA PRIVATE KEY-----
MIIBOgIBAAJBALR9vQyqQGQj...
-----END RSA PRIVATE KEY-----
config: api_key="abcdefghijklmnop"
MDEOF
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "docs(cycle): receipt with literal secret" >/dev/null
}
s_cycle_receipt_with_real_github_pat() {
    mk_cycle_dir
    cat > tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md <<'MDEOF'
# Receipt

Real-looking github token below:

token = ghp_4G2aBcDeFgHiJkLmNoPqRsTuVwXyZ0123
MDEOF
    git add tests/cycle-artifacts/p-example/some-cycle/RECEIPT.md
    git commit -qm "docs(cycle): receipt with real-looking ghp_" >/dev/null
}

s_space_path_runtime() {
    mkdir -p crates/x; echo a > "crates/x/a b.rs"; git add "crates/x/a b.rs"
    git commit -qm "feat: spaced" >/dev/null
}
s_space_path_docs() {
    mkdir -p docs; echo a > "docs/a b.md"; git add "docs/a b.md"
    git commit -qm "docs: spaced" >/dev/null
}

# ── INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH: generated-only route ────────
# MANIFEST.sha256 alone (or with docs) is admissible; with source it is not.

s_manifest_only() {
    echo "deadbeef  prompts/sddk/orchestrator.md" > MANIFEST.sha256
    git add MANIFEST.sha256
    git commit -qm "chore(manifest): regenerate" >/dev/null
}
s_manifest_plus_docs() {
    echo "deadbeef  prompts/sddk/orchestrator.md" > MANIFEST.sha256
    mkdir -p docs; echo hi > docs/m.md
    git add MANIFEST.sha256 docs/m.md
    git commit -qm "chore(manifest)+docs" >/dev/null
}
s_manifest_plus_crates() {
    echo "deadbeef  prompts/sddk/orchestrator.md" > MANIFEST.sha256
    mkdir -p crates/x; echo a > crates/x/a.rs
    git add MANIFEST.sha256 crates/x/a.rs
    git commit -qm "manifest+code smuggle" >/dev/null
}
s_manifest_like_source_path() {
    # A similarly-named path outside the generated-only set stays rejected.
    mkdir -p nested
    echo x > nested/MANIFEST.sha256
    git add nested/MANIFEST.sha256
    git commit -qm "fake manifest path" >/dev/null
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
echo "=== generated-metadata route (INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH) ==="

run_case "MANIFEST.sha256 only"                                    ACCEPT s_manifest_only
run_case "MANIFEST.sha256 + docs"                                   ACCEPT s_manifest_plus_docs
run_case "MANIFEST.sha256 + crates change (smuggle)"                REJECT s_manifest_plus_crates
run_case "nested/MANIFEST.sha256 (not the generated set)"           REJECT s_manifest_like_source_path

echo ""
echo "=== cycle-artifacts documentary allowlist (INC-AIWS1-RECEIPT-PUSH-BLOCK) ==="

run_case "cycle RECEIPT.md only"                                     ACCEPT s_cycle_receipt_only
run_case "cycle RECEIPT.md + followup"                               ACCEPT s_cycle_receipt_plus_followup
run_case "cycle SCOPE-CONTRACT.md only"                              ACCEPT s_cycle_scope_contract_only
run_case "cycle DISCOVERY.md only"                                   ACCEPT s_cycle_discovery_only
run_case "cycle RECEIPT.md + crates change"                          REJECT s_cycle_receipt_plus_crates
run_case "cycle RECEIPT.md + githooks change"                        REJECT s_cycle_receipt_plus_githooks
run_case "cycle RECEIPT.md containing secret patterns"               REJECT s_cycle_receipt_with_secret
run_case "cycle non-documentary file (raw log)"                      REJECT s_cycle_non_documentary_file
run_case "stray file at tests/cycle-artifacts root"                  REJECT s_cycle_stray_top_level_file
run_case "rename runtime -> cycle RECEIPT.md (no bump)"              REJECT s_cycle_rename_runtime_to_cycle p_runtime_add
run_case "rename cycle receipt -> docs"                              ACCEPT s_cycle_rename_receipt_to_docs

run_case "cycle RECEIPT.md with test canary placeholders"            ACCEPT s_cycle_receipt_with_test_canary
run_case "cycle RECEIPT.md with <redacted:N> markers"                ACCEPT s_cycle_receipt_with_redacted_marker
run_case "cycle RECEIPT.md with literal high-confidence secret"      REJECT s_cycle_receipt_with_literal_high_confidence_secret
run_case "cycle RECEIPT.md with real-looking github PAT"             REJECT s_cycle_receipt_with_real_github_pat

echo ""
echo "=== matrix result: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
if [[ $FAIL_COUNT -ne 0 ]]; then
    exit 1
fi
exit 0
