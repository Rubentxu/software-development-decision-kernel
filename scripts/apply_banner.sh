#!/usr/bin/env bash
# apply_banner.sh — prepend canonical Historical/superseded banner to a doc.
# Usage: bash scripts/apply_banner.sh <file>
#
# Idempotent: detects existing banner and skips if already present.
# Preserves YAML frontmatter (between leading --- lines) and inserts the
# banner after it.

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <file>" >&2
  exit 64
fi

file="$1"
if [[ ! -f "$file" ]]; then
  echo "not found: $file" >&2
  exit 66
fi

# shellcheck disable=SC2016  # banner literal con backticks/links markdown
banner='> **Historical / superseded.** This document records previous design context. Current normative architecture and roadmap are linked from [`docs/architecture/README.md`](../architecture/README.md).'

# Idempotency: skip if banner already present.
if grep -qF 'Historical / superseded' "$file"; then
  echo "skip (already bannered): $file"
  exit 0
fi

# If file starts with YAML frontmatter, insert banner after the closing ---.
# INC-DEBT-071: `grep -q` salia en cuanto casaba; con `pipefail` un 141
# espurio de `head` hacia que un fichero CON frontmatter pareciera no tenerlo,
# y el banner se insertaba antes del `---` de apertura. `grep -c` lee entero.
if [ "$(head -1 "$file" | grep -c -- '^---$')" -gt 0 ]; then
  # find line number of second '---' that closes the frontmatter
  close=$(awk 'NR>1 && /^---$/ {print NR; exit}' "$file")
  if [[ -n "$close" ]]; then
    tmp=$(mktemp)
    # shellcheck disable=SC2129  # secuencia de append legible y en orden
    {
      head -n "$close" "$file"
      echo ""
      echo "$banner"
      tail -n +"$((close+1))" "$file"
    } > "$tmp"
    mv "$tmp" "$file"
    echo "bannered (after frontmatter): $file"
    exit 0
  fi
fi

# Plain markdown: prepend banner before first heading.
tmp=$(mktemp)
echo "$banner" > "$tmp"
echo "" >> "$tmp"
cat "$file" >> "$tmp"
mv "$tmp" "$file"
echo "bannered (prepended): $file"