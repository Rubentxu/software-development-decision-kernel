#!/usr/bin/env bash
# test_push_prevention_coherence_mutation.sh — autofalsador del veto de
# coherencia de version (INC-DEBT-068)
#
# QUE PREGUNTA
#
# El veto de `githooks/pre-push` puede estar en el fichero y no hacer nada.
# Un `if` que nunca se cumple, una comparacion invertida, un `continue`
# donde deberia haber un `report+=`: el guard da verde igual. Este fichero
# quita cada diente UNO A UNO y exige que el veredicto cambie. Un diente
# que sobrevive a su mutacion no es un diente.
#
# POR QUE EJECUTA ESTA MATRIX Y NO UNA COPIA
# El falsador corre `tests/test_push_prevention_hook.sh` entero contra un
# hook MUTADO, con `SDDK_PREPUSH_HOOKS_DIR` apuntando a la arena. Los
# casos no se reescriben aqui: un falsador con sus propios casos probaria
# sus propios casos, que es el defecto que INC-DEBT-074 cerro (un guard
# que ejecuta una copia pegada del codigo no vigila el codigo).
#
# POR QUE UNA MUTACION NO APLICADA NUNCA CUENTA COMO DETECCION
# Si el sed no encuentra su ancla el hook queda intacto, la matrix sigue
# verde, y un falsador asi "detecta" una mutacion que no ocurrio. Cada
# sustitucion exige exactamente una ocurrencia y se comprueba con sha256
# antes de contar nada.
#
# EL TRAP RESTAURA Y LO COMPRUEBA
# Nueve de las mutaciones de otro falsador de este repo tocaban el
# fichero real y su trampa solo borraba el backup. Aqui el hook real no se
# muta nunca (solo se mutan copias en la arena), y aun asi la trampa
# restaura y verifica el sha, porque una trampa que restaura "en theory"
# envenena la suite siguiente.
#
# ── UN RESULTADO QUE NO ERA EL ESPERADO, Y SE QUEDA ──────────────────────
#
# La rama `if [[ -z "$declared" ]]` —el carrier presente que no declara
# version— resulto ser DIAGNOSTICA, no decisoria. Cuando `Cargo.toml` ya
# fijo `first`, la rama siguiente (`elif declared != first`) reporta el
# mismo archivo por la misma discrepancia. Es decir: el caso
# "BUNDLE.toml sin clave version" pasaba en verde por la rama de
# discrepancia, NO por la rama fail-closed. Un caso verde por la razon
# equivocada, que es el mismo patron que este repo ha encontrado cuatro
# veces y que no se puede dejar sin decir.
#
# M2 lo mide y lo dice, en vez de buscarle una mutacion que no existe:
# quita la rama y exige que el veredicto NO cambie. Si un dia el veto
# admitiera una combinacion en la que el archivo ilegible fuera el unico
# carrier, M2 caeria y habria que revisar aqui. Hoy no cae, y por eso no
# se cuenta como diente: se cuenta como lo que es.

# shellcheck disable=SC2329  # restore() is invoked indirectly, by the trap
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOOK="$ROOT/githooks/pre-push"
MATRIX="$ROOT/tests/test_push_prevention_hook.sh"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "== test_push_prevention_coherence_mutation.sh (INC-DEBT-068) =="

for f in "$HOOK" "$MATRIX"; do
    if [ ! -f "$f" ]; then
        echo "  [FAIL] falta $f — no hay veto que falsificar"
        echo
        echo "PASS=0 FAIL=1"
        exit 1
    fi
done

WORK="$(mktemp -d)"
BACKUP="$WORK/pre-push.orig"
cp "$HOOK" "$BACKUP"
ORIG_SHA="$(sha256sum "$HOOK" | cut -d' ' -f1)"

restore() {
    local code=$?
    if [ -f "$BACKUP" ]; then
        cp "$BACKUP" "$HOOK"
        local now
        now="$(sha256sum "$HOOK" | cut -d' ' -f1)"
        if [ "$now" != "$ORIG_SHA" ]; then
            echo "  [FAIL] el trap NO restauro el hook byte-identico" >&2
            code=1
        fi
    fi
    rm -rf "$WORK" >/dev/null 2>&1
    exit "$code"
}
trap restore EXIT INT TERM

arena_hooks() {
    local d="$1"
    mkdir -p "$d"
    cp "$BACKUP" "$d/pre-push"
    chmod +x "$d/pre-push"
}

# apply <dir> <nombre> <pares old/new...>
#
# Sustituciones LITERALES sobre la copia de la arena. Cada ancla tiene que
# aparecer EXACTAMENTE una vez, y el recuento se exige en el sitio: un
# ancla que aparece dos veces significaria que se esta mutando el sitio
# equivocado, y una que no aparece significa que no se muto nada.
apply() {
    local dir="$1" name="$2"
    shift 2
    python3 - "$dir/pre-push" "$@" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
args = sys.argv[2:]
if len(args) % 2:
    print("NOTAPPLIED pares impares")
    raise SystemExit(0)
s = p.read_text()
for i in range(0, len(args), 2):
    old, new = args[i], args[i + 1]
    n = s.count(old)
    if n != 1:
        print(f"NOTAPPLIED ancla#{i//2} occurrences={n}")
        raise SystemExit(0)
    s = s.replace(old, new)
p.write_text(s)
print("APPLIED")
PY
}

# matrix <hooks-dir> — la salida COMPLETA. Buscar el nombre de un caso en
# la linea de contadores no encuentra nada: la linea de contadores solo
# tiene numeros, y por eso una primera version de este falsador reporto
# "no cayo" en seis mutaciones que habian caído las seis.
matrix() {
    SDDK_PREPUSH_HOOKS_DIR="$1" bash "$MATRIX" 2>/dev/null
}

# expect_fall <nombre> <needle> <pares...>
# El caso `needle` tiene que aparecer como FAIL en la matrix mutada.
expect_fall() {
    local name="$1" needle="$2"
    shift 2
    local d
    d="$WORK/m$(printf '%s' "$name" | tr -cd '[:alnum:]')"
    arena_hooks "$d"
    local applied
    applied="$(apply "$d" "$name" "$@")"
    if [ "$applied" != "APPLIED" ]; then
        bad "$name — la mutacion NO se aplico ($applied); no cuenta como deteccion"
        return 0
    fi
    local out
    out="$(matrix "$d")"
    # Any FAIL line naming the case counts. A mutation that makes the hook
    # reject the COHERENT pre-phase shows up as "fixture error", not as
    # "expected REJECT, got ACCEPT" — and that is a detection, not a
    # different thing: the tooth was pulled either way. The first version
    # of this needle matched only the verdict wording and reported six
    # mutations as "no cayo" when all six had dropped the matrix.
    if printf '%s\n' "$out" | grep -E '^FAIL' | grep -qF "$needle"; then
        ok "$name"
    else
        bad "$name — la matrix no cayo en ese caso (dijo: $(printf '%s' "$out" | grep 'matrix result' | tail -1))"
    fi
}

# expect_no_fall <nombre> <pares...>
# El hook mutado tiene que seguir adjudicando IGUAL. Se usa para el
# resultado que no era el esperado: ahi la ausencia de caida es el
# hallazgo, y la funcion lo nombra para que no se lea como un fallo.
expect_no_fall() {
    local name="$1"
    shift 1
    local d
    d="$WORK/m$(printf '%s' "$name" | tr -cd '[:alnum:]')"
    arena_hooks "$d"
    local applied
    applied="$(apply "$d" "$name" "$@")"
    if [ "$applied" != "APPLIED" ]; then
        bad "$name — la mutacion NO se aplico ($applied)"
        return 0
    fi
    local out
    out="$(matrix "$d")"
    local res
    res="$(printf '%s' "$out" | grep 'matrix result' | tail -1)"
    if [ "$res" = "=== matrix result: PASS=55 FAIL=0 ===" ]; then
        ok "$name — y la matrix sigue verde, que es el hallazgo"
    else
        bad "$name — se esperaba la matrix verde y se obtuvo: $res"
    fi
}

# ── dientes ────────────────────────────────────────────────────────────────

# M1 — La comparacion de discrepancia, que es el diente central.
# `!=` -> `==` solo reporta cuando coinciden: todo estado partido pasa.
expect_fall \
    "M1 la comparacion de discrepancia se invierte" \
    "INC-DEBT-068: bump de Cargo.toml solo deja los otros dos atras" \
    'elif [[ "$declared" != "$first" ]]; then' \
    'elif [[ "$declared" == "$first" ]]; then'

# M2 — La rama del carrier ilegible. MEDIDO: no decide, y por eso no se
# cuenta como diente. Se documenta, no se disimula.
expect_no_fall \
    "M2 la rama del carrier ilegible diagnostica pero no decide" \
    'if [[ -z "$declared" ]]; then' \
    'if false; then'

# M3 — EL VETO ANTES DE LAS RUTAS. Es el diente que mas costa perder: si
# el veto vive al FINAL,detras de las rutas, cualquier rango que se admita
# por (A) o por (A-v2) nunca lo mira, y el veto queda siendo decoracion en
# los rangos que ya iban a caer.
#
# La mutacion mueve la llamada de sitio: la quita de delante y la pone
# delante del `exit 1` final del hook, que es donde se llega cuando NINGUNA
# ruta admitio. Una version anterior de esta mutacion la metia dentro del
# `if [[ -n "$VERSION_BUMP" ]]`, y eso no era M3: era M6 disfrazado, porque
# en un rango sin bump de Cargo.toml ese bloque no se ejecuta y la llamada
# quedaba tan inalcanzable como si no existiera. Lo que cae con esta es
# justamente lo que M3 afirma: los rangos partidos que (A) admite.
expect_fall \
    "M3 el veto se evalua al final, detras de las rutas de admision" \
    "INC-DEBT-068: bump de Cargo.toml solo deja los otros dos atras" \
    '    if ! version_coherence_violation "$local_sha"; then
        exit 1
    fi
' \
    '' \
    '    echo "ERROR: Push to main rejected — no real release contract found in range." >&2' \
    '    if ! version_coherence_violation "$local_sha"; then
        exit 1
    fi
    echo "ERROR: Push to main rejected — no real release contract found in range." >&2'

# M4 — La salida del veto: informa y deja pasar. Un hook que dice que no
# y luego empuja es peor que uno que no dice nada. El ancla incluye el
# printf que la precede porque `    return 1` seguido de `}` aparece DOS
# veces en el hook, y una sustitucion no unica mutaria el sitio
# equivocado — o peor, loaria como aplicada.
expect_fall \
    "M4 el veto informa pero no bloquea" \
    "INC-DEBT-068: bump de Cargo.toml solo deja los otros dos atras" \
    '    printf '"'"'ERROR: El bump se hace con scripts/release-bump.sh, que mueve los tres a la vez. Un bump a mano es lo que rompe esto.\n'"'"' >&2
    return 1' \
    '    printf '"'"'ERROR: El bump se hace con scripts/release-bump.sh, que mueve los tres a la vez. Un bump a mano es lo que rompe esto.\n'"'"' >&2
    return 0'

# M5 — La extraccion de la version. Si deja de leer la clave `version` y
# lee otra, los tres "declaran" lo mismo y el partido vuelve coherente de
# mentira. Se mide como caida del caso ACCEPT.
expect_fall \
    "M5 la extraccion de version deja de leer la clave" \
    "INC-DEBT-068: el bump coherente de los tres SE ADMITE" \
    "printf '%s\\n' \"\$1\" | sed -n 's/^version *= *\"\\([^\"]*\\)\".*/\\1/p' | head -1" \
    "printf '%s\\n' \"\$1\" | sed -n 's/^schema_version *= *\"\\([^\"]*\\)\".*/\\1/p' | head -1"

# M6 — La funcion veto existe pero nadie la llama. El caso mas tonto, y
# el mas parecido a "todo bien porque nadie la invoco".
expect_fall \
    "M6 la funcion veto no llega a invocar" \
    "INC-DEBT-068: bump de manifest.toml solo deja los otros dos atras" \
    'if ! version_coherence_violation "$local_sha"; then' \
    'if false; then'

# M7 — El veto tiene que mirar el TIP. Mirar el PRIMER commit del rango es
# reprobar un push por un estado partido que ya no existe: el caso "rango
# que parte y se realinea" deja el tip coherente y aun asi un veto asi lo
# rechaza. Se mide sobre ese caso, porque es el unico que distingue
# "mira el tip" de "mira el rango".
expect_fall \
    "M7 el veto inspecciona el primer commit del rango en vez del tip" \
    "INC-DEBT-068: rango que parte y se realinea, tip coherente, SE ADMITE" \
    'if ! version_coherence_violation "$local_sha"; then' \
    'if ! version_coherence_violation "$(printf "%s" "$COMMITS_TO_CHECK" | head -1)"; then'

echo
echo "RESULT: $PASS medidos como se esperaba, $FAIL no."
if [ "$FAIL" -ne 0 ]; then
    exit 1
fi
exit 0
