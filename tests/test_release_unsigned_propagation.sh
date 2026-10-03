#!/usr/bin/env bash
# tests/test_release_unsigned_propagation.sh
#
# Por que este guard existe, y que NO le deja pasar:
#
# Con v2.5.4 publicada y sin firmar, el paso 9c declaro `UNSIGNED` con su
# motivo y el pipeline continuo —correcto— hasta el paso 10, que lanzo
# `scripts/install.sh` sin `SDDK_ALLOW_UNSIGNED` y murio con
#   `cannot verify the authenticity of …: no signature asset published`
# sobre una release que el propio pipeline acababa de publicar a proposito.
#
# Es la TERCERA vez que la misma verdad —"esta release no esta firmada"— se
# decide en un sitio distinto del que se decidio: el 8c la admitio, el 9c la
# declaro, y el 10 la vuelve a negar. El instalador hace BIEN en negarse —
# negarse es lo que significa unsigned, y por eso la bandera se llama ALLOW—.
# Lo que esta mal es que el pipeline no le comunique que la decision ya esta
# tomada. Un gate que muere porque nadie le dijo lo que el tramo anterior ya
# habia decidido no protege: estorba.
#
# Y un segundo defecto en el MISMO tramo: el paso 10 sobreescribia
# `SDDK_PREFIX` y `SDDK_FRAMEWORK_DIR` con rutas absolutas literales del home
# del operador que desarrollo el repo, ignorando las variables que el propio
# script resuelve arriba desde `$HOME` con sobreescritura por entorno. El
# script llevaba 1600 lineas respetando el entorno y en la instalacion lo
# dejaba de respetar, informando de un exito que no era de este prefix.
#
# El contrato que ata:
#   (1) Sin `SDDK_SKIP_SIGNING`, `SDDK_ALLOW_UNSIGNED` NO se propaga. El
#       instalador debe seguir negando una release sin firma. Si la
#       propagacion fuera incondicional, esto seria un bypass universal de la
#       verificacion de supply-chain, y el guard tiene que ser capaz de verlo.
#   (2) Con `SDDK_SKIP_SIGNING=1`, la propagacion ocurre Y es visible (aviso),
#       porque una excepcion silenciosa no es una excepcion: es un valor por defecto silencioso.
#   (3) `SDDK_PREFIX` y `SDDK_FRAMEWORK_DIR` se respetan tal como llegan del
#       entorno, y NO se sobreescriben con una ruta literal.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_SH="${SDDK_RELEASE_SH:-$REPO/scripts/release.sh}"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

[ -f "$RELEASE_SH" ] || { echo "FALLO: no existe $RELEASE_SH"; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ── Extraccion del tramo REAL de la instalacion ────────────────────────────
# Desde el `unset SDDK_BASE_URL` hasta la invocacion de install.sh. Se ejecuta
# el bloque del producto con stubs solo para lo que escribe a disco; la decision
# —que exportar y que no— es del producto.
awk '
    /^unset SDDK_BASE_URL SDDK_VERSION$/ { inblock = 1 }
    inblock { print }
    inblock && /^    \|\| die "install.sh failed"$/ { exit }
' "$RELEASE_SH" > "$WORK/install_block.sh"

if ! grep -q 'install.sh' "$WORK/install_block.sh"; then
    bad "no se pudo extraer el tramo de instalacion del paso 10"
    echo
    echo "PASS=$PASS FAIL=$((FAIL + 1)) SKIP=0"
    exit 1
fi
ok "tramo de instalacion del paso 10 extraido del producto"

# ── Arnes ──────────────────────────────────────────────────────────────────
# install.sh se sustituye por una sonda que imprime el entorno que recibe. Lo
# que se mide es QUE LLEGA AL HIJO, no lo que el bloque exporta en su propio
# shell: un `export` que no se propaga al proceso hijo no sirve de nada, y esa
# es justo la confusion que produjo el defecto.
cat > "$WORK/install_block.sh.probe" <<'PROBE'
echo "ALLOW=${SDDK_ALLOW_UNSIGNED:-<unset>}"
echo "PREFIX=${SDDK_PREFIX:-<unset>}"
echo "FRAMEWORK=${SDDK_FRAMEWORK_DIR:-<unset>}"
PROBE

build_harness() {
    {
        cat <<'STUBS'
set -uo pipefail
warn() { echo "warn: $*"; }
die()  { echo "die: $*"; exit 1; }
# Sonda en el lugar de install.sh. Se sustituye el NOMBRE y se conservan los
# argumentos, en vez de commenting la linea entera: comentar dejaba colgando
# la continuacion `|| die "install.sh failed"` de la linea siguiente y el
# arnes moria por error de SINTAXIS — un fallo del arneslezandose de fallo del
# producto, que es como se leen mal estas cosas.
PROBE_FILE="__PROBE__"
# `bash "$PROBE_FILE"`, NO `cat`: la sonda imprime el ENTORNO que recibe, y
# `cat` imprimiria el texto de la sonda con `${...}` sin expandir — que es
# justo el sintoma de "todo vacio" que hace fallar este guard sin motivo.
probe_install() { bash "$PROBE_FILE"; }
STUBS
        sed 's|^bash scripts/install\.sh |probe_install |' "$WORK/install_block.sh"
    } | sed "s|__PROBE__|$WORK/install_block.sh.probe|" > "$WORK/run.sh"
}
build_harness

run_case() {
    local skip_signing="$1" prefix="$2" framework="$3"
    local out
    out="$(
        SDDK_SKIP_SIGNING="$skip_signing" \
        SDDK_PREFIX="$prefix" \
        SDDK_FRAMEWORK_DIR="$framework" \
        TAG="v9.9.9" \
        bash "$WORK/run.sh" 2>&1
    )"
    OUT="$out"
    ALLOW="$(printf '%s\n' "$out" | sed -n 's/^ALLOW=//p' | head -1)"
    PREFIX_OUT="$(printf '%s\n' "$out" | sed -n 's/^PREFIX=//p' | head -1)"
    FRAMEWORK_OUT="$(printf '%s\n' "$out" | sed -n 's/^FRAMEWORK=//p' | head -1)"
}

# ══════════════════════════════════════════════════════════════════════════
# S1 — sin la bandera de no-firma: NO se propaga. El instalador debe seguir
# negando una release sin firma. Una propagacion incondicional seria un bypass
# universal de la verificacion, y este es el caso que lo detecta.
# ══════════════════════════════════════════════════════════════════════════
run_case 0 "/custom/prefix" "/custom/framework"
if [ "$ALLOW" = "<unset>" ]; then
    ok "S1 sin SDDK_SKIP_SIGNING: SDDK_ALLOW_UNSIGNED NO se propaga (el instalador sigue negando)"
else
    bad "S1 sin SDDK_SKIP_SIGNING llego SDDK_ALLOW_UNSIGNED='$ALLOW' al instalador: la propagacion es un BYPASS UNIVERSAL de la verificacion de supply-chain"
fi

# ══════════════════════════════════════════════════════════════════════════
# S2 — con la bandera: se propaga, y con aviso visible.
# ══════════════════════════════════════════════════════════════════════════
run_case 1 "/custom/prefix" "/custom/framework"
if [ "$ALLOW" = "1" ]; then
    ok "S2 con SDDK_SKIP_SIGNING=1: SDDK_ALLOW_UNSIGNED=1 llega al instalador"
else
    bad "S2 con SDDK_SKIP_SIGNING=1 llego SDDK_ALLOW_UNSIGNED='$ALLOW': el paso 10 seguira negandose a instalar lo que el pipeline publico"
fi
if printf '%s\n' "$OUT" | grep -q 'SDDK_ALLOW_UNSIGNED=1 al instalador'; then
    ok "S2 la propagacion es VISIBLE (aviso), no silenciosa"
else
    bad "S2 la propagacion fue silenciosa: una excepcion sin aviso no es una excepcion"
fi

# ══════════════════════════════════════════════════════════════════════════
# S3 — el prefix del entorno se respeta. El defecto hardcodeaba
# `/home/rubentxu/.local/bin` y escribia ahi informando de un exito.
# ══════════════════════════════════════════════════════════════════════════
if [ "$PREFIX_OUT" = "/custom/prefix" ] && [ "$FRAMEWORK_OUT" = "/custom/framework" ]; then
    ok "S3 SDDK_PREFIX y SDDK_FRAMEWORK_DIR llegan al instalador sin ser sobreescritos"
else
    bad "S3 el instalador recibio prefix='$PREFIX_OUT' framework='$FRAMEWORK_OUT' en vez de /custom/prefix y /custom/framework — el paso instala donde no es y lo informa como exito"
fi

# ══════════════════════════════════════════════════════════════════════════
# S4 — CONTROL DE NO-VACUIDAD. Un bloque que NO propagara pasaria S1 y fallaria
# S2; uno que propagara siempre pasaria S2 y fallaria S1. Ninguno pasa ambos, y
# este control ejecuta los dos y exige el par coherente. Sin el, un guard
# escrito como "el instalador recibe la bandera" mediria la mitad del contrato
# y dejaria pasar el bypass.
# ══════════════════════════════════════════════════════════════════════════
run_case 0 "/p" "/f"; A_WITHOUT="$ALLOW"
run_case 1 "/p" "/f"; A_WITH="$ALLOW"
if [ "$A_WITHOUT" = "<unset>" ] && [ "$A_WITH" = "1" ]; then
    ok "control de no-vacuidad: la propagacion depende de la bandera, no es constante"
else
    bad "control de no-vacuidad: sin bandera='$A_WITHOUT', con bandera='$A_WITH' — la decision no depende de la entrada"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=0"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — la postura de firma se propaga al instalador solo cuando el pipeline la declaro, y el prefix del entorno se respeta."
    exit 0
fi
echo "RESULT: FAIL — el paso 10 vuelve a decidir lo que el 8c ya decidio, o instala donde no es."
exit 1
