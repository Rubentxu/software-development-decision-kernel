#!/usr/bin/env bash
# Contract test: the published-version authority must not accept a PARTIAL
# read of the remote.
#
# ── El defecto que este guard existe para cazar ──────────────────────────────
#
# MEDIDO contra `origin` real en session-90: `git ls-remote --tags` devuelve de
# forma INTERMITENTE una lista parcial y AUN ASI sale con codigo 0. Captura:
# 249 lineas donde el mismo comando devolvia 386, con la lista cortada
# perdiendo v2.10.0, v2.11.x, v2.12.x y v2.13.0. El maximo caia de v2.13.0 a
# v2.9.1 y nada lo indicaba.
#
# `rc -eq 0` es exactamente lo que devuelve la respuesta incompleta, luego
# comprobar el codigo de salida —que es lo que hacia la version anterior— no
# puede atraparlo. Se atrapa leyendo DOS VECES y exigiendo que coincidan.
#
# POR QUE ES GRAVE Y NO UNA MOLESTIA DE RENDIMIENTO: la direccion del error.
# Falsificado sobre el producto real, con un remoto truncado de verdad:
#
#   remoto completo  -> REJECT already-published 2.13.0       (correcto)
#   remoto truncado  -> ACCEPT last-publish=2.9.1 -> 2.13.0   (permite
#                                                              republicar)
#
# Un baseline mas viejo hace que CUALQUIER version ya publicada parezca mayor.
# El fallo de red se convierte en permiso de publicar.
#
# HERMETICO: remoto local falso, cero red. El shim de `git` solo intercepta
# `ls-remote` y delega el resto, luego el resto del contrato sigue ejecutandose
# con el git real.
#
# shellcheck disable=SC2329  # funciones invocadas indirectamente (por nombre)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB="$REPO_ROOT/scripts/lib/release_admission.sh"

if [[ ! -f "$LIB" ]]; then
    echo "FAIL: $LIB missing"
    exit 1
fi

TMPROOT="$(mktemp -d)"
cleanup() {
    local code=$?
    chmod -R u+rw "$TMPROOT" 2>/dev/null || true
    rm -rf "$TMPROOT"
    exit "$code"
}
trap cleanup EXIT

PASS=0
FAIL=0
ok()   { PASS=$((PASS + 1)); echo "  [ok]   $1"; }
bad()  { FAIL=$((FAIL + 1)); echo "  [FAIL] $1"; }

check() { # check <descripcion> <esperado> <medido>
    if [[ "$2" == "$3" ]]; then ok "$1"; else bad "$1"; echo "         esperado: $2"; echo "         medido:   $3"; fi
}

# ── remoto falso, local y determinista ───────────────────────────────────────
FAKE="$TMPROOT/remote.git"
git init -q --bare "$FAKE"
for t in v2.8.0 v2.9.0 v2.9.1 v2.10.0 v2.11.0; do
    # --no-verify: el pre-push hook gobierna el push al remoto REAL. Este es un
    # remoto desechable de un fixture, y correrlo aqui solo anadiria trabajo y
    # directorios temporales sin vigilar nada de este contrato.
    git push -q --no-verify "$FAKE" "refs/tags/$t:refs/tags/$t" 2>/dev/null
done
EXPECTED_MAX="2.11.0"
# El numero de tags semver que el remoto de fixture expone. Lo cuenta el
# propio remoto y no una constante escrita a mano: un numero puesto a mano
# que se desincroniza del fixture haria que el caso "el conteo es lo que
# difiere" midiera una discrepancia inventada, y pasaria por deteccion.
EXPECTED_COUNT="$(git ls-remote --tags "$FAKE" 2>/dev/null \
    | awk '{print $2}' | grep -vF '^{}' \
    | sed -n 's|^refs/tags/v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$|\1|p' \
    | sort -u | grep -c .)"

# ── shim: `git ls-remote` que trunca, SOLO si existe el marcador ─────────────
#
# Por que un shim de PATH y no un remoto raro: el defecto es que DOS lecturas
# del MISMO remoto discrepen, y un repositorio local responde siempre igual.
# Un remoto que se comporta distinto cada vez no se puede construir sin
# interceptar la llamada, y lo que hay que interceptar es la lectura.
BIN="$TMPROOT/bin"
mkdir -p "$BIN"
cat > "$BIN/git" <<'SHIM'
#!/usr/bin/env bash
# Intercepta SOLO `ls-remote`. En las llamadas impares, y solo si existe el
# marcador, devuelve la lista SIN LOS TAGS MAS NUEVOS y sale con codigo 0.
#
# Dos decisiones que las medico el falso de esta misma primera version:
#
#  1. Se quitan los tags mas nuevos, no se recorta un porcentaje. La version
#     que recortaba al 60% NO FALSABA NADA: `ls-remote` ordena por refname y
#     v2.11.0 va antes que v2.8.0, luego el recorte conservaba el maximo y las
#     dos lecturas coincidian — el guard pasaba por el motivo equivocado, que
#     es indistinguible de estar roto. Un falsador que no falsaba ocupa el
#     sitio del que si.
#  2. Se sale con codigo 0. Un corte que devolviera != 0 lo atraparia el
#     `rc` de la version anterior de la lib, y este test no mediria el
#     defecto sino el codigo de salida.
REAL_GIT="${SDDK_TEST_REAL_GIT:?test harness must say where the real git is}"
# `config --get remote.<n>.url`: la URL CRUDA, que es de donde sale el slug.
# `remote get-url` devuelve la expandida por `insteadOf`, y con eso el slug
# decia "no es github" de un remoto que si lo es.
if [[ "${1:-}" == "config" && "${2:-}" == "--get" && "${3:-}" == remote.*.url ]]; then
    # `SDDK_TEST_SLUG_LOCAL` devuelve una ruta local, para el caso de "el
    # remoto no es github". Tiene que ganar TAMBIEN aqui: el slug se resuelve
    # primero por `config --get`, luego un shim que ignorase la bandera en
    # esta rama resolveria el caso por la rama equivocada y pasaria.
    if [[ -n "${SDDK_TEST_SLUG_LOCAL:-}" ]]; then
        printf '%s\n' "${SDDK_RELEASE_ADMISSION_REMOTE:-$3}"
    else
        printf 'git@github.com:%s.git\n' "${SDDK_TEST_SLUG:-acme/widgets}"
    fi
    exit 0
fi
if [[ "${1:-}" == "remote" && "${2:-}" == "get-url" ]]; then
    # De aqui sale el owner/repo con el que se consulta la segunda fuente.
    # Sin esto el remoto de fixture —una ruta local— no es un repositorio de
    # github y el crosscheck se cerraria SIEMPRE: el guard verde por la razon
    # equivocada, que es indistinguible de estar roto. `SDDK_TEST_SLUG_LOCAL`
    # devuelve una ruta local, para el caso de "el remoto no es github".
    if [[ -n "${SDDK_TEST_SLUG_LOCAL:-}" ]]; then
        printf '%s\n' "${SDDK_RELEASE_ADMISSION_REMOTE:-$3}"
    else
        # `acme/widgets` y no `acme/widgets/sddk-fixture`: el slug son DOS
        # segmentos, owner/repo, y una version anterior de este shim colgo un
        # tercer segmento del nombre de fixture. El regex del slug lo
        # rechazaba, el crosscheck se cerraba siempre y el test quedaba rojo
        # por una URL mal formada, no por el defecto que dice medir.
        printf 'git@github.com:%s.git\n' "${SDDK_TEST_SLUG:-acme/widgets}"
    fi
    exit 0
fi
if [[ "${1:-}" == "ls-remote" && -f "$SDDK_TEST_TRUNCATE_MARKER" ]]; then
    COUNTER="$SDDK_TEST_TRUNCATE_MARKER.counter"
    n=0; [[ -f "$COUNTER" ]] && n="$(cat "$COUNTER")"
    n=$((n + 1)); printf '%s' "$n" > "$COUNTER"
    out="$("$REAL_GIT" "$@")"
    if (( n % 2 == 1 )); then
        printf '%s\n' "$out" | grep -vE 'refs/tags/v2\.1[01]\.[0-9]+$'
        exit 0          # <- el punto: sale 0 con la lista incompleta
    fi
    printf '%s\n' "$out"
    exit 0
fi
exec "$REAL_GIT" "$@"
SHIM
chmod +x "$BIN/git"

SDDK_TEST_REAL_GIT="$(command -v git)"
export SDDK_TEST_REAL_GIT
export SDDK_TEST_TRUNCATE_MARKER="$TMPROOT/truncate"
export SDDK_RELEASE_ADMISSION_REMOTE="$FAKE"
export PATH="$BIN:$PATH"

# ── shim: `gh`, la SEGUNDA AUTORIDAD ───────────────────────────────────────
#
# MEDIDO en session-91 contra el remoto real: el guard de dos lecturas daba
# PASS con la respuesta EQUIVOCADA el 30% de las veces (3 de 10 corridas
# devolvieron v2.9.1 con rc=0 y el tag mas nuevo, v2.14.0, en el remoto). No
# por casualidades: el orden de `ls-remote` varia entre lecturas, asi que una
# respuesta parcial de 250 de 387 refs puede dejar fuera los tags recientes sin
# que el codigo de salida lo diga.
#
# Este shim responde con la lista REAL del remoto de fixture, leidas con el
# git de verdad (`$SDDK_TEST_REAL_GIT`, no el shim truncar): si usara el shim,
# la segunda fuente se truncaria con la primera y el test no distinguiria "la
# segunda fuente discrepa" de "las dos dicen lo mismo a medias", que es
# precisamente el caso que hay que medir.
cat > "$BIN/gh" <<'SHIM'
#!/usr/bin/env bash
# Solo `gh api --paginate repos/<slug>/tags?per_page=100`. Cualquier otra
# invocacion sale con 64 para que un uso inesperado se vea en vez de
# devolver una lista vacia que pareceria un remoto sin tags.
spec=""
for a in "$@"; do
    case "$a" in repos/*/tags*) spec="$a" ;; esac
done
if [[ "${1:-}" != "api" || -z "$spec" ]]; then
    echo "gh shim: unexpected invocation: $*" >&2
    exit 64
fi
if [[ -n "${SDDK_TEST_API_FAIL:-}" ]]; then
    exit 1
fi
out="$("$SDDK_TEST_REAL_GIT" ls-remote --tags "$SDDK_RELEASE_ADMISSION_REMOTE" 2>/dev/null \
      | awk '{print $2}' \
      | grep -vF '^{}' \
      | sed -n 's|^refs/tags/\(v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$|\1|p')"
if [[ -n "${SDDK_TEST_API_MISMATCH:-}" ]]; then
    # Quita tags SIN mover el maximo: v2.11.0 sigue siendo el mayor. Un guard
    # que comparase solo maximos pasaria esto sin pestanear, y esa es la
    # diferencia que este shim existe para medir. Una version anterior quita
    # v2.11.0 (el maximo) y con eso se conformaba con medir lo de siempre.
    out="$(printf '%s\n' "$out" | grep -vE '^v2\.(9\.0|9\.1)$')"
fi
if [[ -n "${SDDK_TEST_API_NEWER:-}" ]]; then
    # El caso SIMETRICO: mismo numero de tags, uno MAS NUEVO. Cuenta igual,
    # maximo no. Una huella que solo mirase el conteo pasaria esto, que es lo
    # que hace falta para que el contrato no dependa de una sola mitad de la
    # huella: los dos casos tienen que caer, y cada uno hunde una mitad
    # distinta.
    out="$(printf '%s\n' "$out" | sed 's|^v2\.11\.0$|v2.99.0|')"
fi
printf '%s\n' "$out"
exit 0
SHIM
chmod +x "$BIN/gh"

# PATH sin el shim de `gh`, para el caso "la segunda fuente no esta".
BIN_NOGH="$TMPROOT/bin-nogh"
mkdir -p "$BIN_NOGH"
ln -s "$BIN/git" "$BIN_NOGH/git"

# shellcheck source=/dev/null
# shellcheck disable=SC1091
. "$LIB"

resolve() { # resolve -> imprime el outcome y PROPAGA el codigo de salida
    local rc
    _last_published_resolve
    rc=$?
    echo "$LAST_PUB_OUTCOME"
    # Sin esto el `rc=$?` del llamante lee el de `echo` (siempre 0) y el
    # test pasa por el motivo equivocado: el guard cerrando y el test
    # Acadcando un codigo que no era suyo. Es la misma clase que el v2 con
    # SIGPIPE de 922937c4 — un codigo de salida que no era del que se creia.
    return $rc
}

echo "== control: dos lecturas coinciden, se ACEPTA =="
rm -f "$SDDK_TEST_TRUNCATE_MARKER" "$SDDK_TEST_TRUNCATE_MARKER.counter"
out="$(resolve)"; rc=$?
check "el remoto completo resuelve su maximo real" "v$EXPECTED_MAX" "$out"
check "y sale con exito" "0" "$rc"

echo
echo "== el defecto: lecturas que discrepan, se CIERRA =="
rm -f "$SDDK_TEST_TRUNCATE_MARKER.counter"
: > "$SDDK_TEST_TRUNCATE_MARKER"
out="$(resolve)"; rc=$?
check "no acepta una lectura parcial" "1" "$rc"
case "$out" in
    query_inconsistent:*)
        ok "el motivo declara el desacuerdo y nombra las dos lecturas: $out"
        # El motivo tiene que distinguirse del fallo de red: un fallo de red
        # es "no se pudo leer"; aqui se leyo y las dos lecturas no coinciden.
        # Confundirlos seria canjear un veredicto preciso por uno vago.
        if [[ "$out" == *"|"* ]]; then
            ok "el motivo lleva las dos lecturas separadas"
        else
            bad "el motivo no dice QUE discrepa: $out"
        fi
        ;;
    *)
        bad "el motivo no distingue la inconsistencia del fallo de red: $out"
        ;;
esac
rm -f "$SDDK_TEST_TRUNCATE_MARKER" "$SDDK_TEST_TRUNCATE_MARKER.counter"

echo
echo "== regresion: un remoto sin tags sigue siendo bootstrap =="
EMPTY="$TMPROOT/empty.git"
git init -q --bare "$EMPTY"
SDDK_RELEASE_ADMISSION_REMOTE="$EMPTY" bash -c '
    . "'"$LIB"'"
    _last_published_resolve || exit 9
    [[ "$LAST_PUB_OUTCOME" == bootstrap ]] || exit 8
' && ok "el bootstrap legitimo no se degraded a error por la segunda lectura" \
      || bad "el bootstrap se rompio con la segunda lectura"

echo
echo "== regresion: el remoto inalcanzable sigue fallando cerrado =="
SDDK_RELEASE_ADMISSION_REMOTE="$TMPROOT/no-existe.git" bash -c '
    . "'"$LIB"'"
    if _last_published_resolve; then exit 9; fi
' && ok "un remoto que no responde se rechaza, no se degrada a local" \
      || bad "un remoto inalcanzable dejo pasar algo"

# ── LA SEGUNDA AUTORIDAD ───────────────────────────────────────────────────
#
# El caso de arriba era el que el guard de dos lecturas ya cazaba. Estos son los
# que NO podia cazar, y son los que mas pesan.

echo
echo "== la segunda fuente contradice a la primera, aunque el maximo NO se mueva =="
# Las DOS lecturas de ls-remote coinciden y dan el maximo real. Lo que miente
# es la API, y miente quitando tags intermedios: el maximo sigue siendo
# v2.11.0 en las dos fuentes. Comparar maximos — que es lo que hacia el guard
# anterior — daria verde. El conteo es lo que lo distingue.
out="$(SDDK_TEST_API_MISMATCH=1 resolve)"; rc=$?
check "no acepta una segunda fuente incompleta aunque el maximo coincida" "1" "$rc"
case "$out" in
    crosscheck_mismatch:*)
        ok "el motivo declara el cruce de fuentes: $out"
        if [[ "$out" == *"ls-remote="* && "$out" == *"api="* ]]; then
            ok "el motivo trae la huella de LAS DOS fuentes"
        else
            bad "el motivo no dice que discrepa: $out"
        fi
        # La razon de que el conteo este en el motivo: sin el, el lector tiene
        # que adivinar por que dos fuentes que dicen lo mismo se contradicen.
        if [[ "$out" == *"ls-remote=${EXPECTED_COUNT}|"* && "$out" == *"api=$((EXPECTED_COUNT - 2))|"* ]]; then
            ok "las huellas traen conteo y maximo, y el conteo es el que difiere"
        else
            bad "las huellas no permiten ver que solo difiere el conteo: $out"
        fi
        ;;
    *)
        bad "una segunda fuente incompleta no dio el motivo de cierre: $out"
        ;;
esac
unset SDDK_TEST_API_MISMATCH

echo
echo "== la segunda fuente trae un tag mas nuevo: el conteo no basta =="
# Simetrico del caso anterior y con el otro reparto: aqui el numero de tags es
# el mismo y lo que cambia es el maximo. Si la huella se quedara con el
# conteo, esto pasaria. Los dos casos juntos dicen que la huella usa las dos
# mitades, y no que una de ellas este de adorno.
out="$(SDDK_TEST_API_NEWER=1 resolve)"; rc=$?
check "no acepta un tag mas nuevo que el remoto no tiene" "1" "$rc"
case "$out" in
    crosscheck_mismatch:*)
        ok "el motivo declara el cruce de fuentes: $out"
        if [[ "$out" == *"ls-remote=${EXPECTED_COUNT}|"* ]]; then
            ok "el motivo muestra que el conteo es el mismo y lo que no es el maximo"
        else
            bad "el motivo no deja ver que el conteo coincidio: $out"
        fi
        if [[ "$out" == *"api=${EXPECTED_COUNT}|2.99.0"* ]]; then
            ok "el motivo declara el maximo que la segunda fuente trayo por suyo"
        else
            bad "el motivo no declara el maximo discrepante: $out"
        fi
        ;;
    *)
        bad "una segunda fuente con un tag mas nuevo dio el motivo equivocado: $out"
        ;;
esac
unset SDDK_TEST_API_NEWER

echo
echo "== la segunda fuente no esta: se cierra, no se degrada =="
# Un PATH minimo, no "$BIN_NOGH:$PATH": este empieza por el PATH del sistema y
# NO vuelve a anadir el del proceso. Con "$BIN_NOGH:$PATH" el `gh` de asdf
# seguia ahi, el caso pasaba verde con el `gh` de verdad y no midia nada.
out="$(PATH="$BIN_NOGH:/usr/bin:/bin" bash -c '
    . "'"$LIB"'"
    _last_published_resolve
    echo "$LAST_PUB_OUTCOME"
')"
case "$out" in
    crosscheck_unavailable:gh-not-in-path)
        # El motivo se imprime. Un caso que lo comprueba pero no lo enseña
        # deja al falsador sin nada que medir: la mutacion que lo rompe tiene
        # que hacer desaparecer una ficha del informe, y si el motivo nunca
        # aparece en el informe no hay ficha. Se vio construir asi.
        ok "sin gh el motivo dice exactamente que falta: $out"
        ;;
    *)
        bad "sin gh el motivo no nombra la ausencia de la segunda fuente: $out"
        ;;
esac
# El codigo de salida se lee del comando, no del `if` de dentro. Una primera
# version metia el `if _last_published_resolve; then exit 9; fi` dentro del
# bash -c y luego miraba el rc del bash -c: un `if` cuyo camino falso no se
# toma devuelve 0 SIEMPRE, luego "no resolvio" y "resolvio" se leian igual.
rc2=0
PATH="$BIN_NOGH:/usr/bin:/bin" bash -c '
    . "'"$LIB"'"
    _last_published_resolve
' >/dev/null || rc2=$?
check "sin la segunda fuente no se resuelve" "1" "$rc2"

echo
echo "== la segunda fuente no puede responder: se cierra, no se degrada =="
out="$(SDDK_TEST_API_FAIL=1 resolve)"; rc=$?
check "una API que falla da error de codigo" "1" "$rc"
case "$out" in
    crosscheck_failed:*)
        ok "el motivo distingue 'la API fallo' de 'la API discrepo': $out"
        ;;
    *)
        bad "una API caida dio el motivo equivocado: $out"
        ;;
esac
unset SDDK_TEST_API_FAIL

echo
echo "== un remoto que no es github no puede confirmarse: se cierra =="
out="$(SDDK_TEST_SLUG_LOCAL=1 SDDK_RELEASE_ADMISSION_REMOTE="$FAKE" bash -c '
    . "'"$LIB"'"
    _last_published_resolve
    echo "$LAST_PUB_OUTCOME"
')"
case "$out" in
    crosscheck_unavailable:remote-is-not-github:*)
        ok "un remoto no-github se declara, no se resuelve por su cuenta"
        ;;
    *)
        bad "un remoto no-github dio un motivo que no lo nombra: $out"
        ;;
esac

echo
echo "PASS=$PASS FAIL=$FAIL"
[[ "$FAIL" -eq 0 ]] || exit 1
echo "RESULT: PASS — una lectura parcial del remoto no puede pasar por una version publicada."
echo "         Ni aunque la segunda fuente conserve el maximo."