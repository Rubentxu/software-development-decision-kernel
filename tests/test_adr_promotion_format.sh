#!/usr/bin/env bash
# test_adr_promotion_format.sh — pin test for ADR-0001 §3.4 frontmatter invariants
#
# After ADR-0001 is accepted, every ADR file in docs/architecture/adrs/
# that claims `status: accepted` or `status: released` MUST carry the
# corresponding frontmatter fields:
#   - `accepted` / `released` state ⇒ `accepted_at:` non-empty
#   - `accepted` / `released` state ⇒ `accepted_by_cycle:` non-empty
#   - `released` state ⇒ `released_at:` non-empty
#   - `released` state ⇒ `released_in_milestone:` non-empty
#
# This test fails fast if any ADR claims a binding state without the
# governance evidence. Per ADR-0001 §3.5, the test is the regression
# guard for the frontmatter convention.

set -euo pipefail

# Resolve repo root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ADR_DIR="$REPO_ROOT/docs/architecture/adrs"

if [ ! -d "$ADR_DIR" ]; then
    echo "FAIL: $ADR_DIR not found"
    exit 1
fi

violations=0
checked=0

for adr_file in "$ADR_DIR"/ADR-*.md; do
    [ -f "$adr_file" ] || continue
    checked=$((checked + 1))

    # Read frontmatter (between first --- and second ---)
    frontmatter="$(awk '/^---$/{f++; next} f==1{print}' "$adr_file")"
    [ -n "$frontmatter" ] || {
        echo "FAIL: $adr_file has no frontmatter"
        violations=$((violations + 1))
        continue
    }

    # Extract status
    status="$(printf '%s\n' "$frontmatter" \
        | grep -E '^[[:space:]]*status[[:space:]]*:' \
        | head -1 \
        | sed -E 's/^[[:space:]]*status[[:space:]]*:[[:space:]]*//')"
    case "$status" in
        accepted|released) ;;
        proposed|deprecated|superseded|"") continue ;;  # states not bound by §3.4
        *)
            echo "FAIL: $adr_file has unknown status: $status"
            violations=$((violations + 1))
            continue
            ;;
    esac

    # For accepted/released: check accepted_at + accepted_by_cycle present
    # Use POSIX character class via [:space:] inside double brackets
    if ! printf '%s\n' "$frontmatter" | grep -qE '^[[:space:]]*accepted_at:[[:space:]]*[^[:space:]]'; then
        echo "FAIL: $adr_file (status=$status) missing non-empty accepted_at:"
        violations=$((violations + 1))
    fi
    if ! printf '%s\n' "$frontmatter" | grep -qE '^[[:space:]]*accepted_by_cycle:[[:space:]]*[^[:space:]]'; then
        echo "FAIL: $adr_file (status=$status) missing non-empty accepted_by_cycle:"
        violations=$((violations + 1))
    fi

    # For released only: check released_at + released_in_milestone present
    if [ "$status" = "released" ]; then
        if ! printf '%s\n' "$frontmatter" | grep -qE '^[[:space:]]*released_at:[[:space:]]*[^[:space:]]'; then
            echo "FAIL: $adr_file (status=released) missing non-empty released_at:"
            violations=$((violations + 1))
        fi
        if ! printf '%s\n' "$frontmatter" | grep -qE '^[[:space:]]*released_in_milestone:[[:space:]]*[^[:space:]]'; then
            echo "FAIL: $adr_file (status=released) missing non-empty released_in_milestone:"
            violations=$((violations + 1))
        fi
    fi
done

echo ""
echo "checked: $checked ADR files in $ADR_DIR"
echo "violations: $violations"

if [ "$violations" -gt 0 ]; then
    echo ""
    echo "REGRESSION: frontmatter convention from ADR-0001 §3.4 violated."
    echo "Fix by adding the missing frontmatter fields to each ADR listed above."
    exit 1
fi

# Sanity assertion: at least one ADR is now accepted (the meta-ADR itself + the
# promoted batch in this cycle). This guards against silent rollback to "all
# proposed" without going through the documented promotion process.
accepted_count="$(grep -lE '^[[:space:]]*status[[:space:]]*:[[:space:]]*accepted[[:space:]]*$' "$ADR_DIR"/ADR-*.md 2>/dev/null | wc -l)"
echo "accepted ADRs: $accepted_count"
if [ "$accepted_count" -lt 1 ]; then
    echo ""
    echo "REGRESSION: zero ADRs accepted. Per cycle adr-promotion-m0-meta-and-batch-1,"
    echo "ADR-0001 + at least 2 promoted ADRs must be accepted at this commit."
    exit 1
fi

echo ""
echo "OK: ADR promotion frontmatter convention (ADR-0001 §3.4) honored."
