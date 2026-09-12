#!/bin/bash
# Functional contract test for INC-RELEASE-TAG-FIX closure.
#
# Verifies that scripts/release.sh performs `git push origin main` before
# step 9 (gh release create --target main), so the published tag is
# anchored to the actual release commit and does not require manual
# repointing via `git tag -f ... && git push origin v<X.Y.Z> --force`.
#
# The test pins three invariants by inspecting the script source — it does
# NOT execute `release.sh` end-to-end (that would publish a real GitHub
# Release). Each scenario checks the ordering of relevant blocks:
#
#   (a) step 1c exists, between step 1b (shell contract tests) and step 2
#       (version read), and uses the standard fetch + push logic.
#   (b) step 1c invokes `git push origin main` (not a different refspec,
#       not a force push, not a tag push) — the push target is the branch,
#       so GitHub resolves --target main to the new HEAD.
#   (c) step 1c lives OUTSIDE the SKIP_TESTS guard. Pushing is part of
#       the release contract, not the test gate; --skip-tests must not
#       leave the operator without tag anchoring.
#   (d) Step 1c does not duplicate the bump-commit predicate — the
#       pre-push hook already enforces it. Adding a second predicate here
#       would create two governance surfaces; we explicitly leave the
#       hook as the single source of truth.
#
# INC: INC-RELEASE-TAG-FIX (CL-RELEASE-PIPELINE-INTEGRITY)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RELEASE_SH="$REPO_ROOT/scripts/release.sh"

# --- precondition: script exists ---

if [[ ! -f "$RELEASE_SH" ]]; then
    echo "FAIL: $RELEASE_SH does not exist"
    exit 1
fi

echo "Auditing $RELEASE_SH for INC-RELEASE-TAG-FIX closure"
echo "===================================================="

# Capture line numbers for ordering assertions. Use grep -n which prints
# "line:content"; awk extracts the line number. Numeric comparison is
# done with bash arithmetic.

LINE_1B="$(grep -n 'step "1b/14' "$RELEASE_SH" | head -1 | cut -d: -f1)"
LINE_1C="$(grep -n '^step "1c/14' "$RELEASE_SH" | head -1 | cut -d: -f1)"
LINE_2="$(grep -n '^step "2/14' "$RELEASE_SH" | head -1 | cut -d: -f1)"
LINE_9="$(grep -n '^step "9/14' "$RELEASE_SH" | head -1 | cut -d: -f1)"
# shellcheck disable=SC2034  # diagnostic only
LINE_9_UNUSED="$LINE_9"

echo "step 1b at line: $LINE_1B"
echo "step 1c at line: $LINE_1C"
echo "step 2  at line: $LINE_2"
echo "step 9  at line: $LINE_9"

# --- (a) step 1c ordering ---

if [[ -z "$LINE_1C" ]]; then
    echo "FAIL (a): step 1c/14 does not exist — INC-RELEASE-TAG-FIX is open"
    exit 1
fi

if [[ -z "$LINE_1B" || -z "$LINE_2" ]]; then
    echo "FAIL (a): missing step 1b or step 2 — script structure changed"
    exit 1
fi

if ! [[ "$LINE_1B" -lt "$LINE_1C" && "$LINE_1C" -lt "$LINE_2" ]]; then
    echo "FAIL (a): step 1c is not between step 1b and step 2"
    echo "        expected: 1b ($LINE_1B) < 1c ($LINE_1C) < 2 ($LINE_2)"
    exit 1
fi
echo "PASS (a): step 1c is between step 1b and step 2"

# --- (b) step 1c invokes `git push origin main` (branch, not tag) ---

if ! sed -n "${LINE_1C},$((LINE_2 - 1))p" "$RELEASE_SH" \
        | grep -q 'git push origin main'; then
    echo "FAIL (b): step 1c does not invoke 'git push origin main'"
    exit 1
fi

if sed -n "${LINE_1C},$((LINE_2 - 1))p" "$RELEASE_SH" \
        | grep -q 'git push origin v\?[0-9]'; then
    echo "FAIL (b): step 1c pushes a tag directly — that bypasses the"
    echo "        pre-push hook and the branch-based target resolution."
    echo "        The contract is: push the branch, let gh release create"
    echo "        resolve --target main."
    exit 1
fi

if sed -n "${LINE_1C},$((LINE_2 - 1))p" "$RELEASE_SH" \
        | grep -q -- '--force'; then
    echo "FAIL (b): step 1c uses --force on the branch push — this masks"
    echo "        non-fast-forward failures and silently clobbers origin."
    exit 1
fi
echo "PASS (b): step 1c pushes the branch (no tag, no --force)"

# --- (c) step 1c lives OUTSIDE the SKIP_TESTS guard ---

# The SKIP_TESTS guard opens at step 1 with `if [ "$SKIP_TESTS" = "0" ]`
# and closes with a matching `fi`. We assert that step 1c is AFTER the
# closing fi of the test gate. Use awk to find the matching fi (the first
# `fi` at the same indentation as the `if` open).
SKIP_OPEN_LINE="$(grep -n '^if \[ "\$SKIP_TESTS" = "0" \]' "$RELEASE_SH" | head -1 | cut -d: -f1)"
if [[ -z "$SKIP_OPEN_LINE" ]]; then
    echo "FAIL (c): could not locate SKIP_TESTS guard open"
    exit 1
fi

# Find the first `fi` at column 1 that closes the SKIP_TESTS block.
# The guard starts at $SKIP_OPEN_LINE and the closing fi is the next
# `^fi$` after step 1b completes. Walk down from SKIP_OPEN_LINE.
SKIP_CLOSE_LINE="$(awk -v start="$SKIP_OPEN_LINE" '
    NR >= start && /^fi$/ { print NR; exit }
' "$RELEASE_SH")"

if [[ -z "$SKIP_CLOSE_LINE" ]]; then
    echo "FAIL (c): could not locate SKIP_TESTS guard close"
    exit 1
fi

if [[ "$LINE_1C" -lt "$SKIP_CLOSE_LINE" ]]; then
    echo "FAIL (c): step 1c is inside the SKIP_TESTS guard (line $LINE_1C"
    echo "        before guard close at $SKIP_CLOSE_LINE)"
    echo "        --skip-tests would skip the push, leaving the tag stale."
    exit 1
fi
echo "PASS (c): step 1c is outside the SKIP_TESTS guard"

# --- (d) step 1c does NOT duplicate the bump-commit predicate ---

# The pre-push hook (githooks/pre-push) is the single source of truth for
# the chore(release): bump version predicate. If release.sh step 1c adds
# a second predicate (e.g. checking `git log -1 --format=%s` itself), it
# creates two governance surfaces that can drift. We assert that step 1c
# does not parse `git log -1 --format=%s` itself; it relies on the hook.
if sed -n "${LINE_1C},$((LINE_2 - 1))p" "$RELEASE_SH" \
        | grep -q 'git log -1 --format=%s'; then
    echo "FAIL (d): step 1c duplicates the bump-commit predicate"
    echo "        The pre-push hook is the single source of truth."
    exit 1
fi
echo "PASS (d): step 1c delegates the predicate to the pre-push hook"

# --- (e) step 1c fail-closes on non-fast-forward ---

# If origin/main has advanced concurrently, step 1c must refuse to
# release (rather than silently force-pushing or skipping). The block
# uses `git merge-base --is-ancestor` to detect divergence.
if ! sed -n "${LINE_1C},$((LINE_2 - 1))p" "$RELEASE_SH" \
        | grep -q 'git merge-base --is-ancestor'; then
    echo "FAIL (e): step 1c does not use merge-base ancestor check"
    echo "        Without that check, concurrent advances are not detected"
    echo "        and the script may silently push over a faster remote."
    exit 1
fi

if ! sed -n "${LINE_1C},$((LINE_2 - 1))p" "$RELEASE_SH" \
        | grep -q 'die.*origin/main.*ahead'; then
    echo "FAIL (e): step 1c does not fail-closed with a clear message"
    echo "        on origin/main ahead-of-HEAD."
    exit 1
fi
echo "PASS (e): step 1c fail-closes with merge-base ancestor check"

echo ""
echo "=== All INC-RELEASE-TAG-FIX closure invariants hold ==="
exit 0
