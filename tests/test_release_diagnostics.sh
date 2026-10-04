#!/usr/bin/env bash
# Que el release diga POR QUE se detuvo, y no solo QUE se detuvo.
#
# QUE SE ESTA PROBANDO
# --------------------
# `scripts/release.sh` tiene 73 `die` y 32 no nombran ni el codigo de salida ni
# una causa. El del paso 1 es `die "cargo test --workspace failed"`, y un ENOSPC
# y un test rojo producen el mismo texto. A eso se suman dos MEDIDOS: el paso 3
# estuvo 12 min 30 s esperando el lock de un `CARGO_TARGET_DIR` compartido sin
# que release.sh dijera nada (el aviso lo dio cargo, no el release), y `/tmp` es
# un tmpfs con tope cuya saturacion tumba la suite con `os error 122` sin que
# ningun mensaje nombre el disco (INC-DEBT-069).
#
# `scripts/lib/release_diagnostics.sh` pone una sola autoridad para las tres
# propiedades. Este test la ejerce con las manos; el falsador
# (`test_release_diagnostics_mutation.sh`) le quita los dientes uno a uno.
#
# LO QUE ESTE TEST NO COMPRUEBA, escrito para que no se lea al reves:
#   * no ejecuta `release.sh` entero, luego no prueba que el cableado este bien.
#     Eso lo comprueba `tests/test_release_diagnostics_wiring.sh`.
#   * `cause_of_exit_code` es una tabla, no una inferencia: solo se prueba que
#     los codigos que importan tienen NOMBRE, y que un codigo desconocido
#     declara que no lo tiene en vez de inventar uno.
#   * no mide retencion real de un `cargo` real: usa un ejecutable llamado
#     `cargo` de verdad, porque el criterio de deteccion es el NOMBRE DEL
#     EXECUTABLE. Lo que no se prueba aqui es que el cargo real de esta maquina
#     se llame asi, y eso es una pregunta para `ps`, no para un test.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/release_diagnostics.sh"

PASS=0
FAIL=0

# --- instrumentacion ---------------------------------------------------------

# asert <etiqueta> <0|1> [detalle]
# Registra el resultado de UNA asercion. Un caso con varias aserciones CAE si
# alguna falla: buscar un `[ok]` suelto leeria al reves un caso a medias caido,
# que es el fallo de instrumento mas caro que hay. Por eso el falsador busca el
# NOMBRE del caso (`[ok] C7`) y no el texto de una asercion suelta.
asert() {
    local label="$1" cond="$2" detail="${3:-}"
    if [ "$cond" = "1" ]; then
        printf '  [ok]   %s\n' "$label"
        PASS=$((PASS + 1))
    else
        printf '  [FAIL] %s%s\n' "$label" "${detail:+ -- $detail}"
        FAIL=$((FAIL + 1))
    fi
}

# caso_empieza / caso_termina — el marcador de caso NO es decorativo.
#
# MEDIDO, y por que importa: la primera version imprimia `[ok] C1` con un
# `printf` suelto DESPUES de sus aserciones, asi que el marcador salia igual con
# el caso entero verde o con tres aserciones caidas. El falsador busca el
# marcador para saber si un caso cae, luego con eso habria leido como VERDE un
# caso que la mutacion habia roto: la autofalsacion entera habria sido verde
# sobre una base que ya no lo era.
caso_empieza() { _DESDE_PASS=$PASS; _DESDE_FAIL=$FAIL; }
caso_termina() {
    if [ "$FAIL" -eq "$_DESDE_FAIL" ]; then
        printf '  [ok] %s\n' "$1"
    else
        printf '  [FAIL-CASO] %s (%s asercion(es) caidas)\n' "$1" "$((FAIL - _DESDE_FAIL))"
    fi
}

TMPROOT="$(mktemp -d "${TMPDIR:-/var/home/rubentxu/cargo-targets}/sddk-diagtest.XXXXXX")"
STUBS="$TMPROOT/stubs"
FAKE_CARGO_PIDS=()
mkdir -p "$STUBS"

# El marcador de "ya diagnostique" se FIJA AQUI, antes del primer caso, y al
# temporal de ESTE test. MEDIDO, y solo se ve dentro de un release:
#
#   * el release exporta `RELEASE_DIAGNOSED_FILE` apuntando a SU scratch;
#   * C4 corre `release_diagnose_exit 1` y, como el marcador aun no existe,
#     lo CREA;
#   * C5 corre `release_diagnose_exit 137`, el marcador ya existe, y la
#     idempotencia —que en produccion es exactamente lo correcto— hace que no
#     imprima nada. Tres aserciones cae, y solo dentro del release.
#
# Fuera del release el test pasa 4 de 4 porque `RELEASE_DIAGNOSED_FILE` no esta
# exportada y la guardia cae a la variable de shell, que un subshell reinicia.
# O sea: el defecto es invisible fuera del entorno que lo produce, que es la
# forma mas dificil de encontrar y la que mas dano hace.
#
# La leccion no es "el marcador esta mal" —esta bien, es lo que hace que el
# bloque del fallo salga una vez—. Es que **un caso no puede depender del
# estado que deja el caso anterior**, ni del estado que el entorno le impone.
# Cada caso parte de un entorno limpio, y eso hay que FIJARLO, no suponerlo.
RELEASE_DIAGNOSED_FILE="$TMPROOT/.diagnosed"
export RELEASE_DIAGNOSED_FILE
# Cada caso que llama a `release_diagnose_exit` arranca con el marcador BORRADO.
# Es la parte EJECUTABLE del principio de las lineas de arriba: la idempotencia
# es una propiedad de un PROCESO, no de un caso de test, asi que un caso no
# puede heredar el estado que dejo el anterior —ni el que le impone el
# entorno—. Fijarlo aqui es lo que hace que el caso sea el mismo dentro y
# fuera del release.
limpiar_marcador() { rm -f "$RELEASE_DIAGNOSED_FILE"; }

# Y el test arranca SIEMPRE con el marcador YA CREADO, que es el peor caso.
# No es una elegancia: es lo que hace que el defecto sea detectable sin tener
# que estar dentro de un release. MEDIDO: sin esta linea el test pasaba 4 de 4
# fuera del release y caia dentro, y el falsador —que corre fuera— no lo
# detectaba nunca. Un fixture que solo reproduce el fallo dentro del entorno
# que lo produce es un fixture a medias; este lo reproduce siempre.
: > "$RELEASE_DIAGNOSED_FILE"

# SC2329, MEDIDO: shellcheck dice que esta funcion no se invoca nunca y es
# FALSO -- la invoca el `trap` de la linea siguiente, y shellcheck no traza
# los `trap`. El aviso es `info`, pero `test_build_identity_policy.sh` corre
# ShellCheck sin filtro de severidad, luego para ese gate es un fallo, y eso
# detiene el release en el 1b. El fichero no existia en v2.8.1, luego la
# aparicion es de este bloque y no deuda heredada.
# shellcheck disable=SC2329
cleanup() {
    local pid
    for pid in "${FAKE_CARGO_PIDS[@]:-}"; do
        [ -n "$pid" ] && kill "$pid" 2>/dev/null
    done
    rm -rf "$TMPROOT"
}
trap cleanup EXIT

# shellcheck source=../scripts/lib/release_diagnostics.sh
# SC1091, MEDIDO: el gate `test_build_identity_policy.sh` corre shellcheck SIN
# `-x`, luego el `source=` de arriba —que es la forma correcta cuando si se
# sigue el fichero— no evita el aviso. Se dice explicitamente en vez de dejar
# que el gate decida por el codigo de salida: un `info` sin explicar es un
# fallo que aparece de noche.
# shellcheck disable=SC1091
source "$LIB"

# --- C0: control. Sin nada roto, el diagnostico calla y no inventa ------------
caso_empieza
RELEASE_DIAGNOSED=0
out="$(cause_of_exit_code 0)"
asert "C0: el codigo 0 se nombra exito, no 'sin causa'" \
    "$([[ "$out" == "exito" ]] && echo 1 || echo 0)" "obtenido: $out"

out="$(release_diagnose_exit 0 "contexto que no deberia aparecer" 2>&1)"
asert "C0: una salida 0 no imprime diagnostico" \
    "$([[ -z "$out" ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C0: no imprime texto de medicion" \
    "$([[ "$out" != *memoria* ]] && echo 1 || echo 0)"
caso_termina C0

# --- C1: 137 es una muerte, no un fallo de logica ----------------------------
caso_empieza
out="$(cause_of_exit_code 137)"
asert "C1: 137 nombra SIGKILL" "$([[ "$out" == *SIGKILL* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C1: 137 nombra el OOM killer" "$([[ "$out" == *OOM* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C1: 137 no se presenta como un fallo de logica" \
    "$([[ "$out" != *"fallo de logica"* && "$out" != *"sin causa"* ]] && echo 1 || echo 0)" \
    "obtenido: $out"
caso_termina C1

# --- C2: 122 nombra el disco, que es de donde sale --------------------------
caso_empieza
out="$(cause_of_exit_code 122)"
asert "C2: 122 nombra ENOSPC" "$([[ "$out" == *ENOSPC* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C2: 122 nombra 'no space left on device'" \
    "$([[ "$out" == *"no space left on device"* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C2: 122 no dice 'sin causa con nombre'" \
    "$([[ "$out" != *"sin causa"* ]] && echo 1 || echo 0)" "obtenido: $out"
caso_termina C2

# --- C3: un codigo sin nombre DECLARA que no lo tiene ------------------------
caso_empieza
out="$(cause_of_exit_code 42)"
asert "C3: un codigo desconocido declara que no tiene causa con nombre" \
    "$([[ "$out" == *"sin causa con nombre"* && "$out" == *"42"* ]] && echo 1 || echo 0)" \
    "obtenido: $out"
out="$(cause_of_exit_code 143)"
asert "C3: 143 (128+15, SIGTERM) se lee como senal, no como codigo opaco" \
    "$([[ "$out" == *"senal 15"* ]] && echo 1 || echo 0)" "obtenido: $out"
caso_termina C3

# --- C4: el diagnostico nombra el paso en curso -----------------------------
caso_empieza
limpiar_marcador
RELEASE_DIAGNOSED=0
RELEASE_CURRENT_STEP="1/15 - cargo fmt + clippy + test (workspace)"
RELEASE_SCRATCH="$TMPROOT"
out="$(release_diagnose_exit 1 "contexto de prueba" 2>&1)"
asert "C4: el diagnostico nombra el paso en curso" \
    "$([[ "$out" == *"1/15"* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C4: el diagnostico incluye el contexto que le pasaron" \
    "$([[ "$out" == *"contexto de prueba"* ]] && echo 1 || echo 0)"
asert "C4: el diagnostico no promete saber que test fallo" \
    "$([[ "$out" == *"NO dice cual test fallo"* ]] && echo 1 || echo 0)" "obtenido: $out"
caso_termina C4

# --- C5: el diagnostico imprime la medicion que respalda la causa ------------
caso_empieza
limpiar_marcador
export RELEASE_DIAGNOSED=0
RELEASE_CURRENT_STEP="paso medido"
out="$(release_diagnose_exit 137 2>&1)"
asert "C5: imprime la linea del scratch" \
    "$([[ "$out" == *"scratch $TMPROOT"* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C5: imprime la linea de /tmp, que es donde ocurre el ENOSPC" \
    "$([[ "$out" == *"/tmp"* ]] && echo 1 || echo 0)"
asert "C5: imprime la linea de memoria" \
    "$([[ "$out" == *"memoria"* ]] && echo 1 || echo 0)" "obtenido: $out"
caso_termina C5

# --- C6: la guardia de idempotencia sobrevive a los subshells ---------------
#
# El caso fuerte a proposito: `die` corre en este shell pero el manejador de
# salida vive en otro, y cualquier `$( ... )` en el medio crea un tercero. Con
# una variable de shell como guardia, el bloque saldria dos veces. Se mide desde
# subshells justamente para que eso sea lo que se ejercita.
caso_empieza
limpiar_marcador
RELEASE_DIAGNOSED_FILE="$TMPROOT/.diagnosed"
export RELEASE_DIAGNOSED_FILE
rm -f "$RELEASE_DIAGNOSED_FILE"
export RELEASE_CURRENT_STEP="paso A"
first="$(release_diagnose_exit 1 2>&1)"
second="$(release_diagnose_exit 1 2>&1)"
third="$(release_diagnose_exit 1 2>&1)"
asert "C6: la primera llamada imprime" \
    "$([[ -n "$first" ]] && echo 1 || echo 0)" "primera=${#first} bytes"
asert "C6: la segunda llamada no repite el bloque" \
    "$([[ -z "$second" ]] && echo 1 || echo 0)" "segunda=${#second} bytes"
asert "C6: ni la tercera, sea el numero que sea" \
    "$([[ -z "$third" ]] && echo 1 || echo 0)" "tercera=${#third} bytes"
caso_termina C6

# --- C7: preflight de disco, fail-closed, nombrando el recurso --------------
caso_empieza
export RELEASE_SCRATCH="$TMPROOT"
export SDDK_RELEASE_MIN_FREE_MB=1 SDDK_RELEASE_MIN_AVAIL_MB=1
if release_check_resources "$TMPROOT" >/dev/null 2>&1; then C7_RC=1; else C7_RC=0; fi
asert "C7: con margen de sobra devuelve 0" "$C7_RC"

# Un umbral que el scratch no puede cumplir, y un `df` que dice la cifra: la
# cifra del mensaje tiene que ser la que el stub devuelve, no una del script. Por
# eso el stub lee del entorno en vez de recibir positional: un heredoc sin
# comillas expandiria `$1` del propio test, que no existe.
cat > "$STUBS/df" <<'STUB'
#!/bin/sh
printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\n'
printf 'fake-scratch 999999999 12 %s 50%%%% %s\n' "$FAKE_FREE_MB" "$FAKE_MOUNT"
STUB
chmod +x "$STUBS/df"
out="$(PATH="$STUBS:$PATH" FAKE_FREE_MB=12 FAKE_MOUNT=/mnt/scratch-falso \
    SDDK_RELEASE_MIN_FREE_MB=4096 SDDK_RELEASE_MIN_AVAIL_MB=1 \
    release_check_resources "$TMPROOT" 2>&1)"
rc=$?
asert "C7: por debajo del margen devuelve 1" "$([ $rc -eq 1 ] && echo 1 || echo 0)" "rc=$rc"
asert "C7: nombra el recurso" "$([[ "$out" == *disco* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C7: nombra la cifra que dio df, no una constante" \
    "$([[ "$out" == *"12 MiB"* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C7: nombra el punto de montaje, para saber DONDE falta" \
    "$([[ "$out" == *"/mnt/scratch-falso"* ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C7: nombra el margen que exige" \
    "$([[ "$out" == *"4096 MiB"* ]] && echo 1 || echo 0)" "obtenido: $out"
rm -f "$STUBS/df"
caso_termina C7

# --- C8: el margen de memoria se mide, no se supone -------------------------
caso_empieza
cat > "$STUBS/free" <<'STUB'
#!/bin/sh
printf '              total        used        free      shared  buff/cache   available\n'
printf 'Mem:           98304       40000       1024        2048       40000         %s\n' "$FAKE_AVAIL_MB"
STUB
chmod +x "$STUBS/free"
out="$(PATH="$STUBS:$PATH" FAKE_AVAIL_MB=512 \
    SDDK_RELEASE_MIN_FREE_MB=1 SDDK_RELEASE_MIN_AVAIL_MB=4096 \
    release_check_resources "$TMPROOT" 2>&1)"
rc=$?
asert "C8: memoria por debajo del margen devuelve 1" "$([ $rc -eq 1 ] && echo 1 || echo 0)" "rc=$rc"
asert "C8: nombra la memoria y su cifra" \
    "$([[ "$out" == *memoria* && "$out" == *"512 MiB"* ]] && echo 1 || echo 0)" "obtenido: $out"
if out="$(PATH="$STUBS:$PATH" FAKE_AVAIL_MB=8192 \
    SDDK_RELEASE_MIN_FREE_MB=1 SDDK_RELEASE_MIN_AVAIL_MB=4096 \
    release_check_resources "$TMPROOT" 2>&1)"; then C8_RC=1; else C8_RC=0; fi
asert "C8: memoria suficiente devuelve 0" "$C8_RC"
# Disco y memoria son reglas DISTINTAS: basta con que una falle, y el mensaje
# tiene que senalar la que fallo, no la otra.
#
# MEDIDO: este sub-caso usaba el `df` REAL con un umbral de 999999 MiB, y por
# eso dependia de lo lleno que estuviera el disco de la maquina. Con 350 GiB
# libres fallaba (que es lo que el caso quiere); en cuanto el mismo disco paso
# a 1,1 TiB libres, el umbral se cumplio, la comprobacion paso, la funcion
# devolvio 0 SIN IMPRIMIR NADA y las dos aserciones cayeron. Un caso que
#depends de la maquina mide la maquina: se rompe cuando el disco se vacia, que
# es justo cuando nadie esta mirando este test. C7 ya lo hacia bien, con un `df`
# que dice la cifra; aqui hace falta lo mismo, y la cifra se elige para que
# falle siempre y no dependa de nada externo.
cat > "$STUBS/df" <<'STUB'
#!/bin/sh
printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\n'
printf 'fake-scratch 999999999 12 %s 50%%%% %s\n' "$FAKE_FREE_MB" "$FAKE_MOUNT"
STUB
chmod +x "$STUBS/df"
out="$(PATH="$STUBS:$PATH" FAKE_AVAIL_MB=8192 FAKE_FREE_MB=12 FAKE_MOUNT=/mnt/scratch-falso \
    SDDK_RELEASE_MIN_FREE_MB=4096 SDDK_RELEASE_MIN_AVAIL_MB=1 \
    release_check_resources "$TMPROOT" 2>&1)"
rc=$?
asert "C8: con memoria de sobra pero sin disco, el fallo es de disco" \
    "$([ $rc -eq 1 ] && echo 1 || echo 0)" "rc=$rc obtenido: $out"
asert "C8: y el mensaje no culpa a la memoria" \
    "$([[ "$out" == *disco* && "$out" != *memoria* ]] && echo 1 || echo 0)" "obtenido: $out"
# Y el otro lado de la misma regla, con la cifra del stub: disco y memoria son
# independientes, asi que con las dos de sobra tiene que devolver 0 sin hablar.
if out="$(PATH="$STUBS:$PATH" FAKE_AVAIL_MB=8192 FAKE_FREE_MB=999999999 FAKE_MOUNT=/mnt/scratch-falso \
    SDDK_RELEASE_MIN_FREE_MB=4096 SDDK_RELEASE_MIN_AVAIL_MB=4096 \
    release_check_resources "$TMPROOT" 2>&1)"; then C8_RC=1; else C8_RC=0; fi
asert "C8: y con las dos de sobra devuelve 0 en silencio" \
    "$C8_RC" "obtenido: $out"
rm -f "$STUBS/free" "$STUBS/df"
caso_termina C8

# --- C9: el lock compartido se nombra, y el release no se acusa a si mismo ---
#
# Se usa un ejecutable de verdad llamado `cargo` (el stub) porque el criterio de
# deteccion es `comm`, el NOMBRE DEL EJECUTABLE, no una cadena en los
# argumentos: MEDIDO en esta maquina, un grep de "cargo" tambien encuentra los
# binarios `asv-*` que viven bajo `cargo-targets/` porque su ruta contiene la
# palabra, y tambien el propio shell que lanza el preflight. Con ese criterio el
# aviso sale siempre, y un aviso que sale siempre no informa de nada: es ruido
# con forma de diagnostico, que es peor que no avisar.
#
# El stub escribe su PID: buscarlo despues con `pgrep` seria otra fuente de
# falsos positivos, y el PID que un proceso declara sobre si mismo es el unico
# que no depende de interrogar la tabla de procesos.
caso_empieza
FAKE_CARGO="$STUBS/cargo"
cat > "$FAKE_CARGO" <<'STUB'
#!/bin/sh
echo $$ > "$FAKE_CARGO_PIDFILE"
sleep 60
STUB
chmod +x "$FAKE_CARGO"
LOCKTARGET="$TMPROOT/shared-target"
mkdir -p "$LOCKTARGET"

# lanzar_cargo <cwd> <target|none> <pidfile> — imprime el PID del proceso
# desacoplado (setsid --fork lo deja con PPID 1: por eso es "ajeno" a proposito
# y no por casualidad, que es la condicion que este caso quiere ferreciosamente).
#
# El PID se acumula en un FICHERO y no en un array, y no por gusto: el llamante
# hace `AJENO_PID="$(lanzar_cargo ...)"`, luego `lanzar_cargo` corre en un
# subshell y un array modificado ahi se pierde al salir. MEDIDO: con el array
# ninguno se limpiaba y cada caso veia los procesos del anterior, que es como
# un caso CAE por un residuo y queda leido como defecto del producto.
lanzar_cargo() {
    local cwd="$1" target="$2" pidfile="$3" waited=0 pid
    : > "$pidfile"    # truncar en vez de borrar: `rm` en este entorno va a la basura
    ( cd "$cwd" && CARGO_TARGET_DIR="$target" FAKE_CARGO_PIDFILE="$pidfile" \
        setsid --fork "$FAKE_CARGO" </dev/null >/dev/null 2>&1 )
    while [ ! -s "$pidfile" ] && [ "$waited" -lt 60 ]; do
        sleep 0.1; waited=$((waited + 1))
    done
    pid="$(cat "$pidfile" 2>/dev/null || true)"
    echo "${pid:-LAZCARGO-NO-LANZO}" >> "$TMPROOT/registro-pids"
    echo "$pid"
}

# parar_cargos mata lo que el registro dice que se lanzo, y vacia el registro:
# un caso que hereda el sujeto de otro no ha medido lo que cree haber medido.
parar_cargos() {
    local pid
    if [ -s "$TMPROOT/registro-pids" ]; then
        while read -r pid; do
            [ -n "$pid" ] && [ "$pid" != "LAZCARGO-NO-LANZO" ] && kill "$pid" 2>/dev/null
        done < "$TMPROOT/registro-pids"
    fi
    : > "$TMPROOT/registro-pids"
    sleep 0.4
}

# vivo <pid> — el sujeto del caso EXISTE. Sin esta comprobacion, un caso cuyo
# sujeto no arranco pasa por vacuidad: no cuenta ni como CAIDO ni como VERDE,
# que es el mismo fallo que "una mutacion que no se aplica es SKIP, nunca PASS",
# en su version de guion.
vivo() {
    { [ -n "${1:-}" ] && [ "$1" != "LAZCARGO-NO-LANZO" ] && [ -d "/proc/$1" ]; } && echo 1 || echo 0
}

out="$(CARGO_TARGET_DIR="$LOCKTARGET" release_check_cargo_lock "$LOCKTARGET" 2>&1)"
asert "C9: sin retencion no inventa ninguna" \
    "$([[ -z "$out" ]] && echo 1 || echo 0)" "obtenido: $out"

AJENO_PID="$(lanzar_cargo "$TMPROOT" "$LOCKTARGET" "$TMPROOT/ajeno.pid")"
asert "C9: el cargo ajeno arranco de verdad; si no, el caso pasaria por vacuidad" \
    "$(vivo "$AJENO_PID")" "pid obtenido: '$AJENO_PID'"
out="$(CARGO_TARGET_DIR="$LOCKTARGET" release_check_cargo_lock "$LOCKTARGET" 2>&1)"
asert "C9: nombra el pid del cargo ajeno" \
    "$([[ -n "$AJENO_PID" && "$out" == *"pid $AJENO_PID"* ]] && echo 1 || echo 0)" \
    "obtenido: $out / esperado pid $AJENO_PID"
asert "C9: dice cuanto lleva esperando" \
    "$([[ "$out" =~ [0-9]+s\ esperando ]] && echo 1 || echo 0)" "obtenido: $out"
asert "C9: dice que esto NO va a fallar" \
    "$([[ "$out" == *"NO va a fallar"* ]] && echo 1 || echo 0)" "obtenido: $out"
parar_cargos

# Un cargo que declara OTRO target dir no es nuestra contencion, aunque compile
# desde el mismo sitio. De eso se encarga la rama que descarta en cuanto el
# environ declara uno que no es el nuestro.
OTRO_PID="$(lanzar_cargo "$TMPROOT" "$TMPROOT/otro-target" "$TMPROOT/otro.pid")"
asert "C9: el cargo de control tambien arranco" \
    "$(vivo "$OTRO_PID")" "pid obtenido: '$OTRO_PID'"
out="$(CARGO_TARGET_DIR="$LOCKTARGET" release_check_cargo_lock "$LOCKTARGET" 2>&1)"
asert "C9: un cargo que declara otro target dir no es nuestra contencion" \
    "$([[ -n "$OTRO_PID" && -z "$out" ]] && echo 1 || echo 0)" "obtenido: $out"
parar_cargos

# El cargo que este release lance es descendiente suyo, no "otro proyecto". Sin
# esta exclusion el aviso seria ruido en cada release, y por tanto inutil.
: > "$TMPROOT/propio.pid"
( cd "$TMPROOT" && CARGO_TARGET_DIR="$LOCKTARGET" FAKE_CARGO_PIDFILE="$TMPROOT/propio.pid" \
    "$FAKE_CARGO" >/dev/null 2>&1 ) &
PROPIO_JOB=$!
sleep 0.8
PROPIO_PID="$(cat "$TMPROOT/propio.pid" 2>/dev/null || true)"
asert "C9: el descendiente arranco; si no, 'no lo cuenta' no habria medido nada" \
    "$(vivo "$PROPIO_PID")" "pid obtenido: '$PROPIO_PID'"
out="$(CARGO_TARGET_DIR="$LOCKTARGET" release_check_cargo_lock "$LOCKTARGET" 2>&1)"
asert "C9: no cuenta a un descendiente propio como retencion ajena" \
    "$([[ -z "$out" ]] && echo 1 || echo 0)" "obtenido: $out"
kill "$PROPIO_JOB" 2>/dev/null
[ -n "$PROPIO_PID" ] && echo "$PROPIO_PID" >> "$TMPROOT/registro-pids"
parar_cargos

# El caso que separa "esta reteniendo el target dir" de "su ruta contiene la
# palabra cargo". MEDIDO en esta maquina: hay al menos siete binarios `asv-*`
# vivos bajo `/var/home/rubentxu/cargo-targets/`, y un `grep '[c]argo'` sobre
# las lineas de ordenes los cuenta a todos — ademas de contar el propio shell
# que ejecuta este test. Aqui el sujeto tiene las dos condiciones que harian
# que contarlo: su ruta lleva "cargo" y compila desde la raiz de este repo. Lo
# que NO hace es llamarse cargo, y por eso no es una retencion.
FAKE_OTRO="$STUBS/cargo-asv-helper"
cat > "$FAKE_OTRO" <<'STUB'
#!/bin/sh
echo $$ > "$FAKE_OTRO_PIDFILE"
sleep 60
STUB
chmod +x "$FAKE_OTRO"
: > "$TMPROOT/otro2.pid"
( cd "$ROOT" && CARGO_TARGET_DIR="$LOCKTARGET" FAKE_OTRO_PIDFILE="$TMPROOT/otro2.pid" \
    setsid --fork "$FAKE_OTRO" </dev/null >/dev/null 2>&1 )
waited=0
while [ ! -s "$TMPROOT/otro2.pid" ] && [ "$waited" -lt 60 ]; do sleep 0.1; waited=$((waited + 1)); done
FAKE_OTRO_PID="$(cat "$TMPROOT/otro2.pid" 2>/dev/null || true)"
asert "C9: el sujeto del falso positivo arranco" \
    "$(vivo "$FAKE_OTRO_PID")" "pid obtenido: '$FAKE_OTRO_PID'"
out="$(CARGO_TARGET_DIR="$LOCKTARGET" release_check_cargo_lock "$LOCKTARGET" 2>&1)"
asert "C9: un proceso que NO se llama cargo no es retencion, aunque su ruta la lleve" \
    "$([[ -z "$out" ]] && echo 1 || echo 0)" "obtenido: $out"
[ -n "$FAKE_OTRO_PID" ] && echo "$FAKE_OTRO_PID" >> "$TMPROOT/registro-pids"
parar_cargos
caso_termina C9

# --- resultado ---------------------------------------------------------------

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS - el release nombra el paso, la causa y la medicion."
    exit 0
fi
echo "RESULT: FAIL - hay ramas del diagnostico que no dicen la verdad."
exit 1
