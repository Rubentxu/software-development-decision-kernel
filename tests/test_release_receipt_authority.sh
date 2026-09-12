#!/bin/bash
# Functional contract test for scripts/release-receipt.sh (ARCH-HEX-001 slice 3).
#
# INC: INC-HX-AUTH-001 (writable-state surfaces) — closes GitHub Releases surface.
#
# Scenarios tested:
# (a) plain actor id "system" admitted, receipt written
# (b) plain actor id "anything-else" admitted (System fallback)
# (c) "user:alice" rejected (Human not admitted on github_releases)
# (d) "agent:orchestrator" rejected (Agent not admitted on github_releases)
# (e) SDDK_ACTOR env fallback to "system"
# (f) missing --tag → exit 2
# (g) --out - emits receipt to stdout, valid JSON, schema_version=1
# (h) receipt contains all required fields with correct values
# (i) negative case writes no receipt file (fail-closed)
# (j) prefix heuristic matches the locked v1.81.x contract from
#     `crates/sddk-engine/src/authority.rs::infer_actor_kind`

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HELPER="$REPO_ROOT/scripts/release-receipt.sh"

# RED phase check: helper must exist and be executable
if [[ ! -f "$HELPER" ]]; then
    echo "RED phase: scripts/release-receipt.sh does not exist"
    echo "FAIL: helper missing — test cannot run"
    exit 1
fi

if [[ ! -x "$HELPER" ]]; then
    echo "RED phase: scripts/release-receipt.sh is not executable"
    echo "FAIL: helper not executable — test cannot run"
    exit 1
fi

echo "Helper found and executable: $HELPER"

# Temp directory for test isolation
WORKDIR=""
cleanup() {
    local exit_code=$?
    if [[ -n "$WORKDIR" && -d "$WORKDIR" ]]; then
        rm -rf "$WORKDIR"
    fi
    exit "$exit_code"
}
trap cleanup EXIT

WORKDIR=$(mktemp -d)
echo "Workdir: $WORKDIR"
echo

# JSON sanity helper — extract a top-level scalar field via python3 (always
# available in the test runner).
json_get() {
    local field="$1"
    local payload="$2"
    python3 -c "
import json, sys
data = json.loads(sys.argv[1])
v = data.get(sys.argv[2], '')
if v is None:
    v = ''
print(v)
" "$payload" "$field"
}

# ── (a) plain actor id "system" admitted ──────────────────────────────────────
echo "=== (a) plain 'system' actor → receipt written ==="
RECEIPT_A="$WORKDIR/receipt-a.json"
unset SDDK_ACTOR
bash "$HELPER" --actor-id "system" --tag "v1.168.27" --out "$RECEIPT_A"
[[ -f "$RECEIPT_A" ]] || { echo "FAIL: receipt-a not written"; exit 1; }
PAYLOAD_A="$(cat "$RECEIPT_A")"
KIND_A="$(json_get actor_kind "$PAYLOAD_A")"
SURF_A="$(json_get surface "$PAYLOAD_A")"
[[ "$KIND_A" == "System" ]] || { echo "FAIL (a): actor_kind=$KIND_A"; exit 1; }
[[ "$SURF_A" == "github_releases" ]] || { echo "FAIL (a): surface=$SURF_A"; exit 1; }
echo "  ✓ actor_kind=System surface=github_releases"
echo

# ── (b) plain actor id "anything-else" admitted (System fallback) ──────────────
echo "=== (b) plain 'release-bot' actor → System fallback admitted ==="
RECEIPT_B="$WORKDIR/receipt-b.json"
bash "$HELPER" --actor-id "release-bot" --tag "v1.168.27" --out "$RECEIPT_B"
PAYLOAD_B="$(cat "$RECEIPT_B")"
KIND_B="$(json_get actor_kind "$PAYLOAD_B")"
ID_B="$(json_get actor_id "$PAYLOAD_B")"
[[ "$KIND_B" == "System" ]] || { echo "FAIL (b): actor_kind=$KIND_B"; exit 1; }
[[ "$ID_B" == "release-bot" ]] || { echo "FAIL (b): actor_id=$ID_B"; exit 1; }
echo "  ✓ actor_kind=System actor_id=release-bot"
echo

# ── (c) "user:alice" rejected (Human not admitted on github_releases) ──────────
echo "=== (c) 'user:alice' → rejected (Human) ==="
set +e
OUT_C="$(bash "$HELPER" --actor-id "user:alice" --tag "v1.168.27" --out "$WORKDIR/should-not-exist.json" 2>&1)"
RC_C=$?
set -e
[[ "$RC_C" == "3" ]] || { echo "FAIL (c): expected exit 3, got $RC_C"; exit 1; }
[[ "$OUT_C" == *"rejected"* ]] || { echo "FAIL (c): error msg missing 'rejected': $OUT_C"; exit 1; }
[[ ! -f "$WORKDIR/should-not-exist.json" ]] || { echo "FAIL (c): receipt was written on rejected path"; exit 1; }
echo "  ✓ rejected with code 3, no receipt written"
echo

# ── (d) "agent:orchestrator" rejected (Agent not admitted on github_releases) ──
echo "=== (d) 'agent:orchestrator' → rejected (Agent) ==="
set +e
OUT_D="$(bash "$HELPER" --actor-id "agent:orchestrator" --tag "v1.168.27" --out "$WORKDIR/should-not-exist.json" 2>&1)"
RC_D=$?
set -e
[[ "$RC_D" == "3" ]] || { echo "FAIL (d): expected exit 3, got $RC_D"; exit 1; }
[[ "$OUT_D" == *"rejected"* ]] || { echo "FAIL (d): error msg missing 'rejected': $OUT_D"; exit 1; }
echo "  ✓ rejected with code 3"
echo

# ── (e) SDDK_ACTOR env fallback to "system" ──────────────────────────────────
echo "=== (e) SDDK_ACTOR env fallback ==="
RECEIPT_E="$WORKDIR/receipt-e.json"
SDDK_ACTOR="system" bash "$HELPER" --tag "v1.168.27" --out "$RECEIPT_E"
PAYLOAD_E="$(cat "$RECEIPT_E")"
ID_E="$(json_get actor_id "$PAYLOAD_E")"
[[ "$ID_E" == "system" ]] || { echo "FAIL (e): actor_id=$ID_E"; exit 1; }
echo "  ✓ SDDK_ACTOR=system honored"
echo

# ── (f) missing --tag → exit 2 ───────────────────────────────────────────────
echo "=== (f) missing --tag → exit 2 ==="
set +e
bash "$HELPER" --actor-id "system" >/dev/null 2>&1
RC_F=$?
set -e
[[ "$RC_F" == "2" ]] || { echo "FAIL (f): expected exit 2, got $RC_F"; exit 1; }
echo "  ✓ rejected with code 2"
echo

# ── (g) --out - emits receipt to stdout, valid JSON, schema_version=1 ────────
echo "=== (g) --out - emits to stdout ==="
STDOUT_G="$(bash "$HELPER" --actor-id "system" --tag "v1.168.27" --out -)"
SCHEMA_G="$(json_get schema_version "$STDOUT_G")"
[[ "$SCHEMA_G" == "1" ]] || { echo "FAIL (g): schema_version=$SCHEMA_G"; exit 1; }
echo "  ✓ stdout schema_version=1"
echo

# ── (h) receipt contains all required fields with correct values ──────────────
echo "=== (h) receipt field completeness ==="
REQUIRED_FIELDS=("schema_version" "actor_kind" "actor_id" "tag" "timestamp" "surface" "script")
for f in "${REQUIRED_FIELDS[@]}"; do
    VAL="$(json_get "$f" "$PAYLOAD_A")"
    [[ -n "$VAL" ]] || { echo "FAIL (h): field '$f' empty"; exit 1; }
done
TAG_FIELD="$(json_get tag "$PAYLOAD_A")"
[[ "$TAG_FIELD" == "v1.168.27" ]] || { echo "FAIL (h): tag=$TAG_FIELD"; exit 1; }
echo "  ✓ all required fields present"
echo

# ── (i) negative case writes no receipt file (fail-closed) ────────────────────
echo "=== (i) negative case writes nothing ==="
NEG_PATH="$WORKDIR/should-never-exist.json"
[[ ! -f "$NEG_PATH" ]] || { echo "FAIL (i): precondition — file exists"; exit 1; }
set +e
bash "$HELPER" --actor-id "user:bob" --tag "v1.168.27" --out "$NEG_PATH" >/dev/null 2>&1
RC_I=$?
set -e
[[ "$RC_I" == "3" ]] || { echo "FAIL (i): expected exit 3, got $RC_I"; exit 1; }
[[ ! -f "$NEG_PATH" ]] || { echo "FAIL (i): receipt leaked on rejected path"; exit 1; }
echo "  ✓ no receipt file on rejection"
echo

# ── (j) prefix heuristic matches the locked v1.81.x contract ──────────────────
echo "=== (j) prefix heuristic matches engine::infer_actor_kind ==="
# Negative cases
for ACTOR in "user:alice" "user:bob" "agent:orchestrator" "agent:x"; do
    set +e
    bash "$HELPER" --actor-id "$ACTOR" --tag "v1.168.27" --out "$WORKDIR/n.json" >/dev/null 2>&1
    RC=$?
    set -e
    [[ "$RC" == "3" ]] || { echo "FAIL (j): $ACTOR expected exit 3, got $RC"; exit 1; }
done
# Positive cases
for ACTOR in "system" "release-bot" "ci-runner"; do
    bash "$HELPER" --actor-id "$ACTOR" --tag "v1.168.27" --out "$WORKDIR/p.json"
    PAYLOAD="$(cat "$WORKDIR/p.json")"
    KIND="$(json_get actor_kind "$PAYLOAD")"
    [[ "$KIND" == "System" ]] || { echo "FAIL (j): $ACTOR kind=$KIND"; exit 1; }
done
echo "  ✓ user:* and agent:* rejected; plain ids admitted as System"
echo

echo "=== ALL CONTRACT TESTS PASSED ==="