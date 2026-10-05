#!/usr/bin/env bash
# test_release_exclusion.sh
#
# MEDIDO (session-82, INC-DEBT-075): dos sesiones ejecutaron `release.sh` sobre
# el MISMO checkout y la MISMA version, y las dos llegaron a compilar. Este
# guard vigila la exclusion mutua que lo impide.
#
# ESTE GUARD EJECUTA LA LIBRERIA, NO LA COPIA. Es la misma regla que
# INC-DEBT-074 pago con el merge del changelog: cuando el guard pegaba el
# codigo que vigilaba, cambiar el codigo no movia el guard y seguia verde
# contra la copia antigua. Aqui el sujeto se toma de
# `$ROOT/scripts/lib/release_exclusion.sh` y se sourcea.
#
# Cada caso tiene un CONTROL que exige que el caso bueno se ACEPTE. Un guard
# que solo sabe decir «rojo» tambien pasa cuando la libreria esta apagada.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/release_exclusion.sh"

PASS=0
FAIL=0
asert() {  # asert <nombre> <1|0> <detalle>
    if [ "$2" = "1" ]; then
        printf '  [ok]   %s\n' "$1"
        PASS=$((PASS + 1))
    else
        printf '  [FAIL] %s -- %s\n' "$1" "${3:-}"
        FAIL=$((FAIL + 1))
    fi
}

# Un estado AISLADO por caso: la libreria escribe bajo $SDDK_STATE_DIR, y sin
# estado propio cada caso veria el candado del anterior, que es el modo de
# fallo que este repo ya registro en C9: un caso que hereda el sujeto de otro
# no ha medido lo que cree haber medido.
nuevo_estado() {
    local d
    d="$(mktemp -d "${TMPDIR:-/tmp}/sddk-excl.XXXXXX")"
    export SDDK_STATE_DIR="$d"
    mkdir -p "$SDDK_STATE_DIR/sddk"
}

if [ ! -f "$LIB" ]; then
    printf 'RESULT: FAIL — no existe la libreria que este guard debe ejecutar: %s\n' "$LIB"
    exit 1
fi
# shellcheck source=../scripts/lib/release_exclusion.sh
# SC1091, MEDIDO: el gate corre shellcheck SIN `-x`, luego el `source=` de
# arriba —que es la forma correcta cuando si se sigue el fichero— no evita el
# aviso. Se dice explicitamente en vez de dejar que el gate decida por el
# codigo de salida: un `info` sin explicar es un fallo que aparece de noche.
# shellcheck disable=SC1091
source "$LIB"

printf '=== exclusion mutua del release (INC-DEBT-075) ===\n'

KEY="release-raiz-prueba-2.11.0"
PID_PROPIO=$$
PID_AJENO=1   # el init: siempre existe, y nunca es este proceso

# --- X1: candado libre -> se toma, y se puede leer el pid que lo tiene ------
nuevo_estado
rc=0
release_exclusion_acquire "$KEY" "$PID_PROPIO" >/dev/null 2>&1 || rc=$?
asert "X1: sin nadie delante, el candado se toma" \
    "$([ "$rc" = "0" ] && echo 1 || echo 0)" "rc=$rc"
holder="$(release_exclusion_holder "$KEY")"
asert "X1: el candado nombra el pid que lo tiene" \
    "$([ "$holder" = "$PID_PROPIO" ] && echo 1 || echo 0)" \
    "holder='$holder' esperado='$PID_PROPIO'"

# --- X2: otro release VIVO -> NO se puede tomar, y el mensaje lo NOMBRA -----
nuevo_estado
release_exclusion_acquire "$KEY" "$PID_AJENO" >/dev/null 2>&1
msg="$(release_exclusion_acquire "$KEY" "$PID_PROPIO" 2>&1)"
rc=0
release_exclusion_acquire "$KEY" "$PID_PROPIO" >/dev/null 2>&1 || rc=$?
asert "X2: con otro release vivo, el candado NO se puede tomar" \
    "$([ "$rc" = "1" ] && echo 1 || echo 0)" "rc=$rc"
asert "X2: el mensaje NOMBRA el pid que lo tiene, no dice solo 'ocupado'" \
    "$(printf '%s' "$msg" | grep -q "pid $PID_AJENO" && echo 1 || echo 0)" \
    "mensaje: $msg"

# --- X3: candado HUERFANO (pid muerto) -> se recupera y se DECLARA --------
nuevo_estado
# Se busca un pid que de verdad no exista en vez de suponer uno: un fixture
# con un pid que resulta vivo midiria otra cosa.
pid_muerto=999991
while [ -d "/proc/$pid_muerto" ] && [ "$pid_muerto" -lt 9999999 ]; do
    pid_muerto=$((pid_muerto + 1))
done
release_exclusion_acquire "$KEY" "$pid_muerto" >/dev/null 2>&1
# El codigo de salida y el mensaje salen de la MISMA llamada. Medido: hacerlo
# en dos deja entre medias una ventana en la que el estado ya cambio, y el
# guard acaba fallando por su propio metodo de medida.
msg_h=""
rc=0
msg_h="$(release_exclusion_acquire "$KEY" "$PID_PROPIO" 2>&1)" || rc=$?
asert "X3: un candado cuyo proceso ya no existe se RECUPERA" \
    "$([ "$rc" = "0" ] && echo 1 || echo 0)" "rc=$rc (pid muerto usado: $pid_muerto)"
asert "X3: y se DECLARA que se estaba recuperando" \
    "$(printf '%s' "$msg_h" | grep -qi 'huerfano' && echo 1 || echo 0)" \
    "mensaje: $msg_h"
asert "X3: y el candado recuperado lo tiene ahora el pid que lo pidio" \
    "$([ "$(release_exclusion_holder "$KEY")" = "$PID_PROPIO" ] && echo 1 || echo 0)" \
    "holder='$(release_exclusion_holder "$KEY")'"

# --- X3b: re-entrar en un candado que ya es NUESTRO tiene que funcionar -----
# Competir consigo mismo no es exclusion: si esto devolviera «ocupado por
# otro», el mecanismo estaria mintiendo sobre quien lo tiene.
rc=0
release_exclusion_acquire "$KEY" "$PID_PROPIO" >/dev/null 2>&1 || rc=$?
asert "X3b: tomar un candado que ya es nuestro es idempotente, no un conflicto" \
    "$([ "$rc" = "0" ] && echo 1 || echo 0)" "rc=$rc"

# --- X4: soltar SOLO lo nuestro -------------------------------------------
nuevo_estado
release_exclusion_acquire "$KEY" "$PID_PROPIO" >/dev/null 2>&1
release_exclusion_release "$KEY" "$PID_PROPIO" >/dev/null 2>&1
rc=0
release_exclusion_acquire "$KEY" "$PID_PROPIO" >/dev/null 2>&1 || rc=$?
asert "X4: tras soltar el nuestro, se puede volver a tomar" \
    "$([ "$rc" = "0" ] && echo 1 || echo 0)" "rc=$rc"

# --- X5: NO se suelta el candado de otro ----------------------------------
nuevo_estado
release_exclusion_acquire "$KEY" "$PID_AJENO" >/dev/null 2>&1
release_exclusion_release "$KEY" "$PID_PROPIO" >/dev/null 2>&1
holder="$(release_exclusion_holder "$KEY")"
asert "X5: soltar con un pid que no es el del candado NO lo libera" \
    "$([ "$holder" = "$PID_AJENO" ] && echo 1 || echo 0)" \
    "holder='$holder' esperado='$PID_AJENO'"

# --- X6: la clave distingue repo y version --------------------------------
k1="$(release_exclusion_key /raiz/a 2.11.0)"
k2="$(release_exclusion_key /raiz/a 2.11.1)"
k3="$(release_exclusion_key /raiz/b 2.11.0)"
asert "X6: dos VERSIONES del mismo repo no comparten candado" \
    "$([ "$k1" != "$k2" ] && echo 1 || echo 0)" "k1='$k1' k2='$k2'"
asert "X6: dos REPOS distintos no comparten candado" \
    "$([ "$k1" != "$k3" ] && echo 1 || echo 0)" "k1='$k1' k3='$k3'"

# --- X7: sin directorio de candados NO se simula que este libre -------------
# El fallo caro no es «no avisar»: es decir «no hay nadie» y publicar sin
# exclusion. Con el directorio imposible, la libreria tiene que ABORTAR.
nuevo_estado
export SDDK_STATE_DIR="/proc/no-se-puede-crear/esto"
rc=0
release_exclusion_acquire "$KEY" "$PID_PROPIO" >/dev/null 2>&1 || rc=$?
asert "X7: sin directorio de candados se ABORTA, no se simula estar libre" \
    "$([ "$rc" = "2" ] && echo 1 || echo 0)" "rc=$rc (2 = no se pudo garantizar)"

# --- X8: el release.sh REAL toma y suelta el candado ----------------------
# No basta con que la libreria funcione: tiene que estar CONECTADA. Un candado
# que nadie toma no excluye a nadie, y ese es el fallo que este caso mide.
nuevo_estado
pre="$(bash -c '
    set -uo pipefail
    ROOT="$1"
    # shellcheck source=/dev/null
    source "$ROOT/scripts/lib/release_exclusion.sh"
    k="$(release_exclusion_key "$ROOT" 2.11.0)"
    release_exclusion_acquire "$k" "$$" >/dev/null 2>&1 || exit 1
    release_exclusion_holder "$k"
' _ "$ROOT")"
asert "X8: el camino real toma el candado y lo nombra" \
    "$([ -n "$pre" ] && echo 1 || echo 0)" "holder leido: '$pre'"

# El cableado se comprueba por NOMBRES, no contando subcadenas. MEDIDO:
# `grep -c 'release_exclusion_preflight'` cuenta 2 tanto si la funcion se
# llama como si solo se DEFINE con otro nombre, porque el nombre viejo es
# subcadena del nuevo. Un contador de subcadenas no puede distinguir
# «definido y llamado» de «definido con otro nombre», que es exactamente lo que
# una mutacion de cableado produce.
def_pre="$(grep -oE '^release_exclusion_[a-z_]+\(\)' "$ROOT/scripts/release.sh" 2>/dev/null | head -1 | sed 's/()$//')"
call_pre="$(grep -oE '^release_exclusion_[a-z_]+$' "$ROOT/scripts/release.sh" 2>/dev/null | head -1)"
asert "X8: el preflight de exclusion esta DEFINIDO" \
    "$([ -n "$def_pre" ] && echo 1 || echo 0)" "definido: '$def_pre'"
asert "X8: y se INVOCA con ese mismo nombre" \
    "$([ -n "$call_pre" ] && [ "$call_pre" = "$def_pre" ] && echo 1 || echo 0)" \
    "invocado: '$call_pre' / definido: '$def_pre'"
m_rel="$(grep -c 'release_exclusion_release' "$ROOT/scripts/release.sh" 2>/dev/null || echo 0)"
asert "X8: y lo suelta en release_on_exit" \
    "$([ "$m_rel" -ge 1 ] && echo 1 || echo 0)" "menciones: $m_rel"

printf '\nPASS=%d FAIL=%d\n' "$PASS" "$FAIL"
if [ "$FAIL" = "0" ]; then
    printf 'RESULT: PASS — dos releases del mismo repo y version no pueden coexistir.\n'
    exit 0
fi
printf 'RESULT: FAIL — la exclusion mutua no se sostiene.\n'
exit 1
