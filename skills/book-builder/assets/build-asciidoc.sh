#!/usr/bin/env bash
# Render multi-formato del libro (AsciiDoc + Asciidoctor).
# Uso: build-asciidoc.sh
# Preflight: requiere build/verify-report.jsonl con todos ALL_GREEN.

set -euo pipefail

SRC="${1:-src/book.adoc}"
BUILD="${2:-build}"

echo "==> preflight: verificar ejemplos"
if [ -f build/verify-report.jsonl ]; then
    if grep -qv '"ALL_GREEN"' build/verify-report.jsonl 2>/dev/null; then
        echo "BUILD_REFUSED: hay ejemplos no verificados" >&2
        exit 2
    fi
fi

mkdir -p "$BUILD/html" "$BUILD/pdf" "$BUILD/epub"

echo "==> HTML"
asciidoctor -D "$BUILD/html" "$SRC"

echo "==> PDF"
asciidoctor-pdf -D "$BUILD/pdf" "$SRC"

echo "==> EPUB"
asciidoctor-epub3 -D "$BUILD/epub" "$SRC"

echo "==> manifest"
{
  echo '{'
  echo '  "html": "build/html/book.html",'
  echo '  "pdf":  "build/pdf/book.pdf",'
  echo '  "epub": "build/epub/book.epub",'
  echo "  \"built_at\": \"$(date -Iseconds)\""
  echo '}'
} > "$BUILD/manifest.json"

echo "BUILT"
