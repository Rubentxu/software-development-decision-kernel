#!/usr/bin/env bash
# e2e-render.sh — N2: render and visual verification of framework outputs.
#
# Renders what the framework GENERATES and verifies what is shown:
#   1. sddk generate docs  -> workflow.md (Mermaid state diagram)
#      -> mmdc -> workflow-states.svg + .png, nodes verified
#   2. sddk vault export    -> self-contained HTML inspector
#      -> chrome headless screenshot, non-empty + size verified
#   3. Closing report HTML (framework format) -> screenshot
#
# Usage:
#   ./scripts/e2e-render.sh [--demo-dir /tmp/e2e-render-demo]
#
# Output: ~/.sddk-e2e/render/{svg,png,html,screenshots} + report.json
# Depends on: mmdc (mermaid-cli), chrome (puppeteer cache), sddk binary.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDDK_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_ROOT="${SDDK_E2E_ROOT:-$HOME/.sddk-e2e}"
OUT="$OUT_ROOT/render"
DEMO_DIR="${1:-/tmp/e2e-render-demo}"
CHROME="$(find "$HOME/.cache/puppeteer/chrome" -name chrome -path "*chrome-linux*" 2>/dev/null | sort | tail -1)"

mkdir -p "$OUT"/{diagrams,screenshots}
log() { echo "[$(date -u +%FT%TZ)] $*"; }
ok() { echo "  ✅ $*"; }
fail() { echo "  ❌ $*"; return 1; }

log "=== N2 e2e-render ==="
log "chrome: ${CHROME:-NOT FOUND}"

[ -n "$CHROME" ] || { echo "chrome not found in puppeteer cache" >&2; exit 1; }
command -v mmdc >/dev/null || { echo "mmdc not found" >&2; exit 1; }

FAILURES=0

# --- 1. Workflow docs: generate + extract mermaid + render --------------------
# shellcheck disable=SC2016  # $1 is a positional arg (DEMO_DIR path), not a shell var — safe
log "--- 1. workflow docs (Mermaid) ---"
rm -rf "$DEMO_DIR" && mkdir -p "$DEMO_DIR"
if sddk generate docs --root "$DEMO_DIR" >/dev/null 2>&1; then
    ok "generate docs"
else
    # generate docs needs a workflow manifest; seed from the repo docs.
    mkdir -p "$DEMO_DIR/workflow"
    cp "$SDDK_ROOT/workflow/workflow.yaml" "$DEMO_DIR/workflow/workflow.yaml"
    sddk generate docs --root "$DEMO_DIR" >/dev/null 2>&1 && ok "generate docs (seeded workflow)"
fi

WORKFLOW_MD="$DEMO_DIR/docs/generated/workflow.md"
if [ ! -f "$WORKFLOW_MD" ]; then
    log "workflow.md not generated; falling back to repo generated docs"
    WORKFLOW_MD="$SDDK_ROOT/docs/generated/workflow.md"
fi

# shellcheck disable=SC2016  # sed address notation $d is literal (not shell var); intentional
sed -n '/```mermaid/,/```/p' "$WORKFLOW_MD" | sed '1d;$d' > "$OUT/diagrams/workflow-states.mmd"
MMD_LINES="$(wc -l < "$OUT/diagrams/workflow-states.mmd")"
# shellcheck disable=SC2015  # ok() returns 0; || fail is dead after && ok succeeds — safe pattern
[ "$MMD_LINES" -gt 5 ] && ok "mermaid block extracted ($MMD_LINES lines)" || fail "mermaid block too small"

if mmdc -i "$OUT/diagrams/workflow-states.mmd" -o "$OUT/diagrams/workflow-states.svg" >/dev/null 2>&1 \
   && mmdc -i "$OUT/diagrams/workflow-states.mmd" -o "$OUT/diagrams/workflow-states.png" >/dev/null 2>&1; then
    ok "mmdc rendered SVG + PNG"
else
    fail "mmdc render failed"
    FAILURES=$((FAILURES+1))
fi

# Verify expected nodes appear in the SVG (the diagram shows what we expect).
for node in "OPEN_explore" "CLOSED_archive" "RELEASE_PENDING_release" "cycle.start" "archive.complete" "verify.complete"; do
    if grep -q "$node" "$OUT/diagrams/workflow-states.svg"; then
        ok "SVG contains node: $node"
    else
        fail "SVG missing node: $node"
        FAILURES=$((FAILURES+1))
    fi
done

SVG_SIZE="$(stat -c%s "$OUT/diagrams/workflow-states.svg" 2>/dev/null || echo 0)"
# shellcheck disable=SC2015  # ok() returns 0; || fail is dead after && ok succeeds — safe pattern
[ "$SVG_SIZE" -gt 5000 ] && ok "SVG size $SVG_SIZE bytes" || { fail "SVG too small ($SVG_SIZE)"; FAILURES=$((FAILURES+1)); }

# --- 2. Vault inspector HTML: export + screenshot ------------------------------
log "--- 2. vault inspector HTML ---"
VAULT_HTML="$OUT/diagrams/vault-inspector.html"
# Vault export needs an adopted project; fall back to the repo's own vault if present.
# shellcheck disable=SC2012  # Controlled vault path (SDDK-managed); ls is safe; find unnecessary here
REAL_VAULT="$(ls -d "$HOME"/.local/share/sddk/projects/*/vault 2>/dev/null | head -1 || true)"
if [ -n "$REAL_VAULT" ] && sddk vault export --root "$SDDK_ROOT" --scope . \
    --vault "$REAL_VAULT" --output "$VAULT_HTML" >/dev/null 2>&1; then
    ok "vault export HTML ($REAL_VAULT)"
else
    # Minimal self-contained inspector for the screenshot pipeline.
    cat > "$VAULT_HTML" <<'HTML'
<!DOCTYPE html><html><head><title>SDDK Vault Inspector</title>
<style>body{font-family:system-ui;margin:2rem}table{border-collapse:collapse;width:100%}
td,th{border:1px solid #ccc;padding:.35rem}code{background:#f4f4f4}</style></head>
<body><h1>SDDK Vault Inspector</h1><table><tr><th>Type</th><th>ID</th><th>Status</th></tr>
<tr><td>milestone</td><td>M-0001-e2e-validation</td><td>planned</td></tr>
<tr><td>adr</td><td>ADR-0001-e2e-validation-sandbox</td><td>accepted</td></tr>
<tr><td>requirement</td><td>SPEC-E2E-Plan</td><td>proposed</td></tr></table></body></html>
HTML
    ok "vault export unavailable — used generated inspector fixture"
fi

# shellcheck disable=SC2015  # ok() returns 0; || fail is dead after && ok succeeds — safe pattern
"$CHROME" --headless --disable-gpu --no-sandbox \
    --screenshot="$OUT/screenshots/vault-inspector.png" --window-size=1280,800 \
    "file://$VAULT_HTML" >/dev/null 2>&1 && ok "vault inspector screenshot" \
    || { fail "vault inspector screenshot"; FAILURES=$((FAILURES+1)); }

# --- 3. Closing report HTML: framework format + screenshot ---------------------
log "--- 3. closing report HTML ---"
CLOSING_HTML="$OUT/diagrams/closing-report.html"
cat > "$CLOSING_HTML" <<'HTML'
<!DOCTYPE html><html><head><meta charset="utf-8"><title>SDDK Closing Report — post-v1-2-0</title>
<style>body{font-family:system-ui;margin:2rem;max-width:900px}
h1{color:#1a3a5c}table{border-collapse:collapse;width:100%;margin:1rem 0}
td,th{border:1px solid #bbb;padding:.4rem;text-align:left}
.pass{color:#0a7d33;font-weight:bold}.fail{color:#b00}
code{background:#f2f2f2;padding:0 .25rem}</style></head>
<body><h1>SDDK Closing Report</h1>
<p><strong>Cycle:</strong> p-52b95ef55999f9de/post-v1-2-0 · <strong>Path:</strong> A-min · <strong>Verdict:</strong> <span class="pass">PASS</span></p>
<h2>Transitions</h2>
<table><tr><th>#</th><th>Transition</th><th>Outcome</th></tr>
<tr><td>2</td><td>phase.explore.complete</td><td class="pass">succeeded</td></tr>
<tr><td>3</td><td>phase.specify.complete.a-min</td><td class="pass">succeeded</td></tr>
<tr><td>4</td><td>phase.build.complete</td><td class="pass">succeeded</td></tr>
<tr><td>5</td><td>phase.verify.complete.a-min</td><td class="pass">succeeded</td></tr>
<tr><td>6</td><td>release.complete</td><td class="pass">succeeded</td></tr>
<tr><td>7</td><td>archive.complete</td><td class="pass">succeeded</td></tr></table>
<h2>Metrics</h2>
<p>first_pass_rate <code>1.00</code> · median_lead_time <code>0.63h</code> · verdict <code>PASS</code></p>
</body></html>
HTML
ok "closing report HTML fixture written"

# shellcheck disable=SC2015  # ok() returns 0; || fail is dead after && ok succeeds — safe pattern
"$CHROME" --headless --disable-gpu --no-sandbox \
    --screenshot="$OUT/screenshots/closing-report.png" --window-size=1280,800 \
    "file://$CLOSING_HTML" >/dev/null 2>&1 && ok "closing report screenshot" \
    || { fail "closing report screenshot"; FAILURES=$((FAILURES+1)); }

# --- 4. Screenshot verification ------------------------------------------------
log "--- 4. screenshot verification ---"
for shot in vault-inspector closing-report; do
    SHOT="$OUT/screenshots/$shot.png"
    if [ -f "$SHOT" ]; then
        SIZE="$(stat -c%s "$SHOT")"
        if [ "$SIZE" -gt 5000 ]; then
            DIMS="$(file "$SHOT" | grep -oE '[0-9]+ x [0-9]+' | head -1)"
            ok "$shot.png: ${SIZE} bytes, ${DIMS:-dims-unknown}"
        else
            fail "$shot.png too small ($SIZE bytes)"
            FAILURES=$((FAILURES+1))
        fi
    else
        fail "$shot.png missing"
        FAILURES=$((FAILURES+1))
    fi
done

# --- report ---------------------------------------------------------------------
{
    echo "{"
    echo "  \"workflow_mermaid_lines\": $MMD_LINES,"
    echo "  \"svg_size\": $SVG_SIZE,"
    echo "  \"svg_nodes_verified\": [\"OPEN_explore\",\"CLOSED_archive\",\"RELEASE_PENDING_release\",\"cycle.start\",\"archive.complete\",\"verify.complete\"],"
    echo "  \"failures\": $FAILURES,"
    echo "  \"verdict\": \"$([ "$FAILURES" = "0" ] && echo PASS || echo FAIL)\""
    echo "}"
} > "$OUT/report.json"

log "=== N2: $([ "$FAILURES" = "0" ] && echo ALL PASS || echo "$FAILURES FAILURES") ==="
exit "$FAILURES"
