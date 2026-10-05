#!/bin/bash
# Release diagnostics — why the release stopped, and what it was doing.
#
# EL DEFECTO
# ----------
# El release sabia CUANDO fallaba y no sabia POR QUE. Medido, no supuesto:
#
#   * `scripts/release.sh` tiene 73 llamadas a `die` y 32 de ellas no nombran ni
#     el codigo de salida ni una causa medida. La del paso 1 es la mas clara:
#     `die "cargo test --workspace failed"`. Afirma el hecho y nada mas. Un
#     ENOSPC y un test rojo producen exactamente el mismo texto, y el operador
#     tiene que adivinar cual de los dos fue.
#   * El paso 3 se quedo **12 min 30 s** imprimiendo `Blocking waiting for file
#     lock on build directory` y nada mas (session-79b, MEDIDO). Otro proyecto
#     retenia el `CARGO_TARGET_DIR` compartido. El aviso llego del propio cargo
#     —que decide ser verboso— y `release.sh` no dijo nada, ni antes ni durante.
#   * `/tmp` es un tmpfs con tope, no el disco. Cuando se satura la suite muere
#     con `os error 122` en tests que no estan rotos (INC-DEBT-069, MEDIDO), y
#     ninguno de los mensajes nombra el disco.
#   * El backlog P1 `bl-bl-01M42JGYG4000388551BF9NZ40` describe el caso limite:
#     el OOM killer corto el log a 122880 bytes y el paso 1 se presento como "no
#     dice por que fallo".
#
# Un pipeline que no dice por que se detuvo devuelve el trabajo al que vino a
# evitar: buscar la causa a mano, con la maquina ya en otro estado.
#
# QUE HACE ESTE FICHERO
# ---------------------
# Tres propiedades, cada una con su por que:
#
#   P1 `release_diagnose_exit` — la salida nombra el paso en curso, el codigo y
#      la CAUSA con nombre. Una sola autoridad para los 73 `die`: se invoca
#      desde el manejador de salida y desde `die`, no se parchea cada llamada.
#      Ser una regla central y no 73 Messages es lo que la hace mantenible: dos
#      copias de "como se explica un fallo" divergen en cuanto se toca una.
#
#   P2 `release_check_resources` — antes de gastar la suite, se mira el disco
#      del scratch y la memoria, y si el margen no da se dice QUE recurso, DONDE
#      y CUANTO falta. Fail-closed, porque seguir sin margen solo convierte un
#      fallo en un fallo distinto.
#
#   P3 `release_check_cargo_lock` — si otro `cargo` retiene el target dir
#      compartido, se dice quien es y desde cuando, ANTES de esperar. Avisa y
#      deja continuar: la retencion no invalida la release, la ralentiza, y un
#      preflight que se pasara de severo impediria releases legitimas. La
#      decision de seguir o parar es del operador, y se la pone el dato delante.
#
# LO QUE ESTE FICHERO NO PUEDE HACER
# ----------------------------------
# Diagnosticar una muerte por SIGKILL desde el propio proceso es imposible: no
# hay manejador que se ejecute. Por eso el mecanismo NO es un manejador de
# senal, sino dos cosas que se pueden hacer de verdad:
#   * `step()` ya escribe el marker del paso con `printf` directo a fd 1, y
#     bash no bufferiza, luego el marker esta en disco cuando el proceso muere.
#     Un log cortado por el OOM killer YA dice en que paso fue.
#   * P2 evita que el proceso llegue a morir por falta de espacio.
# Decirlo aqui evita que alguien construya el manejador de senal que no puede
# funcionar y lo mida como cobertura.
#
# Sourceable: define functions only, never calls `exit`. Los llamadores deciden
# como fallar cerrado.

# --- estado que mantiene el llamador -----------------------------------------

# Caché de _cargo_effective_target, indexada por DIRECTORIO.
#
# MEDIDO (session-82): sin `declare -A`, `arr[$dir]` con una ruta en la clave hace
# que bash la evalúe como índice ARITMÉTICO y el subscript falle con «error de
# sintaxis aritmética». El fallo se quedaba silencioso en cuanto el resultado se
# usaba dentro de un `if`, y un guard que pasa porque su comprobación reventó no
# está midiendo nada.
declare -A _SDDK_EFFECTIVE_TARGET_CACHE=()
declare -A _SDDK_CACHE_SEEN=()

# RELEASE_CURRENT_STEP — nombre del paso en curso. `release_step` lo escribe.
RELEASE_CURRENT_STEP="${RELEASE_CURRENT_STEP:-}"
# RELEASE_DIAGNOSED — guardia de idempotencia, cuando no hay fichero marcador.
RELEASE_DIAGNOSED="${RELEASE_DIAGNOSED:-0}"
# RELEASE_DIAGNOSED_FILE — marcador persistente de "ya diagnostiqué". La guardia
# es un FICHERO y no una variable porque `die` corre en este shell pero el
# manejador de salida, y cualquier `$( ... )` que lo envuelva, viven en
# subshells donde una variable de shell ya no existe. Con la variable, el bloque
# salia DOS veces, y un diagnostico repetido se lee como dos fallos distintos
# —que es peor que no diagnosticar, porque desplaza la mirada del operador.
RELEASE_DIAGNOSED_FILE="${RELEASE_DIAGNOSED_FILE:-}"

# release_step <nombre>
# Anuncia un paso y deja constancia de cual es, para que un fallo posterior
# pueda nombrarlo. Envolve el `step()` de release.sh en vez de duplicar su
# formato: una sola linea decide que se anuncia y que se recuerda.
release_step() {
    RELEASE_CURRENT_STEP="$*"
    printf '\n\033[1;34m==>\033[0m %s\n' "$*"
}

# --- P1: la causa, con nombre -------------------------------------------------

# cause_of_exit_code <codigo> → una linea que NOMBRA la causa, o declara que no
# la conoce. Un codigo sin nombre conocido dice "sin causa con nombre"; no
# inventa una, porque una causa inventada es peor que ninguna: dirige la
# investigacion a un sitio falso.
#
# 137 = 128+9 (SIGKILL) y 139 = 128+11 (SIGSEGV) son los que un shell observa
# cuando un hijo muere por senal; el shell no los distingue de un exit code.
cause_of_exit_code() {
    local code="${1:-0}"
    case "$code" in
        0)   echo "exito" ;;
        1)   echo "fallo declarado por release.sh (un 'die'); el mensaje de esa linea dice que se queria" ;;
        2)   echo "uso incorrecto de argumentos o de sintaxis de invocacion" ;;
        122) echo "ENOSPC: no space left on device — el disco o el tmpfs del scratch esta lleno" ;;
        124) echo "timeout: el comando lo recibio y no termino a tiempo" ;;
        126) echo "no encontrado: el fichero existe pero no es ejecutable" ;;
        127) echo "no encontrado: el comando no existe en el PATH" ;;
        137) echo "SIGKILL (128+9): el proceso fue matado — OOM killer o kill externo; NO es un fallo del comando" ;;
        139) echo "SIGSEGV (128+11): segmentacion invalida — fallo de memoria, no de logica" ;;
        134) echo "SIGABRT (128+6): abort() — asercion, panic o doble free" ;;
        135) echo "SIGBUS (128+7): error de bus, tipicamente EIO sobre un backing store" ;;
        *)   if [ "$code" -gt 128 ] 2>/dev/null; then
                 echo "terminado por senal $((code - 128)) (128+$((code - 128))): el proceso murio, no fallo"
             else
                 echo "sin causa con nombre para el codigo $code"
             fi ;;
    esac
}

# _diag_measure — imprime el estado de los recursos que un fallo de disco o de
# memoria usaria como causa. Se imprime SIEMPRE, tambien cuando la causa ya se
# conoce: la causa nombrada sin la medicion deja al operador creyendola sin
# comprobarla, que es el fallo de una afirmacion sin evidencia.
_diag_measure() {
    local target="${1:-$RELEASE_SCRATCH}"
    if command -v df >/dev/null 2>&1 && [ -n "$target" ]; then
        printf '    scratch %s -> %s\n' "$target" \
            "$(df -h "$target" 2>/dev/null | awk 'NR==2 {print $1" "$2" usado de "$3" ("$5")"}' || echo 'no medible')"
    fi
    if [ -d /tmp ]; then
        printf '    /tmp     -> %s\n' \
            "$(df -h /tmp 2>/dev/null | awk 'NR==2 {print $1" "$2" usado de "$3" ("$5")"}' || echo 'no medible')"
    fi
    if command -v free >/dev/null 2>&1; then
        printf '    memoria -> %s\n' \
            "$(free -h 2>/dev/null | awk '/^Mem:/ {print $7" disponible de "$2" ("$3" en uso)"}' || echo 'no medible')"
    fi
}

# release_diagnose_exit <codigo> [contexto]
# Imprime el bloque de diagnostico de una salida no nula. Idempotente: la
# segunda llamada no imprime nada.
#
# Devuelve 0 SIEMPRE. Un diagnostico que devuelve error en el camino de salida
# puede convertir un fallo en un fallo distinto, que es exactamente lo que
# este fichero existe para evitar.
release_diagnose_exit() {
    local code="${1:-0}" context="${2:-}"
    [ "$code" = "0" ] && return 0

    local marker="${RELEASE_DIAGNOSED_FILE:-}"
    if [ -n "$marker" ]; then
        [ -e "$marker" ] && return 0
        : > "$marker" 2>/dev/null || marker=""
    fi
    if [ -z "$marker" ] && [ "${RELEASE_DIAGNOSED:-0}" = "1" ]; then
        return 0
    fi
    RELEASE_DIAGNOSED=1

    {
        printf '\n\033[1;31m  ✗ por que fallo el release\033[0m\n'
        printf '    paso     : %s\n' "${RELEASE_CURRENT_STEP:-<sin paso registrado: el fallo ocurrio antes de announce uno>}"
        printf '    codigo   : %s\n' "$code"
        # El codigo de proceso y el codigo de la causa NO son lo mismo, y
        # confundirlos fue el defecto MEDIDO del noveno intento de 2.9.0: el
        # bloque decia `codigo : 1` y ese 1 era el `exit 1` de `die`, no el del
        # comando. Cuando difieren se dicen los dos, porque un 139 leido sin
        # contexto hace pensar que el release se estrello.
        [ -n "${RELEASE_PROCESS_CODE:-}" ] && [ "${RELEASE_PROCESS_CODE:-}" != "$code" ] \
            && printf '    proceso  : %s (el release sale asi siempre; la causa es la linea de arriba)\n' \
                "${RELEASE_PROCESS_CODE}"
        printf '    causa    : %s\n' "$(cause_of_exit_code "$code")"
        [ -n "$context" ] && printf '    contexto : %s\n' "$context"
        _diag_measure "${RELEASE_SCRATCH:-}"
        printf '    esto NO dice cual test fallo ni cual comando: para eso esta el log de arriba.\n'
    } >&2
    return 0
}

# --- P2: margen de recursos antes de gastar la suite -------------------------

# SDDK_RELEASE_MIN_FREE_MB — margen minimo en el filesystem del scratch.
# 2048 MB: el release construye un binario musl de ~32 MB, un tarball de
# ~12 MB, assets, y la suite completa escribe mas de un GiB en temporales.
# Un umbral por debajo de lo que la propia operacion necesita no es
# prudencia, es el margen de un fallo futuro que se presentara con otro
# nombre.
SDDK_RELEASE_MIN_FREE_MB="${SDDK_RELEASE_MIN_FREE_MB:-2048}"
# SDDK_RELEASE_MIN_AVAIL_MB — memoria disponible minima. 2048 MB cubre el
# `cargo test --workspace` completo con los binarios de test en ejecucion.
SDDK_RELEASE_MIN_AVAIL_MB="${SDDK_RELEASE_MIN_AVAIL_MB:-2048}"

# release_check_resources [scratch]
# 0 si el scratch tiene margen de disco y la maquina de memoria; 1 con el
# recurso, el sitio y la cifra que faltan.
#
# Fail-closed. La alternativa —seguir y esperar a que algo falle— produce
# justamente el defecto que este fichero arregla: un fallo cuya causa no aparece
# en ningun mensaje.
release_check_resources() {
    local target="${1:-${RELEASE_SCRATCH:-}}"
    local failed=0

    if [ -n "$target" ] && [ -d "$target" ] && command -v df >/dev/null 2>&1; then
        local free_mb
        free_mb="$(df -Pm "$target" 2>/dev/null | awk 'NR==2 {print $4}')"
        if [ -n "$free_mb" ] && [ "$free_mb" -lt "$SDDK_RELEASE_MIN_FREE_MB" ]; then
            printf '  ! disco insuficiente en el scratch: %s MiB libres en %s, se necesitan %s MiB\n' \
                "$free_mb" "$(df -P "$target" | awk 'NR==2 {print $6}')" "$SDDK_RELEASE_MIN_FREE_MB" >&2
            failed=1
        fi
    fi

    if command -v free >/dev/null 2>&1; then
        local avail_mb
        avail_mb="$(free -m 2>/dev/null | awk '/^Mem:/ {print $7}')"
        if [ -n "$avail_mb" ] && [ "$avail_mb" -lt "$SDDK_RELEASE_MIN_AVAIL_MB" ]; then
            printf '  ! memoria insuficiente: %s MiB disponibles, se necesitan %s MiB\n' \
                "$avail_mb" "$SDDK_RELEASE_MIN_AVAIL_MB" >&2
            failed=1
        fi
    fi

    return "$failed"
}

# --- P3: quien retiene el target dir compartido ------------------------------

# _is_own_descendant <pid> — 1 si <pid> es este release o desciende de el.
# Sin esta exclusion el preflight se detectaria a si mismo: el `cargo` que
# este mismo release lanza cuenta como "otro proyecto", y el aviso seria ruido
# permanente en cada release.
#
# MEDIDO, y el error fue de direccion: la primera version subia desde `$$`
# buscando <pid>, que es la pregunta INVERTIDA. Se encuentra si <pid> esta por
# encima de nosotros —nunca lo esta, salvo que sea el propio release— y no ve
# lo que de verdad importa, que es lo que esta POR DEBAJO. Y hay un caso en el
# que lo de abajo ni siquiera existe como proceso aparte: bash optimiza
# `( cd X && cargo ... )` haciendo exec del ultimo comando, con lo que el
# subshell CONSERVA su PID y el `cargo` nace con el PID del padre inmediato.
# MEDIDO en el test: ese proceso se colaba como "retencion ajena", que es
# exactamente el aviso que apareceria en cada release real.
_is_own_descendant() {
    local cur="$1"
    while [ -n "$cur" ] && [ "$cur" != "0" ] && [ "$cur" != "1" ]; do
        [ "$cur" = "$$" ] && return 0
        cur="$(ps -o ppid= -p "$cur" 2>/dev/null | tr -d ' ')"
    done
    return 1
}

# _cargo_effective_target <dir> — el target dir que cargo usaria con el cwd
# <dir> y sin declarar ninguno. Cacheado por directorio.
#
# NO se reimplementa la resolucion de cargo: se le PREGUNTA. `cargo metadata` es
# la autoridad, y con la configuracion global de esta maquina
# (`~/.cargo/config.toml` fija `build.target-dir`) la respuesta NO es
# `<dir>/target` sino un unico directorio compartido por TODOS los proyectos
# Rust del host. Reimplementar esa regla aqui la dejaria vieja en cuanto el
# operador cambie su config.
#
# Vacio cuando no se puede resolver, y un vacio NO se adivina: quien no puede
# decir donde compila un cargo no puede afirmar que compila sobre ESTE target.
_cargo_effective_target() {
    local dir="$1" resolved effective
    if [ -n "${_SDDK_CACHE_SEEN[$dir]+x}" ]; then
        printf '%s' "${_SDDK_EFFECTIVE_TARGET_CACHE[$dir]}"
        return 0
    fi
    _SDDK_CACHE_SEEN[$dir]=1
    resolved=""
    if command -v cargo >/dev/null 2>&1 && [ -d "$dir" ]; then
        resolved="$( cd "$dir" 2>/dev/null \
            && cargo metadata --no-deps --format-version 1 2>/dev/null \
            | tr ',' '\n' | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' | head -1 )"
    fi
    if [ -n "$resolved" ]; then
        effective="$(readlink -f "$resolved" 2>/dev/null || echo "$resolved")"
    else
        effective=""
    fi
    _SDDK_EFFECTIVE_TARGET_CACHE[$dir]="$effective"
    printf '%s' "$effective"
    return 0
}

# _cargo_uses_target <pid> <target_real> — 1 si el proceso de ese PID compila
# sobre ESTE target dir.
#
# Tres caminos, del mas exacto al mas aproximado, que se bastan entre si porque
# ningun cargo declara el target dir siempre por el mismo sitio:
#
#   1. `/proc/<pid>/environ` contiene `CARGO_TARGET_DIR=<target>`. Exacto: es lo
#      que el proceso recibio de verdad. Si el fichero no es legible (otro uid)
#      se pasa al siguiente criterio, sin inventar.
#   2. La linea de ordenes lleva `--target-dir <target>`. Explicito.
#   3. El proceso corre desde el repo Y cargo resolveria para ahi ESTE target.
#      MEDIDO (session-82, INC-DEBT-075): el criterio 3 era SOLO «el cwd es el
#      repo», y decia «compila donde le digan, que suele ser este». Ese «suele»
#      es una suPOSICION no verificada, y se ha MEDIDO que es falsa: un cargo
#      ajeno con nuestro mismo cwd puede compilar en el target dir COMPARTIDO
#      de la maquina, que no es el que se le pregunta. El aviso decia entonces
#      «el target dir compartido esta retenido: pid N» sobre un pid que no
#      espera ese target, y quien lo lee no tiene forma de saber que el aviso se
#      refiere a otro directorio.
_cargo_uses_target() {
    local pid="$1" target="$2" env_target subject_cwd effective

    # MEDIDO (session-84 bis 6): leer `/proc/$pid/...` de un proceso que ya
    # termino escribia el error del shell a stderr, y ese error se colaba como
    # SALIDA de la funcion. MEDIDO el mecanismo, que no es obvio: en
    # `tr ... < "$f" 2>/dev/null`, bash aplica las redirecciones EN EL ORDEN
    # ESCRITO, luego la de entrada se intenta antes de que exista la de error y
    # su mensaje escapa. MEDIDO con las dos formas:
    #   `tr '\0' ' ' < /proc/999999/cmdline 2>/dev/null`  -> imprime el error
    #   `tr '\0' ' ' 2>/dev/null < /proc/999999/cmdline`  -> no imprime nada
    # El preflight es un aviso: un error de shell mezclado con el aviso es peor
    # que no avisar, porque quien lo lee no puede distinguir «lo que medi» de
    # «lo que se rompio». MEDIDO como se manifestaba: el 1b de la 2.11.3 paro
    # en `test_cargo_target_attribution.sh` con E2 en rojo y la salida
    # `.../release_diagnostics.sh: linea 330: /proc/<pid>/cmdline: No existe`.
    # No era el defecto que ese guard mide: era un proceso que murio entre que
    # `ps` lo listo y que se leyera su cmdline.
    if [ -r "/proc/$pid/environ" ]; then
        env_target="$(tr '\0' '\n' 2>/dev/null < "/proc/$pid/environ" \
            | awk -F= '/^CARGO_TARGET_DIR=/ {print $2; exit}')" || env_target=""
        if [ -n "$env_target" ]; then
            if [ "$(readlink -f "$env_target" 2>/dev/null || echo "$env_target")" = "$target" ]; then
                return 0
            fi
            # Declarado y DISTINTO: prueba de que no es nuestro, asi que no se
            # sigue al criterio del cwd, que lo contaria igual.
            return 1
        fi
    fi

    case "$(tr '\0' ' ' 2>/dev/null < "/proc/$pid/cmdline")" in
        *"--target-dir $target"*|*"--target-dir=$target"*) return 0 ;;
    esac

    subject_cwd="$(readlink -f "/proc/$pid/cwd" 2>/dev/null || echo '?')"
    [ "$subject_cwd" = "$PWD" ] || return 1

    # Mismo cwd, pero hay que COMPROBAR que ahi cargo usaria nuestro target, no
    # suponerlo.
    effective="$(_cargo_effective_target "$subject_cwd")"
    if [ -n "$effective" ] && [ "$effective" = "$target" ]; then
        return 0
    fi
    return 1
}

# release_check_cargo_lock [target_dir]
# 0 siempre. Imprime una linea por cada `cargo` ajeno que este esperando por el
# mismo target dir, con su PID, su antiguedad y su linea de ordenes. Avisa, no
# muere: la retencion no invalida la release, la ralentiza, y quien decide si
# esperar es el operador — con el dato delante, que es lo que no tenia.
#
# POR QUE `comm` Y NO UN GREP DE "cargo" EN LOS ARGUMENTOS: MEDIDO en esta
# maquina. Un `grep '[c]argo'` sobre las lineas de ordenes devuelve tambien los
# binarios `asv-*` que viven bajo `cargo-targets/`, porque su RUTA contiene la
# palabra, y devuelve el propio shell que esta ejecutando el preflight. Con eso
# el aviso aparece siempre, y un aviso que aparece siempre no informa de nada:
# es ruido con formato de diagnostico, que es peor que no avisar. `comm` es el
# nombre del ejecutable — `asv` no es `cargo`, y ahi no cabe la confusion.
release_check_cargo_lock() {
    local target="${1:-${CARGO_TARGET_DIR:-}}"
    [ -z "$target" ] && return 0
    command -v ps >/dev/null 2>&1 || return 0

    local target_real pid age comm found=0
    target_real="$(readlink -f "$target" 2>/dev/null || echo "$target")"

    while read -r pid age comm; do
        [ "${comm:-}" = "cargo" ] || continue
        [ -n "${pid:-}" ] || continue
        _is_own_descendant "$pid" && continue
        _cargo_uses_target "$pid" "$target_real" || continue
        found=1
        printf '  ! el target dir compartido esta retenido: pid %s, lleva %ss esperando\n' \
            "$pid" "$age" >&2
        printf '      %s\n' "$(tr '\0' ' ' < "/proc/$pid/cmdline" 2>/dev/null | cut -c1-200)" >&2
    done < <(ps -eo pid=,etimes=,comm= 2>/dev/null | awk '{print $1, $2, $3}')

    if [ "$found" = "1" ]; then
        printf '  ! esto NO va a fallar: el build esperara al lock, y puede esperar mucho.\n' >&2
        printf '    Se decide antes de empezar, no 12 minutos despues.\n' >&2
    fi
    return 0
}
