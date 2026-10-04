#!/usr/bin/env bash
# Autofalsacion de la linea base de la version publicada en el gate 2b
# (INC-DEBT-070).
#
# EL DEFECTO
# ----------
# `tests/test_changelog_coverage.sh` sacaba su linea base de `git -C "$ROOT"
# tag`, que es el clon LOCAL. Publicar deja el clon viejo —MEDIDO al cerrar
# v2.8.0: local v2.7.0, remoto v2.8.0— y entonces el gate exige en la seccion
# siguiente commits que ya estan publicados. La forma facil de ponerlo verde es
# **duplicar esas entradas**, que es un changelog describiendo trabajo ya salido
# dentro de un artefacto que se publica.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Tres mutaciones, cada una sobre UN punto de enforcement, y cada una exigiendo
# que caiga el CASO NOMBRADO que lo vigila.
#
# El needle mide la **FORMA del veredicto de ese caso** (`[FAIL] C1:`) y no un
# texto de asercion, por la razon que ya se pago en este repo dos veces: un
# needle ambiguo —el nombre del caso aparece igual en la linea ok y en la fail—
# daria "siguen verdes" justo cuando han caido.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`.
# SC2016: las cadenas de mutacion llevan `$` y `${...}` de proposito — son
# LITERALES que se comparan contra el fuente del gate, y expandirlos ejecutaria
# su contenido en este shell en vez de escribirlo. Sin el disable, el falsador
# compararia contra otra cosa y supondria que la mutacion no aplico: un SKIP
# fabricado a partir de un literal roto.
# shellcheck disable=SC2016
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

GATE="$ROOT/tests/test_changelog_coverage.sh"
TEST="$ROOT/tests/test_changelog_coverage_baseline.sh"

BAK="$(mktemp -d)"
cp "$GATE" "$BAK/gate.sh"
restore() { cp "$BAK/gate.sh" "$GATE"; }
trap 'restore; rm -rf "$BAK"' EXIT
SHA_ANTES="$(sha256sum "$GATE" | cut -d' ' -f1)"

PASS=0
FAIL=0
SKIP=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skp() { echo "  [SKIP] $1"; SKIP=$((SKIP + 1)); }

# Aplica una mutacion entre dos anclas y EXIGE que el fichero haya cambiado.
muta_entre() {  # muta_entre <ancla-inicio> <ancla-fin> <reemplazo>
    local ini="$1" fin="$2" nuevo="$3" despues
    GATE="$GATE" INI="$ini" FIN="$fin" NUEVO="$nuevo" python3 - <<'PY' || return 2
import os, sys
p, ini, fin, nuevo = os.environ["GATE"], os.environ["INI"], os.environ["FIN"], os.environ["NUEVO"]
s = open(p, encoding="utf-8").read()
i = s.index(ini)
j = s.index(fin, i)
open(p, "w", encoding="utf-8").write(s[:i] + nuevo + s[j:])
PY
    despues="$(sha256sum "$GATE" | cut -d' ' -f1)"
    if [[ "$SHA_ANTES" == "$despues" ]]; then
        skp "la mutacion no cambio el fichero; NO cuenta como deteccion"
        return 2
    fi
    return 0
}

# Corre el test y exige que caiga el caso nombrado.
cae() {  # cae <etiqueta> <caso-id>
    local etiqueta="$1" caso="$2" salida
    salida="$(bash "$TEST" 2>&1)"
    # Un caso TIENE VARIAS aserciones, luego un caso medio caido tiene lineas
    # [ok] y lineas [FAIL] a la vez. Medir buscando el [ok] leeria verde un caso
    # caido —el needle ambiguo de nuevo, con otra forma—. Un caso CAE si
    # ALGUNA de sus aserciones es FAIL, luego lo que se busca es el FAIL.
    if ! grep -qF "[FAIL] $caso:" <<<"$salida"; then
        bad "$etiqueta: el caso $caso SIGUE EN VERDE tras la mutacion (no tiene dientes)"
        return 1
    fi
    return 0
}

echo "== Autofalsacion de la linea base publicada del gate 2b =="
echo

# ── M1: la linea base vuelve al clon local ──────────────────────────────────
# El defecto literal. Con el clon viejo, el gate compararia contra v0.9.0 y
# pediria en la seccion siguiente los dos commits que ya salieron en v1.0.0,
# luego C1 pasaria a rojo. Y el rojo cae en la DIRECCION peligrosa: no es
# "no vi nada", es "exijo trabajo ya publicado".
echo "-- M1: la linea base vuelve a leerse de los tags locales"
if muta_entre \
    'LAST_PUB_REMOTE="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}"' \
    'echo "  (published-version authority: $LAST_PUB_SOURCE)"' \
    'LAST_TAG="$(git -C "$ROOT" tag --sort=-version:refname | head -1)"
LAST_PUB_SOURCE="local (reverted)"
'; then
    if cae "M1" "C1"; then
        ok "M1 detectado: leer el clon local hace pedir en la seccion siguiente lo que ya salio"
    fi
    restore
fi

# ── M2: el remoto que no responde degrada a la lista local ──────────────────
# El caso que mas cuesta, porque degradar FUNCIONA: el clon local tiene tags de
# sobra y produce un veredicto. Ese veredicto no se sabe de donde sale, que es
# justo lo que un gate de cobertura no puede tolerar. C3 tiene que caer.
echo "-- M2: un remoto que no responde degrada silenciosamente a la lista local"
if muta_entre \
    'LAST_PUB_REMOTE="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}"' \
    'echo "  (published-version authority: $LAST_PUB_SOURCE)"' \
    'LAST_PUB_REMOTE="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}"
LAST_TAG=""
LAST_PUB_SOURCE=""
# MUTACION: si la consulta remota falla, se usa la lista local en vez de fallar
# cerrado. Es la degradacion que el defecto original practicaba en silencio.
if git -C "$ROOT" remote | grep -qx "$LAST_PUB_REMOTE" && [ -f "$ROOT/scripts/lib/release_admission.sh" ]; then
    . "$ROOT/scripts/lib/release_admission.sh"
    _last_published_resolve || true
    if [ "${LAST_PUB_OUTCOME:-}" != "bootstrap" ] && [ -n "${LAST_PUB_OUTCOME:-}" ]; then
        LAST_TAG="v${LAST_PUB_OUTCOME#v}"
        LAST_PUB_SOURCE="remote ($LAST_PUB_REMOTE)"
    fi
fi
if [ -z "$LAST_TAG" ]; then
    LAST_TAG="$(git -C "$ROOT" tag --sort=-version:refname | head -1 || true)"
    LAST_PUB_SOURCE="local (fallback M2)"
fi
'; then
    if cae "M2" "C3"; then
        ok "M2 detectado: un remoto caido produjo un veredicto en vez de un fallo que nombra la causa"
    fi
    restore
fi

# ── M3: el rango deja de ser calculable y el gate no lo declara ─────────────
# El tag publicado falta en el clon y hay que traerlo. Si no se trae, `git log`
# falla, la lista de commits sale VACIA, y el gate lee "no hay nada que
# comprobar" — con un workspace por delante del tag, que es justo la forma que
# INC-DEBT-040 ya describio como fallo. Un rango no calculable tiene que DECIR
# que no lo es. C1 cae.
echo "-- M3: el tag publicado ausente del clon se da por bueno sin traerlo"
if muta_entre \
    'if ! git -C "$ROOT" rev-parse -q --verify "refs/tags/$LAST_TAG" >/dev/null; then' \
    'echo "  (published-version authority: $LAST_PUB_SOURCE)"' \
    'if false; then
    if ! GIT_TERMINAL_PROMPT=0 git -C "$ROOT" fetch -q --tags "$LAST_PUB_REMOTE" "refs/tags/$LAST_TAG:refs/tags/$LAST_TAG" 2>&1; then
        bad "cannot fetch (mutado)"
        echo
        echo "PASS=$PASS FAIL=$FAIL"
        exit 1
    fi
fi

'; then
    if cae "M3" "C1"; then
        ok "M3 detectado: un rango inc calculable se leyo como 'no hay nada que comprobar'"
    fi
    restore
fi


# ── M4: la seccion congelada se trata como si se pudiera ampliar ───────────
# El caso que el arreglo de la linea base destapo: cuando el workspace ya es la
# version publicada, su seccion salio y pedirle que declare commits posteriores
# es pedirle que describa mal un artefacto que ya se distribuyo. Si el bloque
# congelado desaparece, el gate cae a la comprobacion normal y falla. C4 cae.
echo "-- M4: una seccion ya publicada se trata como ampliable"
if muta_entre \
    'WS_VER="$(sed -n' \
    'done < <(git -C "$ROOT" log --format=%s "$LAST_TAG"..HEAD)' \
    ': # MUTACION: desaparece el tratamiento de la seccion congelada. El gate
# vuelve a la comprobacion normal y le pide a una seccion ya publicada que
# declare el trabajo posterior, que es exactamente el defecto.
'; then
    if cae "M4" "C4"; then
        ok "M4 detectado: el gate pidio a una seccion publicada que declarara trabajo posterior"
    fi
    restore
fi

# ── restauracion byte-identica ──────────────────────────────────────────────
SHA_FINAL="$(sha256sum "$GATE" | cut -d' ' -f1)"
if [[ "$SHA_FINAL" == "$SHA_ANTES" ]]; then
    ok "restauracion byte-identica verificada por sha"
else
    bad "la restauracion no es byte-identica ($SHA_FINAL != $SHA_ANTES)"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[[ "$FAIL" -eq 0 ]]
