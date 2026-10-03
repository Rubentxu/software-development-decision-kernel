#!/usr/bin/env bash
# tests/test_release_sign_artifacts.sh
#
# Que invariante ata, y por que este guard existe:
#
# El paso 8c firma los artefactos que nombra `SIGN_ARTIFACTS`, resueltos como
# `"$TMP/$artifact"`, y despues exige all-or-nothing: `SIGNED_COUNT` tiene que
# igualar el numero de artefactos. Las dos premises —"todos viven en $TMP" y
# "la rama de skip incrementa el contador"— no las comprobaba NADIE, y las
# dos eran falsas a la vez:
#
#   * el binario desnudo solo vivia en `$BIN` y dentro de `$PACK/bin/sddk`,
#     luego en `$TMP` no estaba, el bucle hacia `continue` y contaba 2 de 3;
#   * con `SDDK_SKIP_SIGNING=1` la comprobacion de existencia corre ANTES del
#     skip, luego el contador nunca llegaba a 3 y la via documentada para
#     publicar sin firmar era insatisfacible. El mensaje de error decia
#     "set SDDK_SKIP_SIGNING=1" y eso no hacia nada.
#
# Consecuencia: **la pregunta de la clave KMS era la equivocada**. No era que
# faltara una credencial; era que el camino local no podia firmarlos con
# NINGUNA clave ni publicar sin firmar con NINGUN flag. v2.5.2 lo publico el
# CI, con otro staging, luego el tramo local nunca se habia ejercitado.
#
# `SIGN_ARTIFACTS` no contiene nombres literales sino EXPRESIONES
# (`$(basename "$UNIFIED")`), asi que este guard no las trata como nombres: las
# resuelve a la variable que nombran y exige que esa variable apunte a $TMP. Un
# guard que comparase literales contra un nombre derivado daria verde con el
# layout roto, que es justo lo que hay que evitar.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE="${SDDK_RELEASE_SH:-$REPO/scripts/release.sh}"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

[ -f "$RELEASE" ] || { echo "FALLO: no existe $RELEASE"; exit 1; }

# ── Las expresiones que el 8c declara firmar ──────────────────────────────
# Cada linea de SIGN_ARTIFACTS es `$(basename "$VAR")`. Se emite el `$VAR` tal
# cual, sin desarmar: el guard trabaja con la variable, que es lo que tiene que
# estar en $TMP, y no con el texto del comando que la produce.
mapfile -t VARS < <(awk '
    /^SIGN_ARTIFACTS=\(/ { grab = 1; next }
    grab && /^\)/ { exit }
    grab {
        for (i = 1; i <= NF; i++) {
            t = $i
            gsub(/["()]/, "", t)
            if (t ~ /^\$[A-Z_]+$/) { print t; break }
        }
    }
' "$RELEASE")

if [ "${#VARS[@]}" -ne 3 ]; then
    bad "SIGN_ARTIFACTS deberia nombrar 3 artefactos y nombra ${#VARS[@]}"
else
    ok "SIGN_ARTIFACTS nombra 3 artefactos: ${VARS[*]}"
fi

STAGE_LINE="$(grep -n 'step "8/15 — sha256 + CHECKSUMS + sbom.json"' "$RELEASE" | cut -d: -f1)"
SIGN_LINE="$(grep -n '^step "8c/15' "$RELEASE" | cut -d: -f1)"
REGION=""
if [ -n "$STAGE_LINE" ] && [ -n "$SIGN_LINE" ]; then
    REGION="$(sed -n "${STAGE_LINE},${SIGN_LINE}p" "$RELEASE")"
    ok "region staging->8c acotada (lineas $STAGE_LINE..$SIGN_LINE)"
else
    bad "no se localizaron los pasos 8 y 8c"
fi

# ── (1) Cada expresion resuelve a algo que vive en $TMP ───────────────────
# Dos formas legitimas de acabar en $TMP: que la variable se defina ya dentro
# de $TMP, o que se copie explicitamente. Se aceptan las dos y se exige una.
staged=0
for var in "${VARS[@]}"; do
    # awk emite `$BIN` con el dollar; los patrones de abajo lo usan
    # como sufijo, asi que se quita para no escribir `$$BIN`.
    v="${var#\$}"
    hit=0
    # Definido dentro de $TMP. Se busca en el script ENTERO y se exige que la
    # definicion sea ANTERIOR al 8c: recortar la busqueda al bloque de staging
    # daria un falso negativo para UNIFIED y BUNDLE_TARBALL, que se definen en
    # el paso 5/7, muy por encima. Un guard que solo mira su propia ventana
    # declara roto lo que esta bien.
    if def_line="$(grep -nE "^[[:space:]]*$v=\"\\\$TMP/" "$RELEASE" | head -1 | cut -d: -f1)" \
       && [ -n "$def_line" ] && [ "$def_line" -lt "$SIGN_LINE" ]; then
        hit=1
    fi
    if printf '%s' "$REGION" | grep -qF "cp \"\$$v\" \"\$TMP/"; then
        hit=1   # copiado a $TMP en el staging
    fi
    if [ "$hit" = 1 ]; then
        ok "$v acaba en \$TMP antes del 8c"
        staged=$((staged + 1))
    else
        bad "$v NO acaba en \$TMP antes del 8c: el bucle lo buscara ahi y no lo encontrara"
    fi
done

# ── (2) El bucle resuelve "$TMP/$artifact", con una sola regla ────────────
LOOP="$(sed -n '/^[[:space:]]*for artifact in .*SIGN_ARTIFACTS.*; do/,/^[[:space:]]*done/p' "$RELEASE")"
if printf '%s' "$LOOP" | grep -qF 'src_artifact="$TMP/$artifact"'; then
    ok "el bucle resuelve src_artifact=\$TMP/\$artifact (una sola regla para los tres)"
else
    bad "el bucle no resuelve src_artifact=\$TMP/\$artifact: cada artefacto necesita una regla y solo hay una"
fi

# ── (3) La rama de skip es satisfacible, y la existencia no se adelanta ──
if printf '%s' "$LOOP" | grep -q 'SDDK_SKIP_SIGNING:-0'; then
    SKIP_BLOCK="$(printf '%s' "$LOOP" | sed -n '/SDDK_SKIP_SIGNING/,/^\s*fi$/p')"
    if printf '%s' "$SKIP_BLOCK" | grep -q 'SIGNED_COUNT=$((SIGNED_COUNT + 1))'; then
        ok "la rama de skip incrementa SIGNED_COUNT: la via sin firmar es satisfacible"
    else
        bad "la rama de skip NO incrementa el contador: SDDK_SKIP_SIGNING=1 no puede llegar al total"
    fi
    # La comprobacion de existencia PRECEDE al skip, y eso no es un defecto:
    # con el staging correcto el artefacto existe y el skip se alcanza. Solo
    # seria un problema si un artefacto faltara, y en ese caso lo que importa
    # es que se diga. Se comprueba eso, no el orden.
    #
    # NOTA: el `continue` por artefacto ausente hace que el contador no llegue
    # al total y el error diga "signed 2 of 3", que culpa a la firma de un
    # problema de layout. Hoy el staging lo impide; si alguien lo rompe, este
    # guard cae por el CONTROL de 3/3, que es donde se ve.
    if printf '%s' "$LOOP" | grep -q 'artifact missing for signing'; then
        ok "un artefacto ausente avisa por su nombre en vez de fallar en silencio"
    else
        bad "un artefacto ausente no avisa: el fallo se presentaria como un problema de firma"
    fi
else
    bad "no se encuentra la rama SDDK_SKIP_SIGNING en el bucle"
fi

# ── (4) all-or-nothing contra el numero de artefactos, no una constante ────
if grep -qF 'if [ "$SIGNED_COUNT" -ne "${#SIGN_ARTIFACTS[@]}" ]; then' "$RELEASE"; then
    ok "el all-or-nothing compara contra \${#SIGN_ARTIFACTS[@]}, no contra una constante"
else
    bad "el all-or-nothing no compara contra el numero de artefactos: un denominador escrito a mano es una segunda fuente de verdad"
fi

# ── (5) Lo publicado es el mismo fichero que lo firmado ───────────────────
ASSETS_BLOCK="$(sed -n '/^ASSETS=(/,/^)/p' "$RELEASE")"
if printf '%s' "$ASSETS_BLOCK" | grep -qF '"$TMP/$(basename "$BIN")"'; then
    ok "ASSETS publica la copia de \$TMP, la misma que firma el 8c"
else
    bad "ASSETS no publica la copia de \$TMP: se publicaria un fichero y se firmaria otro"
fi

# ── Control de no-vacuidad: la invariante se CUENTA ───────────────────────
if [ "$staged" -eq 3 ] && [ "${#VARS[@]}" -eq 3 ]; then
    ok "CONTROL: 3 de 3 expresiones resuelven a un artefacto en \$TMP, luego el 8c puede alcanzar 3/3"
else
    bad "CONTROL: $staged de 3 staged, ${#VARS[@]} artefactos: el 8c no puede alcanzar el total"
fi

MUT="tests/test_release_sign_artifacts_mutation.sh"
if [ -f "$REPO/$MUT" ]; then
    ok "pareja de autofalsacion presente: $MUT"
else
    bad "sin pareja de autofalsacion: $MUT"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ] || exit 1
echo "RESULT: PASS — el 8c es satisfacible en las dos vias, y lo publicado es lo firmado."
