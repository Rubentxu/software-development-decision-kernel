#!/usr/bin/env bash
# test_release_final_state_figures.sh — INC-DEBT-072
#
# QUE VIGILA
# Que el paso 15 del release no imprima jamas la cadena "None" en ninguna de
# sus tres cifras, y que cuando no sepa una cifra lo diga con palabras en
# vez de imprimir la ausencia de un dato con formato de dato.
#
# POR QUE EJECUTA LA LIBRERIA Y NO LA COPIA
# Pegar el bloque en el test fue el defecto que INC-DEBT-074 acaba de cerrar
# en el merge del changelog: alli el unico guard del bloque daba verde contra
# una COPIA del codigo viejo, luego no vigilaba el codigo. Este guard
# sourcea scripts/lib/final_state.sh y lo llama. Si alguien reintroduce el
# fallback viejo en la libreria, este guard cae; si lo reintroduce en
# release.sh sin pasar por la libreria, cae el caso de cableado (C10).
#
# POR QUE HAY UN CONTROL (C9)
# Un guard que rechazase todo pasaria su propia falsacion sin vigilar nada.
# C9 exige que el caso bueno se ACEPTE, luego un guard degenerado en
# "siempre FAIL" no puede colarse.
#
# POR QUE HAY UN CASO DE CABLEADO (C10)
# Un guard que mide algo que ningun gate consulta no vigila: informa. Es la
# forma que toma INC-DEBT-065. C10 exige que release.sh sourcee la libreria
# y que el paso 15 la llame, para que lo que este fichero mide sea
# literalmente lo que el operador ve.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/final_state.sh"
RELEASE="$ROOT/scripts/release.sh"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

SANDBOX="$(mktemp -d)"
trap 'rm -rf "$SANDBOX"' EXIT

echo "== test_release_final_state_figures.sh (INC-DEBT-072) =="

# ---------------------------------------------------------------- libreria
if [ ! -f "$LIB" ]; then
    bad "existe scripts/lib/final_state.sh"
    echo
    echo "PASS=$PASS FAIL=$FAIL"
    echo "RESULT: FAIL — no hay codigo que vigilar."
    exit 1
fi
# shellcheck source=../scripts/lib/final_state.sh
# shellcheck disable=SC1091
. "$LIB"

# Un caso por recibo sintetico. `out` es lo que imprimiria el paso 15.
#
# SIN BORRADO, y no por descuido: `rm -f` en este entorno es un shim que
# IMPRIME en stdout ("mavis-trash: moved to trash: ..."), luego cualquier
# `$(...)` que lo contenga se traga ese ruido y todas las aserciones que
# comparan con igualdad exacta caen. MEDIDO al escribir este guard: 5 de 12
# caian por eso y ninguna por el codigo vigilado. Cada caso usa su propio
# fichero, con un contador, en vez de uno comun que se borra entre casos.
RECEIPT_N=0
bundle_of() { # bundle_of <json o "" si no existe>
    RECEIPT_N=$((RECEIPT_N + 1))
    local f="$SANDBOX/receipt-$RECEIPT_N.json"
    : > "$f"
    [ -n "$1" ] && printf '%s' "$1" > "$f"
    final_state_bundle_version "$f"
}

echo
echo "-- las tres cifras nunca imprimen la cadena None --"

# C1 — EL CASO REAL, medido sobre el recibo de 2.10.0.
out="$(bundle_of '{"version":"2.10.0","bundle":true,"bundle_version":null,"layout":"flat"}')"
if [ "$out" = "2.10.0" ]; then
    ok "C1: flat + bundle_version null imprime la version del binario, no None"
else
    bad "C1: flat + bundle_version null deberia dar 2.10.0 — obtenido: '$out'"
fi

# C2 — layout versionado: el recibo declara cual es y se respeta.
out="$(bundle_of '{"version":"2.10.0","bundle":true,"bundle_version":"2.9.9","layout":"versioned"}')"
if [ "$out" = "2.9.9" ]; then
    ok "C2: layout versionado imprime el bundle_version declarado"
else
    bad "C2: layout versionado deberia dar 2.9.9 — obtenido: '$out'"
fi

# C3 — la clave AUSENTE. El fallback viejo aqui SI disparaba ("?"), luego
# este caso es el que el sentinel viene a sustituir por algo legible.
out="$(bundle_of '{"version":"2.10.0","bundle":true}')"
if [ "$out" = "$FINAL_STATE_UNDECLARED" ]; then
    ok "C3: clave ausente dice lo que no sabe, no un '?'"
else
    bad "C3: clave ausente deberia dar '$FINAL_STATE_UNDECLARED' — obtenido: '$out'"
fi

# C4 — null SIN layout flat: no se puede atribuir la version del binario,
# luego no se inventa. Este caso separa "se que es flat" de "no lo se".
out="$(bundle_of '{"version":"2.10.0","bundle":true,"bundle_version":null,"layout":"otro"}')"
if [ "$out" = "$FINAL_STATE_UNDECLARED" ]; then
    ok "C4: null sin layout flat no se atribuye la version del binario"
else
    bad "C4: null sin layout flat deberia dar '$FINAL_STATE_UNDECLARED' — obtenido: '$out'"
fi

# C5 — recibo corrupto.
out="$(bundle_of 'esto no es json')"
if [ "$out" = "$FINAL_STATE_UNDECLARED" ]; then
    ok "C5: recibo ilegible dice lo que no sabe"
else
    bad "C5: recibo ilegible deberia dar '$FINAL_STATE_UNDECLARED' — obtenido: '$out'"
fi

# C6 — recibo inexistente.
out="$(bundle_of '')"
if [ "$out" = "$FINAL_STATE_UNDECLARED" ]; then
    ok "C6: recibo ausente dice lo que no sabe"
else
    bad "C6: recibo ausente deberia dar '$FINAL_STATE_UNDECLARED' — obtenido: '$out'"
fi

# C7 — flat + null + sin version: la rama flat no puede imprimir "" a secas,
# que es la misma clase de defecto (un dato que no es un dato).
out="$(bundle_of '{"bundle":true,"bundle_version":null,"layout":"flat"}')"
if [ "$out" = "$FINAL_STATE_UNDECLARED" ]; then
    ok "C7: flat sin version tampoco imprime una cadena vacia"
else
    bad "C7: flat sin version deberia dar '$FINAL_STATE_UNDECLARED' — obtenido: '$out'"
fi

# C8 — EL INVARIANTE, comprobado sobre la serie entera y no caso por caso.
# Se recorre de nuevo porque una invariante que solo se deduce de C1..C7 no
# esta vigilada: basta con que uno de esos siete casos se anada para que la
# invariante deje de mirarse.
leaked=0
for payload in \
    '{"version":"2.10.0","bundle":true,"bundle_version":null,"layout":"flat"}' \
    '{"version":"2.10.0","bundle":true,"bundle_version":"2.9.9","layout":"versioned"}' \
    '{"version":"2.10.0","bundle":true}' \
    '{"version":"2.10.0","bundle":true,"bundle_version":null,"layout":"otro"}' \
    'esto no es json' \
    '' \
    '{"bundle":true,"bundle_version":null,"layout":"flat"}'
do
    o="$(bundle_of "$payload")"
    if [ "$o" = "None" ] || [ "$o" = "null" ] || [ -z "$o" ]; then
        leaked=$((leaked + 1))
        echo "         filtrado desde: $o"
    fi
done
if [ "$leaked" -eq 0 ]; then
    ok "C8: ninguna de las 7 formas imprime None, null ni cadena vacia"
else
    bad "C8: $leaked de 7 formas imprimen una ausencia con formato de dato"
fi

# ---------------------------------------------------------------- control
echo
echo "-- control: el guard tiene que poder ACEPTAR --"

# C9 — el caso bueno se acepta. Sin esto, un guard que rechaza todo pasaria
# la falsacion entera sin haber vigilado nada.
# El recibo es REAL y bien formado (flat, el caso de 2.10.0), no uno
# inexistente: un control que pasa porque la libreria no encuentra el
# fichero no prueba que sepa aceptar un recibo bueno.
RECEIPT_N=$((RECEIPT_N + 1))
good="$SANDBOX/receipt-$RECEIPT_N.json"
printf '%s' '{"version":"2.10.0","bundle":true,"bundle_version":null,"layout":"flat"}' > "$good"
verdict=0
got_good="$(final_state_bundle_version "$good")" || verdict=$?
if [ "$verdict" -eq 0 ] && [ "$got_good" = "2.10.0" ]; then
    ok "C9: un recibo flat bien formado se ACEPTA y dice 2.10.0 (el guard no es un no-constante)"
else
    bad "C9: un recibo flat valido deberia dar 2.10.0 y salir 0 — obtenido '$got_good' (rc=$verdict)"
fi

# ---------------------------------------------------------------- cableado
echo
echo "-- cableado: lo que este guard mide es lo que el operador ve --"

# C10a — release.sh sourcea la libreria DE VERDAD.
# El patron exige que la linea TERMINE en el .sh. MEDIDO: con un grep
# laxo (`lib/final_state.sh` a secas) la asercion aceptaba una linea que
# sourcea `lib/final_state.sh.disabled` — es decir, daba verde con la
# libreria apagada, que es el fallo mas caro posible en un check de
# cableado. Lo que un check de cableado tiene que probar es que el codigo
# se esta EJECUTANDO.
if grep -qE '^[[:space:]]*\.[[:space:]]+[^#]*lib/final_state\.sh"?[[:space:]]*$' "$RELEASE"; then
    ok "C10a: release.sh sourcea scripts/lib/final_state.sh en una linea real"
else
    bad "C10a: release.sh NO sourcea la libreria — este guard no mediria lo que se imprime"
fi

# C10b — el paso 15 LLAMA a la funcion, y no la menciona.
# MEDIDO: con un grep de token a secas (`final_state_figures` en cualquier
# parte del fichero) la asercion daba verde aunque la llamada no existiera,
# porque el nombre aparece dentro de un comentario y dentro del nombre de un
# test. Un check de cableado que probo la PROSA no prueba el cableado, asi
# que se exige una linea de CODIGO: la llamada, con su argumento.
if grep -qE '^[[:space:]]*final_state_figures[[:space:]]+"\$' "$RELEASE"; then
    ok "C10b: el paso 15 LLAMA a final_state_figures, no la menciona"
else
    bad "C10b: el paso 15 no llama a final_state_figures — la libreria seria codigo muerto"
fi

# C10c — el fallback viejo NO puede seguir en release.sh. MEDIDO: la primera
# version de esta asercion buscaba el patron en TODO el fichero, asi que un
# comentario que lo describiera la hacia fallar a si mismo — que es lo que
# paso, y es la misma clase que el SC1073 que casi impide a este bloque
# entrar. Ahora se exige que el patron este en una linea de CODIGO.
if grep -qE '^[[:space:]]*[^#]*get\("bundle_version", "\?"' "$RELEASE"; then
    bad "C10c: el fallback viejo (.get con default) ha vuelto a release.sh"
else
    ok "C10c: el fallback .get(clave, \"?\") no esta en el codigo de release.sh"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — el paso 15 no imprime la ausencia de un dato con forma de dato."
    exit 0
fi
echo "RESULT: FAIL — quedan $FAIL asercion(es) caidas."
exit 1
