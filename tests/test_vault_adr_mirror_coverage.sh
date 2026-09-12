#!/usr/bin/env bash
# Pin test: every accepted ADR in docs/architecture/adrs/ MUST have a
# corresponding mirror file in ~/.sddk-knowledge/sddk-framework/adrs/.
# The vault is the human-readable view; the mirror is the projection.
#
# Anchor: p-63676b11dc0ef88f/vault-mirror-accepted-adrs, v1.168.38.
#
# This test runs `scripts/mirror_adrs_to_vault.py` (idempotent: skips existing
# files) and asserts that zero NEW files were created — i.e. the vault
# already mirrors all repo ADRs. If new mirrors are needed, the script ran
# the work; this test only verifies the invariant.
#
# Per AGENTS §2.7: "the Vault is the human source, never the runtime authority."
# The vault mirrors are views, not authorities; this test ensures the view
# is up-to-date with the authority (repo ADRs).

set -euo pipefail

REPO_ROOT="${1:-$(git rev-parse --show-toplevel)}"
VAULT_ADR_DIR="${HOME}/.sddk-knowledge/sddk-framework/adrs"

if [[ ! -d "$VAULT_ADR_DIR" ]]; then
  echo "FAIL: vault ADR directory not found at $VAULT_ADR_DIR"
  exit 1
fi

# Collect accepted ADR filenames in the repo (just the stem)
mapfile -t ACCEPTED < <(
  python3 - "$REPO_ROOT" <<'PY'
import re, sys
from pathlib import Path
root = Path(sys.argv[1]) / "docs/architecture/adrs"
for f in sorted(root.glob("ADR-*.md")):
    text = f.read_text()
    m = re.match(r"^---\n(.+?)\n---", text, re.DOTALL)
    if not m:
        continue
    if "status: accepted" in m.group(1):
        print(f.stem)
PY
)

if [[ ${#ACCEPTED[@]} -eq 0 ]]; then
  echo "FAIL: no accepted ADRs found in repo (unexpected)"
  exit 1
fi

MISSING=()
for stem in "${ACCEPTED[@]}"; do
  if [[ ! -f "$VAULT_ADR_DIR/$stem.md" ]]; then
    MISSING+=("$stem")
  fi
done

if [[ ${#MISSING[@]} -gt 0 ]]; then
  echo "FAIL: ${#MISSING[@]} accepted ADR(s) missing from vault:"
  for s in "${MISSING[@]}"; do
    echo "  - $s"
  done
  echo "Run: python3 scripts/mirror_adrs_to_vault.py"
  exit 1
fi

# Additional invariant: every mirror file must carry the canonical
# frontmatter fields required by template/adr.md.
REQUIRED_FIELDS=(
  "type: adr"
  "status: accepted"
  "created:"
  "accepted_at:"
  "accepted_by_cycle:"
  "related_adrs:"
  "repo_authority:"
)

INVALID=()
for stem in "${ACCEPTED[@]}"; do
  f="$VAULT_ADR_DIR/$stem.md"
  for field in "${REQUIRED_FIELDS[@]}"; do
    if ! grep -qF "$field" "$f"; then
      INVALID+=("$stem: missing $field")
    fi
  done
done

if [[ ${#INVALID[@]} -gt 0 ]]; then
  echo "FAIL: ${#INVALID[@]} mirror(s) missing required fields:"
  for i in "${INVALID[@]}"; do
    echo "  - $i"
  done
  exit 1
fi

# Final invariant: the mirror script must be idempotent (re-running creates 0 new files).
BEFORE=$(ls "$VAULT_ADR_DIR" | wc -l)
python3 "$REPO_ROOT/scripts/mirror_adrs_to_vault.py" >/dev/null
AFTER=$(ls "$VAULT_ADR_DIR" | wc -l)

if [[ "$BEFORE" != "$AFTER" ]]; then
  echo "FAIL: mirror script is not idempotent (created $((AFTER - BEFORE)) new files on re-run)"
  exit 1
fi

echo "OK: ${#ACCEPTED[@]} accepted ADRs mirrored in vault ($VAULT_ADR_DIR)"
echo "OK: idempotent (re-run creates 0 new files)"
