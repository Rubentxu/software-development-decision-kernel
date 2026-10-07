#!/bin/bash
# Release admission — A5-1 §6.
#
# The release contract is SEMANTIC: a release HEAD must carry a REAL,
# monotonically increasing `[workspace.package] version` change relative to
# its first parent. A commit subject (e.g. `chore(release): bump version`) is
# a human convention, never authority.
#
# Single machine-readable source for the invariant: `githooks/pre-push` uses
# the same "real version change" notion for push admission (a weaker check: it
# does not require monotonicity), while release admission (this file) is
# strictly stronger.
#
# Two implementations live here:
#   - release_admission_check (v1, default): compares HEAD version vs HEAD^
#     version. Strict monotonicity. Fails when a docs-only commit follows a
#     bump (because HEAD becomes the docs-only commit and HEAD^ stays at the
#     bumped version only if HEAD IS the bump, not if the bump is N commits
#     behind).
#   - release_admission_check_v2 (cycle-c, opt-in): compares HEAD version vs
#     the maximum version among published tags (v*) reachable from origin.
#     During development without a published release, falls back to HEAD^.
#     Selected when the environment variable SDDK_RELEASE_ADMISSION_MODE=v2
#     is set, or when the caller invokes v2 explicitly.
#
# Sourceable: defines functions only, never calls `exit`. Callers decide how to
# fail closed.

# cargo_ws_version_at <rev> → prints the [workspace.package] version at <rev>,
# or nothing if the revision/file/key is absent.
cargo_ws_version_at() {
    local rev="$1" content
    content="$(git show "$rev:Cargo.toml" 2>/dev/null || true)"
    [[ -z "$content" ]] && return 0
    echo "$content" | awk '
        /^\[workspace\.package\]/ { in_block=1; next }
        /^\[/ { in_block=0 }
        in_block && /^version[[:space:]]*=/ {
            match($0, /"[^"]*"/); print substr($0, RSTART+1, RLENGTH-2); exit
        }
    '
}

# semver_gt <a> <b> → 0 iff a > b under dotted numeric comparison.
semver_gt() {
    local a="$1" b="$2"
    [[ "$a" == "$b" ]] && return 1
    local IFS=. av bv i ai bi
    # shellcheck disable=SC2206
    av=($a)
    # shellcheck disable=SC2206
    bv=($b)
    for i in 0 1 2; do
        ai="${av[$i]:-0}"
        bi="${bv[$i]:-0}"
        [[ "$ai" =~ ^[0-9]+$ ]] || ai=0
        [[ "$bi" =~ ^[0-9]+$ ]] || bi=0
        if ((10#$ai > 10#$bi)); then return 0; fi
        if ((10#$ai < 10#$bi)); then return 1; fi
    done
    return 1
}

# release_admission_check [HEAD-rev]
# Prints `ACCEPT <prev> -> <head>` and returns 0, or `REJECT <reason>` and
# returns 1. Never exits.
#
# V1 (default) compares HEAD version vs HEAD^ version. Strict monotonicity.
# When SDDK_RELEASE_ADMISSION_MODE=v2 is set, the v2 variant is invoked
# instead (compares against last published tag version).
release_admission_check() {
    if [[ "${SDDK_RELEASE_ADMISSION_MODE:-}" == "v2" ]]; then
        release_admission_check_v2 "$@"
        return $?
    fi
    local head_rev="${1:-HEAD}" head_version prev_version
    head_version="$(cargo_ws_version_at "$head_rev")"
    prev_version="$(cargo_ws_version_at "$head_rev^")"
    if [[ -z "$head_version" ]]; then
        echo "REJECT missing-head-version"
        return 1
    fi
    if [[ -z "$prev_version" ]]; then
        echo "REJECT missing-parent-version"
        return 1
    fi
    if ! semver_gt "$head_version" "$prev_version"; then
        echo "REJECT non-monotonic $prev_version -> $head_version"
        return 1
    fi
    echo "ACCEPT $prev_version -> $head_version"
    return 0
}

# last_published_version
# Prints the maximum version among v* tags reachable from origin, stripped
# of the `v` prefix. Honours GIT_TERMINAL_PROMPT=0 to avoid hanging on
# protected branches.
#
# Outcome semantics — distinguishes THREE distinct cases (SCOPE-CONTRACT
# cycle-c §3.3): the caller MUST treat them differently. The companion
# `last_published_outcome` global holds the resolved outcome string.
#
#   $LAST_PUB_OUTCOME = "v<X.Y.Z>"  →  remote answered, here is the
#                                       maximum semver tag (or, if printed
#                                       by this function, the empty
#                                       max means "bootstrap" — see
#                                       `last_published_outcome_after`
#                                       helper).
#   $LAST_PUB_OUTCOME = "bootstrap"  →  remote answered, no v* tags yet
#                                       (legitimate bootstrap).
#   $LAST_PUB_OUTCOME = "query_failed:<reason>"  →  remote answer could
#                                       not be obtained; this MUST
#                                       trip a fail-closed REJECT, not
#                                       silently fall back to HEAD^.
#   $LAST_PUB_OUTCOME = "query_inconsistent:<a>|<b>"  →  the remote answered
#                                       TWICE and the two answers disagree.
#                                       See "LA SEGUNDA LECTURA" below: a
#                                       successful read is not the same as a
#                                       COMPLETE read, and this is the case
#                                       that a single `rc -eq 0` check cannot
#                                       see. MUST also fail closed.
#   $LAST_PUB_OUTCOME = "crosscheck_unavailable:<why>"
#   $LAST_PUB_OUTCOME = "crosscheck_failed:<slug>"
#   $LAST_PUB_OUTCOME = "crosscheck_mismatch:<ls>|<api>"
#                                       →  the SECOND AUTHORITY could not
#                                       confirm the first one, or contradicted
#                                       it. See "LA TERCERA FUENTE" below. All
#                                       three fail closed: an unconfirmed read
#                                       is not a confirmed one.
#
# Sourceable. Exports $LAST_PUB_OUTCOME for the caller. The previous
# contract (return-1 on empty) is preserved for back-compat callers that
# only check the printed value.

# Extrae el maximo semver de una respuesta cruda de `git ls-remote --tags`.
#
# Es una FUNCION SEPARADA a proposito: el recorte es la parte que hay que
# mirar cuando dos lecturas no coinciden, y duplicarlo dentro de las dos
# ramas seria dos copias que divergen en cuanto una cambia — la misma clase
# de defecto que este fichero ya registro con la lista de aliases y con las
# siete filas de frontera de UAT. Una sola extraccion, dos lecturas.
_semver_tags_from_refs() {
    printf '%s\n' "$1" \
        | awk '{print $2}' \
        | grep -vF '^{}' \
        | sed -n 's|^refs/tags/v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$|\1|p' \
        | sort -u
}

_max_published_tag_from_refs() {
    _semver_tags_from_refs "$1" | sort -V -r | head -1
}

# remote_repo_slug <remote> -> "owner/repo" si el remoto es un repositorio de
# github.com; sale con 1 si no lo es (un remoto local, un fork en otro host).
#
# Es la pieza que convierte "el remoto" en algo consultable por la API. No es
# decorativa: sin ella no hay segunda fuente, y sin segunda fuente la lectura
# parcial no se puede confirmar (ver LA TERCERA FUENTE).
_remote_repo_slug() {
    local remote="$1" url owner_repo
    # Se lee la URL CRUDA de la configuracion, no `git remote get-url`.
    # MEDIDO, y no es una preferencia: `get-url` devuelve la URL YA
    # EXPANDIDA por `url.<x>.insteadOf`, y `ls-remote` expande exactamente la
    # misma. Preguntar a los dos cosas distintas sobre la misma identidad no
    # tiene salida —uno tiene que mentir—, y el que miente es el slug, que
    # acaba diciendo "esto no es un repositorio de github" de un remoto que si
    # lo es. `insteadOf` es un alias de TRANSPORTE: saber que aqui las
    # conexiones van a un directorio local no dice a que repositorio pertenece
    # el proyecto, y la segunda fuente tiene que recibir la identidad real o
    # no esta mirando el mismo repositorio.
    url="$(git config --get "remote.$remote.url" 2>/dev/null || true)"
    if [[ -z "$url" ]]; then
        # El remoto puede venir dado como ruta (`/ruta/al/repo.git`) en vez de
        # como nombre. `get-url` no aplica a esos, asi que es el unico que
        # puede resolverlos, y devolvera algo que no es github — que es la
        # respuesta correcta para un repo local.
        url="$(git remote get-url "$remote" 2>/dev/null || true)"
    fi
    [[ -z "$url" ]] && return 1
    case "$url" in
        git@github.com:*)      owner_repo="${url#git@github.com:}" ;;
        ssh://git@github.com/*) owner_repo="${url#ssh://git@github.com/}" ;;
        https://github.com/*)   owner_repo="${url#https://github.com/}" ;;
        http://github.com/*)    owner_repo="${url#http://github.com/}" ;;
        *) return 1 ;;
    esac
    owner_repo="${owner_repo%/}"
    owner_repo="${owner_repo%.git}"
    [[ "$owner_repo" =~ ^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$ ]] || return 1
    printf '%s\n' "$owner_repo"
}

# published_tags_via_api <owner/repo> -> una linea por tag semver.
#
# MEDIDO: `gh api --paginate repos/<slug>/tags?per_page=100` devuelve exactamente
# el mismo conjunto de tags semver que `git ls-remote --tags`, sin las refs
# peeled (`^{}`), que son las 75 lineas de diferencia entre 387 y 312. Verificar
# eso ANTES de basar un gate en el es lo que separa una autoridad real de una
# suposicion: si los conjuntos no hubieran coincidido, la reconcil habria
# estado midiendo dos cosas distintas y habria cerrado siempre, que es la
# forma educada de no detectar nada.
_published_tags_via_api() {
    gh api --paginate "repos/$1/tags?per_page=100" --jq '.[].name' 2>/dev/null \
        | sed -n 's|^v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$|\1|p' \
        | sort -u
}

# tags_fingerprint <lista de versiones> -> "<n>|<max>"
#
# El conteo es la parte que hace el trabajo. Comparar solo el maximo deja pasar
# toda lectura parcial que no se lleve el tag mas nuevo, y MEDIDO en session-91
# eso son 2 de cada 3 truncos de este repo. El conteo no: una lista a la que le
# faltan refs tiene menos lineas, y eso no depende de que el maximo se mueva.
_tags_fingerprint() {
    local count max
    count="$(printf '%s\n' "$1" | grep -cE '^[0-9]+\.[0-9]+\.[0-9]+$')"
    max="$(printf '%s\n' "$1" | grep -E '^[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
    printf '%s|%s\n' "${count:-0}" "${max:-}"
}

last_published_version() {
    local remote="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}"
    local attempt="${SDDK_RELEASE_ADMISSION_ATTEMPTS:-5}" outcome="query_failed:no-attempt" n
    # Reintentos, y lo que NO son.
    #
    # MEDIDO: con el crosscheck ya puesto, el fallo por trunco paso de "el 30%
    # de las veces daba la respuesta EQUIVOCADA" a "cerraba cerrado". Cerrar
    # es correcto y no admite, pero un gate que se rinde al primer contraste
    # se queja cada pocas corridas por un fallo de red que no va a durar, y un
    # gate que se queja a menudo acaba siendo el que se relaja.
    #
    # Reintentar NO baja el liston: cada ronda exige las MISMAS tres
    # confirmaciones (dos lecturas de git y una de la API, con huellas
    # iguales). Lo que cambia es cuantas veces se pide antes de declarar que no
    # se pudo confirmar. Con el trunco medido (~6% por lectura, dos lecturas por
    # ronda) una ronda sola falla alrededor de 1 de cada 100 y tres rondas
    # bajan eso a menos de 1 entre 10^5; seguir PRODUCTO y con ruido de red no
    # es un plan de entrega.
    #
    # EL DESCANSO ENTRE RONDAS NO ES DECORATIVO, Y ESTA MEDIDO.
    # La tasa de trunco no es una constante de este remoto: en la misma sesion
    # se midio 3 de 50 lecturas truncadas (6%) en una ventana y 0 de 20 en
    # otra, con el mismo repositorio y el mismo comando. Es una propiedad del
    # INSTANTE de red. Tres rondas seguidas se ejecutan en menos de dos
    # segundos, luego caen de vuelta dentro de la misma ventana de corte y
    # repiten el mismo fallo — medido: una ronda de 3 intentos fallo con
    # `ls-remote=224|2.9.1` mientras 20 lecturas seguidas SPS sale completas.
    # Reintentar sin esperar no es reintentar: es preguntar tres veces al mismo
    # corte.
    for ((n = 1; n <= attempt; n++)); do
        # Se llama SIN captura de stdout a proposito. `$(...)` pondria la ronda
        # en un subshell, y `LAST_PUB_OUTCOME` —lo unico que dice si esta
        # ronda servo— se modificaria ahi dentro y no volveria: el padre se
        # quedaria con el motivo de la ronda anterior y leeria un exito o un
        # fallo de otra ronda. La ronda deja el tag en `_LAST_PUB_TAG` y el
        # motivo en `LAST_PUB_OUTCOME`, ambos en el shell que llama.
        _last_published_attempt "$remote"
        if [[ "$LAST_PUB_OUTCOME" == bootstrap || "$LAST_PUB_OUTCOME" == v* ]]; then
            printf '%s' "$_LAST_PUB_TAG"
            return 0
        fi
        outcome="$LAST_PUB_OUTCOME"
        # Descanso entre rondas, nunca despues de la ultima: esperar antes de
        # rendirse no cambia nada y hace el push mas lento para todos.
        if (( n < attempt )); then
            sleep "${SDDK_RELEASE_ADMISSION_BACKOFF:-1}"
        fi
    done
    LAST_PUB_OUTCOME="$outcome"
    return 1
}

# Una ronda completa: dos lecturas de `git ls-remote` mas la segunda
# autoridad. Pone el veredicto en LAST_PUB_OUTCOME y devuelve su codigo.
_last_published_attempt() {
    local remote="$1" raw rc tag raw2 rc2 tag2
    raw="$(GIT_TERMINAL_PROMPT=0 git ls-remote --tags "$remote" 2>&1)"
    rc=$?
    if [[ $rc -ne 0 ]]; then
        # Capture the error reason (one line if multi-line).
        local err
        err="$(printf '%s\n' "$raw" | head -1)"
        LAST_PUB_OUTCOME="query_failed:${err:-exit-$rc}"
        return 1
    fi
    local tag
    tag="$(_max_published_tag_from_refs "$raw")"

    # ── LA SEGUNDA LECTURA, Y POR QUE ESTA ───────────────────────────────────
    #
    # MEDIDO en este repo contra `origin` real: `git ls-remote --tags` devuelve
    # de forma INTERMITENTE una lista PARCIAL y aun asi sale con codigo 0.
    # Capturada: 249 lineas donde el mismo comando devolvia 386, y la lista
    # cortada perdia desde v2.2.12 en adelante — v2.10.0, v2.11.x, v2.12.x y
    # v2.13.0 fuera. El maximo caia de v2.13.0 a v2.9.1 y NADIE se enteraba.
    #
    # Es grave por la DIRECCION, no por la frecuencia, y la falsacion esta
    # medida sobre el producto real con un remoto truncado de verdad:
    #
    #   remoto completo  -> REJECT already-published 2.13.0   (correcto)
    #   remoto truncado  -> ACCEPT last-publish=2.9.1 -> 2.13.0
    #
    # El segundo caso ADMITE republicar una version ya publicada, porque un
    # baseline mas viejo hace que cualquier version nueva parezca mayor. El
    # fallo de red se convierte en permiso de publicar, y un release pipeline
    # cuyo gate de admision depende de una lectura que puede volver incompleta
    # sin decir nada no esta admitiendo: esta adivinando.
    #
    # La lectura unica no podia atraparlo: `rc -eq 0` es exactamente lo que
    # devuelve la respuesta incompleta. Lo que SI se puede afirmar sin conocer
    # el total esperado de refs es que DOS lecturas independientes no pueden
    # truncarse igual por casualidad, luego el desacuerdo es prueba de que al
    # menos una va mal. No se elige la "mejor" de las dos — elegir seria
    # adivinar con dos monedas —: se cierra.
    #
    # El COSTE son dos viajes de red en vez de uno, y es medido como precio de
    # un cierre duro, no como coste oculto: el 1b corre esto y el pre-push.
    raw2="$(GIT_TERMINAL_PROMPT=0 git ls-remote --tags "$remote" 2>&1)"
    rc2=$?
    if [[ $rc2 -ne 0 ]]; then
        local err2
        err2="$(printf '%s\n' "$raw2" | head -1)"
        LAST_PUB_OUTCOME="query_failed:${err2:-exit-$rc2}"
        return 1
    fi
    tag2="$(_max_published_tag_from_refs "$raw2")"
    if [[ "$tag" != "$tag2" ]]; then
        LAST_PUB_OUTCOME="query_inconsistent:${tag:-<none>}|${tag2:-<none>}"
        return 1
    fi

    # ── LA TERCERA FUENTE, Y POR QUE DOS LECTURAS NO ALCANZABAN ──────────────
    #
    # MEDIDO en session-91 contra este remoto real, con el resolutor real:
    #
    #   4/10 corridas -> v2.14.0   (correcto)
    #   3/10 corridas -> query_inconsistent   (el guard de dos lecturas CERRA:
    #                                            hace su trabajo)
    #   3/10 corridas -> v2.9.1    (PASS FALSO: el guard pasa y la respuesta es
    #                                 incorrecta)
    #
    # Treinta por ciento de PASS incorrecto, con el codigo de salida a 0 en las
    # tres ramas. Y el motivo esta capturado, no inferido: la lectura parcial
    # que produce `v2.9.1` tiene 250 lineas donde la completa tiene 387, son un
    # subconjunto EXACTO de la completa (`comm -12` -> 250), y NO viene ordenada
    # lexicograficamente como la completa — v2.14.0 ocupa la linea 338 de la
    # completa y no esta, y v2.9.1 ocupa la ultima. El orden de la respuesta
    # VARIA entre lecturas, luego el corte de red se lleva el final de ESE
    # orden y no siempre se lleva los tags recientes.
    #
    # Eso deja a las dos lecturas en una posicion que no se salva leyendo dos
    # veces: dos truncos pueden coincidir, y MEDIDO de 50 lecturas fueron 3
    # parciales y SOLO 1 de esas 3 movio el maximo — las otras 2 pasaban porque
    # el maximo no se habia movido. Comparar dos maximos detecta 1 de cada 3
    # truncos y deja pasar 2 de cada 3. La garantia era "1/3 de deteccion", y se
    # escribio como si fuera "ninguna pasa".
    #
    # Lo que SI separa una respuesta completa de una parcial, sin conocer el
    # total esperado de antemano, es preguntarle a OTRO sitio. La API REST de
    # GitHub no habla el protocolo git: es otro transporte, otro host y otro
    # endpoint, y una respuesta parcial de `info/refs` no tiene por que tocar
    # `api.github.com`. No es la MISMA lectura dos veces — que es lo que ya no
    # bastaba —; es una lectura de una fuente que no comparte el modo de
    # fallo.
    #
    # Y si la segunda fuente no esta, no se degrada: se cierra con el motivo
    # accionable. Un baseline sin confirmar NO es un baseline confirmado.
    local slug api_tags ls_fp api_fp
    if ! command -v gh >/dev/null 2>&1; then
        LAST_PUB_OUTCOME="crosscheck_unavailable:gh-not-in-path"
        return 1
    fi
    if ! slug="$(_remote_repo_slug "$remote")"; then
        LAST_PUB_OUTCOME="crosscheck_unavailable:remote-is-not-github:$remote"
        return 1
    fi
    if ! api_tags="$(_published_tags_via_api "$slug")"; then
        LAST_PUB_OUTCOME="crosscheck_failed:$slug"
        return 1
    fi
    ls_fp="$(_tags_fingerprint "$(_semver_tags_from_refs "$raw")")"
    api_fp="$(_tags_fingerprint "$api_tags")"
    if [[ "$ls_fp" != "$api_fp" ]]; then
        LAST_PUB_OUTCOME="crosscheck_mismatch:ls-remote=$ls_fp|api=$api_fp"
        return 1
    fi

    if [[ -z "$tag" ]]; then
        LAST_PUB_OUTCOME="bootstrap"
    else
        LAST_PUB_OUTCOME="v${tag}"
    fi
    _LAST_PUB_TAG="$tag"
    return 0
}

# Helper for v2 path: resolve the outcome in one call and export it.
# Returns 0 on bootstrap or known-version, 1 on query_failed (callers
# must REJECT in that case).
_last_published_resolve() {
    LAST_PUB_OUTCOME=""
    # Invoke last_published_version only to populate the side-effect
    # variable; the printed value is discarded.
    last_published_version >/dev/null 2>&1 || true
    if [[ -z "${LAST_PUB_OUTCOME:-}" ]]; then
        # Defensive: last_published_version should always set the var.
        LAST_PUB_OUTCOME="query_failed:no-outcome"
        return 1
    fi
    case "$LAST_PUB_OUTCOME" in
        bootstrap|v*) return 0 ;;
        # El remoto respondio DOS VECES y las dos respuestas no coinciden. Se
        # cierra por la misma razon que un fallo de red, y NO se degrada a la
        # lista local: una lista local parcial daria el mismo veredicto
        # equivocado que la respuesta remota parcial.
        query_failed:*|query_inconsistent:*) return 1 ;;
        # Y lo mismo para la segunda autoridad. "No pude confirmarlo" y "lo
        # confirme" no pueden leerse igual, y "lo confirme y no cuadra" menos.
        # Los tres casos se declaran y se rechazan; ninguno degrada.
        crosscheck_unavailable:*|crosscheck_failed:*|crosscheck_mismatch:*) return 1 ;;
        *) LAST_PUB_OUTCOME="query_failed:unknown-shape"; return 1 ;;
    esac
}

# release_admission_check_v2 [HEAD-rev]
# V2 admission:
#   - if no published release exists: fall back to v1 against HEAD^ (bootstrap).
#   - else compare HEAD version against the maximum version among published v*
#     tags reachable from origin.
# Behaviour:
#   - Last-published < head → ACCEPT last-publish=<last> -> <head>
#   - Last-published == head → REJECT already-published <version>
#   - Last-published >  head → REJECT not-above-last-publish <last> -> <head>
# Sourceable, never exits.
release_admission_check_v2() {
    local head_rev="${1:-HEAD}" head_version last_pub prev_version
    head_version="$(cargo_ws_version_at "$head_rev")"
    if [[ -z "$head_version" ]]; then
        echo "REJECT missing-head-version"
        return 1
    fi

    # Resolve the last-published outcome via the helper. This sets the
    # global $LAST_PUB_OUTCOME to one of:
    #   - "v<X.Y.Z>"   → bootstrap path or comparison path, depending
    #   - "bootstrap"  → remote answered, no v* tags (legitimate bootstrap)
    #   - "query_failed:<reason>" → REJECT (fail-closed; do NOT fall back
    #                                  to HEAD^)
    if ! _last_published_resolve; then
        # El motivo va ENTERO, no recortado por `query_failed:`. Antes se
        # imprimia `${LAST_PUB_OUTCOME#query_failed:}` y los outcomes mas
        # nuevos —`crosscheck_mismatch:…`— no tienen ese prefijo, luego el
        # `${var#prefijo}` no recorta nada y se imprimia entero por casualidad
        # y no por diseno. Un motivo de rechazo que no se lee es un rechazo
        # que hay que adivinar.
        echo "REJECT unresolved last-pub=${LAST_PUB_OUTCOME}"
        return 1
    fi

    if [[ "$LAST_PUB_OUTCOME" == "bootstrap" ]]; then
        # No published releases yet (bootstrap). Compare against HEAD^
        # for monotonicity, fail-closed on ties or decreases.
        prev_version="$(cargo_ws_version_at "$head_rev^")"
        if [[ -z "$prev_version" ]]; then
            echo "REJECT missing-parent-version"
            return 1
        fi
        if ! semver_gt "$head_version" "$prev_version"; then
            echo "REJECT non-monotonic-bootstrap $prev_version -> $head_version"
            return 1
        fi
        echo "ACCEPT last-publish=bootstrap -> $head_version"
        return 0
    fi

    # Outcome is "v<X.Y.Z>".
    last_pub="${LAST_PUB_OUTCOME#v}"
    if [[ "$head_version" == "$last_pub" ]]; then
        echo "REJECT already-published $head_version"
        return 1
    fi
    if ! semver_gt "$head_version" "$last_pub"; then
        echo "REJECT not-above-last-publish $last_pub -> $head_version"
        return 1
    fi
    echo "ACCEPT last-publish=$last_pub -> $head_version"
    return 0
}
