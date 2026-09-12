#!/bin/bash
# Functional contract test for INC-VAULT-MIRROR-AUTO closure.
#
# Verifies that scripts/release.sh invokes
# scripts/mirror_adrs_to_vault.py between step 8 (checksums + sbom)
# and step 9 (publish), so vault ADR mirrors at
# ~/.sddk-knowledge/sddk-framework/adrs/ stay in sync with the accepted
# ADRs in docs/architecture/adrs/ without requiring a manual
# post-release invocation.
#
# The test pins four invariants by inspecting the script source — it
# does NOT execute release.sh end-to-end (that would publish a real
# GitHub Release). Each scenario checks the ordering and behaviour:
#
#   (a) step 8b exists, between step 8 and step 9.
#   (b) the invocation uses `python3 scripts/mirror_adrs_to_vault.py`
#       (no flags — the script is currently a no-arg main(); passing
#       unknown flags would break the no-arg contract).
#   (c) the failure mode is `warn`, not `die` — vault is human
#       knowledge per AGENTS §2.7, not runtime authority; a mirror
#       failure must not abort the release.
#   (d) the step is unconditional — not gated by --skip-tests or
#       --dry-run. Vault sync is part of the release contract, not
#       the test gate.
#   (e) the script invocation is wrapped in `if MIRROR_OUT="$(...)"`
#       capturing both stdout and stderr so the operator sees the
#       mirror counts (created/skipped) on success or the error on
#       failure.
#
# INC: INC-VAULT-MIRROR-AUTO (CL-RELEASE-PIPELINE-INTEGRITY)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RELEASE_SH="$REPO_ROOT/scripts/release.sh"
MIRROR_PY="$REPO_ROOT/scripts/mirror_adrs_to_vault.py"

# --- precondition: script + mirror both exist ---

if [[ ! -f "$RELEASE_SH" ]]; then
    echo "FAIL: $RELEASE_SH does not exist"
    exit 1
fi

if [[ ! -f "$MIRROR_PY" ]]; then
    echo "FAIL: $MIRROR_PY does not exist — INC-VAULT-MIRROR-AUTO closure requires the mirror script"
    exit 1
fi

echo "Auditing $RELEASE_SH for INC-VAULT-MIRROR-AUTO closure"
echo "===================================================="

LINE_8="$(grep -n '^step "8/14' "$RELEASE_SH" | head -1 | cut -d: -f1)"
LINE_8B="$(grep -n '^step "8b/14' "$RELEASE_SH" | head -1 | cut -d: -f1)"
LINE_9="$(grep -n '^step "9/14' "$RELEASE_SH" | head -1 | cut -d: -f1)"
# shellcheck disable=SC2034  # diagnostic only
LINE_9_UNUSED="$LINE_9"

echo "step 8  at line: $LINE_8"
echo "step 8b at line: $LINE_8B"
echo "step 9  at line: $LINE_9"

# --- (a) step 8b ordering ---

if [[ -z "$LINE_8B" ]]; then
    echo "FAIL (a): step 8b/14 does not exist — INC-VAULT-MIRROR-AUTO is open"
    exit 1
fi

if [[ -z "$LINE_8" || -z "$LINE_9" ]]; then
    echo "FAIL (a): missing step 8 or step 9 — script structure changed"
    exit 1
fi

if ! [[ "$LINE_8" -lt "$LINE_8B" && "$LINE_8B" -lt "$LINE_9" ]]; then
    echo "FAIL (a): step 8b is not between step 8 and step 9"
    echo "        expected: 8 ($LINE_8) < 8b ($LINE_8B) < 9 ($LINE_9)"
    exit 1
fi
echo "PASS (a): step 8b is between step 8 and step 9"

# --- (b) the invocation uses the right script with no flags ---

if ! sed -n "${LINE_8B},$((LINE_9 - 1))p" "$RELEASE_SH" \
        | grep -q "python3 .*mirror_adrs_to_vault\.py"; then
    echo "FAIL (b): step 8b does not invoke mirror_adrs_to_vault.py"
    exit 1
fi

# The script's main() takes no argv; if release.sh passes flags,
# sys.argv processing inside Python will raise IndexError on argv[1].
if sed -n "${LINE_8B},$((LINE_9 - 1))p" "$RELEASE_SH" \
        | grep -qE 'mirror_adrs_to_vault\.py\s+--?[a-zA-Z]'; then
    echo "FAIL (b): step 8b passes flags to mirror_adrs_to_vault.py"
    echo "        The script's main() takes no arguments; passing flags"
    echo "        would raise IndexError on sys.argv[1]."
    exit 1
fi
echo "PASS (b): step 8b invokes the mirror script with no flags"

# --- (c) failure mode is `warn`, not `die` ---

# Vault mirrors are human knowledge per AGENTS §2.7; the repo ADR is
# the runtime authority. A mirror failure must NOT abort the release.
# Assert that the step's failure branch uses `warn`, not `die`.
if ! sed -n "${LINE_8B},$((LINE_9 - 1))p" "$RELEASE_SH" \
        | grep -q 'warn.*mirror\|mirror.*non-fatal\|mirror.*failed'; then
    echo "FAIL (c): step 8b's failure branch does not log a warn message"
    echo "        Vault is human knowledge; mirror failures must not abort"
    echo "        the release."
    exit 1
fi

# And critically, no `die` inside the mirror block
if sed -n "${LINE_8B},$((LINE_9 - 1))p" "$RELEASE_SH" \
        | grep -q 'die.*mirror\|mirror.*die'; then
    echo "FAIL (c): step 8b uses 'die' on mirror failure — vault is"
    echo "        human knowledge, not runtime authority. Mirror failure"
    echo "        must be soft (warn), not abort."
    exit 1
fi
echo "PASS (c): step 8b's failure mode is warn (not die)"

# --- (d) step 8b is unconditional ---

# Find the bounds of the SKIP_TESTS guard (if any). Step 8b must live
# outside that guard.
SKIP_OPEN_LINE="$(grep -n '^if \[ "\$SKIP_TESTS" = "0" \]' "$RELEASE_SH" | head -1 | cut -d: -f1)"
if [[ -n "$SKIP_OPEN_LINE" ]]; then
    SKIP_CLOSE_LINE="$(awk -v start="$SKIP_OPEN_LINE" '
        NR >= start && /^fi$/ { print NR; exit }
    ' "$RELEASE_SH")"
    if [[ -n "$SKIP_CLOSE_LINE" && "$LINE_8B" -lt "$SKIP_CLOSE_LINE" ]]; then
        echo "FAIL (d): step 8b is inside the SKIP_TESTS guard"
        echo "        Vault sync is part of the release contract, not the"
        echo "        test gate; --skip-tests must not skip it."
        exit 1
    fi
fi

# And outside any DRY_RUN guard for the publish block. The dry-run
# short-circuit is at line 370+; step 8b is BEFORE that line, so the
# dry-run path still runs it. Confirm by checking that step 8b's line
# number is less than the dry-run check line.
DRY_LINE="$(grep -n '^if \[ "\$DRY_RUN" = "1" \]' "$RELEASE_SH" | head -1 | cut -d: -f1)"
if [[ -n "$DRY_LINE" && "$LINE_8B" -gt "$DRY_LINE" ]]; then
    echo "FAIL (d): step 8b is AFTER the dry-run short-circuit —"
    echo "        dry-runs would skip vault sync."
    exit 1
fi
echo "PASS (d): step 8b is unconditional (outside SKIP_TESTS, before DRY_RUN)"

# --- (e) stdout+stderr captured into MIRROR_OUT for operator visibility ---

if ! sed -n "${LINE_8B},$((LINE_9 - 1))p" "$RELEASE_SH" \
        | grep -q 'MIRROR_OUT=.*2>&1'; then
    echo "FAIL (e): step 8b does not capture both stdout and stderr"
    echo "        The operator needs to see mirror counts (created/skipped)"
    echo "        on success and any traceback on failure."
    exit 1
fi
echo "PASS (e): step 8b captures stdout+stderr into MIRROR_OUT"

echo ""
echo "=== All INC-VAULT-MIRROR-AUTO closure invariants hold ==="
exit 0
