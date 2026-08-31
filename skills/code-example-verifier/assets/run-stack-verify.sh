#!/usr/bin/env bash
# Verificación de workspace stack-agnostic.
# Lee planning/stack-profile.yml y ejecuta la cadena del stack activo.
# Funciona para Rust (cargo), Python (uv+pytest+ruff), Go, JS/TS, Java...
#
# Uso: run-stack-verify.sh                       # stack primario
#       run-stack-verify.sh --secondary         # stacks secundarios (multi-stack)
#       run-stack-verify.sh --stack=python      # forzar stack específico
#
# Requiere: yq o python3 para parsear YAML.
# Salida: 0 = ALL_GREEN, distinto de 0 = BLOCKED (ver stderr).

set -uo pipefail

REPO="${BOOK_EXAMPLES_REPO:-.}"
STACK_PROFILE="${STACK_PROFILE:-$REPO/planning/stack-profile.yml}"

if [ ! -f "$STACK_PROFILE" ]; then
    echo "BLOCKED: stack-profile no encontrado en $STACK_PROFILE" >&2
    exit 2
fi

# Parseo simple del YAML para extraer los comandos del stack activo.
# Usa python3 (disponible en cualquier entorno del workspace).
parse_yaml() {
    local key="$1"
    python3 - "$STACK_PROFILE" "$key" <<'PY'
import sys, re
path, key = sys.argv[1], sys.argv[2]
with open(path) as f:
    content = f.read()
# Extrae el valor inmediato (sin anidar) bajo la clave solicitada.
# Soporta "primary.fmt_tool" buscando esa cadena exacta.
m = re.search(rf'^\s*{re.escape(key)}\s*:\s*[\'"]?([^\'"\n]+)[\'"]?', content, re.MULTILINE)
if m:
    print(m.group(1).strip())
PY
}

# Detección: ¿es Rust, Python, Go, JS, Java?
LANGUAGE=$(parse_yaml "primary.language")
case "$LANGUAGE" in
    rust)
        echo "==> Stack detectado: Rust (cargo)"
        cd "$REPO" || exit 2
        parse_yaml "primary.fmt_tool"   | bash || { echo "FMT_ERROR" >&2; exit 1; }
        parse_yaml "primary.build_tool" | bash || { echo "BUILD_ERROR" >&2; exit 1; }
        parse_yaml "primary.test_runner" | bash || { echo "TEST_FAILURE" >&2; exit 1; }
        parse_yaml "primary.lint_tool"  | bash || { echo "LINT_WARNING" >&2; exit 1; }
        ;;
    python)
        echo "==> Stack detectado: Python (uv/pytest/ruff)"
        cd "$REPO" || exit 2
        parse_yaml "primary.fmt_tool"   | bash || { echo "FMT_ERROR" >&2; exit 1; }
        parse_yaml "primary.lint_tool"  | bash || { echo "LINT_WARNING" >&2; exit 1; }
        parse_yaml "primary.test_runner" | bash || { echo "TEST_FAILURE" >&2; exit 1; }
        ;;
    go)
        echo "==> Stack detectado: Go"
        cd "$REPO" || exit 2
        parse_yaml "primary.fmt_tool"   | bash || { echo "FMT_ERROR" >&2; exit 1; }
        parse_yaml "primary.lint_tool"  | bash || { echo "LINT_WARNING" >&2; exit 1; }
        parse_yaml "primary.test_runner" | bash || { echo "TEST_FAILURE" >&2; exit 1; }
        ;;
    javascript|typescript)
        echo "==> Stack detectado: JS/TS"
        cd "$REPO" || exit 2
        parse_yaml "primary.fmt_tool"   | bash || { echo "FMT_ERROR" >&2; exit 1; }
        parse_yaml "primary.lint_tool"  | bash || { echo "LINT_WARNING" >&2; exit 1; }
        parse_yaml "primary.test_runner" | bash || { echo "TEST_FAILURE" >&2; exit 1; }
        ;;
    java)
        echo "==> Stack detectado: Java"
        cd "$REPO" || exit 2
        parse_yaml "primary.fmt_tool"   | bash || { echo "FMT_ERROR" >&2; exit 1; }
        parse_yaml "primary.lint_tool"  | bash || { echo "LINT_WARNING" >&2; exit 1; }
        parse_yaml "primary.test_runner" | bash || { echo "TEST_FAILURE" >&2; exit 1; }
        ;;
    *)
        echo "BLOCKED: lenguaje '$LANGUAGE' no soportado por el runner." >&2
        echo "Amplía run-stack-verify.sh con las recetas de book-stack-detector." >&2
        exit 3
        ;;
esac

echo "ALL_GREEN"
