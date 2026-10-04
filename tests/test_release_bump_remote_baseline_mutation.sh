#!/usr/bin/env bash
# Autofalsacion del arreglo de linea base remota de `scripts/release-bump.sh`.
#
# EL DEFECTO
# ----------
# `release-bump.sh` sacaba la "ultima version publicada" de `git tag` sobre el
# clon LOCAL, mientras `scripts/lib/release_admission.sh` —que usa el hook de
# admision de push y el gate 9b— la sacaba de `git ls-remote --tags origin`.
# DOS respuestas a la misma pregunta, y la local se queda vieja: `gh release
# create` publica el tag en el remoto y nada del pipeline actualiza el clon.
#
# MEDIDO en este repo (session-77): `v2.5.6` estaba publicada y aparecia en
# `git ls-remote --tags origin`, mientras `git tag` se paraba en `v2.5.5`. El
# script concluia entonces "the workspace declares the pending release (2.5.6)"
# y le habria pasado a `release.sh` un tag YA PUBLICADO. No era un numero
# equivocado en un informe: era el camino a republicar una release.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Reintroduce la lectura local y exige que los casos de linea base remota de
# `tests/test_release_bump_derivation.sh` CAIGAN. Si siguen verdes, el guard no
# mide lo que dice medir.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`: se comprueba por sha
# que el fichero cambio, y la restauracion se comprueba byte a byte al final.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUMP="$ROOT/scripts/release-bump.sh"
DERIV="$ROOT/tests/test_release_bump_derivation.sh"
SHA_ANTES="$(sha256sum "$BUMP" | cut -d' ' -f1)"

BAK="$(mktemp -d)"
cp "$BUMP" "$BAK/release-bump.sh"
restore() { cp "$BAK/release-bump.sh" "$BUMP"; }
trap 'restore; rm -rf "$BAK"' EXIT

PASS=0
FAIL=0
SKIP=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skp() { echo "  [SKIP] $1"; SKIP=$((SKIP + 1)); }

echo "== Autofalsacion de la linea base remota de release-bump.sh =="
echo

echo "-- M1: la linea base vuelve a leerse de los tags locales"
BUMP="$BUMP" python3 - <<'PY'
import os, sys
p = os.environ["BUMP"]
s = open(p, encoding="utf-8").read()
start = s.index('LAST_PUB_REMOTE="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}"')
end = s.index('CURRENT="${LAST_TAG#v}"')
legacy = ("LAST_TAG=\"$(git tag --sort=-v:refname "
          "| grep -E '^v[0-9]+\\.[0-9]+\\.[0-9]+$' | head -1 || true)\"\n"
          'LAST_PUB_SOURCE="local (reverted)"\n\n')
open(p, "w", encoding="utf-8").write(s[:start] + legacy + s[end:])
PY
if [[ $? -ne 0 ]]; then
    skp "la mutacion no encontro su texto; NO cuenta como deteccion"
    restore
    exit 1
fi
SHA_DESPUES="$(sha256sum "$BUMP" | cut -d' ' -f1)"
if [[ "$SHA_ANTES" == "$SHA_DESPUES" ]]; then
    skp "la mutacion no cambio el fichero; NO cuenta como deteccion"
    restore
    exit 1
fi

SALIDA="$(bash "$DERIV" 2>&1)"

# Se mide la FORMA del resultado, no la presencia del nombre del caso: el
# nombre aparece igual en la linea PASS y en la FAIL, asi que buscarlo a pelo
# daria "siguen verdes" justo cuando han caido. Tercera vez esta sesion que un
# needle ambiguo reporta el veredicto invertido; el arreglo en los tres casos
# fue medir la FORMA, no el contenido.
if grep -qE "^PASS +\[[^]]*\] remote baseline" <<<"$SALIDA"; then
    bad "M1: los casos de linea base remota siguen verdes con la lectura local (no tienen dientes)"
elif grep -qE "^FAIL +\[[^]]*\] remote baseline wins over a stale local tag list" <<<"$SALIDA"; then
    ok "M1 detectado: reintroducir la lectura local hace caer la derivacion por linea base remota"
else
    bad "M1: el caso principal no aparecio en la salida; inconclusive"
    grep -E "^(PASS|FAIL)  \[" <<<"$SALIDA" | tail -6
fi

restore
SHA_FINAL="$(sha256sum "$BUMP" | cut -d' ' -f1)"
if [[ "$SHA_FINAL" == "$SHA_ANTES" ]]; then
    ok "restauracion byte-identica verificada por sha"
else
    bad "la restauracion no es byte-identica ($SHA_FINAL != $SHA_ANTES)"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[[ "$FAIL" -eq 0 ]]
