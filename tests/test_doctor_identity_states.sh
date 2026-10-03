#!/usr/bin/env bash
# Mide los CUATRO estados del check `binary.build_identity` con binarios REALES
# y el repo REAL. Es la evidencia de la fase verify de cl-doctor-build-identity.
#
# Por que un script y no mediciones a mano: la verify de este ciclo NO consiste
# en repetir la suite, sino en comprobar que los cuatro estados se distinguen
# con el binario de verdad — y sobre todo que el impostor y «sin checkout» NO dan
# rojo. Un rojo falso en `dev doctor` entrena a ignorar los rojos, que es peor
# que no tener check.
#
# Requiere un binario construido con SDDK_GIT_SHA (concluyente) y otro sin ella
# (no concluyente). El script no construye: recibe las dos rutas, porque
# construirlos aqui haria que la medicion dependiera de cuando se ejecuto.
#
# ── MODO DE UN SOLO BINARIO, y por que existe ───────────────────────────────
# MEDIDO: pasando el binario CONCLUYENTE en los dos huecos, O2, O3, O4, O5 y
# O7 pasan y SOLO O6 falla (PASS=17 FAIL=2, las dos aserciones de O6). O6 es
# el unico objetivo que necesita el segundo binario, porque es el unico que
# mide una procedencia no concluyente — las otras cinco comprobaciones dependen
# solo de la identidad concluyente y del checkout contra el que se compara.
#
# Eso importa porque el camino de release YA tiene un binario concluyente: el
# que construye, con SDDK_GIT_SHA exportado. Ejecutar aqui el segundo build
# solo para O6 costaria una compilacion entera mas por publicacion, para medir
# un estado que ademas es el de STOP 6 — el estado en el que el check, por
# diseno, NO decide.
#
# Asi que con un solo binario el guard corre O2-O5 y O7, y DECLARA O6 como
# NOT_RUN con su motivo. No lo cuenta como passed, no lo salta en silencio y no
# relaja el resto: la cuenta de veredictos que O7 verifica baja de 5 a 4, que es
# lo que de verdad se midio. Con dos binarios, todo igual que antes (19).
set -uo pipefail

CONCLUSIVE="${1:?uso: test_doctor_identity_states.sh <binario-concluyente> [binario-no-concluyente]}"
INCONCLUSIVE="${2:-}"
if [ -n "$INCONCLUSIVE" ]; then
    ESPERADOS=5
else
    ESPERADOS=4
fi
REPO="$(cd "$(dirname "$0")/.." && pwd)"
PASS=0
FAIL=0

# Cada veredicto se guarda AL MEDIRSE. O7 no vuelve a ejecutar nada: si los
# directorios temporales ya se han enviado a la basura, un `verdict` repetido
# devolveria el shell mas silencioso del mundo — y O7 pasaria sin haber medido
# ninguno de los cinco veredictos que dice medir.
DETALLES=()
MEDIDOS=0

registrar() {
    DETALLES+=("$1")
    MEDIDOS=$((MEDIDOS + 1))
}

verdict() {
    local bin="$1" dir="$2" out
    out="$(cd "$dir" && "$bin" dev doctor --format json 2>/dev/null)"
    printf '%s' "$out" | python3 -c "
import json,sys
try:
    d=json.load(sys.stdin)
except Exception:
    print('SIN_JSON'); raise SystemExit
for c in d.get('checks',[]):
    if c.get('tool')=='binary.build_identity':
        print(('PRESENT' if c.get('present') else 'MISSING') + '|' + (c.get('detail') or ''))
        break
else:
    print('AUSENTE|')
"
}

check() {
    local name="$1" expect="$2" actual="$3"
    if [ "$expect" = "$actual" ]; then
        echo "  [ok]   $name"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] $name: esperaba '$expect', obtuvo '$actual'"
        FAIL=$((FAIL + 1))
    fi
}

echo "== O2: checkout de sddk-framework, binario al dia =="
V="$(verdict "$CONCLUSIVE" "$REPO")"
registrar "${V#*|}"
check "verde" "PRESENT" "${V%%|*}"
check "y lo dice con OK" "OK" "$(printf '%s' "${V#*|}" | cut -d: -f1)"

echo
echo "== O3: el MISMO binario contra un checkout atrasado =="
ATRAS="$(mktemp -d)"
if git clone -q --no-local "$REPO" "$ATRAS/clone" 2>/dev/null; then
    ( cd "$ATRAS/clone" && git checkout -q HEAD~1 2>/dev/null )
    V="$(verdict "$CONCLUSIVE" "$ATRAS/clone")"
    registrar "${V#*|}"
    check "ROJO" "MISSING" "${V%%|*}"
    check "y lo dice con FALLO" "FALLO" "$(printf '%s' "${V#*|}" | cut -d: -f1)"
    check "y nombra la relacion" "SI" \
        "$(printf '%s' "${V#*|}" | grep -qiE 'ancestro|direccion' && echo SI || echo NO)"
else
    echo "  [FAIL] no se pudo clonar para el escenario de atraso"
    FAIL=$((FAIL + 3))
fi
command -v mavis-trash >/dev/null 2>&1 && mavis-trash -- "$ATRAS" >/dev/null 2>&1

echo
echo "== O5: repo IMPOSTOR — tiene crates/sddk-cli/Cargo.toml de otro paquete =="
IMP="$(mktemp -d)"
(
    cd "$IMP" || exit 1
    git init -q .
    git config user.email t@t
    git config user.name t
    echo x > a && git add -A && git commit -qm i
    mkdir -p crates/sddk-cli
    printf '[package]\nname = "otro-crate-que-no-es-sddk"\nversion = "0.0.0"\n' \
        > crates/sddk-cli/Cargo.toml
    git add -A && git commit -qm impostor
) >/dev/null 2>&1
V="$(verdict "$CONCLUSIVE" "$IMP")"
registrar "${V#*|}"
check "verde" "PRESENT" "${V%%|*}"
check "y en N/A, no en FALLO" "N/A" "$(printf '%s' "${V#*|}" | cut -d: -f1)"
check "y no dice diverged" "NO" \
    "$(printf '%s' "${V#*|}" | grep -qi diverged && echo SI || echo NO)"
command -v mavis-trash >/dev/null 2>&1 && mavis-trash -- "$IMP" >/dev/null 2>&1

echo
echo "== O4: SIN checkout de sddk — directorio que no es repo =="
PLAIN="$(mktemp -d)"
V="$(verdict "$CONCLUSIVE" "$PLAIN")"
registrar "${V#*|}"
check "verde" "PRESENT" "${V%%|*}"
check "y en N/A" "N/A" "$(printf '%s' "${V#*|}" | cut -d: -f1)"
command -v mavis-trash >/dev/null 2>&1 && mavis-trash -- "$PLAIN" >/dev/null 2>&1

echo
if [ -n "$INCONCLUSIVE" ]; then
    echo "== O6: identidad NO CONCLUYENTE (source: git) — STOP 6 no le permite decidir =="
    V="$(verdict "$INCONCLUSIVE" "$REPO")"
    registrar "${V#*|}"
    check "verde" "PRESENT" "${V%%|*}"
    check "y en N/A" "N/A" "$(printf '%s' "${V#*|}" | cut -d: -f1)"
    check "y explica POR QUE no decide" "SI" \
        "$(printf '%s' "${V#*|}" | grep -q 'STOP 6' && echo SI || echo NO)"
else
    # NOT_RUN declarado, no un passed y no un silencio. El motivo esta medido:
    # es el UNICO objetivo que necesita el segundo binario, y el camino de
    # release ya tiene el primero. Contarlo como PASS seria afirmar una medida
    # que no se tomo.
    echo "== O6: identidad NO CONCLUYENTE (source: git) — NOT_RUN =="
    echo "  [NOT_RUN] requiere un binario SIN SDDK_GIT_SHA, y en este modo solo se"
    echo "            paso el concluyente. Es el unico objetivo que depende de el:"
    echo "            MEDIDO, pasando el concluyente en los dos huecos, O2-O5 y O7"
    echo "            pasan y solo O6 falla. O6 es el estado en el que el check, por"
    echo "            diseno (STOP 6), NO decide; la cobertura de este estado es de"
    echo "            sesion, no de pipeline, y queda declarado como tal."
fi

echo
echo "== O7: todos los veredictos traen motivo =="
# Primero que haya muestras, y que sean las que este modo mide ($ESPERADOS): un bucle
# sobre directorios ya
# destruidos pasaria con cero y se declararia conforme. Este aserto es el que
# impide que eso vuelva a pasar — y si un escenario futuro no se registra, falla
# aqui en vez de dejar que O7 mienta en su nombre.
# El recuento de motivos vacios vive en UNA FUNCION, y el test la ejercita
# contra una lista conocida antes de fiarse de ella. Es la forma de cerrar el
# hueco que la falsificacion encontro: anular la aritmetia de un aserto no lo
# delata a nadie, porque el unico guard que dependia de ella era el propio
# aserto anulado. Con la funcion compartida, el meta-aserto usa EL MISMO codigo
# que el aserto real — una copia no vigilaria nada, que es la septima vez que
# esta serie lo paga.
contar_vacios() {
    local n=0 D
    for D in "$@"; do
        [ -n "$D" ] || n=$((n + 1))
    done
    printf '%s' "$n"
}

check "el recuento cuenta de verdad" "1" "$(contar_vacios uno dos '' tres cuatro)"

check "se midieron los $ESPERADOS veredictos (contador)" "$ESPERADOS" "$MEDIDOS"

# El tamaño de la muestra se cuenta por dos mecanismos distintos — el contador de
# `registrar` y las lineas de la lista materializada. Medir dos veces lo mismo es
# lo unico que convierte "han entrado cinco muestras" en algo comprobable.
LINEAS="$(printf '%s\n' ${DETALLES+"${DETALLES[@]}"} | grep -c .)"
check "se midieron los $ESPERADOS veredictos (lineas)" "$ESPERADOS" "$LINEAS"

# Y los motivos vacios, por dos caminos independientes: la funcion de arriba y
# `grep -c '^$'` sobre la lista ya materializada. Anular uno deja al otro en pie.
check "ningun veredicto sin motivo (funcion)" "0" \
    "$(contar_vacios ${DETALLES+"${DETALLES[@]}"})"
check "ningun veredicto sin motivo (grep)" "0" \
    "$(printf '%s\n' ${DETALLES+"${DETALLES[@]}"} | grep -c '^$')"

echo
echo "== el codigo de salida de doctor sigue siendo el de antes =="
# El cambio de contrato declarado es que `all_present == false` da exit 1. Con
# el binario al dia tiene que seguir dando 0, o el doctor habria roto de mas.
( cd "$REPO" && "$CONCLUSIVE" dev doctor >/dev/null 2>&1 ); RC=$?
check "doctor sale 0 con el binario al dia" "0" "$RC"

echo
echo "PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ] || exit 1
