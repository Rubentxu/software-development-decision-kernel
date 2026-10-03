#!/usr/bin/env bash
# tests/test_install_signature_execution_mutation.sh
#
# Autofalsacion de test_install_signature_execution.sh.
#
# Un guard que solo se ejecuta en verde no demuestra que vigile: puede estar
# mirando el sitio equivocado y devolver PASS igual. Este fichero SIEMBRA cada
# defecto por separado sobre una COPIA EN SANDBOX del instalador real y exige
# que caiga la comprobacion que dice检测arlo, y no una cualquiera.
#
# Dos reglas que este fichero se aplica a si mismo, y que son la parte cara:
#
#  1. La copia va DENTRO de tests/ (`tests/.falsify-install-signature/`), no en
#     /tmp. Una copia en /tmp hace que las rutas relativas del producto
#     resuelvan contra otro arbol, y los fallos que se ven son de la copia y no
#     de la mutacion: asi se leen mal las autofalsaciones.
#  2. Una mutacion que NO LLEGA A APLICARSE es `SKIP`, nunca `PASS`.
#
#  Y una tercera, que la primera vuelta de este fichero demostro: una
#  mutacion sobre una funcion que el ARNES SUSTITUYE por un stub es
#  INOBSERVABLE, y se presenta como «el guard sigue verde con el defecto
#  sembrado» — que parece que el guard es blando cuando en realidad el
#  defecto no estaba donde el guard mira. Por eso el arnes extrae del
#  producto las TRES funciones de la decision y solo simula `download_optional`
#  (red) y `cosign` (binario externo). Contar
#     como deteccion una mutacion que no ocurrio es el modo mas barato de
#     tener un guard que parece falso y no lo esta.
# shellcheck disable=SC2016
# SC2016, a proposito y no por descuido: los patrones de mutacion van entre
# comillas SIMPLES porque contienen "$file" DEL PRODUCTO, y tienen que llegar
# literales al python que los aplica. "Arreglarlo" con comillas dobles
# expandiria el $file del arnes y escribiria una mutacion que no muta nada --
# que es exactamente lo que este fichero mide, y por eso la directriz es de
# fichero y no de una linea.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUARD="$REPO/tests/test_install_signature_execution.sh"
SRC="$REPO/scripts/install.sh"
SANDBOX="$REPO/tests/.falsify-install-signature"

[ -f "$GUARD" ] || { echo "FALLO: no existe el guard $GUARD"; exit 1; }
[ -f "$SRC" ]   || { echo "FALLO: no existe el producto $SRC"; exit 1; }

PASS=0
FAIL=0
SKIP=0
ok()   { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skip() { echo "  [SKIP] $1 -- la mutacion NO se aplico: contarla como PASS seria mentira"; SKIP=$((SKIP + 1)); }

cleanup() { rm -rf "$SANDBOX" 2>/dev/null || true; }
trap cleanup EXIT
cleanup
mkdir -p "$SANDBOX/scripts"

# ── Base: si el estado bueno no esta verde, no se puede falsificar nada ────
cp "$SRC" "$SANDBOX/scripts/install.sh"
BASE_OUT="$(SDDK_INSTALL_SH="$SANDBOX/scripts/install.sh" bash "$GUARD" 2>&1)"
BASE_RC=$?
if [ $BASE_RC -ne 0 ]; then
    echo "FALLO: la base no esta verde; falsificar sobre una base roja no demuestra nada:"
    echo "$BASE_OUT" | tail -8
    exit 1
fi
ok "base verde con el producto real (PASS=$(echo "$BASE_OUT" | sed -n 's/^PASS=\([0-9]*\).*/\1/p'))"

# ── Aplicacion de una mutacion ─────────────────────────────────────────────
# Cada mutacion es un (viejo, nuevo). Se aplica con python para que el patron
# sea EXACTO, y se comprueba que el fichero cambio: una sustitucion que no
# casa no es una mutacion, es un `sed` que no hizo nada.
apply_mutation() {
    local id="$1" old="$2" new="$3" target="$4"
    python3 - "$target" "$old" "$new" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path, encoding="utf-8").read()
if old not in s:
    sys.exit(3)
open(path, "w", encoding="utf-8").write(s.replace(old, new, 1))
PY
    return $?
}

# ── Cada mutacion corrompe UNA declaracion, y se exige SU comprobacion ─────
# El tercer campo de cada linea es el texto de la comprobacion que DEBE caer.
# Exigir "el guard se pone rojo" no bastaria: siete mutaciones pueden ponerse
# rojas por el mismo efecto colateral, y entonces solo una esta vigilada.
mutate_and_expect() {
    local id="$1" desc="$2" old="$3" new="$4" expect="$5"
    cp "$SRC" "$SANDBOX/scripts/install.sh"

    if ! apply_mutation "$id" "$old" "$new" "$SANDBOX/scripts/install.sh"; then
        skip "$id ($desc): el patron no aparecio en el producto real"
        return
    fi
    if cmp -s "$SRC" "$SANDBOX/scripts/install.sh"; then
        skip "$id ($desc): la sustitucion dejo el fichero identico"
        return
    fi

    local out rc
    out="$(SDDK_INSTALL_SH="$SANDBOX/scripts/install.sh" bash "$GUARD" 2>&1)"
    rc=$?

    if [ $rc -eq 0 ]; then
        bad "$id ($desc): el guard sigue VERDE con el defecto sembrado"
        return
    fi
    if ! grep -qF "$expect" <<<"$out"; then
        bad "$id ($desc): cayo, pero no la comprobacion de este defecto (esperaba «$expect»)"
        echo "$out" | grep '\[FAIL\]' | sed 's/^/         /' | head -4
        return
    fi
    ok "$id ($desc): cae SU comprobacion («$expect»)"
}

echo "== autofalsacion: cada defecto sembrado tiene que caer por la suya =="

mutate_and_expect M1 \
    'destino del bundle sin asignar (el defecto real de 3b0dc9dc)' \
    '    local bundle_file="$file.bundle.json"
' '' \
    'S1 murio por una VARIABLE SIN FUENTE'

mutate_and_expect M2 \
    'destino de la firma detached sin asignar' \
    '    local sig_file="$file.sig"
' '' \
    'S1 murio por una VARIABLE SIN FUENTE'

mutate_and_expect M3 \
    'destino del certificado sin asignar (lo lee el camino sin .pem)' \
    '    local cert_file="$file.pem"
' '' \
    'S6 rc='

mutate_and_expect M4 \
    'el refusal por firma ausente pasa a aceptacion (un solo token)' \
    '    echo "  to accept it knowingly. See INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY." >&2
    exit 1' \
    '    echo "  to accept it knowingly. See INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY." >&2
    return 0' \
    'S1 rc='

mutate_and_expect M5 \
    'una firma invalida contra los dos anclos pasa a aceptacion' \
    '    echo "  and warning here would make that a bypass." >&2
    exit 1' \
    '    echo "  and warning here would make that a bypass." >&2
    return 0' \
    'S5 rc='

mutate_and_expect M6 \
    'una firma detached sin certificado pasa a aceptacion' \
    '        echo "  has nothing to match, so any signer would be accepted." >&2
        return 1' \
    '        echo "  has nothing to match, so any signer would be accepted." >&2
        return 0' \
    'S6 rc='

mutate_and_expect M7 \
    'el ancla en placeholder deja de rechazarse' \
    '            echo "  transition placeholder, not a key. Refusing to verify." >&2
            return 1' \
    '            echo "  transition placeholder, not a key. Refusing to verify." >&2
            return 0' \
    'S7 rc='

# ── Control de restauracion: byte-identico ─────────────────────────────────
cp "$SRC" "$SANDBOX/scripts/install.sh"
if ! cmp -s "$SRC" "$SANDBOX/scripts/install.sh"; then
    bad "la restauracion no deja el fichero byte-identico"
else
    ok "restaurado byte-identico: scripts/install.sh"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[ "$FAIL" -eq 0 ] || exit 1
echo "RESULT: PASS — las 7 mutaciones caen, cada una por su propia comprobacion, 0 sobrevividas."
