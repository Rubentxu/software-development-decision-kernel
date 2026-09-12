#!/usr/bin/env bash
# Pin test: every lint in deprecated_patterns.toml with `default: deny` MUST
# report zero hits against the live workspace. ARCH-LINT-M9 enforcement
# invariant (cycle-7 closure path).
#
# Anchor: p-63676b11dc0ef88f/synthesis-dissent-runner-extension, v1.168.37.
#
# Per ADR-0001 §3.5 (regression prevention), this test fails closed when a
# promoted-to-deny lint accumulates hits. Promoting a lint to deny without
# zero hits is a violation of the M9 acceptance criteria documented in the
# registry TOML's `[acceptance_for_m9_blocking_enforcement]` section.

set -euo pipefail

REGISTRY="${1:-docs/architecture/lints/deprecated_patterns.toml}"

if [[ ! -f "$REGISTRY" ]]; then
  echo "FAIL: registry not found at $REGISTRY"
  exit 1
fi

# Read the deny lint IDs from the registry.
mapfile -t DENY_IDS < <(python3 - "$REGISTRY" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    data = tomllib.load(f)
print("\n".join(l["id"] for l in data.get("lints", []) if l.get("default") == "deny"))
PY
)

if [[ ${#DENY_IDS[@]} -eq 0 ]]; then
  echo "OK: no deny lints in registry"
  exit 0
fi

# Run the lint runner and count hits per deny lint ID.
HITS_JSON="$(sddk dev lint deprecated-patterns --format json)"
python3 - "$HITS_JSON" "${DENY_IDS[@]}" <<'PY'
import sys, json
raw = sys.argv[1]
deny_ids = set(sys.argv[2:])
data = json.loads(raw)
hits = data.get("hits", [])
from collections import Counter
counts = Counter(h["lint_id"] for h in hits if h["lint_id"] in deny_ids)

failures = [(lid, counts[lid]) for lid in deny_ids if counts[lid] > 0]
if failures:
    print(f"FAIL: {len(failures)} deny lint(s) have hits:")
    for lid, n in failures:
        print(f"  - {lid}: {n} hit(s)")
    sys.exit(1)
print(f"OK: {len(deny_ids)} deny lint(s) all at 0 hits: {sorted(deny_ids)}")
PY
