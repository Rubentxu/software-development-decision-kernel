#!/usr/bin/env bash
# shellcheck disable=SC2016
# SC2016, MEDIDO y deliberado: cada mutacion es un payload de python que se
# pasa entre comillas simples precisamente para que el shell NO lo expanda —
# el codigo python tiene que llegar a `python3` con sus `$` intactos, porque
# forma parte de los strings que se comparan con el fichero—. Pasarlos a
# comillas dobles los expandiria aqui y la mutacion buscaria otra cosa, luego
# caeria por el motivo equivocado y el falsador contaria una deteccion que no
# ha medido la propiedad.

# Autofalsacion de la SEGUNDA AUTORIDAD de admision de release.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# `tests/test_release_admission_truncation.sh` declara que una lectura parcial
# del remoto no puede pasar por una version publicada. Ese contrato tiene tres
# dientes que un test verde no demuestra que tenga: el conteo de la huella, la
# reconciliacion entre fuentes, y el cierre duro cuando la segunda fuente no
# esta. Este script quita cada uno por separado y exige que caiga el caso que
# lo vigila.
#
# POR QUE HACE FALTA Y NO BASTA EL TEST EN VERDE
# ---------------------------------------------
# MEDIDO en session-91, con la MISMA suite en verde en las dos versiones:
# antes del crosscheck la suite tambien era verde, y el resolutor real daba
# `v2.9.1` —un baseline de siete versiones mas viejo que el real— en 3 de 10
# corridas con codigo de salida 0. El motivo de que el test anterior no lo
# viera es que sus fixtures nunca produjeron el caso: la lectura truncada que
# se capturo medido (250 de 387 refs, subconjunto exacto, orden distinto) deja
# el maximo intacta dos de cada tres veces. Un guard que solo mide el maximo
# esta verde midiendo la mitad de lo que cree.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`: cada una comprueba por
# sha que el fichero cambio de verdad, y la restauracion se comprueba byte a
# byte. Un parche que no encuentra su texto se contaria como una deteccion y
# el falsador seria verde sobre una base que no vigila nada.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/release_admission.sh"
TEST="$ROOT/tests/test_release_admission_truncation.sh"

PASS=0
FAIL=0
SKIP=0

if [[ ! -f "$LIB" ]]; then
    echo "FAIL: $LIB missing"
    exit 1
fi
if [[ ! -f "$TEST" ]]; then
    echo "FAIL: $TEST missing"
    exit 1
fi

ok()   { PASS=$((PASS + 1)); echo "  [ok]   $1"; }
bad()  { FAIL=$((FAIL + 1)); echo "  [FAIL] $1"; }
skip() { SKIP=$((SKIP + 1)); echo "  [SKIP] $1"; }

sha_of() { sha256sum "$1" | awk '{print $1}'; }
ORIG_SHA="$(sha_of "$LIB")"

# restore_lib — deja el fichero byte-identico a como estaba.
restore_lib() {
    if [[ -f "$LIB.sddk-mutation-backup" ]]; then
        mv -f "$LIB.sddk-mutation-backup" "$LIB"
    fi
}
trap restore_lib EXIT

# mutate <nombre> <python que reemplaza (old, new)>
#
# El payload corre DENTRO del repo y devuelve 0 solo si encontro y sustituyo
# el texto. Devolver 0 sin haber cambiado nada seria un parche que no casa, y
# eso es SKIP, nunca PASS.
mutate() {
    local name="$1" payload="$2" before after
    before="$(sha_of "$LIB")"
    cp -p "$LIB" "$LIB.sddk-mutation-backup"
    if ! (cd "$ROOT" && python3 -c "$payload"); then
        restore_lib
        return 2
    fi
    after="$(sha_of "$LIB")"
    if [[ "$before" == "$after" ]]; then
        restore_lib
        return 2
    fi
    echo "  --- $name (fichero alterado: $([[ "$before" == "$ORIG_SHA" ]] && echo sí || echo NO)) ---"
    return 0
}

# espera_caida <nombre> <ficha del caso que debe caer>
#
# La ficha tiene que desaparecer del informe. Se mide por AUSENCIA del caso,
# no por el codigo de salida global: un caso cae y otro se cuela y el rc global
# no distingue.
#
# Y la ficha tiene que ESTAR en la base. Sin esa comprobacion, una mutacion
# que no tiene nada que ver con el caso diminishes una ficha que ya no estaba
# y contaria como deteccion —el falsador en verde por una ficha que no
# existia—. Por eso `ficha_en_base` se corre una vez sobre el informe sin
# mutar y aborta si alguna ficha no aparece.
espera_caida() {
    local name="$1" ficha="$2" out rc
    out="$(bash "$TEST" 2>&1)"; rc=$?
    if printf '%s' "$out" | grep -qF -- "$ficha"; then
        bad "$name NO cae: '$ficha' sigue en verde (rc=$rc)"
        printf '%s\n' "$out" | tail -10 | sed 's/^/         | /'
        return 1
    fi
    ok "$name cae: '$ficha' ya no aparece en el informe"
    return 0
}

echo "== base: el contrato pasa sin mutacion =="
base_out="$(bash "$TEST" 2>&1)"; base_rc=$?
if [[ "$base_rc" -eq 0 ]] && printf '%s' "$base_out" | grep -qF 'PASS=20 FAIL=0'; then
    ok "la suite base pasa y su total es el declarado (20)"
else
    bad "la suite base no pasa o no da 20: rc=$base_rc"
    printf '%s\n' "$base_out" | tail -12 | sed 's/^/         | /'
fi

# Cada ficha tiene que estar en el informe de la BASE. Una ficha que no esta
# no se puede "hacer desaparecer" y el falsador contaria una deteccion que no
# midio nada.
ficha_en_base() {
    if printf '%s' "$base_out" | grep -qF -- "$1"; then
        ok "la base declara la ficha de '$1'"
    else
        bad "la ficha '$1' NO esta en el informe de la base: la mutacion que la nombre no tendria nada que hacer caer"
    fi
}
echo
echo "== las fichas existen en la base =="
ficha_en_base 'api=3|2.11.0'
ficha_en_base 'api=5|2.99.0'
ficha_en_base 'crosscheck_unavailable:gh-not-in-path'

# ─────────────────────────────────────────────────────────────────────────────
# M1 — la huella compara SOLO el maximo.
#
# Es la mutacion mas importante del fichero y la que explica el defecto medido:
# el conteo es lo que distingue una lista completa de una parcial cuando el
# maximo no se ha movido. Sin el, el caso "la segunda fuente contradice aunque
# el maximo NO se mueva" deja de caer —que es exactamente lo que hacia la
# suite verde sin estar midiendo lo que dice.
# ─────────────────────────────────────────────────────────────────────────────
echo
echo "== M1: la huella se queda con el maximo y tira el conteo =="
if mutate M1 '
import sys
p = "scripts/lib/release_admission.sh"
s = open(p).read()
old = """    printf '"'"'%s|%s\\n'"'"' "${count:-0}" "${max:-}\""""
new = """    printf '"'"'%s|%s\\n'"'"' 0 "${max:-}\""""
if old not in s:
    sys.exit(1)
open(p, "w").write(s.replace(old, new, 1))
'; then
    espera_caida M1 "api=3|2.11.0"
    restore_lib
else
    skip "M1 no se aplico: el texto no casa con el fichero"
fi

# ─────────────────────────────────────────────────────────────────────────────
# M2 — la huella compara SOLO el conteo.
#
# El otro lado del mismo reloj: un maximo mas alto con el mismo numero de tags
# (uno viejo sustituido por uno nuevo) pasaria. Con el conteo de vuelta
# este caso cae, y sin el no.
# ─────────────────────────────────────────────────────────────────────────────
echo
echo "== M2: la huella se queda con el conteo y tira el maximo =="
if mutate M2 '
import sys
p = "scripts/lib/release_admission.sh"
s = open(p).read()
old = """    printf '"'"'%s|%s\\n'"'"' "${count:-0}" "${max:-}\""""
new = """    printf '"'"'%s|%s\\n'"'"' "${count:-0}" 0"""
if old not in s:
    sys.exit(1)
open(p, "w").write(s.replace(old, new, 1))
'; then
    espera_caida M2 "api=5|2.99.0"
    restore_lib
else
    skip "M2 no se aplico: el texto no casa con el fichero"
fi

# ─────────────────────────────────────────────────────────────────────────────
# M3 — no se comprueba que `gh` este.
#
# Sin esta comprobacion el codigo sigue y acaba devolviendo un baseline que
# nadie confirmo. El caso "la segunda fuente no esta: se cierra" deja de caer.
# ─────────────────────────────────────────────────────────────────────────────
echo
echo "== M3: se resuelve igual sin la segunda fuente =="
if mutate M3 '
import sys
p = "scripts/lib/release_admission.sh"
s = open(p).read()
old = """    if ! command -v gh >/dev/null 2>&1; then
        LAST_PUB_OUTCOME="crosscheck_unavailable:gh-not-in-path"
        return 1
    fi"""
if old not in s:
    sys.exit(1)
open(p, "w").write(s.replace(old, "", 1))
'; then
    espera_caida M3 "crosscheck_unavailable:gh-not-in-path"
    restore_lib
else
    skip "M3 no se aplico: el texto no casa con el fichero"
fi

# ─────────────────────────────────────────────────────────────────────────────
# M4 — no se reconcilian las dos huellas.
#
# El crosscheck se desenchufa entero: la API deja de usarse y todo lo que
# queda es la lectura unica con su codigo de salida a 0, que es el defecto
# original.
# ─────────────────────────────────────────────────────────────────────────────
echo
echo "== M4: se resuelve sin reconciliar contra la segunda fuente =="
if mutate M4 '
import sys
p = "scripts/lib/release_admission.sh"
s = open(p).read()
old = """    if [[ "$ls_fp" != "$api_fp" ]]; then
        LAST_PUB_OUTCOME="crosscheck_mismatch:ls-remote=$ls_fp|api=$api_fp"
        return 1
    fi"""
if old not in s:
    sys.exit(1)
open(p, "w").write(s.replace(old, "", 1))
'; then
    espera_caida M4 "api=3|2.11.0"
    restore_lib
else
    skip "M4 no se aplico: el texto no casa con el fichero"
fi

# ─────────────────────────────────────────────────────────────────────────────
# M5 — el resolutor acepta los outcomes del crosscheck.
#
# El crosscheck calcula bien y su veredicto se tira a la basura en la frontera
# de arriba. Todo el trabajo de reconciliar queda en el aire y el resolutor
# vuelve a devolver un baseline sin confirmar, que es el defecto que la
# segunda autoridad pretendia cerrar.
# ─────────────────────────────────────────────────────────────────────────────
echo
echo "== M5: el resolutor acepta lo que el crosscheck cerro =="
if mutate M5 '
import sys
p = "scripts/lib/release_admission.sh"
s = open(p).read()
old = """        crosscheck_unavailable:*|crosscheck_failed:*|crosscheck_mismatch:*) return 1 ;;"""
if old not in s:
    sys.exit(1)
open(p, "w").write(s.replace(old, "", 1))
'; then
    espera_caida M5 "crosscheck_unavailable:gh-not-in-path"
    restore_lib
else
    skip "M5 no se aplico: el texto no casa con el fichero"
fi

# ─────────────────────────────────────────────────────────────────────────────
# M6 — la segunda fuente se cambia por la lista LOCAL de tags.
#
# La variante mas绝缘 de "se confirmo con otra fuente": aqui hay dos
# confirmaciones y ambas son la MISMA lectura. Un remoto truncado y la lista
# local —que solo tiene lo que ya esta descargado— darian el mismo veredicto
# sobre lo ya descargado, que es no dizer nada del remoto.
# ─────────────────────────────────────────────────────────────────────────────
echo
echo "== M6: la segunda fuente es la lista local, no otra fuente =="
if mutate M6 '
import sys
p = "scripts/lib/release_admission.sh"
s = open(p).read()
old = """    if ! api_tags="$(_published_tags_via_api "$slug")"; then"""
new = """    api_tags="$(git tag --list '"'"'v[0-9]*'"'"' | sed '"'"'s/^v//'"'"')"
    if false; then"""
if old not in s:
    sys.exit(1)
open(p, "w").write(s.replace(old, new, 1))
'; then
    espera_caida M6 "api=3|2.11.0"
    restore_lib
else
    skip "M6 no se aplico: el texto no casa con el fichero"
fi

# ─────────────────────────────────────────────────────────────────────────────
# Restauracion byte-identica. Sin esto, una mutacion que seRestore mal y deja
# el fichero cambiado contaminaria la suite que viene despues y el falsador
# seria verde por un orden de ejecucion que nadie eligio.
# ─────────────────────────────────────────────────────────────────────────────
echo
echo "== restauracion byte-identica =="
if [[ "$(sha_of "$LIB")" == "$ORIG_SHA" ]]; then
    ok "el fichero de la lib quedo como estaba"
else
    bad "el fichero de la lib quedo ALTERADO: el falsador mediria sobre una base sucia"
    restore_lib
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[[ "$FAIL" -eq 0 ]] || exit 1
echo "RESULT: PASS — cada comprobacion de la segunda autoridad cae cuando se quita."