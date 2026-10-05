#!/usr/bin/env bash
# Falsador de la fidelidad del codigo de salida de los guards (session-84).
#
# PROPIEDAD que este falsador mide: el codigo de salida de un guard DESCRIBE lo
# que el guard midio, y no como termino su limpieza.
#
# MEDIDO, con `probe-trap-exit5.sh` (PASS=3 FAIL=0): con `set -euo pipefail` y
# `trap 'rm -rf "$DIR"' EXIT`, un fallo del borrado ABORTA el script con 1. Ese
# 1 es indistinguible de "el guard fallo": un cuerpo-verde y un cuerpo-rojo
# salen los dos con 1. En el 1b del release eso significa que un guard que
# APROBO puede tumbar la release, y un guard que FALLO no explica por que.
#
# El defecto se midio EN VIVO sobre el sujeto: `uat_ctx_001_adoption_convergence.sh`
# imprimia `PASS: 20/20 applies complete` y salia 1, con `mavis-trash: failed to
# trash` como ultima linea. MEDIDO el alcance en el bucle del 1b: 29 guards
# shell con `trap EXIT`; 3 con `set -e` y borrado en el trap; 2 expuestos y 1 ya
# bien escrito (`test_release_receipt_authority.sh`, que es el patron copiado).
#
# POR QUE un shim de `rm` por PATH y no una mutacion del fichero: el falsador
# NO puede editar el sujeto. MEDIDO por que: mutar `tests/` desde el 1b es la
# bomba de reloj de `5edcef00`, y el falsador de INC-DEBT-076 se delato a si
# mismo por exactamente eso. Ademas `sed` con delimitador `|` choca con el `||`
# del patron —MEDIDO: `sed: opción desconocida para 's'`, rc=1— luego una
# mutacion por `sed` de este patron es silenciosamente INAPLICABLE, que es la
# clase de defecto que este mismo falsador existe para cazar. El shim mide el
# mecanismo real sin reescribir una sola linea del sujeto.
#
# MEDIDO: `release-receipt.sh` (el helper que ejecuta el guard de lockstep) no
# usa `rm` en su cuerpo, luego el shim solo intercepta el borrado del trap. No
# se afirma que el shim sea invisible para el resto del guard: se mide que el
# codigo de salida no cambia, que es la propiedad, y el resto es ruido
# compartido entre las dos ramas que se comparan.
set -uo pipefail

PASS=0
FAIL=0
SKIP=0
ok()   { printf '  [ok]   %s\n' "$1"; PASS=$((PASS+1)); }
bad()  { printf '  [FAIL] %s\n' "$1"; FAIL=$((FAIL+1)); }
skip() { printf '  [SKIP] %s\n' "$1"; SKIP=$((SKIP+1)); }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 2

GUARDS="tests/test_authority_helper_lockstep.sh tests/test_vault_coherence_alignment.sh"

# El shim: un `rm` que SIEMPRE falla. No se toca el sujeto; solo se le antepone
# un directorio al PATH para el proceso del guard.
SHIM_DIR="$(mktemp -d)"
CTRL_DIR="$(mktemp -d)"
# MEDIDO: un segundo `trap ... EXIT` SOBREESCRIBE al primero, luego declararlos
# por separado dejaba `$SHIM_DIR` sin limpiar cada vez que se llegaba al bloque
# D. Es la misma clase que el defecto que este falsador mide —un recurso de
# salida que no se sostiene— y se evita con UN solo trap que limpia las dos
# cosas y preserva el codigo de salida.
MUT_DIR=""
cleanup_falsador() {
    local exit_code=$?
    if [ -n "$MUT_DIR" ] && [ -d "$MUT_DIR" ]; then
        find "$MUT_DIR" -type f -delete 2>/dev/null
        rmdir "$MUT_DIR" 2>/dev/null
    fi
    [ -n "$SHIM_DIR" ] && [ -d "$SHIM_DIR" ] && rm -rf "$SHIM_DIR" >/dev/null 2>&1
    [ -n "$CTRL_DIR" ] && [ -d "$CTRL_DIR" ] && rm -rf "$CTRL_DIR" >/dev/null 2>&1
    exit "$exit_code"
}
# shellcheck disable=SC2329
trap cleanup_falsador EXIT

printf '%s\n' '#!/usr/bin/env bash' 'echo "rm: no se puede borrar (shim del falsador)" >&2' 'exit 1' \
    > "$SHIM_DIR/rm"
chmod +x "$SHIM_DIR/rm"

# Un `rm` que SIEMPRE funciona, para el grupo de control: si el shim de arriba no
# cambia nada, es que no esta en el PATH y el bloque C no mide nada.
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' > "$CTRL_DIR/rm"
chmod +x "$CTRL_DIR/rm"

# Verifica que el mecanismo de shim funciona ANTES de atribuirle una conclusion.
# Sin esta comprobacion, un PATH mal construido daria "el codigo no cambia" y
# el falsador lo leeria como la propiedad. MEDIDO: es el fallo del detector que
# Increpitaba de rojo a `test_release_receipt_authority.sh` en el detector
# anterior — un instrumento que no demuestra que mide, no mide.
SHIM_WORKS=0
PROBE_DIR="$(mktemp -d)"
if PATH="$SHIM_DIR:$PATH" rm -rf "$PROBE_DIR" >/dev/null 2>&1; then
    SHIM_WORKS=0
else
    SHIM_WORKS=1
fi
[ -d "$PROBE_DIR" ] && rm -rf "$PROBE_DIR" >/dev/null 2>&1

echo "  0) el shim de rm esta en el PATH y falla de verdad (control del instrumento)"
if [ "$SHIM_WORKS" -eq 1 ]; then
    ok "rm por PATH falla: el shim esta en el PATH y el bloque C mide algo"
else
    bad "rm por PATH NO falla: el shim no esta en el PATH y el bloque C no mide nada"
fi
echo

echo "  A) el codigo de salida coincide con el veredicto que el guard IMPRIME"
# MEDIDO: cada guard declara su veredicto con su propia forma, y adivinarlo por
# una unica regex es como el detector anterior acuso de rojo a un guard que
# fallaba legitimamente. Se mide cada guard contra SU convencion:
#   lockstep imprime `FAIL: ...` en lineas propias y sale 1
#   vault    imprime `RESULT: FAIL|FAIL=n` y sale 1
declare -A PATRON=(
    [tests/test_authority_helper_lockstep.sh]='^FAIL:'
    [tests/test_vault_coherence_alignment.sh]='^RESULT: FAIL|FAIL=[1-9]'
)
# La marca de que el guard llego a SU FINAL y no se murio antes. MEDIDO: sin
# esta comprobacion, el bloque C decia `[ok] rc 127 con y sin borrado roto` —
# o sea "el codigo no depende de la limpieza" — para un guard que no habia
# arrancado. Comparar dos no-arranques produce codigos identicos y el bloque
# lo leia como la propiedad. Es la clase de «el instrumento pasa porque no mide»
# que este falsador existe para cazar, y se cazo a si mismo.
declare -A LLEGO=(
    [tests/test_authority_helper_lockstep.sh]='ALL CROSS-CRATE PIN TESTS PASSED'
    [tests/test_vault_coherence_alignment.sh]='^RESULT: PASS'
)
for g in $GUARDS; do
    out="$(PATH="$CTRL_DIR:$PATH" bash "$g" 2>&1)"; rc=$?
    re="${PATRON[$g]}"
    imprime_fall="$(printf '%s' "$out" | grep -cE "$re")"
    if [ "$rc" -eq 0 ] && [ "$imprime_fall" -eq 0 ]; then
        ok "$g: imprime sin fallos y sale 0"
    elif [ "$rc" -ne 0 ] && [ "$imprime_fall" -gt 0 ]; then
        ok "$g: imprime un fallo y sale $rc (codigo y veredicto coinciden)"
    else
        bad "$g: imprime_fall=$imprime_fall y sale $rc — el codigo no describe el veredicto"
    fi
done
echo

echo "  B) el trap captura \$?, tolera el fallo del borrado y sale con el codigo real"
# SC2016, MEDIDO: shellcheck pide comillas dobles porque ve un `$` entre
# comillas simples. Aqui el `$` tiene que ser LITERAL: se busca la cadena
# `exit "$exit_code"` TAL COMO esta escrita en el fichero ajeno, y con comillas
# dobles se expandiria contra el shell de este falsador y no encontraria nada.
# El aviso es correcto sobre el shell y equivocado sobre la intencion, que es
# leer texto, no evaluarlo.
# shellcheck disable=SC2016
for g in $GUARDS; do
    # MEDIDO: se lee el fichero SIN comentarios. Sin este filtro, el bloque B
    # daba rojo (`trap_antiguo=1`) porque el comentario que explica el defecto
    # cita el `trap` antiguo — y la cita es la linea 61, prosa. Es
    # «MENCIONAR no es EJECUTAR», la misma clase que INC-DEBT-076 y
    # INC-DEBT-077, cometida por TERCERA vez y esta dentro del falsador que
    # existe para cazar esa clase. Un detector que cuenta prosa como codigo
    # obliga a degradar el codigo para que el detector quede verde, que es al
    # reves: el comentario que documenta el arreglo es lo que hay que poder
    # escribir. MEDIDO: con el filtro, `trap_antiguo=0` en los dos guards.
    src_codigo="$(grep -vE '^[[:space:]]*#' "$g")"
    captura="$(printf '%s' "$src_codigo" | grep -cE 'local exit_code=\$\?')"
    tolera="$(printf '%s' "$src_codigo" | grep -cE '\|\| true')"
    sale="$(printf '%s' "$src_codigo" | grep -cE 'exit "\$exit_code"')"
    trap_antiguo="$(printf '%s' "$src_codigo" | grep -cE "^trap 'rm -rf .*' EXIT")"
    if [ "$captura" -ge 1 ] && [ "$tolera" -ge 1 ] && [ "$sale" -ge 1 ] && [ "$trap_antiguo" -eq 0 ]; then
        ok "$g: captura=\$? tolera el borrado y sale con el codigo real; sin trap antiguo"
    else
        bad "$g: captura=$captura tolera=$tolera sale=$sale trap_antiguo=$trap_antiguo"
    fi
done
echo

echo "  C) con el borrado roto, el codigo de salida NO cambia (la propiedad)"
# El grupo de control usa un `rm` que SIEMPRE sale 0, luego la unica diferencia
# entre las dos ramas es si el borrado falla. Si el codigo fuera el mismo, el
# codigo depende de la limpieza.
#
# MEDIDO: este bloque tambien exige que el guard LLEGUE a su final en la rama
# de control. Sin esa exigencia daba `[ok] rc 127 con y sin borrado roto` para
# un guard que no habia arrancado — dos no-arranques dan el mismo codigo y el
# bloque lo leia como la propiedad. Un bloque que no puede distinguir «el
# veredicto se conservo» de «nada llego a ejecutarse» no mide la propiedad.
for g in $GUARDS; do
    out_ctrl="$(PATH="$CTRL_DIR:$PATH" bash "$g" 2>&1)"; rc_ctrl=$?
    out_shim="$(PATH="$SHIM_DIR:$PATH" bash "$g" 2>&1)"; rc_shim=$?
    veredicto_ctrl="$(printf '%s' "$out_ctrl" | grep -cE "${PATRON[$g]}")"
    veredicto_shim="$(printf '%s' "$out_shim" | grep -cE "${PATRON[$g]}")"
    llego_ctrl="$(printf '%s' "$out_ctrl" | grep -cE "${LLEGO[$g]}")"
    if [ "$llego_ctrl" -eq 0 ]; then
        bad "$g: la rama de control no llego al final del guard (rc=$rc_ctrl) — comparar dos no-arranques no mide la propiedad"
    elif [ "$rc_ctrl" != "$rc_shim" ]; then
        bad "$g: rc $rc_ctrl con borrado bueno y $rc_shim con borrado roto — el codigo depende de la limpieza"
    elif [ "$veredicto_ctrl" != "$veredicto_shim" ]; then
        bad "$g: el rc no cambia pero el veredicto impreso SI ($veredicto_ctrl -> $veredicto_shim)"
    else
        ok "$g: rc $rc_ctrl con y sin borrado roto, y el veredicto impreso es el mismo"
    fi
done
echo

echo "  D) un guard con el trap ANTIGUO (mutado en copia) SI cae: el falsador tiene dientes"
# La copia vive en un temporal y su ruta se reescribe para que SCRIPT_DIR
# apunte al repo: los guards derivan REPO_ROOT de BASH_SOURCE, asi que una copia
# en /tmp mediria "fichero ausente", no el trap. Por eso la copia se COLOCA en
# un directorio bajo el repo y se borra al terminar.
MUT_DIR="$REPO_ROOT/.falsador-traps-$$"
mkdir -p "$MUT_DIR" || { bad "no se pudo crear $MUT_DIR"; echo "  PASS=$PASS FAIL=$FAIL SKIP=$SKIP"; exit 1; }

for g in $GUARDS; do
    base="$(basename "$g")"
    mutado="$MUT_DIR/$base"
    # El trap antiguo: sin preservar \$?, sin tolerar el fallo del borrado.
    sed -e "s|^trap cleanup_work_dir EXIT$|trap 'rm -rf \"\\\$WORK_DIR\"' EXIT|" \
        -e "s|^trap cleanup_fixture_dir EXIT$|trap 'rm -rf \"\\\$FIXTURE_DIR\"' EXIT|" \
        "$g" > "$mutado" 2>/dev/null
    chmod +x "$mutado" 2>/dev/null

    if ! grep -qE "^trap 'rm -rf" "$mutado" 2>/dev/null; then
        skip "$base: la mutacion NO se aplico, asi que D no mide nada (SKIP, nunca PASS)"
        continue
    fi

    out_ctrl="$(PATH="$CTRL_DIR:$PATH" bash "$mutado" 2>&1)"; rc_ctrl=$?
    out_shim="$(PATH="$SHIM_DIR:$PATH" bash "$mutado" 2>&1)"; rc_shim=$?
    veredicto_ctrl="$(printf '%s' "$out_ctrl" | grep -cE "${PATRON[$g]}")"
    veredicto_shim="$(printf '%s' "$out_shim" | grep -cE "${PATRON[$g]}")"

    if [ "$rc_ctrl" != "$rc_shim" ]; then
        ok "$base: mutado, el codigo cambia con el borrado ($rc_ctrl -> $rc_shim) — el defecto es real y el arreglo lo quita"
    elif [ "$veredicto_ctrl" -eq 0 ] && [ "$veredicto_shim" -eq 0 ]; then
        bad "$base: mutado, el codigo NO cambio con el borrado roto — el falsador no tiene dientes"
    else
        ok "$base: mutado, rc identico pero el veredicto se conserva en ambas ($rc_shim)"
    fi
done
echo

echo "  PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[ "$FAIL" -eq 0 ] || exit 1
