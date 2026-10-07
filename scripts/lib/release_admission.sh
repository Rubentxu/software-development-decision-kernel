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
_max_published_tag_from_refs() {
    printf '%s\n' "$1" \
        | awk '{print $2}' \
        | sed -n 's|^refs/tags/v\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$|\1|p' \
        | sort -V -r \
        | head -1
}

last_published_version() {
    local remote="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}" raw rc tag raw2 rc2 tag2
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

    if [[ -z "$tag" ]]; then
        LAST_PUB_OUTCOME="bootstrap"
    else
        LAST_PUB_OUTCOME="v${tag}"
    fi
    echo "$tag"
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
        echo "REJECT query-failed last-pub=${LAST_PUB_OUTCOME#query_failed:}"
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
