#!/bin/bash
# check_binary_freshness.sh — ¿puedo fiar una medicion hecha con este binario?
#
# POR QUE ESTA EN EL REPO Y NO DENTRO DEL BINARIO
# -----------------------------------------------
# `sddk dev build-id --check` ya sabia nombrar la relacion entre un binario y
# un checkout: `matches`, `behind`, `diverged`, `no_checkout`, `unknown`. MEDIDO
# contra HEAD, acierta y explica: nombra los dos commits y dice por que. O sea
# que la DETECCION existe y funciona.
#
# El problema es DONDE vive el JUICIO. `--check` se ejecuta **desde el binario
# que juzga**, y eso rompe la propiedad por construccion, de dos maneras:
#
#   1. **No puedes comprobar algo antes de instalarlo.** Para correr el check
#      necesitas el artefacto, luego el artefacto solo puede pronunciarse
#      sobre si mismo cuando ya esta instalado y ya se ha usado.
#   2. **Un artefacto viejo no puede ni declarar su propia ignorancia.** El
#      subcomando `dev build-id` entro en `032e9553`; un binario anterior no lo
#      tiene, contesta "unrecognized subcommand" y sale con 2. MEDIDO con un
#      stub. Ese es el estado MAS viejo de todos, y desde dentro es
#      invisible: no hay nada que ejecutar que lo delate.
#
# Asi que aqui se separa lo que si puede venir del artefacto de lo que no:
#
#   - **HECHO**: el binario dice cual es su commit y si su arbol estaba sucio.
#     Un artefacto no puede mentir sobre lo que es, y eso no necesita su
#     permiso para ser dato.
#   - **JUICIO**: si ese commit esta antes o despues del HEAD de ESTE
#     checkout. El punto de referencia es el checkout, luego el juicio es de
#     aqui.
#
# El juez es el checkout —que por definicion esta al dia— y el juzgado solo
# aporta un hecho. Por eso el veredicto es fiable aunque el binario sea viejo.
#
# USO
# ---
#   bash scripts/check_binary_freshness.sh [ruta/al/binario] [--format texto|json]
#
# CODIGO DE SALIDA
# ----------------
#   0  `matches` — el binario contiene exactamente este HEAD
#   1  cualquier relacion que impida certificar una medicion
#   2  uso incorrecto, o el binario no se puede ejecutar
#
# LO QUE NO SE AFIRMA
# -------------------
#   - Que el checkout este limpio. Si tiene cambios sin commitear, `matches`
#     significa "coincide con HEAD", que ya no describe el arbol. Se DECLARA en
#     la salida en vez de decidirse aqui, porque el estado sucio del checkout es
#     del que llama, no del binario.
#   - Nada sobre el contenido del worktree mas alla del commit.

set -uo pipefail

FORMAT="texto"
BIN=""
# MEDIDO: la primera version recorria `for arg in "$@"` con `shift` DENTRO. En
# bash el `for ... in "$@"` expande la lista UNA vez al empezar, luego el
# `shift` no cambia lo que se itera: `--format` leia `${2:-}` de la lista
# original, se comia la RUTA del binario como valor de formato y el binario
# quedaba vacio. Salia con "no encuentro un binario ejecutable" con un
# binario perfectamente valido en la linea de ordenes. Un `while` con `shift`
# si respeta el avance.
while [ $# -gt 0 ]; do
    case "$1" in
        --format)
            [ $# -ge 2 ] || { echo "--format necesita un valor" >&2; exit 2; }
            FORMAT="$2"; shift 2 ;;
        --format=*)
            FORMAT="${1#*=}"; shift ;;
        -h|--help)
            sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        -*)
            echo "opcion desconocida: $1" >&2; exit 2 ;;
        *)
            [ -z "$BIN" ] || { echo "mas de un binario: $BIN y $1" >&2; exit 2; }
            BIN="$1"; shift ;;
    esac
done

if [ "$FORMAT" != "texto" ] && [ "$FORMAT" != "json" ]; then
    echo "formato desconocido: $FORMAT (texto|json)" >&2
    exit 2
fi

if [ -z "$BIN" ]; then
    BIN="$(command -v sddk || true)"
fi
if [ -z "$BIN" ] || [ ! -x "$BIN" ]; then
    echo "FATAL: no encuentro un binario ejecutable de sddk (usa scripts/check_binary_freshness.sh <ruta>)" >&2
    exit 2
fi

# ── el checkout es la autoridad ──────────────────────────────────────────────
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
HEAD_SHA="$(git rev-parse HEAD 2>/dev/null || true)"
DIRTY_TREE="false"
if [ -n "$HEAD_SHA" ] && ! git diff --quiet 2>/dev/null; then
    DIRTY_TREE="true"
fi

# ── el hecho: que dice de si mismo el binario ────────────────────────────────
RAW="$("$BIN" dev build-id --format json 2>/dev/null)" && QUERY_RC=0 || QUERY_RC=$?

json_field() {  # json_field <clave>
    # MEDIDO: la primera version exigia comillas alrededor del valor y por eso
    # `dirty` —que es un BOOLEANO sin comillas en el JSON— devolvia vacio, el
    # checker caia en la rama de `matches` y daba OK a un binario construido
    # sobre un arbol sucio. No lo vio la revision: lo cazó el guard, en su
    # primera corrida honesta, con el caso `dirty`.
    # Por eso la comilla es opcional a ambos lados: `"sha": "abc"` y
    # `"dirty": true` tienen que salir igual.
    # Y el recorte de ESPACIOS no es cosmetico: la clase `[^,"}]` no excluye
    # los espacios, luego `"dirty": true` salia capturado como `true  ` y la
    # comparacion exacta contra "true" fallaba. MEDIDO: eso hizo que el
    # checker diera `matches`/OK a un binario construido sobre un arbol
    # sucio, y lo cazó el caso `dirty` del guard.
    printf '%s' "$RAW" \
        | tr -d '\n' \
        | sed -n "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"\{0,1\}\([^,\"}]*\)\"\{0,1\}.*/\1/p" \
        | head -1 \
        | tr -d '[:space:]'
}

REL=""
DETALLE=""
VEREDICTO="N/A"

if [ -z "$REPO_ROOT" ] || [ -z "$HEAD_SHA" ]; then
    REL="no-checkout"
    DETALLE="no hay checkout que sirva de referencia, luego no hay nada"
    DETALLE="$DETALLE respecto de que ser viejo. No es un veredicto de frescura;"
    DETALLE="$DETALLE quien mida decide si eso le sirve."
    VEREDICTO="N/A"
elif [ "$QUERY_RC" -ne 0 ] || [ -z "$RAW" ]; then
    REL="no-build-id"
    DETALLE="el binario no responde a 'dev build-id' (rc=$QUERY_RC). El subcomando"
    DETALLE="$DETALLE entro en 032e9553, luego este artefacto es ANTERIOR a ese"
    DETALLE="$DETALLE commit por construccion — es el estado mas viejo de todos, y"
    DETALLE="$DETALLE desde dentro seria invisible: no hay nada que ejecutar que lo delate."
    VEREDICTO="FALLO"
else
    SHA="$(json_field sha)"
    ART_DIRTY="$(json_field dirty)"
    ART_VERSION="$(json_field version)"

    case "$SHA" in
        ""|unknown)
            REL="unknown-commit"
            DETALLE="el binario declara commit 'unknown': no puede decir que es,"
            DETALLE="$DETALLE luego no puede certificarse. Un artefacto que no se"
            DETALLE="$DETALLE identifica no es una medicion con nombre."
            VEREDICTO="FALLO"
            ;;
        *)
            if [ "$ART_DIRTY" = "true" ]; then
                REL="dirty"
                DETALLE="el binario se construyo sobre un arbol con cambios sin"
                DETALLE="$DETALLE commitear (commit ${SHA:0:8}), luego el artefacto"
                DETALLE="$DETALLE no es reproducible: el mismo commit da dos binarios."
                VEREDICTO="FALLO"
            elif [ "$SHA" = "$HEAD_SHA" ]; then
                REL="matches"
                DETALLE="el binario es exactamente este HEAD."
                VEREDICTO="OK"
            elif git merge-base --is-ancestor "$SHA" "$HEAD_SHA" 2>/dev/null; then
                REL="behind"
                DETALLE="el binario (${SHA:0:8}) es ancestro del HEAD del checkout"
                DETALLE="$DETALLE (${HEAD_SHA:0:8}): al checkout le faltan los cambios"
                DETALLE="$DETALLE que el binario no tiene. Declara la MISMA version"
                DETALLE="$DETALLE que el workspace, luego la comparacion de versiones sale verde."
                VEREDICTO="FALLO"
            elif git merge-base --is-ancestor "$HEAD_SHA" "$SHA" 2>/dev/null; then
                REL="ahead"
                DETALLE="el binario (${SHA:0:8}) tiene commits que este checkout no"
                DETALLE="$DETALLE tiene (${HEAD_SHA:0:8}). No es un problema de"
                DETALLE="$DETALLE frescura: el checkout es el que esta atrasado."
                VEREDICTO="OK"
            else
                REL="diverged"
                DETALLE="ni el binario (${SHA:0:8}) ni el checkout (${HEAD_SHA:0:8})"
                DETALLE="$DETALLE contiene al otro. No hay orden entre ellos y medir"
                DETALLE="$DETALLE con uno no dice nada sobre el otro."
                VEREDICTO="FALLO"
            fi
            ;;
    esac
    SHA_FULL="$SHA"
    [ -n "${ART_VERSION:-}" ] && VER_BIN="$ART_VERSION" || VER_BIN="?"
fi

if [ "$FORMAT" = "json" ]; then
    printf '{"binary":"%s","relation":"%s","verdict":"%s","binary_commit":"%s","checkout_head":"%s","checkout_dirty":%s,"detail":"%s"}\n' \
        "$BIN" "$REL" "$VEREDICTO" "${SHA_FULL:-}" "${HEAD_SHA:-}" \
        "$DIRTY_TREE" "$DETALLE"
else
    echo "binario   : $BIN"
    echo "checkout  : ${HEAD_SHA:-<ninguno>}"
    echo "relacion  : $REL"
    echo "veredicto : $VEREDICTO"
    [ -n "${SHA_FULL:-}" ] && echo "commit    : ${SHA_FULL:0:8} (binario $VER_BIN)"
    [ "$DIRTY_TREE" = "true" ] && \
        echo "AVISO     : el checkout tiene cambios sin commitear, luego 'matches'"
    echo "detalle   : $DETALLE"
fi

[ "$VEREDICTO" = "FALLO" ] && exit 1
exit 0
