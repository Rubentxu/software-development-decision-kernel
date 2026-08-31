#!/bin/bash
# Functional contract test for githooks/pre-push mechanical push prevention
# INC: INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION (CL-APPLY-PUSH-DISCIPLINE cluster)
#
# Scenarios tested:
# (a) push WITHOUT release commit to main → rejected with INC identifier
# (b) push WITH chore(release): bump version commit to main → accepted
# (c) push to non-main branch → accepted unconditionally
#
# No network access required (local-path remotes only), tempfile-isolated.

set -euo pipefail

# Absolute path to this script's directory
# shellcheck disable=SC2329  # cleanup() invoked via trap on EXIT — shellcheck false positive
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOK_PATH="$REPO_ROOT/githooks/pre-push"

# RED phase check: hook must exist and be executable
if [[ ! -f "$HOOK_PATH" ]]; then
    echo "RED phase: githooks/pre-push does not exist"
    echo "FAIL: hook file missing — test cannot run"
    exit 1
fi

if [[ ! -x "$HOOK_PATH" ]]; then
    echo "RED phase: githooks/pre-push is not executable"
    echo "FAIL: hook not executable — test cannot run"
    exit 1
fi

echo "Hook found and executable: $HOOK_PATH"

# Temp directory for test isolation
WORKDIR=""
ORIGIN_DIR=""

# shellcheck disable=SC2329  # cleanup() invoked via trap on EXIT — shellcheck false positive
cleanup() {
    local exit_code=$?
    if [[ -n "$WORKDIR" && -d "$WORKDIR" ]]; then
        chmod -R u+rw "$WORKDIR" 2>/dev/null || true
        rm -rf "$WORKDIR"
    fi
    if [[ -n "$ORIGIN_DIR" && -d "$ORIGIN_DIR" ]]; then
        chmod -R u+rw "$ORIGIN_DIR" 2>/dev/null || true
        rm -rf "$ORIGIN_DIR"
    fi
    exit "$exit_code"
}
trap cleanup EXIT

# Create temp directories for isolation
WORKDIR=$(mktemp -d)
ORIGIN_DIR=$(mktemp -d)

echo "=== Contract test: githooks/pre-push ==="
echo "Work dir: $WORKDIR"
echo "Origin dir: $ORIGIN_DIR"

# --- Setup: create a bare repo as "origin" with an initial release commit ---
git init --bare "$ORIGIN_DIR" >/dev/null 2>&1

# Clone the bare repo
git clone "file://$ORIGIN_DIR" "$WORKDIR/clone" >/dev/null 2>&1
cd "$WORKDIR/clone"

# Configure identity for commits
git config user.email "test@example.com"
git config user.name "Test User"

# Create initial release commit on main (so origin has main with a release commit)
echo "initial" > init.txt
git add init.txt
git commit -m "chore(release): bump version 0.0.1 -> 1.0.0" >/dev/null
git branch -M main

# Point the clone's hooks to our repo's githooks directory
git config core.hooksPath "$REPO_ROOT/githooks"

# Push initial release commit to origin
echo "Pushing initial release commit to origin..."
if ! git push origin main 2>&1; then
    echo "FAIL: initial push with release commit was rejected"
    exit 1
fi
echo "Initial release commit pushed successfully"

echo ""
echo "--- Scenario (c): push to non-main branch (should always pass) ---"

# Create a feature branch with a non-release commit
git checkout -b feature/test 2>/dev/null
echo "non-release content" > file.txt
git add file.txt
git commit -m "feat(uat): add test file" >/dev/null

# Push feature branch — should succeed even without release commit
if git push origin feature/test 2>&1; then
    echo "PASS: feature branch push succeeded as expected"
else
    echo "FAIL: feature branch push was rejected (hook misconfigured?)"
    exit 1
fi

echo ""
echo "--- Scenario (a): push to main WITHOUT release commit (should be rejected) ---"

# Switch back to main and make a non-release commit
git checkout main 2>/dev/null
echo "regular commit" > regular.txt
git add regular.txt
git commit -m "fix(uat): regular fix" >/dev/null

# Capture stderr and exit code
# Note: git push returns 1 when hook rejects, but the error goes to stderr
OUTPUT=$(git push origin main 2>&1 || echo "PUSH_FAILED_WITH_CODE_$?")
EXIT_CODE=${PIPESTATUS[0]}

# Extract actual exit code from output if push failed
if echo "$OUTPUT" | grep -q "PUSH_FAILED_WITH_CODE_"; then
    ACTUAL_EXIT_CODE=$(echo "$OUTPUT" | grep "PUSH_FAILED_WITH_CODE_" | sed 's/.*PUSH_FAILED_WITH_CODE_\([0-9]*\).*/\1/')
    OUTPUT=$(echo "$OUTPUT" | grep -v "PUSH_FAILED_WITH_CODE_")
else
    ACTUAL_EXIT_CODE=$EXIT_CODE
fi

echo "Exit code: $ACTUAL_EXIT_CODE"
echo "Output:"
echo "$OUTPUT"

if [[ $ACTUAL_EXIT_CODE -eq 0 ]]; then
    echo "FAIL: push to main succeeded but should have been rejected"
    exit 1
fi

if ! echo "$OUTPUT" | grep -q "INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION"; then
    echo "FAIL: error message does not name the INC identifier"
    exit 1
fi

if ! echo "$OUTPUT" | grep -qi "release"; then
    echo "FAIL: error message does not name the discipline (release)"
    exit 1
fi

echo "PASS: push to main without release commit was correctly rejected"

echo ""
echo "--- Scenario (b): push to main WITH release commit (should succeed) ---"

# Amend the last commit to be a release commit
git commit --amend -m "chore(release): bump version 1.56.0 -> 1.57.0" >/dev/null

# Push main with release commit — should succeed
if git push origin main 2>&1; then
    echo "PASS: main push with release commit succeeded as expected"
else
    echo "FAIL: main push with release commit was rejected (hook misconfigured?)"
    exit 1
fi

# Verify the origin main advanced
MAIN_SHA_AFTER=$(git rev-parse refs/heads/main)
git fetch origin main >/dev/null 2>&1
ORIGIN_MAIN_SHA=$(git rev-parse origin/main)

if [[ "$MAIN_SHA_AFTER" != "$ORIGIN_MAIN_SHA" ]]; then
    echo "FAIL: origin/main did not advance to the pushed commit"
    exit 1
fi

echo "PASS: origin/main advanced correctly"

echo ""
echo "=== All contract test scenarios passed ==="
exit 0
