#!/usr/bin/env bash
# Autofalsacion de la extraccion de notas de release.
#
# El valor de esto es una sola cosa: que el gate 2b ("la seccion declarada
# describe el trabajo que se publica") verifique un documento que ALGUIEN recibe.
# MEDIDO antes del cambio: la seccion existia, el gate la comprobaba, y el body de
# la release eran 127 bytes con la frase "published by scripts/release.sh". Un
# gate que verifica la fidelidad de algo que nadie lee es un gate que no mide.
#
# Y por eso hace falta falsarlo: `extract_changelog_section` es una funcion de
# shell con un regex multilinea y un heredoc, que es exactamente la forma que
# devuelve cadena vacia sin que nadie se entere. Si devuelve vacia y el release
# sigue, hemos reintroducido el agujero con mas pasos.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO" || exit 1

WORK="$(mktemp -d)"
PASS=0
FAIL=0
SKIP=0
declare -a LINES=()

dispose() {
    if command -v mavis-trash >/dev/null 2>&1; then
        mavis-trash -- "$1" >/dev/null 2>&1 || rm -rf "$1"
    else
        rm -rf "$1"
    fi
}
trap 'dispose "$WORK"' EXIT

# La funcion tal cual esta en el script, extraida para poder ejercitarla sola.
cat > "$WORK/extract.sh" <<'SH'
extract_changelog_section() {
    local version="$1" changelog="$2"
    python3 - "$version" "$changelog" <<'PYEOF'
import re, sys
version, path = sys.argv[1], sys.argv[2]
try:
    text = open(path, encoding="utf-8").read()
except OSError as e:
    sys.stderr.write("no se puede leer %s: %s\n" % (path, e))
    sys.exit(1)
m = re.search(r"^## \[%s\][^\n]*\n(.*?)(?=^## \[|\Z)" % re.escape(version),
              text, re.M | re.S)
if not m:
    sys.stderr.write("CHANGELOG.md no tiene seccion ## [%s]\n" % version)
    sys.exit(1)
body = m.group(1).strip()
if not body:
    sys.stderr.write("la seccion ## [%s] esta vacia\n" % version)
    sys.exit(1)
print(body)
PYEOF
}
SH
# shellcheck source=/dev/null
. "$WORK/extract.sh"

# ── F1: la seccion de la version en curso se extrae entera ──────────────────
cat > "$WORK/CHANGELOG.md" <<'MD'
# Changelog

## [2.20.0] - 2026-10-06

### Features
  - feat(x): lo que se publico

## [2.19.0] - 2026-10-05

### Features
  - feat(x): lo anterior, que NO debe colarse
MD
OUT="$(extract_changelog_section 2.20.0 "$WORK/CHANGELOG.md")"; RC=$?
if [ $RC -eq 0 ] && printf '%s' "$OUT" | grep -qF 'lo que se publico' \
   && ! printf '%s' "$OUT" | grep -qF 'lo anterior'; then
    PASS=$((PASS + 1)); LINES+=("PASS | F1 la seccion correcta se extrae y no se lleva la anterior")
else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | F1 la seccion correcta se extrae (rc=$RC)")
fi

# ── F2: version inexistente -> fallo, no cadena vacia ────────────────────────
# Esta es LA propiedad. El agujero original no era "no hay notas", era "no hay
# notas y nadie se entera": un extractor que devuelve "" y sale 0 produce un
# release con cuerpo vacio y verde.
OUT="$(extract_changelog_section 9.99.9 "$WORK/CHANGELOG.md" 2>&1)"; RC=$?
if [ $RC -ne 0 ] && ! printf '%s' "$OUT" | grep -qF '2.20.0'; then
    PASS=$((PASS + 1)); LINES+=("PASS | F2 una version inexistente falla y no devuelve otra seccion")
else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | F2 una version inexistente debe fallar (rc=$RC)")
fi

# ── F3: seccion vacia -> fallo ──────────────────────────────────────────────
cat > "$WORK/empty.md" <<'MD'
# Changelog

## [3.0.0] - 2026-10-06

## [2.0.0] - 2026-01-01
MD
OUT="$(extract_changelog_section 3.0.0 "$WORK/empty.md" 2>&1)"; RC=$?
if [ $RC -ne 0 ]; then
    PASS=$((PASS + 1)); LINES+=("PASS | F3 una seccion vacia falla en vez de publicar un cuerpo mudo")
else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | F3 una seccion vacia debe fallar (rc=$RC)")
fi

# ── F4: changelog inexistente -> fallo ──────────────────────────────────────
OUT="$(extract_changelog_section 2.20.0 "$WORK/no-existe.md" 2>&1)"; RC=$?
if [ $RC -ne 0 ]; then
    PASS=$((PASS + 1)); LINES+=("PASS | F4 un changelog ilegible falla")
else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | F4 un changelog ilegible debe fallar")
fi

# ── F5: el punto y el corchete de la version no son comodines ───────────────
# Un regex sin `re.escape` haria que `2.1[4]` casara con `2.14`, que es
# publicar el changelog de otra version creyendo que es el de esta.
cat > "$WORK/dots.md" <<'MD'
# Changelog

## [2.140.0] - 2026-10-06

### Features
  - feat(x): la version con ceros

## [2.14.0] - 2026-10-06

### Features
  - feat(x): la version correcta
MD
OUT="$(extract_changelog_section 2.14.0 "$WORK/dots.md" 2>&1)"; RC=$?
if [ $RC -eq 0 ] && printf '%s' "$OUT" | grep -qF 'la version correcta' \
   && ! printf '%s' "$OUT" | grep -qF 'la version con ceros'; then
    PASS=$((PASS + 1)); LINES+=("PASS | F5 2.14.0 no casa con 2.140.0")
else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | F5 el punto de la version debe ser literal (rc=$RC)")
fi

# ── CONTROLES ──────────────────────────────────────────────────────────────
echo "== controles =="
OUT="$(extract_changelog_section 2.20.0 "$WORK/CHANGELOG.md")"; RC=$?
if [ $RC -eq 0 ] && printf '%s' "$OUT" | grep -qF 'lo que se publico'; then
    PASS=$((PASS + 1)); LINES+=("PASS | control: la extraccion normal sigue funcionando")
else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | control: la extraccion normal dejo de funcionar")
fi
# El release real, contra el changelog real de este checkout.
REAL_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
OUT="$(extract_changelog_section "$REAL_VERSION" "$REPO/CHANGELOG.md" 2>&1)"; RC=$?
if [ $RC -eq 0 ] && [ -n "$OUT" ]; then
    PASS=$((PASS + 1)); LINES+=("PASS | control: el changelog real de $REAL_VERSION se extrae ($(printf '%s' "$OUT" | wc -c) bytes)")
else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | control: el changelog real de $REAL_VERSION NO se extrae")
fi

echo
for l in "${LINES[@]}"; do echo "$l"; done
echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[ $FAIL -eq 0 ] && echo "RESULT: PASS" || echo "RESULT: FAIL"
exit $(( FAIL > 0 ))