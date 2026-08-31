#!/usr/bin/env bash
# e2e-all.sh — orchestrates the full E2E validation suite.
#
#   N1  installation E2E (variants a-d)  -> scripts/e2e-install.sh
#   N2  render + visual verification     -> scripts/e2e-render.sh
#   ML  multi-language validation        -> scripts/validate-project.sh --lang
#   RR  consolidated report              -> docs/validation/e2e-report.md
#
# Usage:
#   ./scripts/e2e-all.sh                          # N1+N2 (+ML if --lang given)
#   ./scripts/e2e-all.sh --lang python            # include one language
#   ./scripts/e2e-all.sh --lang all               # rust python go node c
#   ./scripts/e2e-all.sh --version v1.3.0         # pinned release for N1
#
# Output: ~/.sddk-e2e/*/report.json + docs/validation/e2e-report.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDDK_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_ROOT="${SDDK_E2E_ROOT:-$HOME/.sddk-e2e}"
VERSION="${SDDK_VERSION:-latest}"
LANG="${E2E_LANG:-}"

while [ $# -gt 0 ]; do
    case "$1" in
        --lang) LANG="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

log() { echo "[$(date -u +%FT%TZ)] $*"; }

log "=== E2E-ALL: version=$VERSION lang=${LANG:-none} ==="

SUMMARY="$OUT_ROOT/summary.json"
echo "{" > "$SUMMARY"
FIRST=1

record() {
    local name="$1" verdict="$2"
    if [ "$FIRST" = "1" ]; then FIRST=0; else echo "," >> "$SUMMARY"; fi
    printf '  "%s": "%s"' "$name" "$verdict" >> "$SUMMARY"
}

# --- N1: installation ---------------------------------------------------------
log "--- N1: installation ---"
if bash "$SCRIPT_DIR/e2e-install.sh" --variant all --version "$VERSION"; then
    record n1-install PASS
else
    record n1-install FAIL
fi

# --- N2: render ----------------------------------------------------------------
log "--- N2: render ---"
if bash "$SCRIPT_DIR/e2e-render.sh"; then
    record n2-render PASS
else
    record n2-render FAIL
fi

# --- ML: multi-language --------------------------------------------------------
case "$LANG" in
    "") log "--- ML: skipped (pass --lang) ---" ;;
    all) for l in rust python go node c; do
             log "--- ML: $l ---"
             if bash "$SCRIPT_DIR/validate-project.sh" "fixture-$l" --lang "$l" --fixture; then
                 record "ml-$l" PASS
             else
                 record "ml-$l" FAIL
             fi
         done ;;
    *)  log "--- ML: $LANG ---"
        if bash "$SCRIPT_DIR/validate-project.sh" "fixture-$LANG" --lang "$LANG" --fixture; then
            record "ml-$LANG" PASS
        else
            record "ml-$LANG" FAIL
        fi ;;
esac

echo "" >> "$SUMMARY"
echo "}" >> "$SUMMARY"

# --- RR: consolidated report ---------------------------------------------------
REPORT="$SDDK_ROOT/docs/validation/e2e-report.md"
{
    echo "# SDDK E2E Validation Report"
    echo
    echo "**Date:** $(date -u +%Y-%m-%dT%H:%MZ)"
    echo "**Version under test:** $VERSION"
    echo "**Stack:** podman + act (local CI) + mmdc + chrome headless"
    echo
    echo "## Summary"
    echo
    echo '| Suite | Verdict |'
    echo '|-------|---------|'
    while IFS= read -r line; do
        name="$(echo "$line" | grep -oE '"[a-z0-9-]+"' | head -1 | tr -d '"' || true)"
        verdict="$(echo "$line" | grep -oE '"(PASS|FAIL)"' | tr -d '"' || true)"
        [ -n "$name" ] && echo "| $name | $verdict |"
    done < "$SUMMARY"
    echo
    echo "## Evidence"
    echo
    echo "- N1 reports: \`~/.sddk-e2e/{a,b,c,d}/report.json\`"
    echo "- N2 artifacts: \`~/.sddk-e2e/render/diagrams/workflow-states.svg\` + \`screenshots/*.png\`"
} > "$REPORT"
log "report: $REPORT"
cat "$SUMMARY"
log "=== E2E-ALL done ==="
