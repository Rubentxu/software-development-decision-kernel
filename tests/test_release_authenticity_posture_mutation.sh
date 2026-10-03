#!/usr/bin/env bash
# tests/test_release_authenticity_posture_mutation.sh
#
# Que prueba esto, y por que existe aparte del guard:
#
# Un guard que ejecuta el codigo del producto solo es creible si se demuestra
# que CAE cuando el producto se rompe. Sin esto, un arnes mal escrito —un
# `grep` que no encuentra, un harness que no ejecuta nada, una asercion que
# compara contra su propia copia— puede dar verde indefinidamente sin haber
# medido una vez. Cada mutacion rompe UNA invariante y exige que el guard la
# note por SU comprobacion.
#
# La regla que gobierna este fichero, y que ya ha startlingmente fallado en
# esta serie: **una mutacion que no llega a aplicarse es SKIP, nunca PASS**.
# Un falsador que reporta "M3 detectada" cuando su mutacion no encontro el
# texto, y lo unico que cayo fue M1, esta declarando una deteccion que no
# ocurrio. Por eso aqui cada mutacion:
#   (1) comprueba que el texto ancla EXISTIA antes de mutar,
#   (2) comprueba que DEJO de existir despues,
#   (3) comprueba que el guard salio distinto de 0, y
#   (4) restaura y verifica restauracion byte-identica por sha256.
#
# Las siete mutaciones, una por invariante:
#   M1  la postura ignora si hay firmas  -> rompe S3
#   M2  la postura siempre UNSIGNED      -> rompe S2 y el control
#   M3  la postura siempre VERIFY        -> rompe S1, S4 y S5
#   M4  desaparece la via DECLARED_SKIP  -> rompe S4
#   M5  se antepone el salto al hecho   -> rompe S5 (precedence)
#   M6  el 9c ignora la postura          -> rompe S4 (cableado)
#   M7  el cierre deja de calificar      -> rompe S1 y S4
#
# shellcheck disable=SC2016
#
# El disable de arriba es A NIVEL DE FICHERO y es deliberado.
#
# Session-75: `test_build_identity_policy.sh` corria shellcheck sobre el shell
# tocado y este fichero salia con 7 avisos SC2016. Los siete son FALSOS
# POSITIVOS, y el mecanismo es la propia tecnica del guard: lo que hay entre
# comillas simples es el TEXTO LITERAL que se sustituye dentro de
# `release.sh`. Si el shell de este test expandiera los `${...}` al construir la
# mutacion, estariamos mutando una cosa distinta de la que queremos mutar, la
# mutacion "aplicaria" sin cambiar el codigo, y el arnes la contaria como
# PASS -- exactamente el falso PASS que este fichero existe para cazar. Por eso
# el disable va aqui, con el motivo escrito, y no como siete directivas
# sueltas: el motivo es la propiedad, las directivas serian la sintaxis.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE_SH="$REPO/scripts/release.sh"
GUARD="$REPO/tests/test_release_authenticity_posture.sh"
# La arena va DENTRO de tests/ a proposito: una falsificacion en /tmp hace que
# las rutas relativas del producto resuelvan contra otro arbol, y los fallos
# que se ven entonces son los de la copia y no los de la mutacion.
SANDBOX="$REPO/tests/.falsify-authenticity"

PASS=0
FAIL=0
SKIP=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skp() { echo "  [skip] $1"; SKIP=$((SKIP + 1)); }

[ -f "$RELEASE_SH" ] || { echo "FALLO: no existe $RELEASE_SH"; exit 1; }
[ -f "$GUARD" ]     || { echo "FALLO: no existe $GUARD"; exit 1; }

BASE_SHA="$(sha256sum "$RELEASE_SH" | awk '{print $1}')"

cleanup() { rm -rf "$SANDBOX" 2>/dev/null || true; }
trap cleanup EXIT

# ── Aplicar una mutacion ───────────────────────────────────────────────────
# $1 nombre  $2 texto ancla (debe existir)  $3 texto sustituto
#
# Devuelve 0 si la mutacion se aplico, 2 si el ancla no existia. La distincion
# importa: 2 es SKIP, y 0 es el unico caso en el que se puede exigir que el
# guard caiga.
apply_mutation() {
    local name="$1" anchor="$2" replacement="$3"
    local target="$SANDBOX/release.sh"

    mkdir -p "$SANDBOX"
    cp "$RELEASE_SH" "$target"

    # Las tres comprobaciones —ancla presente, sustitucion aplicada, ancla
    # ausente— se hacen con SUBCADENA EXACTA en python, nunca con `grep -F`.
    #
    # `grep -F` con un patron multilinea trata CADA LINEA como un patron
    # independiente y acierta si encuentra cualquiera. Un ancla de cuatro
    # lineas cuya primera linea sobrevive a su propia sustitucion —que es
    # justo lo que hacen M2, M3 y M4, que cambian `echo DECLARED_SKIP` y
    # dejan intacto el `if` de encima— daria "la mutacion no se aplico" con
    # el fichero ya mutado. Cuatro mutaciones Skip, y el guard entero
    # declarado INCOMPLETO por un comprobador que no comprobaba.
    #
    # Codigos de salida: 0 aplicada, 2 no aplicable (SKIP), 3 error.
    python3 - "$target" "$anchor" "$replacement" <<'PY'
import io, sys

path, anchor, repl = sys.argv[1], sys.argv[2], sys.argv[3]
s = io.open(path, encoding="utf-8").read()

n = s.count(anchor)
if n == 0:
    sys.stderr.write("ancla ausente en el producto\n")
    sys.exit(2)
if n != 1:
    sys.stderr.write("ancla aparece %d veces (se exige 1)\n" % n)
    sys.exit(2)

mutated = s.replace(anchor, repl, 1)

# La mutacion tiene que haber cambiado ALGO. Si no, el fichero es identico y
# el guard no tiene nada nuevo que medir.
if mutated == s:
    sys.stderr.write("la sustitucion no cambia el contenido\n")
    sys.exit(2)

# Y el ancla tiene que haber DESAPARECIDO como subcadena exacta. Es la
# comprobacion que grep no hacia bien.
if mutated.count(anchor) != 0:
    sys.stderr.write("el ancla sigue presente tras la sustitucion\n")
    sys.exit(2)

io.open(path, "w", encoding="utf-8").write(mutated)
PY
    local rc=$?
    if [ "$rc" -eq 2 ]; then
        echo "        (mutacion $name no aplicable)"
        return 2
    fi
    if [ "$rc" -ne 0 ]; then
        echo "        (error aplicando $name)"
        return 2
    fi
    return 0
}

# ── Exigir que el guard caiga ──────────────────────────────────────────────
# $1 nombre  $2 descripcion de la invariante rota
expect_fall() {
    local name="$1" desc="$2"
    local out rc
    out="$(SDDK_RELEASE_SH="$SANDBOX/release.sh" bash "$GUARD" 2>&1)"; rc=$?
    if [ "$rc" -eq 0 ]; then
        bad "$name NO lo detecto — la invariante rota ($desc) paso sin que nadie la notara"
        printf '%s\n' "$out" | sed 's/^/        /' | tail -12
        return 1
    fi
    # No basta con que caiga: tiene que caerse por SU comprobacion. Un guard
    # que muere por un error de sintaxis "detecta" cualquier mutacion, y eso
    # no es falsificacion, es un fallo de ejecucion disfrazado.
    if ! printf '%s\n' "$out" | grep -q '\[FAIL\]'; then
        bad "$name cayo, pero sin veredicto: fallo por otra causa (harness roto), no por la invariante"
        printf '%s\n' "$out" | sed 's/^/        /' | tail -12
        return 1
    fi
    ok "$name detectada por su comprobacion ($desc)"
    return 0
}

# ══════════════════════════════════════════════════════════════════════════
# M1 — la postura ignora si hay firmas sobre la mesa.
# ══════════════════════════════════════════════════════════════════════════
if apply_mutation M1 \
    'if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ] && [ "$sig_files_present" -eq 0 ]; then' \
    'if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ]; then'; then
    expect_fall M1 "una bandera de no-firma apaga una verificacion que si se podia hacer" || true
else
    skp "M1 no se aplico"
fi

# ══════════════════════════════════════════════════════════════════════════
# M2 — la postura es siempre UNSIGNED (el 9c se vuelve decoracion).
# ══════════════════════════════════════════════════════════════════════════
if apply_mutation M2 \
    '    if [ "${SDDK_SKIP_AUTHENTICITY_CHECK:-0}" = "1" ]; then
        echo "DECLARED_SKIP"
        return 0
    fi' \
    '    if [ "${SDDK_SKIP_AUTHENTICITY_CHECK:-0}" = "1" ]; then
        echo "UNSIGNED"
        return 0
    fi'; then
    expect_fall M2 "una release firmada se salta sin verificar — el paso ya no mide nada" || true
else
    skp "M2 no se aplico"
fi

# ══════════════════════════════════════════════════════════════════════════
# M3 — la postura es siempre VERIFY (el camino sin firma deja de existir).
# ══════════════════════════════════════════════════════════════════════════
if apply_mutation M3 \
    '    if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ] && [ "$sig_files_present" -eq 0 ]; then
        echo "UNSIGNED"
        return 0
    fi' \
    '    if false; then
        echo "UNSIGNED"
        return 0
    fi'; then
    expect_fall M3 "no queda ninguna salida para la release firmada sin firma" || true
else
    skp "M3 no se aplico"
fi

# ══════════════════════════════════════════════════════════════════════════
# M4 — desaparece la via DECLARED_SKIP.
# ══════════════════════════════════════════════════════════════════════════
if apply_mutation M4 \
    '    if [ "${SDDK_SKIP_AUTHENTICITY_CHECK:-0}" = "1" ]; then
        echo "DECLARED_SKIP"
        return 0
    fi' \
    '    if false; then
        echo "DECLARED_SKIP"
        return 0
    fi'; then
    expect_fall M4 "SDDK_SKIP_AUTHENTICITY_CHECK=1 deja de ser una via declarada" || true
else
    skp "M4 no se aplico"
fi

# ══════════════════════════════════════════════════════════════════════════
# M5 — se antepone el salto pedido al hecho de que no hay nada que verificar.
# ══════════════════════════════════════════════════════════════════════════
if apply_mutation M5 \
    '    if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ] && [ "$sig_files_present" -eq 0 ]; then
        echo "UNSIGNED"
        return 0
    fi
    if [ "${SDDK_SKIP_AUTHENTICITY_CHECK:-0}" = "1" ]; then
        echo "DECLARED_SKIP"
        return 0
    fi' \
    '    if [ "${SDDK_SKIP_AUTHENTICITY_CHECK:-0}" = "1" ]; then
        echo "DECLARED_SKIP"
        return 0
    fi
    if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ] && [ "$sig_files_present" -eq 0 ]; then
        echo "UNSIGNED"
        return 0
    fi'; then
    expect_fall M5 "el salto pedido se antepone al hecho comprobable" || true
else
    skp "M5 no se aplico"
fi

# ══════════════════════════════════════════════════════════════════════════
# M6 — el 9c deja de consultar la postura (cableado, no politica).
# ══════════════════════════════════════════════════════════════════════════
if apply_mutation M6 \
    '    AUTH_POSTURE="$(release_authenticity_posture "$SIG_FILES_PRESENT")"' \
    '    AUTH_POSTURE="VERIFY"'; then
    expect_fall M6 "la politica existe pero el 9c no la consulta" || true
else
    skp "M6 no se aplico"
fi

# ══════════════════════════════════════════════════════════════════════════
# M7 — el cierre deja de calificar el PASS.
# ══════════════════════════════════════════════════════════════════════════
if apply_mutation M7 \
    '        warn "public-release gate PASS — pero la release se publico SIN verificar autenticidad"' \
    '        ok "public-release gate PASS"'; then
    expect_fall M7 "una release sin verificar se describe como verificada" || true
else
    skp "M7 no se aplico"
fi

# ── Restauracion byte-identica ─────────────────────────────────────────────
# Se mide con sha, no con "el fichero parece el mismo". Un falsador que
# restaura en desorden se parece a uno que restaura bien, y la diferencia
# aparece en la siguiente ejecucion, que ya no mide el producto.
cleanup
if [ "$(sha256sum "$RELEASE_SH" | awk '{print $1}')" = "$BASE_SHA" ]; then
    ok "el producto quedo byte-identico (sha $BASE_SHA) — ninguna mutacion se filtro"
else
    bad "el producto NO quedo byte-identico: una mutacion escapó y el guarda mide un arbol contaminado"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
if [ "$FAIL" -eq 0 ] && [ "$SKIP" -eq 0 ]; then
    echo "RESULT: PASS — las 7 invariantes del 9c caen, cada una por su comprobacion, 0 sobrevividas, 0 sin aplicar."
    exit 0
fi
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: INCOMPLETO — $SKIP mutaciones no llegaron a aplicarse. Un SKIP no es un PASS: el guard no esta verificado."
    exit 1
fi
echo "RESULT: FAIL — hay invariantes que el guard no vigila."
exit 1
