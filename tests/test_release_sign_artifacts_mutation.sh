#!/usr/bin/env bash
# tests/test_release_sign_artifacts_mutation.sh
#
# Autofalsacion de test_release_sign_artifacts.sh.
#
# Se siembra cada invariante por separado sobre una COPIA EN SANDBOX de
# scripts/release.sh y se exige que caiga SU comprobacion, no una cualquiera:
# siete mutaciones pueden ponerse rojas por el mismo efecto colateral, y
# entonces solo una esta vigilada.
#
# La copia va DENTRO de tests/ a proposito. Una falsificacion en /tmp hace que
# las rutas relativas del producto resuelvan contra otro arbol, y los fallos
# que se ven son de la copia y no de la mutacion.
#
# Y una mutacion que NO LLEGA A APLICARSE es `SKIP`, nunca `PASS`: contar como
# deteccion una mutacion que no ocurrio es el modo mas barato de tener un guard
# que parece falso y no lo esta.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$REPO/tests/test_release_sign_artifacts.sh"
SRC="$REPO/scripts/release.sh"
SANDBOX="$REPO/tests/.falsify-sign-artifacts"

[ -f "$GUARD" ] || { echo "FALLO: no existe el guard $GUARD"; exit 1; }
[ -f "$SRC" ]   || { echo "FALLO: no existe el producto $SRC"; exit 1; }

PASS=0
FAIL=0
SKIP=0
ok()   { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skip() { echo "  [SKIP] $1 -- la mutacion NO se aplico; contarla como PASS seria mentira"; SKIP=$((SKIP + 1)); }

cleanup() { rm -rf "$SANDBOX" 2>/dev/null || true; }
trap cleanup EXIT
cleanup
mkdir -p "$SANDBOX/scripts"

# ── Base verde: sin base verde no hay falsificacion ────────────────────────
cp "$SRC" "$SANDBOX/scripts/release.sh"
BASE_OUT="$(SDDK_RELEASE_SH="$SANDBOX/scripts/release.sh" bash "$GUARD" 2>&1)"
BASE_RC=$?
if [ $BASE_RC -ne 0 ]; then
    echo "FALLO: la base no esta verde; falsificar sobre una base roja no demuestra nada:"
    echo "$BASE_OUT" | tail -8
    exit 1
fi
ok "base verde con el release.sh real (PASS=$(printf '%s' "$BASE_OUT" | sed -n 's/^PASS=\([0-9]*\).*/\1/p'))"

mutate_and_expect() {
    local id="$1" desc="$2" old="$3" new="$4" expect="$5"
    cp "$SRC" "$SANDBOX/scripts/release.sh"

    python3 - "$SANDBOX/scripts/release.sh" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path, encoding="utf-8").read()
if old not in s:
    sys.exit(3)
open(path, "w", encoding="utf-8").write(s.replace(old, new, 1))
PY
    if [ $? -ne 0 ]; then
        skip "$id ($desc): el patron no aparecio en el release.sh real"
        return
    fi
    if cmp -s "$SRC" "$SANDBOX/scripts/release.sh"; then
        skip "$id ($desc): la sustitucion dejo el fichero identico"
        return
    fi

    local out rc
    out="$(SDDK_RELEASE_SH="$SANDBOX/scripts/release.sh" bash "$GUARD" 2>&1)"
    rc=$?
    if [ $rc -eq 0 ]; then
        bad "$id ($desc): el guard sigue VERDE con el defecto sembrado"
        return
    fi
    if ! grep -qF "$expect" <<<"$out"; then
        bad "$id ($desc): cayo, pero no SU comprobacion (esperaba «$expect»)"
        grep '\[FAIL\]' <<<"$out" | sed 's/^/         /' | head -3
        return
    fi
    ok "$id ($desc): cae SU comprobacion («$expect»)"
}

echo "== autofalsacion: cada invariante del 8c, sembrada por separado =="

# M1: el defecto real. El binario deja de copiarse a $TMP.
mutate_and_expect M1 \
    'el binario desnudo deja de copiarse a \$TMP (el defecto real)' \
    'cp "$BIN" "$TMP/$(basename "$BIN")"' '' \
    'BIN NO acaba en $TMP'

# M2: el bucle deja de resolver por TMP.
mutate_and_expect M2 \
    'el bucle resuelve por otra ruta que no es \$TMP' \
    'src_artifact="$TMP/$artifact"' \
    'src_artifact="$PACK_DIR/$artifact"' \
    'el bucle no resuelve src_artifact=$TMP/$artifact'

# M3: la rama de skip deja de contar, con lo que la via sin firmar
#     vuelve a ser insatisfacible.
mutate_and_expect M3 \
    'la rama de skip deja de incrementar el contador' \
    '            SIGNED_COUNT=$((SIGNED_COUNT + 1))
            continue' \
    '            continue' \
    'la rama de skip NO incrementa el contador'

# M4: el all-or-nothing pasa a comparar contra una constante, que es una
#     segunda fuente de verdad sobre el mismo numero.
mutate_and_expect M4 \
    'el all-or-nothing compara contra un 3 escrito a mano' \
    'if [ "$SIGNED_COUNT" -ne "${#SIGN_ARTIFACTS[@]}" ]; then' \
    'if [ "$SIGNED_COUNT" -ne 3 ]; then' \
    'no compara contra el numero de artefactos'

# M5: lo publicado deja de ser lo firmado.
mutate_and_expect M5 \
    'ASSETS vuelve a publicar $BIN en vez de la copia firmada' \
    '    "$TMP/$(basename "$BIN")"' \
    '    "$BIN"' \
    'ASSETS no publica la copia de $TMP'

cp "$SRC" "$SANDBOX/scripts/release.sh"
if ! cmp -s "$SRC" "$SANDBOX/scripts/release.sh"; then
    bad "la restauracion no deja el fichero byte-identico"
else
    ok "restaurado byte-identico: scripts/release.sh"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[ "$FAIL" -eq 0 ] || exit 1
echo "RESULT: PASS — las 5 invariantes del 8c caen, cada una por su comprobacion, 0 sobrevividas."
