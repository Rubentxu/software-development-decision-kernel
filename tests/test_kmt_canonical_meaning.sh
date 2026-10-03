#!/usr/bin/env bash
# Gate de ADR-0154: KMT tiene UN significado, y el rename no se aplica a medias.
#
# Por qué este guard existe. ADR-0154 decide que `KMT` = Knowledge Merkle Tree
# y que el evaluador de frescura pasa a llamarse `KnowledgeFreshness`. El
# rename es puramente nominal, y un rename a medias deja el peor de los dos
# mundos: el nombre viejo para el codigo viejo y el nuevo sin uso — dos formas
# de llamar a la misma cosa, que es exactamente lo que el ADR cierra.
#
# ESTE GUARD ES ESTADO ACTUAL, NO PROSPECTO. Antes de que el rename se aplique,
# mide lo que hoy existe y falla si el mundo no es el que el ADR describe. Su
# valor es doble: obliga a que las cifras del ADR sigan siendo verdad, y
# cuando el rename llegue, este mismo guard es el que dice si se completo.
#
# Falla cerrado: si el simbolo no se encuentra, no se da por bueno.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE="$ROOT/crates"
PASS=0
FAIL=0

ok()   { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "=== ADR-0154: una sola definicion de KMT ==="

# El ADR se lee antes de los checks: dos de ellos distinguen el estado
# `proposed` (rename pendiente, que es lo correcto hoy) del estado ya renombrado.
ADR="$ROOT/docs/architecture/adrs/ADR-0154-KMT-CANONICAL-MEANING.md"

# ── (a) el evaluador de frescura se llama KnowledgeFreshness ────────────────
# Estado PREVIO al rename: el simbolo viejo debe existir. Estado POST: el
# nuevo. Este guard se lee igual en los dos, que es lo que lo hace util.
if grep -rqE 'pub struct KnowledgeFreshness' "$ENGINE" 2>/dev/null; then
    ok "el evaluador de frescura ya se llama KnowledgeFreshness (rename aplicado)"
elif grep -rqE 'pub struct KMT;' "$ENGINE" 2>/dev/null; then
    ok "el evaluador de frescura sigue como KMT (rename pendiente, ADR proposed)"
else
    bad "no encuentro ni KnowledgeFreshness ni 'pub struct KMT;' en crates/"
fi

# ── (b) las dos expansiones retiradas NO pueden seguir en el codigo ─────────
# 'Knowledge Management Tiers' describe una intencion que nunca se implemento.
#
# Y aqui esta la distincion que hace que el guard sea cierto en los dos estados
# del ADR. Con el ADR `proposed` esas dos menciones son EXPECTADAS: son
# precisamente la condicion de partida que el ADR mide. Fallar por ellas seria
# un guard que exige un rename que el propio ADR prohíbe todavia, o sea un
# guard que obliga a hacer lo que el roadmap dice no hacer todavia.
# Lo que si es un fallo, en cualquier estado, es que aparezcan MAS de las dos
# conocidas: eso seria la expansion reintroducida por la puerta de atras.
T=$(grep -rl 'Knowledge Management Tiers' "$ENGINE" --include=*.rs 2>/dev/null | wc -l)
if [ "$T" -eq 0 ]; then
    ok "'Knowledge Management Tiers' ya no aparece en el codigo (rename aplicado)"
elif [ "$T" -le 1 ] && grep -q '^status: proposed' "$ADR" 2>/dev/null; then
    ok "'Knowledge Management Tiers' sigue en $T fichero, como describe el ADR proposed"
else
    bad "'Knowledge Management Tiers' aparece en $T ficheros; el ADR preveia como maximo 1"
fi

# ── (c) KMT no puede designar dos tipos a la vez ────────────────────────────
# El defecto que el ADR mide: 'KMT' es un evaluador (knowledge.rs) y 'KmtIndex'
# es un indice de arbol (reactive_verify.rs), y la prosa llama al segundo con
# las siglas del primero.
#
# El patron es INSENSIBLE A MAYUSCULAS a proposito, y esa es una correccion
# medida: la primera version usaba `pub struct KMT[A-Za-z]*`, que no casa
# `KmtShadow` porque el simbolo empieza con 'Kmt' y no con 'KMT'. Un guard que
# solo ve la forma exacta del simbolo queya conoce es un guard que no vigila
# la aparicion del siguiente.
STRUCTS=$(grep -rhoEi 'pub struct kmt[A-Za-z]*' "$ENGINE" --include=*.rs 2>/dev/null \
          | sed 's/.*[Ss]truct //' | sort -u)
NS=$(printf '%s\n' "$STRUCTS" | grep -c .)
# HOY hay dos: `KMT` (el evaluador) y `KmtIndex` (el indice del arbol). Esa
# duplicidad ES el defecto que el ADR mide, luego con el ADR `proposed` el
# estado correcto es 2 y no 1. Lo que no puede pasar es que sean mas de dos, o
# que sigan siendo dos con el ADR ya aceptado.
if [ "$NS" -eq 1 ]; then
    ok "un solo struct KMT*: $(printf '%s' "$STRUCTS" | tr -d '\n') (estado final)"
elif [ "$NS" -eq 2 ] && grep -q '^status: proposed' "$ADR" 2>/dev/null; then
    ok "2 structs KMT* ($(printf '%s' "$STRUCTS" | tr '\n' ' ')): la colision que el ADR mide"
else
    bad "$NS structs con nombre KMT*: $(printf '%s' "$STRUCTS" | tr '\n' ' ')"
fi

# ── (c-bis) un rename a medias deja dos autoridades ─────────────────────────
# El riesgo que el ADR declara: un refactor aplicado a medias deja el nombre
# viejo para el codigo viejo y el nuevo sin uso. Con el ADR `proposed` hoy no
# hay renombre, asi que la coexistencia solo es fallo si ocurre; con el ADR
# `accepted`, la coexistencia es exactamente el fallo.
AMBOS=0
grep -rqE 'pub struct KnowledgeFreshness' "$ENGINE" 2>/dev/null && \
    grep -rqE 'pub struct KMT;' "$ENGINE" 2>/dev/null && AMBOS=1
if [ "$AMBOS" -eq 0 ]; then
    ok "no coexisten KnowledgeFreshness y KMT: el rename no esta a medias"
elif grep -q '^status: proposed' "$ADR" 2>/dev/null; then
    bad "coexisten el nombre viejo y el nuevo con el ADR en proposed: rename a medias"
else
    bad "coexisten KnowledgeFreshness y KMT tras el rename: dos autoridades"
fi

# ── (d) la prosa no puede llamar 'KMT' a algo que no sea el evaluador ───────
# reactive_verify.rs y card.rs usan 'KMT' en comentarios para hablar del ARBOL,
# que es el error de lectura que motivo el ADR. Con el ADR `proposed` esos
# comentarios son la condicion de partida MEDIDA (3 lineas, 2 ficheros), luego
# no son un fallo: son lo que el ADR dice que hay que arreglar. Lo que si es un
# fallo es que aparezcan mas de los conocidos, o que sobrevivan con el ADR ya
# aceptado — que es cuando el rename deberia habersele aplicado.
# ── (d) la prosa no puede llamar 'KMT' a lo que no es el índice ─────────────
# El defecto que el ADR mide: `reactive_verify.rs` y `card.rs` llamaban «KMT» al
# ARBOL, que era el nombre del EVALUADOR. Post-rename, `KMT` es el nombre
# canónico del árbol y decir «the KMT index» es correcto — asi que este check
# ya no puede buscar «KMT cerca de una palabra de arbol», porque AHORA ESO ES
# LO CORRECTO. Lo que sigue siendo un fallo es el patron original: usar «KMT»
# como si fuera el evaluador de frescura.
#
# Version previa: contaba ficheros con 'KMT' junto a unit/indice/arbol, y en
# cuanto el rename se aplico dio 1 FAIL sobre dos lineas que eran correctas. Un
# guard que exige arreglar lo que esta bien entrena a ignorar sus propios rojos.
# El fallo estaba en el instrumento, y se corrige aqui, no bajando el liston.
MALO=0
while IFS= read -r f; do
    [ -n "$f" ] || continue
    # Lo que se busca es 'KMT' hablando de frescura (el evaluador con el nombre
    # viejo). Se excluye explicitamente 'KMT index'/'KMT unit',
    # que es el nombre canonico del arbol desde este mismo ADR.
    n=$(grep -nE '^\s*(///|//!|//)' "$f" 2>/dev/null \
        | grep -iE '\bKMT\b' \
        | grep -viE 'kmt (unit )?(index|indice|tree|arbol)' \
        | grep -c .)
    MALO=$((MALO + n))
done < <(grep -rl 'KMT' "$ENGINE" --include=*.rs 2>/dev/null)
if [ "$MALO" -eq 0 ]; then
    ok "ningun comentario llama KMT al evaluador de frescura (nombre retirado)"
elif [ "$MALO" -le 2 ] && grep -q '^status: proposed' "$ADR" 2>/dev/null; then
    ok "$MALO comentario(s) usan KMT para el evaluador: la condicion de partida que el ADR mide"
else
    bad "$MALO comentario(s) siguen llamando 'KMT' al evaluador de frescura, cuyo nombre es KnowledgeFreshness"
fi

# ── (e) el ADR y el codigo tienen que CONTAR LA MISMA HISTORIA ──────────────
# Este check existia como «el ADR esta proposed», y al aplicarse el rename paso a
# verde AFIRMANDO «el rename sigue bloqueado» mientras el rename estaba aplicado.
# Un guard que describe un estado sin comprobarlo es peor que no tener guard, y
# este es el cuarto caso de la serie.
#
# Lo que si es cierto en cualquier estado: el estado del ADR y el del codigo no
# pueden contradecirse. Rename aplicado con ADR `proposed` significa que alguien
# se salto el orden que el propio roadmap escribio.
if [ ! -f "$ADR" ]; then
    bad "no existe docs/architecture/adrs/ADR-0154-KMT-CANONICAL-MEANING.md"
elif grep -rqE 'pub struct KnowledgeFreshness' "$ENGINE" 2>/dev/null; then
    if grep -q '^status: accepted' "$ADR" 2>/dev/null; then
        ok "ADR-0154 accepted y el rename aplicado: la historia cuadra"
    else
        bad "el rename esta aplicado pero ADR-0154 sigue $(grep -m1 '^status:' "$ADR" | cut -d' ' -f2)"
    fi
else
    if grep -q '^status: accepted' "$ADR" 2>/dev/null; then
        bad "ADR-0154 accepted pero el rename NO esta aplicado: decision sin ejecutar"
    else
        ok "ADR-0154 $(grep -m1 '^status:' "$ADR" | cut -d' ' -f2) y el rename pendiente: la historia cuadra"
    fi
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — KMT tiene una sola definicion, o la que tiene esta escrita."
    exit 0
fi
echo "RESULT: FAIL — el mundo ya no es el que ADR-0154 describe, o el rename se aplico a medias."
exit 1
