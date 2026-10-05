#!/usr/bin/env bash
# test_release_final_state_figures_mutation.sh — autofalsador de
# test_release_final_state_figures.sh (INC-DEBT-072)
#
# QUE PREGUNTA
# 1. ¿Cada comprobacion del guard tiene dientes? Se corrompe el codigo de
#    una en una y se exige que caiga.
# 2. ¿Cada comprobacion cae POR SU PROPIA RAZON? Se declara, ANTES de
#    correr, que checks deben caer con cada mutacion, y se exige igualdad
#    exacta con lo que cae de verdad. Asi una mutacion que caiera por el
#    efecto colateral de otra queda al descubierto, que es la clase de
#    defecto que session-82 encontro en su propio falsificador.
# 3. ¿El harness distingue "no mutó" de "pasó"? Una mutacion cuyo patron no
#    esta en el fichero se declara SKIP, NUNCA PASS: una suite que solo
#    compila es indistinguible de una que no se ejecuto.
#
# POR QUE ALGUNAS MUTACIONES CAEN VARIOS CHECKS
# No es redundancia, y declararlo evita leerlo como redundancia. C3 y C4
# observan la MISMA rama del codigo (la que responde "no lo se"), luego una
# mutacion de esa rama las cae a las dos: corrupte UNA declaracion. Lo que
# esta prohibido es una mutacion que corrompa DOS declaraciones, porque
# entonces no se puede decir cual de las dos cascada — que es exactamente
# el defecto que session-82 se encontro a si mismo.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_LIB="$ROOT/scripts/lib/final_state.sh"
SRC_REL="$ROOT/scripts/release.sh"
SRC_GUARD="$ROOT/tests/test_release_final_state_figures.sh"

PASS=0
FAIL=0
SKIP=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skp() { echo "  [SKIP] $1"; SKIP=$((SKIP + 1)); }

echo "== test_release_final_state_figures_mutation.sh (autofalsador) =="

for f in "$SRC_LIB" "$SRC_REL" "$SRC_GUARD"; do
    if [ ! -f "$f" ]; then
        echo "  [FAIL] falta $f — no hay nada que falsificar"
        echo
        echo "PASS=$PASS FAIL=$((FAIL + 1)) SKIP=$SKIP"
        exit 1
    fi
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# make_root <dest> — un root minimo con los tres ficheros que el guard lee.
make_root() {
    local dest="$1"
    mkdir -p "$dest/scripts/lib" "$dest/tests"
    cp "$SRC_LIB" "$dest/scripts/lib/final_state.sh"
    cp "$SRC_REL" "$dest/scripts/release.sh"
    cp "$SRC_GUARD" "$dest/tests/test_release_final_state_figures.sh"
}

# mutate <root> <id> — aplica una corrupcion. Devuelve 0 si se aplico, 1 si
# el patron no estaba (eso es SKIP, no PASS).
mutate() {
    python3 - "$1" "$2" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
mid = sys.argv[2]
lib = root / "scripts/lib/final_state.sh"
rel = root / "scripts/release.sh"

FLAT_LINE = '    print(receipt.get("version") or UNDECLARED)\n'
ELSE_TAIL = 'else:\n    print(UNDECLARED)\n'
FLAT_TEST = 'elif receipt.get("layout") == "flat":\n'
EXCEPT_TAIL = '    print(UNDECLARED)\n    raise SystemExit(0)\n'

# Anclajes de CODIGO, no tokens sueltos.
#
# MEDIDO, y es el segundo defecto que este falsador se encuentra a si mismo:
# M7 y M9 usaban `replace(<token>, ..., 1)`, y en el release.sh REAL el
# token aparece PRIMERO dentro de un comentario y dentro del nombre de un
# test. Luego las dos mutaciones deformaban PROSA y dejaban el codigo
# intacto, y el guard daba verde: el falsador declaraba haber corrompido algo
# que no habia corrompido. Un anclaje de mutacion tiene que ser la linea de
# codigo exacta, no la primera vez que aparece una palabra.
SOURCE_LINE = '. "$ROOT/scripts/lib/final_state.sh"\n'
CALL_LINE = 'final_state_figures "$SDDK_PREFIX" "$SDDK_FRAMEWORK_DIR"\n'

MUT = {
    # C1 sola: la rama flat deja de decir la version del binario.
    "M1": (lib, FLAT_LINE, '    print(UNDECLARED)\n'),
    # C2 sola: el bundle_version declarado deja de respetarse.
    "M2": (lib, '    print(declared)\n', '    print(UNDECLARED)\n'),
    # C3+C4: una sola rama ("no lo se") deja de usar el sentinel.
    "M3": (lib, ELSE_TAIL, 'else:\n    print("?")\n'),
    # C1+C4: la pregunta "es flat" se invierte. Una sola declaracion.
    "M4": (lib, FLAT_TEST, 'elif receipt.get("layout") != "flat":\n'),
    # C5+C6+C8: un recibo ilegible pasa a imprimir None.
    "M5": (lib, EXCEPT_TAIL, '    print("None")\n    raise SystemExit(0)\n'),
    # C7+C8: flat sin version imprime cadena vacia.
    "M6": (lib, FLAT_LINE, '    print(receipt.get("version") or "")\n'),
    # C10a: la linea que sourcea la libreria se desactiva. Se muta LA
    # LLAMADA, no la primera mencion de la palabra.
    "M7": (rel, SOURCE_LINE, '. "$ROOT/scripts/lib/final_state.sh.disabled"\n'),
    # C10c: vuelve el fallback viejo, CONSERVANDO la llamada.
    #
    # MEDIDO, y es la misma leccion que session-82 se aplico a si mismo: la
    # primera version de M8 SUSTITUIA la llamada por el fallback viejo, luego
    # corrompia DOS declaraciones — no habia llamada y habia fallback — y el
    # falsador caia [C10b C10c] sin poder decir cual de las dos cascada. Una
    # mutacion compuesta no puede atribuir su caida. Aqui se ANADE el
    # fallback sin tocar la llamada, luego lo unico que cae es C10c.
    "M8": (rel, CALL_LINE,
           CALL_LINE
           + 'BUNDLE_VER=$(python3 -c \'print({"bundle_version": None}.get("bundle_version", "?"))\')\n'),
    # C10b: la llamada desaparece y no arrastra el fallback viejo.
    "M9": (rel, CALL_LINE, 'echo "  bundle:        version-desconocida"\n'),
    # Patron inexistente: el harness DEBE reportar SKIP, nunca PASS.
    "M_GHOST": (lib, "___esta_linea_no_existe___", "x"),
}

if mid not in MUT:
    print(f"mutacion desconocida: {mid}", file=sys.stderr)
    raise SystemExit(2)

path, old, new = MUT[mid]
text = path.read_text()
if old not in text:
    raise SystemExit(3)          # no muto -> SKIP
if old == new:
    raise SystemExit(4)          # mutacion degenerada -> SKIP
path.write_text(text.replace(old, new, 1))
raise SystemExit(0)
PY
}

# falling <root> — etiquetas de los checks que caEN, deduplicadas.
falling() {
    ( cd "$1" && bash tests/test_release_final_state_figures.sh 2>&1 ) \
        | grep -oE '\[FAIL\] C[0-9]+[a-c]?:' \
        | sed 's/\[FAIL\] //; s/://' | sort -u | tr '\n' ' ' | sed 's/ $//'
}

# expected <id> — el conjunto que DEBE caer, declarado antes de correr.
#
# MEDIDO, y dos de estas declaraciones PRIMERO se escribieron mal. Se
# documentan las rectificaciones porque un falsador que corrige su propia
# expectativa sin decirlo es un falsador que aprende a aceptar lo que ve:
#
#   M1 — se declaro [C1] y cae [C1 C9]. C9 usa un recibo flat, luego la
#        rama flat la observa tambien. Es la MISMA declaracion vista por dos
#        checks, no dos declaraciones.
#   M4 — se declaro [C1 C4] y cae [C1 C3 C4 C9]. Invertir "es flat" mete
#        tambien en la rama flat a un recibo SIN campo layout (C3), y saca
#        de ella al recibo flat de C9. Una sola declaracion ("es flat"),
#        cuatro checks que la observan.
#
# Ninguna de las dos rectificaciones es una comprobacion de menos: son la
# misma pregunta mirada desde cuatro sitios, y por eso el falsador declara
# el conjunto real en vez de el que uno espera.
expected() {
    case "$1" in
        M1) echo "C1 C9" ;;
        M2) echo "C2" ;;
        M3) echo "C3 C4" ;;
        M4) echo "C1 C3 C4 C9" ;;
        M5) echo "C5 C6 C8" ;;
        M6) echo "C7 C8" ;;
        M7) echo "C10a" ;;
        M8) echo "C10c" ;;
        M9) echo "C10b" ;;
        *)   echo "" ;;
    esac
}

echo
echo "-- cada corrupcion cae, y cae por su propia razon --"
for mid in M1 M2 M3 M4 M5 M6 M7 M8 M9; do
    root="$WORK/$mid"
    make_root "$root"

    if ! mutate "$root" "$mid"; then
        skp "$mid: el patron no estaba — NO se cuenta como deteccion"
        continue
    fi

    got="$(falling "$root")"
    want="$(expected "$mid")"

    if [ -z "$got" ]; then
        bad "$mid: la corrupcion se aplico y el guard dio verde — no vigila"
    elif [ "$got" = "$want" ]; then
        ok "$mid: caió exactamente [$got], que es lo declarado"
    else
        bad "$mid: debia caer [$want] — cayo [$got]"
    fi
done

echo
echo "-- una mutacion que NO muta es SKIP, nunca PASS --"
root="$WORK/M_GHOST"
make_root "$root"
if mutate "$root" M_GHOST; then
    bad "M_GHOST: se aplico una mutacion con un patron inexistente"
else
    got="$(falling "$root")"
    if [ -z "$got" ]; then
        ok "M_GHOST: no muto, se reporto SKIP y el guard sigue verde de verdad"
    else
        bad "M_GHOST: no muto pero el guard cayo [$got] — el harness no distingue no-mutacion"
    fi
fi

echo
echo "-- el guard base acepta (si no, todo lo de arriba seria ruido) --"
base="$WORK/base"
make_root "$base"
out="$( ( cd "$base" && bash tests/test_release_final_state_figures.sh 2>&1 ) )"
rc=$?
if [ "$rc" -eq 0 ]; then
    ok "el guard sin corromper sale 0 (PASS=$(printf '%s' "$out" | grep -oE 'PASS=[0-9]+' | head -1))"
else
    bad "el guard sin corromper sale $rc — las mutaciones de arriba no significan nada"
    printf '%s\n' "$out" | tail -12
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — cada comprobacion tiene dientes y ninguna es la otra repetida."
    exit 0
fi
echo "RESULT: FAIL — el guard no se sostiene."
exit 1
