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

echo
echo "PASS=$PASS FAIL=$FAIL"
[[ "$FAIL" -eq 0 ]] || exit 1
echo "RESULT: PASS — una lectura parcial del remoto no puede pasar por una version publicada."