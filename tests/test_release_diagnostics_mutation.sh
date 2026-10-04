#!/usr/bin/env bash
# Autofalsacion del diagnostico de release, y de su cableado.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Nueve mutaciones, cada una corrompiendo UN punto de enforcement y cada una
# exigiendo que caiga el caso nominado que lo vigila. Una mutacion compuesta
# no puede decir cual de las comprobaciones cayo por efecto colateral, que es
# como se declara un guard que vigila menos de lo que cree.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`: cada una comprueba por
# sha que el fichero cambio de verdad, y la restauracion se comprueba byte a
# byte. Sin esa comprobacion, un parche que no encuentra su texto —porque el
# fichero cambio de forma— se contaria como una deteccion y el falsador seria
# verde sobre una base que no vigila nada.
#
# EL NEEDLE ES EL NOMBRE DEL CASO, NO EL TEXTO DE UNA ASERCION
# -----------------------------------------------------------
# Los casos se marcan con `caso_termina`, que imprime `[ok] C1` solo si TODAS
# las aserciones del caso pasaron. Buscar una asercion suelta leeria al reves
# un caso a medias caido. Ese defecto estuvo en este repo durante el
# desarrollo de estas pruebas y lo encontro el propio falsador al mirar el
# numero de casos que caian: cero, cuando la mutacion habia roto tres
# aserciones. Los marcadores ya no son decorativos.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/release_diagnostics.sh"
RELEASE="$ROOT/scripts/release.sh"
TEST="$ROOT/tests/test_release_diagnostics.sh"
WIRE="$ROOT/tests/test_release_diagnostics_wiring.sh"

PASS=0
FAIL=0
SKIP=0
OUT="$(mktemp)"
BACKUP="$(mktemp)"
trap 'rm -f "$OUT" "$BACKUP"' EXIT

banner() { printf '\n== %s\n' "$*"; }

# --- base: una autofalsacion sobre una base roja no demuestra nada ------------

banner "BASE — los dos guards tienen que estar verdes antes de quitarles dientes"
if bash "$TEST" > "$OUT" 2>&1; then
    printf '  [ok]   base: %s verde\n' "$(basename "$TEST")"
else
    printf '  [FATAL] base: %s ya esta ROJO; no se puede falsar sobre una base rota\n' "$(basename "$TEST")"
    tail -20 "$OUT"
    exit 1
fi
if bash "$WIRE" > "$OUT" 2>&1; then
    printf '  [ok]   base: %s verde\n' "$(basename "$WIRE")"
else
    printf '  [FATAL] base: %s ya esta ROJO\n' "$(basename "$WIRE")"
    tail -20 "$OUT"
    exit 1
fi

# --- la maquina de mutaciones ------------------------------------------------

# mutar <etiqueta> <fichero> <test> <caso> <que-debe-caer> <codigo python>
#
# Devuelve: PASS si el caso esperado CAE, FAIL si sigue verde, SKIP si el parche
# no se aplico. En los tres casos restaura el fichero byte-identico.
mutar() {
    local etiqueta="$1" fichero="$2" test="$3" caso="$4" que="$5" pycode="$6"
    local sha_antes sha_mut sha_restore

    sha_antes="$(sha256sum "$fichero" | cut -d' ' -f1)"
    cp "$fichero" "$BACKUP"

    if ! MUT_FILE="$fichero" python3 -c "$pycode" 2>/dev/null; then
        printf '  [SKIP] %s — el parche lanzo error\n' "$etiqueta"
        SKIP=$((SKIP + 1))
        cp "$BACKUP" "$fichero"
        return
    fi
    sha_mut="$(sha256sum "$fichero" | cut -d' ' -f1)"
    if [ "$sha_antes" = "$sha_mut" ]; then
        printf '  [SKIP] %s — el parche NO cambio el fichero: no esta midiendo nada\n' "$etiqueta"
        SKIP=$((SKIP + 1))
        cp "$BACKUP" "$fichero"
        return
    fi

    # INTEGRIDAD DEL INSTRUMENTO: un parche que deja el fichero sin sintaxis
    # valida no mide la propiedad que dice medir —mide que el script no
    # arranca, que hace caer TODOS los casos—, y se contaria como deteccion.
    # MEDIDO, y no fue hipotetico: la primera version de M1 se carried el `;;`
    # al reescribir la rama 137, el `source` del test fallaba y caian los diez
    # casos. El falsador lo-annunciaba como "M1 detectada" con una honestidad
    # que no tenia. Un parche degenerado es `SKIP`, con su motivo, nunca `PASS`.
    if ! bash -n "$fichero" 2>/dev/null; then
        printf '  [SKIP] %s — el parche dejo el fichero SIN SINTAXIS VALIDA: mide que no arranca, no la propiedad\n' \
            "$etiqueta"
        SKIP=$((SKIP + 1))
        cp "$BACKUP" "$fichero"
        return
    fi

    bash "$test" > "$OUT" 2>&1
    local caidas
    caidas="$(grep -cE '^\s+\[FAIL-CASO\]' "$OUT")"

    # restaurar ANTES de juzgar el resultado, para que un fallo del falsador no
    # deje el arbol mutado.
    cp "$BACKUP" "$fichero"
    sha_restore="$(sha256sum "$fichero" | cut -d' ' -f1)"
    if [ "$sha_restore" != "$sha_antes" ]; then
        printf '  [FATAL] %s — la restauracion no fue byte-identica\n' "$etiqueta"
        exit 1
    fi

    if grep -qE "^[[:space:]]*\[ok\] ${caso}\$" "$OUT"; then
        printf '  [FAIL] %s — %s SIGUE VERDE. %s\n' "$etiqueta" "$caso" "$que"
        FAIL=$((FAIL + 1))
    else
        printf '  [ok]   %s — cae %s (%s caso(s) abajo). %s\n' "$etiqueta" "$caso" "$caidas" "$que"
        PASS=$((PASS + 1))
    fi
}

# --- M1..M8: la libreria ----------------------------------------------------

banner "M1..M8 — la libreria de diagnostico"

mutar "M1 tabla de causas sin la rama 137" "$LIB" "$TEST" C1 \
    "sin 137, un OOM deja de nombrarse y cae al 'sin causa con nombre'. Solo C1: 137 aparece en un solo caso." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = re.sub(r"^        137\) echo .*$",
           "        137) echo \"sin causa con nombre para el codigo 137\" ;;",
           s, flags=re.M)
open(p, "w").write(s)
'

mutar "M2 el diagnostico no nombra el paso" "$LIB" "$TEST" C4 \
    "sin el nombre del paso, un log cortado solo dice que algo fallo." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = re.sub(r"^        printf .    paso     : %s.*$", "        :", s, flags=re.M)
open(p, "w").write(s)
'

mutar "M3 la guardia de idempotencia vuelve a ser una variable" "$LIB" "$TEST" C6 \
    "una variable no cruza subshells, y el bloque sale dos veces: se lee como dos fallos." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("if [ -n \"$marker\" ]; then", "if [ -z \"$marker\" ]; then")
open(p, "w").write(s)
'

mutar "M4 el preflight de recursos nunca falla" "$LIB" "$TEST" C7 \
    "un gate que nunca dice que no informa de nada, y aqui informa de ENOSPC." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("    return \"$failed\"", "    return 0")
open(p, "w").write(s)
'

mutar "M5 _is_own_descendant vuelve a preguntar al reves" "$LIB" "$TEST" C9 \
    "subir desde \$\$ buscando el PID no ve lo que esta por debajo, y el cargo propio se cuela." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("_is_own_descendant() {\n    local cur=\"$1\"",
              "_is_own_descendant() {\n    local objetivo=\"$1\"\n    local cur=\"$$\"")
s = s.replace("        [ \"$cur\" = \"$$\" ] && return 0",
              "        [ \"$cur\" = \"$objetivo\" ] && return 0")
open(p, "w").write(s)
'

mutar "M6 el criterio de retencion vuelve a ser una cadena en los argumentos" "$LIB" "$TEST" C9 \
    "MEDIDO: con un grep, todo lo que lleve cargo en la ruta —los asv-* de esta maquina, el propio shell— se cuenta." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("        [ \"${comm:-}\" = \"cargo\" ] || continue",
              "        case \"$comm\" in *cargo*) ;; *) continue ;; esac")
s = s.replace("ps -eo pid=,etimes=,comm= 2>/dev/null",
              "ps -eo pid=,etimes=,args= 2>/dev/null")
open(p, "w").write(s)
'

mutar "M7 un codigo sin causa con nombre recibe una causa inventada" "$LIB" "$TEST" C3 \
    "una causa inventada dirige la investigacion a un sitio falso: peor que declararla desconocida." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("echo \"sin causa con nombre para el codigo $code\"",
              "echo \"problema del sistema\"")
open(p, "w").write(s)
'

mutar "M8 el diagnostico imprime la causa pero no la medicion" "$LIB" "$TEST" C5 \
    "una causa afirmada sin cifras obliga a creerla en vez de comprobarla." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = re.sub(r"^        _diag_measure .*$", "        :", s, flags=re.M)
open(p, "w").write(s)
'

# --- M9: el cableado en release.sh -------------------------------------------

banner "M9 — el cableado"

mutar "M9 el trap vuelve a limpiar sin diagnosticar" "$RELEASE" "$WIRE" E3 \
    "sin el manejador, el release sigue siendo el de antes: limpio y mudo." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("trap release_on_exit EXIT", "trap cleanup_release_scratch EXIT")
open(p, "w").write(s)
'

mutar "M10 el guard de wiring se quita --skip-tests y vuelve a dispararse" "$WIRE" "$WIRE" E0 \
    "sin --skip-tests el dry-run alcanza el 1b, el 1b corre este fichero, y el guard se llama a si mismo." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("\"$RELEASE\" --dry-run --skip-tests", "\"$RELEASE\" --dry-run")
open(p, "w").write(s)
'

mutar "M11 el 1b vuelve a descartar la salida del test que falla" "$RELEASE" "$WIRE" E0 \
    "con >/dev/null, el release vuelve a no decir por que fallo y su unica consigna es mirar a mano sin decir donde." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace(
    "        if ! bash \"$t\" >\"$t_log\" 2>&1; then",
    "        if ! bash \"$t\" >/dev/null; then")
s = s.replace(
    "            if ! python3 \"$p\" >\"$p_log\" 2>&1; then",
    "            if ! python3 \"$p\" >/dev/null; then")
open(p, "w").write(s)
'

mutar "M12 el 1b vuelve a enseñar solo la cola del log" "$RELEASE" "$WIRE" E0 \
    "con tail, los fallos del principio del log desaparecen y el informe parece completo sin estarlo." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("            tail -10 \"$t_log\" >&2", "            true")
s = s.replace("            tail -10 \"$p_log\" >&2", "            true")
open(p, "w").write(s)
'

# --- resumen -----------------------------------------------------------------

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -eq 0 ] && [ "$SKIP" -eq 0 ]; then
    printf 'RESULT: PASS — las doce comprobaciones tienen dientes, y el arbol quedo intacto.\n'
    exit 0
fi
if [ "$FAIL" -ne 0 ]; then
    printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
    exit 1
fi
printf 'RESULT: DECLARADO — %s mutacion(es) no se aplicaron y no cuentan como deteccion.\n' "$SKIP"
exit 1
