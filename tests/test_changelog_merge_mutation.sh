#!/usr/bin/env bash
# Autofalsacion de `tests/test_changelog_merge.sh` (INC-DEBT-074).
#
# QUE COMPRUEBA Y POR QUE HACE FALTA
# ----------------------------------
# Un guard que nadie ha visto caer no es un guard: es decoracion. Este
# script corre el guard contra la libreria MUTADA y exige que caiga, una
# mutacion cada vez y por su PROPIA comprobacion.
#
# Cada mutacion corrompe UNA SOLA cosa, y cada una se mide por la
# comprobacion que le corresponde. Una mutacion compuesta no puede decir
# cual de las dos cosas provoco la caida, y MEDIDO: dos de las siete
# mutaciones de la primera version eran compuestas de facto y caian por
# el motivo equivocado.
#
# EL ARMA QUE USA
# ---------------
# El guard saca su codigo de `$ROOT/scripts/lib/changelog_merge.sh`, luego
# este script monta un REPO EN MINIATURA con la libreria mutada y el guard
# intacto, y corre el guard ahi. No hace falta abrir un seam en el codigo
# de produccion para poder romperlo: basta con darle otro sitio donde
# estar. Al final se comprueba que la libreria REAL conserva su sha: un
# falsificador que se lleva el codigo por delante no ha medido nada.
#
# POR QUE LOS PARES VAN EN UN HEREDOC
# -----------------------------------
# El texto de una mutacion es codigo de la libreria, con comillas simples
# (`$'\\n'`), dobles y `$`. Meterlo en una cadena de Bash exige anidar
# comillas, y `$'\\n'` CIERRA la cadena que lo contiene. MEDIDO: ese
# callejon produjo un par partido en dos lineas y un error de sintaxis.
# Con un heredoc de comillas el texto llega literal, sin escapado. Y el
# separador de las dos mitades es `%%` y no un TAB, porque un TAB escrito
# a mano en el codigo fuente es invisible y se pierde en cuanto alguien
# reformatea el fichero.
#
# MUTACION NO APLICADA = SKIP, NUNCA PASS
# ---------------------------------------
# Un patron que no casa no muta nada, y un patron que casa en dos sitios es
# una mutacion compuesta. `mutate` exige ambos casos y devuelve SKIP con su
# motivo, porque contarlo como PASS seria mentir sobre lo que se sabe.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/changelog_merge.sh"
GUARD="$ROOT/tests/test_changelog_merge.sh"
WORK="$(mktemp -d)" || { echo "FAIL: sin sandbox"; exit 1; }

# shellcheck disable=SC2329
cleanup() { rm -rf "$WORK" >/dev/null 2>&1 || true; }
trap cleanup EXIT

PRISTINE_SHA="$(sha256sum < "$LIB" | cut -d' ' -f1)"

PASS=0
FAIL=0
SKIP=0
ok()   { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skip() { echo "  [SKIP] $1 -- no se puede contar como PASS"; SKIP=$((SKIP + 1)); }

# ── la mutacion ───────────────────────────────────────────────────────────
# stdin: las lineas `viejo%%nuevo`, con `%%` de separador. Debe haber al
# menos un `%` en cada linea, el texto viejo tiene que casar EXACTAMENTE
# una vez, y el texto nuevo no puede estar vacio: un nuevo vacio BORRA la
# linea, rompe la libreria entera y hace que el guard caiga por todo en
# vez de por su comprobacion. MEDIDO en la primera version.
mutate() {  # mutate <lib_destino> <pares_file>
    python3 - "$1" "$2" <<'PY'
import pathlib, sys
dst = pathlib.Path(sys.argv[1])
text = dst.read_text(encoding="utf-8")
for raw in pathlib.Path(sys.argv[2]).read_text(encoding="utf-8").splitlines():
    if not raw.strip():
        continue
    if "%%" not in raw:
        print("SIN_SEPARADOR\t" + raw[:60])
        sys.exit(3)
    old, new = raw.split("%%", 1)
    n = text.count(old)
    if n != 1:
        print(f"CASAS_{n}\t" + old[:60])
        sys.exit(3)
    if new == "":
        print("REEMPLAZO_VACIO\t" + old[:60])
        sys.exit(3)
    text = text.replace(old, new, 1)
dst.write_text(text, encoding="utf-8")
print("APLICADA")
PY
}

# ── correr el guard contra la libreria mutada ─────────────────────────────
# run_mutated <etiqueta> <needle_esperado>...   (los pares llegan por stdin)
run_mutated() {
    local label="$1"; shift
    local mini="$WORK/$label"
    mkdir -p "$mini/scripts/lib" "$mini/tests"
    cp "$LIB" "$mini/scripts/lib/changelog_merge.sh"
    cp "$GUARD" "$mini/tests/test_changelog_merge.sh"

    local pairs_file="$WORK/$label.pairs"
    cat > "$pairs_file"

    local m
    if ! m="$(mutate "$mini/scripts/lib/changelog_merge.sh" "$pairs_file")"; then
        skip "$label -- la mutacion no se aplico: $m"
        return 2
    fi

    local before after
    before="$(sha256sum < "$LIB" | cut -d' ' -f1)"
    after="$(sha256sum < "$mini/scripts/lib/changelog_merge.sh" | cut -d' ' -f1)"
    if [ "$before" = "$after" ]; then
        skip "$label -- la mutacion no cambio el fichero"
        return 2
    fi

    local out rc
    out="$(cd "$mini" && bash tests/test_changelog_merge.sh 2>&1)"
    rc=$?

    if [ "$rc" -eq 0 ]; then
        bad "$label -- el guard SIGUE EN VERDE con la libreria mutada"
        return 1
    fi

    local needle
    for needle in "$@"; do
        # El needle es un FRAGMENTO del mensaje, no el mensaje entero: la
        # comprobacion se imprime como "[FAIL] <detalle> -- <motivo>", luego
        # anteponer "[FAIL] " al needle no casaria con una linea de verdad.
        if ! printf '%s\n' "$out" | grep -F "$needle" | grep -qF '[FAIL]'; then
            bad "$label -- el guard cayo, pero no por '$needle'"
            printf '%s\n' "$out" | grep '\[FAIL\]' | head -5 | sed 's/^/          /'
            return 1
        fi
    done

    ok "$label -- el guard cayo por su propia comprobacion"
    return 0
}

echo "=== autofalsacion del merge del changelog (INC-DEBT-074) ==="
echo

# ── base: el guard tiene que pasar con la libreria intacta ────────────────
echo "-- base --"
if bash "$GUARD" >/dev/null 2>&1; then
    ok "el guard pasa con la libreria intacta"
else
    bad "el guard ya falla con la libreria intacta; las mutaciones no medirian nada"
    echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
    exit 1
fi
echo

# M1 ─ el dedup deja de dispararse. Es el defecto original de INC-DEBT-074.
#      Se invierte la CONDICION en vez de borrar el bloque.
run_mutated "M1_sin_dedup" \
    "el feat NO se duplico" "el test NO se duplico" <<'PAIRS'
if grep -qxF -- "$fp" "$fingerprints_file"; then%%if false; then
PAIRS

# M2 ─ la huella mira 2 palabras en vez de 4. Demasiado gruesa: BORRA un
#      commit que si se publico, que es peor que duplicarlo.
run_mutated "M2_huella_2_palabras" \
    "se anade (la huella mira 4)" <<'PAIRS'
cut -d' ' -f1-4%%cut -d' ' -f1-2
PAIRS

# M3 ─ la huella pierde la clave (tipo+scope). Colisionan `fix(a)` y
#      `test(a)` con el payload identico.
run_mutated "M3_sin_clave" \
    "OTRO TIPO con el mismo payload" <<'PAIRS'
    printf '%s|%s' "$key" "$fingerprint"%%    printf '%s' "$fingerprint"
PAIRS

# M4 ─ el merge de grupos deja de REUSAR el grupo existente y crea otro con
#      el mismo nombre: la seccion sale con dos `### Other`.
run_mutated "M4_grupos_duplicados" \
    "sin group-headers duplicados" <<'PAIRS'
if (hdr[j] == hdr[g]) { g = j; break }%%if (0) { g = j; break }
PAIRS

# M5 ─ lo no clasificable se DESCARTA en vez de conservarse: el fallo en la
#      direccion peligroso, contenido perdido en vez de ruido. Se toca SOLO
#      la linea que anade el item al grupo. La primera version invertia la
#      condicion del filtro, lo que ademas desactivaba el dedup entero y
#      hacia caer el guard por un motivo compuesto.
run_mutated "M5_descarta_no_clasificable" \
    "no parseable se CONSERVA" <<'PAIRS'
        pending+="$line"$'\n'%%        [ -n "$fp" ] && pending+="$line"$'\n'
PAIRS

# M6 ─ la disposicion se deduce del cuerpo merged (que SIEMPRE tiene
#      contenido) en vez de de si sobrevivio algun item: la cuarta
#      disposicion queda inalcanzable. Es el bug que se corrigio.
run_mutated "M6_disposicion_morta" \
    "merged_nothing' declarada" <<'PAIRS'
    if grep -q '^  - ' "$filtered"; then%%    if [ -s "$merged_body" ]; then
PAIRS

# M7 ─ el EMISOR deja de descartar los grupos sin items. Se muta el awk y no
#      el filtro porque el awk es el UNICO autor de esa propiedad: cuando
#      los dos la aplicaban, quitar una sola no hacia caer el guard
#      (medido), que es lo que hace una propiedad imposible de falsar.
run_mutated "M7_grupos_vacios" \
    "sin group-headers vacios" <<'PAIRS'
            if (n == 0) continue%%            if (0) continue
PAIRS

echo
echo "=== la libreria real quedo intacta? ==="
FINAL_SHA="$(sha256sum < "$LIB" | cut -d' ' -f1)"
if [ "$PRISTINE_SHA" = "$FINAL_SHA" ]; then
    ok "sha de scripts/lib/changelog_merge.sh sin cambios ($FINAL_SHA)"
else
    bad "la libreria REAL cambio durante la falsificacion: $PRISTINE_SHA -> $FINAL_SHA"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
if [ "$FAIL" -gt 0 ]; then
    echo "RESULT: FAIL — $FAIL mutacion(es) no fueron detectadas."
    exit 1
fi
if [ "$SKIP" -gt 0 ]; then
    echo "RESULT: FAIL — $SKIP mutacion(es) no se pudieron aplicar; un SKIP no es un PASS."
    exit 1
fi
echo "RESULT: PASS — las siete mutaciones caen, cada una por su comprobacion."
exit 0
