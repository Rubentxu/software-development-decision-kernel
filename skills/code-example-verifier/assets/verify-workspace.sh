#!/usr/bin/env bash
# Cadena de verificación del workspace del libro.
# Reproduce EXACTAMENTE la CI de .github/workflows/ci.yml del repo.
#
# Uso:
#   verify-workspace.sh                  # workspace entero
#   verify-workspace.sh bevy-book-chapter-12   # un crate concreto (-p)
#
# Salida: 0 = ALL_GREEN, distinto de 0 = BLOCKED (ver stderr).

set -euo pipefail

REPO="${BOOK_EXAMPLES_REPO:-${BOOK_EXAMPLES_REPO:-<your-book-repo>}}"
CRATE="${1:-}"

if [ ! -d "$REPO" ]; then
    echo "BLOCKED: repo no encontrado en $REPO" >&2
    exit 2
fi

cd "$REPO"

# Determinar el flag de paquete si se especifica un crate
PKG_ARG=""
if [ -n "$CRATE" ]; then
    PKG_ARG="-p $CRATE"
fi

echo "==> cargo fmt --all --check"
if ! cargo fmt --all --check; then
    echo "FMT_ERROR" >&2
    exit 1
fi

echo "==> cargo check --all-targets --locked"
if ! cargo check $PKG_ARG --all-targets --locked; then
    echo "COMPILE_ERROR" >&2
    exit 1
fi

echo "==> cargo test --locked"
if ! cargo test $PKG_ARG --locked; then
    echo "TEST_FAILURE" >&2
    exit 1
fi

echo "==> cargo clippy --all-targets --locked -- -D warnings"
if ! cargo clippy $PKG_ARG --all-targets --locked -- -D warnings; then
    echo "CLIPPY_WARNING" >&2
    exit 1
fi

echo "ALL_GREEN"
