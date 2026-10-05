#!/usr/bin/env bash
# release_exclusion.sh
#
# Exclusion MUTUA entre dos ejecuciones del release sobre el MISMO repo y la
# MISMA version.
#
# POR QUE ESTO EXISTE, MEDIDO (session-82, INC-DEBT-075)
# ------------------------------------------------------
# Dos sesiones de agente convivieron ejecutando `scripts/release.sh` sobre el
# mismo checkout, el mismo HEAD y la misma version:
#
#     pid 38340  una sesion   PPID 26812
#     pid 99022  otra sesion  PPID 3001 (systemd --user)
#
# Las dos llegaron a compilar. El `cargo` de la segunda retuvo el target dir
# compartido y el caso C9 de `test_release_diagnostics.sh` cayo. La contencion
# era REAL —el diagnostico no mintio—, pero no habia quien dijera que dos
# releases del mismo tag no deberian existir a la vez.
#
# LO QUE ESTO NO ES, y la distincion es el Nucleo
# ------------------------------------------------
# `release.sh` YA avisaba de la retencion del target dir, y con buen criterio:
# dos PROYECTOS distintos compilando en un target dir compartido es el uso
# normal de una maquina de desarrollo, y morir ahi impediria releases
# legitimas. Ese aviso se queda como esta. Lo que faltaba era otro caso:
#
#   - dos proyectos distintos en un target dir compartido  -> AVISAR (correcto)
#   - dos releases del MISMO repo y la MISMA version      -> ABORTAR
#
# El segundo es el que tiene consecuencia destructiva: los dos podrian llegar
# al paso 9 y escribir el mismo artefacto, o uno sobrescribir al otro con
# `--force`. NO se ha medido que ocurra; la ventana existe. Un aviso no detiene
# a nadie, y lo que hace falta aqui es que el segundo NO SEA POSIBLE.
#
# EL CANDADO ES UN FICHERO, Y POR QUE NO UN DIRECTORIO
# ----------------------------------------------------
# La primera version de este fichero usaba `mkdir` como primitivo atomico y
# `rm -rf` para soltar. MEDIDO en esta maquina: **`rm -rf "$var"` NO BORRA
# NADA** —el shim de `rm` recibe el argumento sin expandir y dice
# `'/var/.../$lockdir': No such file or directory`—, y el resultado medido fue
# que la liberacion fallaba de forma INTERMITENTE. Eso rompia el candado en las
# dos direcciones: al no borrar, un candado ya libre parecia ocupado (falso
# rojo), y al releer un `pid` que el borrado aun no habia retirado, un candado
# recien soltado parecia tomado (falso VERDE, que es peor). Reintentar no lo
# arregla: lo que no se puede es depender de un primitivo que no borra.
#
# Se reescribio sobre las tres primitivas que MEDIDAMENTE funcionan aqui:
#
#   * crear en exclusiva  ->  ( set -C; printf '%s' "$pid" > "$fichero" )
#                            `noclobber` hace que la redireccion falle si el
#                            fichero existe, y eso es atomico sin necesidad de `mkdir`.
#   * soltar              ->  `unlink "$fichero"` (medido: borra; `rm`, no)
#   * recuperar un huerfano -> sobrescribir el fichero con nuestro pid
#
# Y la recuperacion de un candado huerfano ya NO necesita borrar nada: el
# directorio —el fichero— existe y su dueno esta MUERTO, luego la exclusividad
# ya esta resuelta y solo queda escribir encima. Eso la hace determinista.
#
# LO QUE ESTE MECANISMO NO GARANTIZA, y se declara
# -----------------------------------------------
# Dos procesos que ven el MISMO candado huerfano a la vez pueden escribir
# encima los dos y creerse los dos con el candado. La ventana es el tiempo
# entre leer un pid muerto y escribir el propio, y exige que el dueno previo
# haya muerto en ese instante. Se declara en vez de ocultarse: un candado con
# esta ventana es estrictamente MEJOR que no tener candado, y el caso que de
# verdad importa —dos releases vivos del mismo tag— no la comparte, porque ahi
# el pid es vivo y nadie escribe encima.

# release_exclusion_dir — el directorio donde viven los candados. Imprime la
# ruta en stdout y devuelve 1 si no se puede usar.
#
# Vive bajo el estado de SDDK y NO bajo el repo, por dos razones ya medidas en
# este repo: el preflight exige arbol limpio, asi que un candado dentro del
# checkout lo ensuciaria; y `release.sh` reapunta `TMPDIR` al scratch de la
# CORRIDA, luego cualquier ruta derivada de TMPDIR es distinta en cada
# ejecucion y no excluiria de nada.
release_exclusion_dir() {
    local base="${SDDK_STATE_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/sddk}"
    local dir="$base/release-locks"
    if [ -d "$dir" ]; then
        printf '%s' "$dir"
        return 0
    fi
    if mkdir -p "$dir" 2>/dev/null && [ -d "$dir" ]; then
        printf '%s' "$dir"
        return 0
    fi
    return 1
}

# _release_exclusion_path <key> — ruta del fichero de candado.
_release_exclusion_path() {
    local dir
    dir="$(release_exclusion_dir)" || return 1
    printf '%s/%s.lock' "$dir" "$1"
}

# _release_exclusion_create <fichero> <pid> — 0 si se CREO de nuevo, 1 si ya
# existia. `noclobber` dentro de un subshell: la redireccion falla si el
# fichero esta, y el subshell deja intacto el `noclobber` de quien llama.
_release_exclusion_create() {
    local fichero="$1" pid="$2"
    if ( set -C; printf '%s' "$pid" > "$fichero" ) 2>/dev/null; then
        return 0
    fi
    return 1
}

# release_exclusion_holder <key> — imprime el pid que tiene el candado, o
# cadena vacia si no lo tiene nadie.
release_exclusion_holder() {
    local fichero
    fichero="$(_release_exclusion_path "$1")" || return 1
    [ -f "$fichero" ] || return 0
    tr -dc '0-9' < "$fichero" 2>/dev/null || true
}

# _release_exclusion_alive <pid> — 1 si ese proceso existe.
_release_exclusion_alive() {
    local pid="${1:-}"
    [ -n "$pid" ] || return 1
    [ -d "/proc/$pid" ]
}

# release_exclusion_acquire <key> <pid>
#
#   0  candado tomado (o ya era nuestro)
#   1  lo tiene OTRO pid vivo -> mensaje en stderr nombrando quien
#   2  no se puede usar el directorio de candados (NO se simula que este libre)
#
# MEDIDO lo que hace con un candado HUERFANO: si el pid escrito ya no existe
# (el proceso murio, con o sin su `trap` — por ejemplo al apagarse el host), el
# candado se RECUPERA y se DECLARA, porque un candado huerfano que bloquea
# para siempre convierte un mecanismo de seguridad en la razon de que no se
# pueda publicar.
release_exclusion_acquire() {
    local key="$1" pid="${2:-$$}" fichero holder
    if ! fichero="$(_release_exclusion_path "$key")"; then
        printf '  ! exclusion mutua NO disponible: no se puede crear %s\n' \
            "${SDDK_STATE_DIR:-$HOME/.local/state}/sddk/release-locks" >&2
        printf '    sin candado NO se puede garantizar que dos releases del mismo\n' >&2
        printf '    tag no se pisen. Se aborta en vez de publicar sin exclusion.\n' >&2
        return 2
    fi

    if _release_exclusion_create "$fichero" "$pid"; then
        return 0
    fi

    holder="$(release_exclusion_holder "$key")"
    if [ -n "$holder" ] && [ "$holder" = "$pid" ]; then
        # YA ES NUESTRO. Re-entrar no es competir consigo mismo: devolver
        # «ocupado por otro» haria que un segundo intento desde el MISMO proceso
        # se presentara como exclusion, que es la forma de que el mecanismo
        # mienta justo cuando hay que fiarse de el. MEDIDO al escribir el
        # guard: sin esto, recuperar un candado huerfano y volver a tomar desde
        # el mismo shell devolvia 1 y el guard leia un fallo donde no lo habia.
        return 0
    fi
    if [ -n "$holder" ] && _release_exclusion_alive "$holder"; then
        printf '  x ya hay un release en curso de esta MISMA version (pid %s)\n' \
            "$holder" >&2
        printf '      lo tiene: %s\n' "$fichero" >&2
        printf '    Dos releases del mismo tag no pueden convivir: los dos pueden\n' >&2
        printf '    llegar al paso 9 y escribir el mismo artefacto.\n' >&2
        printf '    Espera a que termine, o comprueba que ese proceso es tuyo.\n' >&2
        return 1
    fi

    # CANDADO HUERFANO: el dueno ya no existe, luego la exclusividad esta
    # resuelta y solo queda escribir encima. Sin borrar nada, que es lo que
    # hacia intermitente la version con `rm`.
    printf '  ! candado huerfano de un release que ya no esta vivo (pid %s)\n' \
        "${holder:-sin pid legible}" >&2
    printf '    Se recupera el candado y se continua.\n' >&2
    if printf '%s' "$pid" > "$fichero" 2>/dev/null; then
        return 0
    fi
    printf '  x no se pudo recuperar el candado huerfano: %s\n' "$fichero" >&2
    return 1
}

# release_exclusion_release <key> <pid> — suelta el candado SOLO si es nuestro.
#
# Un candado no es un fichero cualquiera: si se suelta sin comprobar el pid, el
# primer proceso que termine puede dejar libre el candado del otro, y entonces la
# exclusion deja de existir justo cuando hacia falta. Y se suelta con `unlink`,
# no con `rm`: MEDIDO, `rm -rf "$var"` no borra en este entorno.
release_exclusion_release() {
    local key="$1" pid="${2:-$$}" fichero holder
    fichero="$(_release_exclusion_path "$key")" || return 0
    [ -f "$fichero" ] || return 0
    holder="$(release_exclusion_holder "$key")"
    if [ -n "$holder" ] && [ "$holder" != "$pid" ]; then
        printf '  ! candado de la version %s lo tiene pid %s, no este (%s); no se suelta\n' \
            "$key" "$holder" "$pid" >&2
        return 0
    fi
    unlink "$fichero" 2>/dev/null || true
    return 0
}

# release_exclusion_key <root> <version> — la clave del candado. Un repo y una
# version, y no solo el repo: dos versiones del mismo repo pueden compilar a la
# vez sin pisarse, y bloquear eso seria correcto en teoria y molesto en la
# practica, que es como se mueren las mutex.
release_exclusion_key() {
    local root="$1" version="$2" flat
    flat="$(printf '%s' "$root" | tr '/' '_')"
    printf 'release-%s-%s' "$flat" "$version"
}
