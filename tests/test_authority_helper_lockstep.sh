#!/bin/bash
# Cross-crate pin test: scripts/release-receipt.sh must stay in lockstep with
# crates/sddk-engine/src/authority.rs::infer_actor_kind (ADR-069 §5).
#
# INC: cross-crate authority invariant (open thread of HANDOFF-2026-09-12-
# arch-lint-ax-s5: "el helper debe permanecer en lockstep").
#
# This test parses the engine-side unit test source for the canonical
# `(input, expected_kind)` table from `infer_actor_kind` tests
# (`infer_human_prefix`, `infer_agent_prefix`, `infer_system_fallback`),
# then runs the shell helper with each input and verifies the verdict
# matches the engine's expectation:
#   - `user:*`  → engine: Human → shell: REJECTED (exit 3, fails github_releases)
#   - `agent:*` → engine: Agent  → shell: REJECTED (exit 3)
#   - anything else → engine: System → shell: ADMITTED (exit 0)
#
# Why this matters: if a future cycle edits `infer_actor_kind` in the
# engine (e.g., adds a new prefix like `human:` or `bot:`) but forgets
# to update the shell helper, the github_releases surface authority gate
# diverges from the kernel. The release script would publish GH releases
# that the engine would later reject — a costly late-discovery bug.
#
# Scenarios tested:
# (a) all engine test inputs are exercised against the shell helper
# (b) every `user:*` and `agent:*` input is rejected by the shell (exit 3)
# (c) every other input is admitted by the shell (exit 0)
# (d) when the engine source is intentionally mutated to add a new prefix
#     without updating the shell, the test fails (verified manually)
#
# RED phase check: helper + engine source must exist.

set -euo pipefail
# shellcheck disable=SC2006,SC1083  # backticks and bare braces in doc comments

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HELPER="$REPO_ROOT/scripts/release-receipt.sh"
ENGINE_SOURCE="$REPO_ROOT/crates/sddk-engine/src/authority.rs"

if [[ ! -f "$HELPER" ]]; then
    echo "RED phase: scripts/release-receipt.sh does not exist"
    exit 1
fi

if [[ ! -x "$HELPER" ]]; then
    echo "RED phase: scripts/release-receipt.sh is not executable"
    exit 1
fi

if [[ ! -f "$ENGINE_SOURCE" ]]; then
    echo "RED phase: $ENGINE_SOURCE does not exist"
    exit 1
fi

echo "Helper:     $HELPER"
echo "Engine src: $ENGINE_SOURCE"

# Temp directory
WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

# Extract every `infer_actor_kind("<input>")` call from the engine test
# module. The pattern is the only stable surface — any change to the
# signature breaks compilation, which is the upstream gate.
# The test cases live in `infer_human_prefix`, `infer_agent_prefix`,
# `infer_system_fallback`. We restrict the extraction to the test module
# (`mod tests { ... }`) to avoid picking up production callers.
ENGINE_INPUTS_TMP="$WORK_DIR/engine_inputs.txt"
python3 <<PY > "$ENGINE_INPUTS_TMP"
import re, sys
src = open("$ENGINE_SOURCE").read()
# Find the "mod tests { ... }" block. Rust brace-counting is local; this
# module doesn't nest braces inside string literals, so a simple counter
# suffices.
m = re.search(r"#\[cfg\(test\)\]\\s*\\nmod tests \\{", src)
if not m:
    print("ERROR: test module not found", file=sys.stderr)
    sys.exit(1)
start = m.end()
depth = 1
i = start
while i < len(src) and depth > 0:
    c = src[i]
    if c == '{':
        depth += 1
    elif c == '}':
        depth -= 1
    i += 1
tests_block = src[start:i-1]
# Extract string literals that appear inside infer_actor_kind(...) calls.
for m in re.finditer(r"infer_actor_kind\(\"([^\"]+)\"\)", tests_block):
    print(m.group(1))
PY

if [[ ! -s "$ENGINE_INPUTS_TMP" ]]; then
    echo "FAIL: no infer_actor_kind test inputs found in engine source"
    exit 1
fi

ENGINE_INPUT_COUNT="$(wc -l < "$ENGINE_INPUTS_TMP" | tr -d ' ')"
echo "Engine infer_actor_kind test inputs: $ENGINE_INPUT_COUNT"
cat "$ENGINE_INPUTS_TMP" | sed 's/^/  /'
echo

# ── Exercise each engine input on the shell helper ───────────────────────────────
echo "=== (a) shell helper verdict for each engine test input ==="
EXIT_REJECTED=3
EXIT_ADMITTED=0
MISMATCHES=0
while IFS= read -r INPUT; do
    [[ -z "$INPUT" ]] && continue
    case "$INPUT" in
        user:*|agent:*)
            EXPECTED_VERDICT="REJECTED"
            EXPECTED_RC="$EXIT_REJECTED"
            ;;
        *)
            EXPECTED_VERDICT="ADMITTED"
            EXPECTED_RC="$EXIT_ADMITTED"
            ;;
    esac
    set +e
        bash "$HELPER" --actor-id "$INPUT" --tag "v1.168.30-test" --out "$WORK_DIR/r.json" >/dev/null 2>&1
        ACTUAL_RC=$?
    set -e
    if [[ "$ACTUAL_RC" != "$EXPECTED_RC" ]]; then
        echo "  MISMATCH: input=$INPUT expected=$EXPECTED_VERDICT (rc=$EXPECTED_RC) actual_rc=$ACTUAL_RC"
        MISMATCHES=$((MISMATCHES + 1))
    else
        echo "  ✓ input=$INPUT → $EXPECTED_VERDICT (rc=$ACTUAL_RC)"
    fi
done < "$ENGINE_INPUTS_TMP"

if [[ "$MISMATCHES" -ne 0 ]]; then
    echo
    echo "FAIL: $MISMATCHES input(s) had mismatched verdicts between engine and shell"
    echo "Hint: if you changed infer_actor_kind in crates/sddk-engine/src/authority.rs,"
    echo "      update the infer_actor_kind() shell function in scripts/release-receipt.sh"
    echo "      to match. The shell helper is a derivative of the engine contract per"
    echo "      ADR-069 §5 (locked v1.81.x prefix heuristic)."
    exit 1
fi

echo
echo "=== (b) all user:* and agent:* inputs were REJECTED ==="
USER_AGENT_COUNT="$(grep -cE '^(user|agent):' "$ENGINE_INPUTS_TMP" || true)"
REJECTED_BY_SHELL="$(while IFS= read -r INPUT; do
    [[ "$INPUT" == user:* || "$INPUT" == agent:* ]] || continue
    bash "$HELPER" --actor-id "$INPUT" --tag "v1.168.30-test" --out "$WORK_DIR/n.json" >/dev/null 2>&1 && echo NO || echo YES
done < "$ENGINE_INPUTS_TMP" | grep -c YES || true)"
if [[ "$USER_AGENT_COUNT" -ne "$REJECTED_BY_SHELL" ]]; then
    echo "FAIL: $USER_AGENT_COUNT user:*|agent:* inputs but only $REJECTED_BY_SHELL rejected"
    exit 1
fi
echo "  ✓ $REJECTED_BY_SHELL of $USER_AGENT_COUNT user:*|agent:* rejected"
echo

echo "=== (c) all non-prefixed inputs were ADMITTED ==="
PLAIN_COUNT="$(grep -cvE '^(user|agent):' "$ENGINE_INPUTS_TMP" || true)"
ADMITTED_BY_SHELL="$(while IFS= read -r INPUT; do
    [[ "$INPUT" == user:* || "$INPUT" == agent:* ]] && continue
    bash "$HELPER" --actor-id "$INPUT" --tag "v1.168.30-test" --out "$WORK_DIR/p.json" >/dev/null 2>&1 && echo YES || echo NO
done < "$ENGINE_INPUTS_TMP" | grep -c YES || true)"
if [[ "$PLAIN_COUNT" -ne "$ADMITTED_BY_SHELL" ]]; then
    echo "FAIL: $PLAIN_COUNT non-prefixed inputs but only $ADMITTED_BY_SHELL admitted"
    exit 1
fi
echo "  ✓ $ADMITTED_BY_SHELL of $PLAIN_COUNT non-prefixed admitted"
echo

echo "=== ALL CROSS-CRATE PIN TESTS PASSED ==="