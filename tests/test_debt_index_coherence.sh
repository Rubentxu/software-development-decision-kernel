#!/bin/bash
# Contract test for scripts/check_debt_index_coherence.sh.
#
# The guard only earns its place if it FAILS when the defect is
# reintroduced. A guard that cannot go red is decoration, so every
# case below builds a throwaway debt directory and asserts the exact
# outcome — including the direction of the fix, because the guard's
# message tells the reader to correct the INDEX, not the document.
#
# Temp dirs only. No network. No mutation of the real repo.

# shellcheck disable=SC2329  # functions are invoked indirectly (by name)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GUARD="$REPO_ROOT/scripts/check_debt_index_coherence.sh"

if [[ ! -f "$GUARD" ]]; then
    echo "FAIL: $GUARD missing"
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

PASS=0
FAIL=0

# seed_repo <dir> — a minimal but real-shaped debt tree.
#   $1 = dir
seed_repo() {
    local dir="$1"
    mkdir -p "$dir/docs/debt"
    cat > "$dir/docs/debt/INC-DEMO-OPEN.md" <<'EOF'
---
id: INC-DEMO-OPEN
status: open
severity: high
priority: P1
---
# open
EOF
    cat > "$dir/docs/debt/INC-DEMO-CLOSED.md" <<'EOF'
---
id: INC-DEMO-CLOSED
status: closed
severity: medium
priority: P2
---
# closed
EOF
    cat > "$dir/docs/debt/INC-DEMO-PROSE.md" <<'EOF'
# prose only, no frontmatter

**Estado:** OPEN

body
EOF
    # The shape INC-DEBT-052 really had: bold but LOWERCASE `status:`, which is
    # neither YAML frontmatter nor the `**Estado:**` dialect the guard reads.
    # It looks machine-readable and is not, which is worse than prose.
    cat > "$dir/docs/debt/INC-DEMO-BOLD-STATUS.md" <<'EOF'
# bold lowercase status, no frontmatter

**status:** resolved (session-99)

body
EOF
}

# write_index <dir> <rows...> where each row is "ID|file.md|meta"
write_index() {
    local dir="$1"
    shift
    {
        echo "# Debt Documentation"
        echo
        for row in "$@"; do
            local id file meta
            IFS='|' read -r id file meta <<<"$row"
            printf -- '- **[%s](./%s)** (%s)\n' "$id" "$file" "$meta"
        done
    } > "$dir/docs/debt/README.md"
}

# run_case <expected: PASS|FAIL> <expected_exit> <name> <index-row>...>
# The last rows are written to the index of a fresh temp repo.
run_case() {
    local expected="$1" want_exit="$2" name="$3"
    shift 3
    local dir
    dir="$(mktemp -d "$TMPROOT/case.XXXXXX")"
    seed_repo "$dir"
    write_index "$dir" "$@"

    local out rc
    # SDDK_DEBT_DIR is what makes the guard look at THIS fixture. Without
    # it the guard resolves its own repo and every case would pass for the
    # wrong reason — which is what the first run of this test proved.
    out="$(SDDK_DEBT_DIR="$dir/docs/debt" bash "$GUARD" 2>&1)"
    rc=$?

    if [[ "$expected" == "PASS" ]]; then
        if [[ $rc -eq $want_exit ]]; then
            PASS=$((PASS + 1))
            printf '  [ok]   %s\n' "$name"
        else
            FAIL=$((FAIL + 1))
            printf '  [FAIL] %s (expected exit %s, got %s)\n' "$name" "$want_exit" "$rc"
            printf '         %s\n' "${out//$'\n'/$'\n'         }"
        fi
    else
        if [[ $rc -eq $want_exit ]]; then
            PASS=$((PASS + 1))
            printf '  [ok]   %s\n' "$name"
        else
            FAIL=$((FAIL + 1))
            printf '  [FAIL] %s (expected exit %s, got %s)\n' "$name" "$want_exit" "$rc"
            printf '         %s\n' "${out//$'\n'/$'\n'         }"
        fi
    fi
}

echo "== guard: coherencia indice<->documento de deuda =="

# --- coherent baselines -------------------------------------------------
run_case PASS 0 \
    "index open matches document open" \
    "INC-DEMO-OPEN|INC-DEMO-OPEN.md|high/P1, open"

run_case PASS 0 \
    "index closed matches document closed" \
    "INC-DEMO-CLOSED|INC-DEMO-CLOSED.md|medium/P2, closed"

run_case PASS 0 \
    "index annotates the closing session and stays coherent" \
    "INC-DEMO-CLOSED|INC-DEMO-CLOSED.md|medium/P2, **closed** in session-33, **index reconciled** in session-44"

# --- the defect this guard exists for ------------------------------------
run_case FAIL 1 \
    "index says open, document says closed (the session-43 class)" \
    "INC-DEMO-CLOSED|INC-DEMO-CLOSED.md|medium/P2, open"

run_case FAIL 1 \
    "index says closed, document says open (the inverse)" \
    "INC-DEMO-OPEN|INC-DEMO-OPEN.md|high/P1, closed"

run_case PASS 0 \
    "index says blocked, document says open (both unsettled, coherent by design)" \
    "INC-DEMO-OPEN|INC-DEMO-OPEN.md|high/P1, blocked"

# The guard compares open-ness, not the literal word. A document that
# says `blocked` is still unresolved, so an index saying `blocked` for
# an `open` document is NOT a divergence — asserting otherwise here
# would be asserting a defect the guard deliberately does not have.
# The first run of this test made exactly that mistake.

# --- unreadable / unknown must not pass vacuously ------------------------
run_case FAIL 1 \
    "index points at a file that does not exist" \
    "INC-DEMO-GONE|INC-DEMO-GONE.md|high/P1, open"

run_case PASS 0 \
    "document with prose-only status is read, not ignored" \
    "INC-DEMO-PROSE|INC-DEMO-PROSE.md|high/P1, open"

# A dialect the guard does not know is UNREADABLE, never "coherent because we
# could not tell". This is INC-DEBT-052 as it really was: the document declared
# `**status:** resolved`, the index agreed, and the only honest outcome is that
# nobody can verify the agreement. Accepting the row would make a debt entry
# whose state no one can read indistinguishable from one whose state checks out.
run_case FAIL 1 \
    "status in an unknown dialect is unreadable, not assumed coherent" \
    "INC-DEMO-BOLD|INC-DEMO-BOLD-STATUS.md|medium/P1, resolved"

# --- prose must not neutralize the declared state -------------------------
# Regression for a real defect found in session-45d, on this repo's own
# index. The guard used to collect EVERY state word in the row's metadata
# segment, so a row that declared `closed` but mentioned `open` in its
# annotation matched both states. The intersection was then always
# non-empty, so `idx_unsettled` was true for that row no matter what, and
# the genuine divergence went unreported.
#
# This is the worst shape a guard can have: its own prose switched it off,
# and the suite still printed "the guard fails exactly when the index
# diverges". Each case below declares the state OPPOSITE to its
# document, so the only thing that can make the row pass is prose.
#
# No parentheses in the annotation: the row parser captures the metadata
# with `([^)]*)`, so a `)` in the prose truncates the segment. That is a
# separate limitation, recorded here so these cases stay meaningful.
run_case FAIL 1 \
    "prose mentioning 'open' does not neutralize a declared closed state" \
    "INC-DEMO-OPEN|INC-DEMO-OPEN.md|medium/P2, closed — still open until the ledger adapter lands"

run_case FAIL 1 \
    "prose mentioning 'closed' does not neutralize a declared open state" \
    "INC-DEMO-CLOSED|INC-DEMO-CLOSED.md|medium/P2, open — the defect is corrected and fail-closed, exit 4"

echo
echo "== wiring: el guard tiene que ejecutarse en algun sitio =="

# Fixture coverage alone does not tell you the guard ever RUNS against the real
# repo. `ci.yml:46` shellchecks `scripts/*.sh` — that is lint, not execution —
# and this file itself only ever points the guard at throwaway trees. Between
# them, nothing executed the guard: it sat at exit 1 on this repo from
# session-65b with 10/10 green tests. So the wiring is itself load-bearing and
# needs its own assertion, or it can be deleted without anything going red.
if grep -q "check_debt_index_coherence\.sh" "$REPO_ROOT/scripts/release.sh"; then
    PASS=$((PASS + 1))
    printf '  [ok]   release.sh ejecuta el guard real contra el repo\n'
else
    FAIL=$((FAIL + 1))
    printf '  [FAIL] release.sh no invoca el guard; sus casos pasan contra fixtures y el guard real puede quedar rojo sin que nadie lo note\n'
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ $FAIL -gt 0 ]]; then
    echo "RESULT: FAIL — el guard de coherencia no detecta lo que debe detectar."
    exit 1
fi
echo "RESULT: PASS — el guard falla exactamente cuando el indice diverge."
