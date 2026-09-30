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

        # Publish the seed as v1.0.0 on the remote so the tag-baseline route
        # (INC-DEBT-040 variant 3) has a real baseline. Cases that need a
        # different published state delete/re-tag from here.
        git tag -a v1.0.0 -m "seed release" >/dev/null
        git push -q origin v1.0.0 2>/dev/null

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

# INC-DEBT-040 variant 3: tag-baseline admission route.

# The scenario from the incident: the bump (1.0.0 -> 1.1.0) was already
# merged into origin/main in an earlier push; the current range only carries
# runtime changes. Range-local (A) cannot see the bump; the tip version
# exceeds the published v1.0.0, so the push must be ADMITTED.
pre_bump_already_on_remote() {
    seed_cargo "1.1.0"; git add Cargo.toml
    git commit -qm "chore(release): bump version 1.0.0 -> 1.1.0" >/dev/null
}
s_runtime_after_merged_bump() {
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat(engine): code under a previously merged bump" >/dev/null
}

# Workspace version does NOT exceed the last published tag and the range has
# no bump: must stay REJECTED (no release contract).
s_runtime_no_bump_no_tag_gain() {
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat(engine): code without release contract" >/dev/null
}

# Workspace version EQUALS the published tag (release already published) and
# the range has no bump: REJECTED (nothing new to publish).
s_runtime_equal_to_published() {
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat(engine): code at already-published version" >/dev/null
}

# The tag baseline route must be FAIL-CLOSED: with the tag query broken
# (origin renamed so ls-remote fails) a no-bump runtime range must be
# rejected. Checked by invoking the hook DIRECTLY (git push contacts the
# remote before the hook runs, so the push-level case would pass for the
# wrong reason); a broken remote also breaks `git push` itself.
hook_direct_case() { # <expect>
    local expect="$1"
    local in_line out code
    in_line="refs/heads/main $(git rev-parse HEAD) refs/heads/main $(git rev-parse origin/main 2>/dev/null || echo 0000000000000000000000000000000000000000)"
    out=$(printf '%s\n' "$in_line" | bash "$REPO_ROOT/githooks/pre-push" 2>&1)
    code=$?
    local got="ACCEPT"
    [[ $code -ne 0 ]] && got="REJECT"
    if [[ "$got" == "$expect" ]]; then
        echo "PASS  [direct-$expect] tag query broken (fail-closed)"
        return 0
    fi
    echo "FAIL  [direct-expected $expect, got $got] tag query broken (fail-closed)"
    printf '%s\n' "$out" | sed 's/^/        /'
    return 1
}
s_runtime_tag_query_broken() {
    git remote rename origin origin-unreachable >/dev/null 2>&1
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat(engine): code while tag query cannot answer" >/dev/null
}

# Bootstrap: remote answered with NO v* tags; any declared version admits.
s_runtime_bootstrap_no_tags() {
    git push origin :refs/tags/v1.0.0 >/dev/null 2>&1
    git tag -d v1.0.0 >/dev/null 2>&1
    mkdir -p crates/x; echo a > crates/x/a.rs; git add crates/x/a.rs
    git commit -qm "feat(engine): first declared release, no published tags" >/dev/null
}

# Range with a real bump does not need the tag route (control): accepted even
# when the tip version equals the published tag because the bump is in-range.
s_bump_equal_to_published() {
    seed_cargo "1.2.0"; git add Cargo.toml
    git commit -qm "chore(release): bump version 1.1.0 -> 1.2.0" >/dev/null
}

# Docs-only route unaffected by the tag baseline (control).
s_docs_after_merged_bump() {
    mkdir -p docs; echo hi > docs/n.md; git add docs/n.md
    git commit -qm "docs: note under merged bump" >/dev/null
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
# INC-DEBT-040 variant 3: while the workspace version exceeds the last
# published tag (declared-but-unpublished window), the tag-baseline route
# admits any non-empty range, so these rename cases lose their range-local
# REJECT. Amended expectations (the same scenarios with the window open are
# asserted as ACCEPT in the tag-baseline section below); the window-closed
# equivalents ("runtime changes, no bump, tip == published tag") still hold.
run_case "rename runtime -> docs"                               ACCEPT s_rename_runtime_to_docs p_runtime_add
run_case "rename docs -> runtime"                               ACCEPT s_rename_docs_to_runtime p_docs_add
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
run_case "rename runtime -> cycle RECEIPT.md (no bump)"              ACCEPT s_cycle_rename_runtime_to_cycle p_runtime_add
run_case "rename cycle receipt -> docs"                              ACCEPT s_cycle_rename_receipt_to_docs

run_case "cycle RECEIPT.md with test canary placeholders"            ACCEPT s_cycle_receipt_with_test_canary
run_case "cycle RECEIPT.md with <redacted:N> markers"                ACCEPT s_cycle_receipt_with_redacted_marker
run_case "cycle RECEIPT.md with literal high-confidence secret"      REJECT s_cycle_receipt_with_literal_high_confidence_secret
run_case "cycle RECEIPT.md with real-looking github PAT"             REJECT s_cycle_receipt_with_real_github_pat

echo ""
echo "=== tag-baseline admission route (INC-DEBT-040 variant 3) ==="

# The incident scenario: the bump was already merged into origin/main, the
# range carries only runtime changes; the tip version exceeds the published
# tag, so the push must be admitted via the tag baseline (A-v2).
run_case "runtime changes, bump already merged, tip > published tag"  ACCEPT s_runtime_after_merged_bump pre_bump_already_on_remote
run_case "runtime changes, no bump, tip == published tag"             REJECT s_runtime_equal_to_published
run_case "bootstrap: no published tags, any declared version"         ACCEPT s_runtime_bootstrap_no_tags
run_case "control: real bump in range with tip == published"          ACCEPT s_bump_equal_to_published
run_case "control: docs-only under merged bump"                       ACCEPT s_docs_after_merged_bump pre_bump_already_on_remote

# Blanket-window semantics: while the tip version exceeds the published tag
# the release contract exists at the tip, so ranges that would require a
# fresh bump (e.g. renames touching runtime paths) are admissible. These
# amend the range-local expectations of the original matrix, which applied
# only when NO declared-but-unpublished window was open. Window-closed
# cases (tip == published) above still enforce the original matrix.
run_case "AMENDED: rename runtime -> docs during open window"         ACCEPT s_rename_runtime_to_docs p_runtime_add
run_case "AMENDED: rename docs -> runtime during open window"         ACCEPT s_rename_docs_to_runtime p_docs_add
run_case "AMENDED: rename runtime -> cycle RECEIPT.md, open window"   ACCEPT s_cycle_rename_runtime_to_cycle p_runtime_add

# Fail-closed: the tag-baseline route never admits when the remote cannot
# answer. Direct hook invocation (see hook_direct_case). The subshell result
# is captured as PASS/FAIL text so the counters stay in the parent shell.
res="$(
    dir="$TMPROOT/case-failclosed-$RANDOM-$$"
    mkdir -p "$dir/origin"
    git init --bare "$dir/origin" >/dev/null 2>&1
    git clone "file://$dir/origin" "$dir/clone" >/dev/null 2>&1
    if (
        cd "$dir/clone" &&
        git config user.email "t@example.com" &&
        git config user.name "T" &&
        git config core.hooksPath "$REPO_ROOT/githooks" &&
        git checkout -b main >/dev/null 2>&1 &&
        seed_cargo "1.1.0" &&
        git add Cargo.toml &&
        git commit -qm "chore: init at declared version" &&
        git push -q origin main &&
        git tag -a v1.0.0 -m seed &&
        git push -q origin v1.0.0 &&
        git remote rename origin origin-unreachable &&
        mkdir -p crates/x && echo a > crates/x/a.rs &&
        git add crates/x/a.rs &&
        git commit -qm "feat: runtime while tag query broken"
    ); then
        if hook_direct_case REJECT >/dev/null 2>&1; then
            echo "PASS"
        else
            echo "FAIL"
        fi
    else
        echo "FIXFAIL"
    fi
    chmod -R u+rw "$dir" 2>/dev/null || true
    rm -rf "$dir"
)"
case "$res" in
    PASS) PASS_COUNT=$((PASS_COUNT + 1)); echo "PASS  [direct-REJECT] tag query broken (fail-closed)" ;;
    FAIL) FAIL_COUNT=$((FAIL_COUNT + 1)); echo "FAIL  [direct-expected REJECT] tag query broken (fail-closed)" ;;
    *) FAIL_COUNT=$((FAIL_COUNT + 1)); echo "FAIL  fixture error: tag query broken (fail-closed)" ;;
esac

echo ""
echo "=== matrix result: PASS=$PASS_COUNT FAIL=$FAIL_COUNT ==="
if [[ $FAIL_COUNT -ne 0 ]]; then
    exit 1
fi
exit 0
