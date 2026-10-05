#!/usr/bin/env bash
# test_release_exclusion_mutation.sh
#
# Autofalsador de `tests/test_release_exclusion.sh` (INC-DEBT-075).
#
# QUE COMPRUEBA Y POR QUE HACE FALTA
# ----------------------------------
# Un guard que nadie ha visto caer no es un guard: es decoracion. Este script
# corre el guard contra el codigo MUTADO y exige que caiga por su PROPIA
# comprobacion.
#
# EL ARMA QUE USA, y por que no borra el sujeto
# ---------------------------------------------
# El guard saca su codigo de `$ROOT/scripts/lib/release_exclusion.sh`, luego
# este script monta un REPO EN MINIATURA con el fichero mutado y el guard
# intacto, y corre el guard ahi. No hace falta abrir un seam en produccion para
# poder romperlo: basta con darle otro sitio donde estar. Al final se comprueba
# que el sujeto REAL conserva su sha: un falsificador que se lleva el codigo
# por delante no ha medido nada.
#
# MEDIDO al escribir este fichero, y por que existe el modo estricto: la
# primera version mutaba la libreria IN PLACE con un aplicador propio. Se
# comio dos casos —uno sin aplicar (contado como si hubiera medido) y otro
#Applied que dejo la libreria con sintaxis rota, de modo que el guard cayo por
# todo en vez de por su comprobacion—, y el propio falsador emitio errores de
# bash mientras corria. El aplicador de aqui no puede hacer ninguna de las dos
# cosas: exige que el texto viejo case EXACTAMENTE una vez, exige reemplazo no
# vacio, y trabaja sobre una copia.
#
# POR QUE LOS PARES VAN EN UN HEREDOC
# -----------------------------------
# El texto de una mutacion es codigo de la libreria, con comillas, `$` y
# escapes. Meterlo en una cadena de Bash exige anidar comillas y se rompe. Con
# un heredoc de comillas el texto llega literal. El separador de las dos mitades
# es `%%` y no un TAB, porque un TAB escrito a mano es invisible y se pierde en
# cuanto alguien reformatea el fichero.
#
# MUTACION NO APLICADA = SKIP, NUNCA PASS
# ---------------------------------------
# Un patron que no casa no muta nada, y un patron que casa en dos sitios es una
# mutacion compuesta. `mutate` exige ambos casos y devuelve SKIP con su motivo,
# porque contarlo como PASS seria mentir sobre lo que se sabe.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB_REL="scripts/lib/release_exclusion.sh"
REL_REL="scripts/release.sh"
GUARD_REL="tests/test_release_exclusion.sh"
WORK="$(mktemp -d)" || { printf 'FAIL: sin sandbox\n'; exit 1; }

# shellcheck disable=SC2329
cleanup() { rm -rf "$WORK" >/dev/null 2>&1 || true; }
trap cleanup EXIT

PRISTINE_LIB="$(sha256sum < "$ROOT/$LIB_REL" | cut -d' ' -f1)"
PRISTINE_REL="$(sha256sum < "$ROOT/$REL_REL" | cut -d' ' -f1)"

PASS=0
FAIL=0
SKIP=0
ok()   { printf '  [ok]   %s\n' "$1"; PASS=$((PASS + 1)); }
bad()  { printf '  [FAIL] %s\n' "$1"; FAIL=$((FAIL + 1)); }
skip() { printf '  [SKIP] %s -- no se puede contar como PASS\n' "$1"; SKIP=$((SKIP + 1)); }

# mutate <fichero_destino> <fichero_de_pares>
# lineas `viejo%%nuevo`. Exige: separador presente, el viejo casa EXACTAMENTE
# una vez, y el nuevo no esta vacio (un vacio BORRA la linea, rompe el fichero
# entero y hace que el guard caiga por todo en vez de por su comprobacion).
mutate() {
    python3 - "$1" "$2" <<'PY'
import pathlib, sys
dst = pathlib.Path(sys.argv[1])
text = dst.read_text(encoding="utf-8")
for raw in pathlib.Path(sys.argv[2]).read_text(encoding="utf-8").splitlines():
    if not raw.strip():
        continue
    if "%%" not in raw:
        print("SIN_SEPARADOR\t" + raw[:60]); sys.exit(3)
    old, new = raw.split("%%", 1)
    n = text.count(old)
    if n != 1:
        print("CASAS_%d\t%s" % (n, old[:60])); sys.exit(3)
    if new == "":
        print("REEMPLAZO_VACIO\t" + old[:60]); sys.exit(3)
    text = text.replace(old, new, 1)
dst.write_text(text, encoding="utf-8")
print("APLICADA")
PY
}

# montar_mini <etiqueta> — repo en miniatura con guard y release.sh intactos.
montar_mini() {
    local mini="$WORK/$1"
    mkdir -p "$mini/scripts/lib" "$mini/tests"
    cp "$ROOT/$LIB_REL" "$mini/$LIB_REL"
    cp "$ROOT/$REL_REL" "$mini/$REL_REL"
    cp "$ROOT/$GUARD_REL" "$mini/$GUARD_REL"
    printf '%s' "$mini"
}

# run_mutated <etiqueta> <fichero_relativo_a_mutar> <needle_esperado>
# Los pares llegan por stdin.
run_mutated() {
    local label="$1" objetivo="$2" needle="$3"
    local mini pairs_file m out before after

    mini="$(montar_mini "$label")"
    pairs_file="$WORK/$label.pairs"
    cat > "$pairs_file"

    if ! m="$(mutate "$mini/$objetivo" "$pairs_file")"; then
        skip "$label -- la mutacion no se aplico: $m"
        return 0
    fi

    if [ "$objetivo" = "$LIB_REL" ]; then
        before="$PRISTINE_LIB"; after="$(sha256sum < "$mini/$LIB_REL" | cut -d' ' -f1)"
    else
        before="$PRISTINE_REL"; after="$(sha256sum < "$mini/$REL_REL" | cut -d' ' -f1)"
    fi
    if [ "$before" = "$after" ]; then
        skip "$label -- la mutacion no cambio el fichero"
        return 0
    fi

    out="$(cd "$mini" && bash "$GUARD_REL" 2>&1)"
    if printf '%s' "$out" | grep -q 'RESULT: PASS'; then
        bad "$label — la mutacion se aplico y el guard SIGUE en verde"
    elif printf '%s' "$out" | grep -qF "[FAIL] $needle"; then
        ok "$label"
    else
        bad "$label — cayo, pero NO por su comprobacion (esperaba: $needle)"
        printf '%s\n' "$out" | grep '\[FAIL\]' | head -3 | sed 's/^/         /'
    fi
    return 0
}

printf '=== autofalsacion de la exclusion mutua (INC-DEBT-075) ===\n'

# --- CONTROL: con el codigo intacto el guard tiene que pasar ----------------
mini0="$(montar_mini control)"
out0="$(cd "$mini0" && bash "$GUARD_REL" 2>&1)"
if printf '%s' "$out0" | grep -q 'RESULT: PASS'; then
    ok "control: con el codigo intacto el guard pasa"
else
    bad "control: con el codigo INTACTO el guard ya falla"
    printf '%s\n' "$out0" | tail -6 | sed 's/^/         /'
    exit 1
fi

# --- M1: tomar un candado que ya es nuestro vuelve a ser conflicto ---------
run_mutated "M1: tomar un candado propio se vuelve conflicto" \
    "$LIB_REL" "X3b: tomar un candado que ya es nuestro es idempotente, no un conflicto" <<'EOF'
[ "$holder" = "$pid" ]; then%%[ "$holder" = "$pid-otro-proceso" ]; then
EOF

# --- M2: el aviso deja de nombrar al pid que lo tiene ---------------------
run_mutated "M2: el aviso de exclusion ya no nombra el pid que lo tiene" \
    "$LIB_REL" "X2: el mensaje NOMBRA el pid que lo tiene, no dice solo" <<'EOF'
printf '  x ya hay un release en curso de esta MISMA version (pid %s)\n' \%%printf '  x ya hay un release en curso de esta MISMA version\n' \
EOF

# --- M3: sin directorio de candados se responde que no hay nadie ----------
run_mutated "M3: sin directorio de candados se simula estar libre" \
    "$LIB_REL" "X7: sin directorio de candados se ABORTA, no se simula estar libre" <<'EOF'
        return 2%%        return 0
EOF

# --- M4: soltar el candado de otro lo libera igualmente -------------------
# La comparacion pasa a `=` con un pid que NUNCA casa, de modo que el bloque
# «no es mio, no se suelta» se salta SIEMPRE. MEDIDO: la primera version
# cambio `!=` por `!=` con otro valor, lo que deja la condicion siempre
# VERDADERA y el `release` se niega a soltar SIEMPRE — es decir, media
# mutacion, y el guard pasaba por el motivo equivocado.
run_mutated "M4: soltar el candado de otro proceso lo libera igualmente" \
    "$LIB_REL" "X5: soltar con un pid que no es el del candado NO lo libera" <<'EOF'
if [ -n "$holder" ] && [ "$holder" != "$pid" ]; then%%if [ -n "$holder" ] && [ "$holder" = "$pid-nunca-casa" ]; then
EOF

# --- M5: la clave del candado ignora la version ----------------------------
run_mutated "M5: la clave del candado ignora la version" \
    "$LIB_REL" "X6: dos VERSIONES del mismo repo no comparten candado" <<'EOF'
printf 'release-%s-%s' "$flat" "$version"%%printf 'release-%s' "$flat"
EOF

# --- M6: la exclusion no esta conectada al preflight del release -----------
# Esta muta `release.sh`, no la libreria: el cableado es una mitad del defecto y
# una libreria correcta que nadie llama no excluye a nadie.
run_mutated "M6: el release deja de tomar el candado en el preflight" \
    "$REL_REL" "X8: y se INVOCA con ese mismo nombre" <<'EOF'
release_exclusion_preflight() {%%release_exclusion_preflight_nunca_llamado() {
EOF

# --- M7: el candado no se suelta al salir ---------------------------------
run_mutated "M7: el release deja de soltar el candado al salir" \
    "$REL_REL" "X8: y lo suelta en release_on_exit" <<'EOF'
        release_exclusion_release "$RELEASE_EXCLUSION_KEY" "$$" || true%%        :
EOF

# --- el sujeto REAL sigue intacto ----------------------------------------
if [ "$(sha256sum < "$ROOT/$LIB_REL" | cut -d' ' -f1)" = "$PRISTINE_LIB" ] \
   && [ "$(sha256sum < "$ROOT/$REL_REL" | cut -d' ' -f1)" = "$PRISTINE_REL" ]; then
    ok "el sujeto real conserva su sha: este falsador no se llevo el codigo"
    PASS=$((PASS + 1))
else
    bad "el sujeto real ha cambiado: el falsador se llevo el codigo por delante"
fi

printf '\nPASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" = "0" ] && [ "$SKIP" = "0" ]; then
    printf 'RESULT: PASS — las siete mutaciones caen, cada una por su comprobacion.\n'
    exit 0
fi
if [ "$FAIL" = "0" ]; then
    printf 'RESULT: INCOMPLETO — %d mutacion(es) NO se aplicaron: un falsador que no\n' "$SKIP"
    printf '         muta no ha medido nada, y eso no es un PASS.\n'
    exit 1
fi
printf 'RESULT: FAIL — hay comprobaciones que no tienen dientes.\n'
exit 1
