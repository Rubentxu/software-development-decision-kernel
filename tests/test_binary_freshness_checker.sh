#!/bin/bash
# test_binary_freshness_checker.sh — el juez de frescura tiene dientes.
#
# QUE MIDE
# --------
# `scripts/check_binary_freshness.sh` decide si se puede certificar una
# medicion hecha con un binario. Se prueba SIETE estados, porque un checker
# que solo distingue "viejo" de "nuevo" no ha disagreements nada:
#
#   matches · behind · ahead · diverged · dirty · unknown-commit ·
#   no-build-id
#
# El ultimo es el que justifica el bloque: es un artefacto ANTERIOR al
# subcomando `dev build-id` (`032e9553`), luego no puede ni reportar su propia
# antiguedad. Desde dentro del artefacto ese estado es invisible por
# construccion —no hay nada que ejecutar que lo delate— y desde fuera es el
# mas viejo de todos. Si el guard no lo cae, el juez no existe.
#
# HERMÉTICO
# ----------
# Todo corre en un repo git temporal, con stubs que declaran la identidad que
# se les pide. No toca el repo real, ni el binario real, ni la red.
#
# AUTOFALSACION
# -------------
# Al final corre el MISMO juego de expectativas contra una copia del checker
# con la clasificacion rota (todo `matches` / OK). Si esa copia rota ALSO
# satisface las expectativas, las aserciones no tienen dientes y el guard se
# declara FAIL: un guard verde sobre una base que no puede caerse es
# decoracion. Es la version de script de "una mutacion que no cae no mide
# nada".

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CHECKER="$REPO_ROOT/scripts/check_binary_freshness.sh"

PASS=0
FAIL=0
WORK=""

ok()  { printf '  [ok]   %s\n' "$1"; PASS=$((PASS+1)); }
ko()  { printf '  [FAIL] %s\n' "$1"; FAIL=$((FAIL+1)); }

cleanup() { [ -n "$WORK" ] && [ -d "$WORK" ] && command rm -rf -- "$WORK"; }
trap cleanup EXIT

# ── fabrica de repos y stubs ────────────────────────────────────────────────
mk_stub() {  # mk_stub <ruta> <sha|unknown> <dirty> ; sin subcomando si sha == NONE
    local path="$1" sha="$2" dirty="${3:-false}"
    mkdir -p "$(dirname "$path")"
    if [ "$sha" = "NONE" ]; then
        cat > "$path" <<'STUB'
#!/bin/bash
# Artefacto anterior a 032e9553: no conoce 'dev build-id'.
echo "error: unrecognized subcommand 'build-id'" >&2
exit 2
STUB
    else
        cat > "$path" <<STUB
#!/bin/bash
if [ "\$1" = "dev" ] && [ "\$2" = "build-id" ]; then
  printf '{\n  "version": "9.9.9",\n  "identity": {\n    "sha": "$sha",\n    "source": "env",\n    "dirty": $dirty\n  }\n}\n'
  exit 0
fi
echo "sddk 9.9.9"
STUB
    fi
    chmod +x "$path"
}

# Prepara un repo con C1 <- C2 <- C3 <- C4 y una rama divergente D1 desde C1.
# MEDIDO: la primera version corria esto dentro de `$( ... )`, luego el
# `WORK=...` se perdia en el subshell y todos los stubs se escribian en "/".
# El subshell de git hace falta —el repo tiene su cwd— pero las variables que
# el resto del guard lee tienen que salir por un fichero o por un `global`.
SHAS=""
setup_repo() {
    WORK="$(mktemp -d "${TMPDIR:-/var/home/rubentxu/cargo-targets}/bfresh.XXXXXX")"
    mkdir -p "$WORK/repo"
    (
        cd "$WORK/repo" || exit 1
        git init -q .
        git config user.email guard@test
        git config user.name guard
        echo c1 > f; git add f; git commit -qm C1; C1="$(git rev-parse HEAD)"
        echo c2 > f; git commit -qam C2;   C2="$(git rev-parse HEAD)"
        echo c3 > f; git commit -qam C3;   C3="$(git rev-parse HEAD)"
        git checkout -q -b div "$C1"
        echo d1 > g; git add g; git commit -qm D1; D1="$(git rev-parse HEAD)"
        git checkout -q main 2>/dev/null || git checkout -q master
        echo c4 > f; git commit -qam C4;   C4="$(git rev-parse HEAD)"
        echo c5 > f; git commit -qam C5;   C5="$(git rev-parse HEAD)"
        # se vuelve a C4 para que HEAD=C4 y C5 sea un DESCENDIENTE suyo:
        # asi el binario va por delante y el checkout es el que esta atrasado
        git checkout -q --detach "$C4"
        printf '%s\n' "$C1 $C2 $C3 $C4 $C5 $D1" > "$WORK/shas"
    )
    SHAS="$(cat "$WORK/shas")"
}

# espera <etiqueta> <relacion_esperada> <exit_esperado> <ruta-binario> [extra-setup]
espera() {
    local etiq="$1" rel="$2" rc_esp="$3" bin="$4"
    shift 4
    # `${1:-}` y no `$1`: con `set -u`, leer el quinto argumento cuando el
    # caller no lo paso aborta el guard entero, que es la forma mas cara de
    # tener un `if` mal escrito. MEDIDO: asi fallo la primera version.
    local extra="${1:-}"
    [ -n "$extra" ] && { ( cd "$WORK/repo" && eval "$extra" ); }
    local out rc got
    out="$(cd "$WORK/repo" && bash "$CHECKER" "$bin" 2>&1)"; rc=$?
    got="$(printf '%s' "$out" | sed -n 's/^relacion  : //p' | head -1)"
    if [ "$got" = "$rel" ] && [ "$rc" -eq "$rc_esp" ]; then
        ok "$etiq -> $rel (rc=$rc)"
    else
        ko "$etiq: esperaba $rel/rc=$rc_esp y obtuvo '$got'/rc=$rc"
        printf '%s\n' "$out" | sed 's/^/         /' | head -8
    fi
}

echo "== guard: el juez de frescura del binario (INC-DEBT-064) =="
echo

if [ ! -x "$CHECKER" ]; then
    echo "FATAL: $CHECKER no existe o no es ejecutable"
    exit 1
fi

setup_repo
read -r C1 C2 C3 C4 C5 D1 <<< "$SHAS"
if [ -z "${C4:-}" ]; then
    echo "FATAL: no pude construir el repo de fixture"
    exit 1
fi
echo "  repo de fixture con C1..C4 y una rama divergente D1"
echo

echo "-- los estados que deben IMPIDEN certificar --"
mk_stub "$WORK/b_behind"  "$C3"    false   # el checkout tiene C4 encima
espera "behind"          "behind"        1 "$WORK/b_behind"
mk_stub "$WORK/b_dirty"   "$C4"    true
espera "dirty"           "dirty"         1 "$WORK/b_dirty"
mk_stub "$WORK/b_unkn"    "unknown" false
espera "commit desconocido" "unknown-commit" 1 "$WORK/b_unkn"
mk_stub "$WORK/b_nobid"   "NONE"    false
espera "sin subcomando build-id" "no-build-id" 1 "$WORK/b_nobid"
mk_stub "$WORK/b_div"     "$D1"    false
# SIN checkout de setup: despues del detach HEAD=C4, y D1 (rama desde C1)
# frente a C4 ya es diverged. El checkout que hacia falta de mas era el
# que movia HEAD al tip de la rama —C5— y dejaba los casos `matches` y
# `ahead` ejecutandose contra el commit equivocado. MEDIDO: asi fallaban
# los dos, y el diagnostico apuntaba al checker cuando el que estaba mal
# era el arnes del guard.
espera "diverged"        "diverged"      1 "$WORK/b_div"

echo
echo "-- el estado mas viejo de todos, medido aparte --"
# Repetido a proposito y con su propia linea de veredicto, porque es el que
# justifica el bloque: un artefacto sin el subcomando es el MAS viejo, y desde
# dentro seria invisible. Si aqui saliera OK, el juez no existiria.
nb_out="$(cd "$WORK/repo" && bash "$CHECKER" "$WORK/b_nobid" 2>&1)"; nb_rc=$?
if printf '%s' "$nb_out" | grep -q '032e9553' \
   && printf '%s' "$nb_out" | grep -qi 'veredicto : FALLO' \
   && [ "$nb_rc" -eq 1 ]; then
    ok "el detalle NOMBRA 032e9553 y declara FALLO, no dice 'no se puede saber'"
else
    ko "el estado sin subcomando no se distingue: $(printf '%s' "$nb_out" | tr '\n' ' ' | cut -c1-120)"
fi

echo
echo "-- los estados que NO impiden certificar --"
mk_stub "$WORK/b_match" "$C4" false
espera "matches"        "matches"       0 "$WORK/b_match"
mk_stub "$WORK/b_ahead" "$C5" false
espera "ahead"          "ahead"         0 "$WORK/b_ahead"

echo
echo "-- salida utilizable como evidencia de gate --"
j="$(cd "$WORK/repo" && bash "$CHECKER" "$WORK/b_behind" --format json 2>&1)"
if printf '%s' "$j" | grep -q '"relation":"behind"' \
   && printf '%s' "$j" | grep -q '"verdict":"FALLO"' \
   && printf '%s' "$j" | grep -q '"checkout_head":"'"$C4"'"'; then
    ok "--format json nombra relacion, veredicto y el HEAD contra el que se judged"
else
    ko "la salida json no lleva lo que un gate necesita: $j"
fi

echo
echo "-- AUTOFALSACION: el mismo juego contra un checker roto --"
ROTO="$WORK/checker-roto.sh"
# Un sed GLOBAL, no uno porClase: la primera version enumeraba cinco
# patrones con su sangria exacta y tres no casaban, luego la
# autofalsacion decia "3 de 5" y la lectura era que al checker le costaba
# notar una rotura. Lo que costaba era al mutador: las cinco relaciones
# se rompen con una sola sustitucion que no mira la sangria.
sed -E 's/REL="[a-z-]+"/REL="matches"/' \
"$CHECKER" > "$ROTO"

rotas_detectadas=0
for caso in behind dirty unknown-commit no-build-id diverged; do
    case "$caso" in
        behind)          stub="$WORK/b_behind" ;;
        dirty)           stub="$WORK/b_dirty" ;;
        unknown-commit)  stub="$WORK/b_unkn" ;;
        no-build-id)     stub="$WORK/b_nobid" ;;
        diverged)        stub="$WORK/b_div" ;;
    esac
    out="$(cd "$WORK/repo" && bash "$ROTO" "$stub" 2>&1)"; rc=$?
    got="$(printf '%s' "$out" | sed -n 's/^relacion  : //p' | head -1)"
    ver="$(printf '%s' "$out" | sed -n 's/^veredicto : //p' | head -1)"
    if [ "$got" != "$caso" ] || { [ "$rc" -eq 1 ] && [ "$ver" != "FALLO" ]; }; then
        rotas_detectadas=$((rotas_detectadas+1))
    fi
done

if [ "$rotas_detectadas" -eq 5 ]; then
    ok "las 5 clasificaciones rotas producen una relacion distinta: las aserciones muerden"
elif [ "$rotas_detectadas" -eq 0 ]; then
    ko "el checker roto reproduce las mismas salidas: las aserciones NO tienen dientes"
else
    ko "solo $rotas_detectadas de 5 clasificaciones rotas se notaron (se esperaban 5)"
fi

echo
echo "== $PASS checks, $FAIL fallos =="
[ "$FAIL" -eq 0 ] || { echo "RESULT: FAIL"; exit 1; }
echo "RESULT: PASS — el juez distingue los siete estados y sus aserciones tienen dientes"
