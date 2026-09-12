#!/usr/bin/env bash
# Pin test: every `default: allow` lint in deprecated_patterns.toml MUST
# carry a populated `explanation = """..."""` block with the four required
# sub-keywords (Hits, Verdict, Reason, Unblock). ARCH-LINT-M9.2 invariant.
#
# Anchor: p-63676b11dc0ef88f/lint-registry-advisory-audit, v1.168.34.
#
# Per ADR-0001 §3.5 (regression prevention), the test fails closed when any
# advisory lint loses its explanation block or any required sub-keyword.

set -euo pipefail

REGISTRY="${1:-docs/architecture/lints/deprecated_patterns.toml}"

if [[ ! -f "$REGISTRY" ]]; then
  echo "FAIL: registry not found at $REGISTRY"
  exit 1
fi

python3 - "$REGISTRY" <<'PY'
import sys, tomllib
path = sys.argv[1]
with open(path, "rb") as f:
    data = tomllib.load(f)

advisories = [l for l in data.get("lints", []) if l.get("default") == "allow"]
print(f"checked {len(advisories)} advisory lints in {path}")

failures = []
for lint in advisories:
    expl = lint.get("explanation", "").strip()
    if not expl:
        failures.append(f"{lint['id']}: missing explanation")
        continue
    missing_kw = [kw for kw in ("Hits:", "Verdict:", "Reason:", "Unblock:") if kw not in expl]
    if missing_kw:
        failures.append(f"{lint['id']}: explanation missing keywords {missing_kw}")

if failures:
    print(f"FAIL: {len(failures)} violation(s):")
    for f in failures:
        print(f"  - {f}")
    sys.exit(1)
print("OK: all advisory lints carry populated explanation blocks")
PY
