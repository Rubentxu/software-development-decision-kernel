#!/usr/bin/env bash
# Test the duplicate-guard merge added to release-bump.sh (session-30).
# Uses a fixed sandbox path; creates only what it needs.
set -uo pipefail
SB="/tmp/sddk-bump-merge-test"
mkdir -p "$SB"

cat > "$SB/CHANGELOG.md" <<'EOF'
# Changelog

## [2.2.0] - 2026-09-28

### Fixes
  - fix(a): uno

## [2.1.0] - 2026-09-27

### Fixes
  - fix(b): dos
EOF

CH="$SB/CHANGELOG.md"
NEXT=2.2.0
ENTRY_FILE="$SB/entry.md"
printf '## [%s] - 2026-09-28\n\n### Fixes\n  - fix(c): tres\n\n' "$NEXT" > "$ENTRY_FILE"

# ── the block under test, copied verbatim from release-bump.sh ─────────────
if [ -n "$(grep -nE "^## \[$NEXT\]" "$CH" 2>/dev/null | head -1)" ]; then
    EXIST_LINE="$(grep -nE "^## \[$NEXT\]" "$CH" | head -1 | cut -d: -f1)"
    AFTER_LINE="$(grep -nE '^## \[' "$CH" | awk -F: -v s="$EXIST_LINE" '$1 > s {print $1; exit}')"
    TOTAL="$(wc -l < "$CH")"
    [ -z "$AFTER_LINE" ] && AFTER_LINE="$((TOTAL + 1))"
    TMP_MERGE="$SB/merged"
    {
        head -n "$((AFTER_LINE - 1))" "$CH" | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'
        echo
        tail -n +2 "$ENTRY_FILE"
        echo
        [ "$AFTER_LINE" -le "$TOTAL" ] && tail -n "+$AFTER_LINE" "$CH"
    } > "$TMP_MERGE" && mv "$TMP_MERGE" "$CH"
    MERGED=yes
else
    MERGED=no
fi
# ──────────────────────────────────────────────────────────────────────────

echo "=== merged: $MERGED ==="
cat "$CH"
echo "=== assertions ==="

failures=0
n_headers="$(grep -cE '^## \[2\.2\.0\]' "$CH")"
if [ "$n_headers" -eq 1 ]; then
    echo "  ok   exactly one '## [2.2.0]' header (got $n_headers)"
else
    echo "  FAIL expected 1 header for 2.2.0, got $n_headers"; failures=$((failures+1))
fi

if grep -q 'fix(a): uno' "$CH"; then
    echo "  ok   the pre-existing item survived"
else
    echo "  FAIL the pre-existing item was lost"; failures=$((failures+1))
fi

if grep -q 'fix(c): tres' "$CH"; then
    echo "  ok   the new item was merged in"
else
    echo "  FAIL the new item was not merged"; failures=$((failures+1))
fi

if grep -q 'fix(b): dos' "$CH"; then
    echo "  ok   the following section (2.1.0) survived"
else
    echo "  FAIL the following section was lost"; failures=$((failures+1))
fi

# The following section must keep its own version header.
if grep -qE '^## \[2\.1\.0\]' "$CH"; then
    echo "  ok   the 2.1.0 header is intact"
else
    echo "  FAIL the 2.1.0 header is gone"; failures=$((failures+1))
fi

exit "$failures"
