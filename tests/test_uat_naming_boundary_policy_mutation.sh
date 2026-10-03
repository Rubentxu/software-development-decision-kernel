#!/usr/bin/env bash
# Autofalsacion de tests/test_uat_naming_boundary_policy.sh.
#
# DOS REGLAS QUE ESTE SCRIPT HACE CUMPLIR, Y QUE SON SU MOTIVO DE EXISTIR
# ----------------------------------------------------------------------
# 1. Cada mutacion corrompe UNA sola cosa, y hay que ver caer al guard. Una
#    mutacion compuesta no puede decir cual de las comprobaciones cayo por efecto
#    colateral, que es como se declara un guard que vigila menos de lo que cree.
# 2. UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`. La primera version de
#    este falsificador reporto M3 como detectada cuando su mutacion no habia
#    encontrado el texto --el veto se habia movido dentro de `escanear()`-- y lo
#    unico que cayo fue M1. Se declaro satisfecho midiendo otra cosa. Cada
#    mutacion aqui comprueba por sha que el fichero cambio antes de exigir nada.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
G="$ROOT/tests/test_uat_naming_boundary_policy.sh"
F="$ROOT/crates/sddk-gateway/tests/aiw_s7a_producer_l0.rs"
SHA_G="$(sha256sum "$G" | cut -d' ' -f1)"
SHA_F="$(sha256sum "$F" | cut -d' ' -f1)"
PASS=0
FAIL=0
BAK="$(mktemp -d)"
cp "$G" "$BAK/g.sh"
cp "$F" "$BAK/f.rs"
restore() { cp "$BAK/g.sh" "$G"; cp "$BAK/f.rs" "$F"; }
trap 'restore; rm -rf "$BAK"' EXIT

ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

run() { bash "$G" 2>&1 | awk '/^PASS=/{v=$0} END{print (v=="" ? "PASS=? FAIL=?" : v)}'; }

# Aplica una mutacion y EXIGE que el fichero haya cambiado. Sin esta comprobacion
# un texto que ya no existe hace que la mutacion no haga nada, el guard no caiga
# por su culpa, y el paso se reporte como deteccion.
muta() {  # muta <ruta> <python-inline>
    local ruta="$1" codigo="$2" antes despues
    antes="$(sha256sum "$ruta" | cut -d' ' -f1)"
    if ! python3 -c "$codigo"; then
        bad "SKIP: la mutacion no se pudo aplicar sobre ${ruta##*/}; no cuenta como deteccion"
        restore
        return 2
    fi
    despues="$(sha256sum "$ruta" | cut -d' ' -f1)"
    if [[ "$antes" == "$despues" ]]; then
        bad "SKIP: la mutacion sobre ${ruta##*/} no cambio el fichero; no cuenta como deteccion"
        restore
        return 2
    fi
    return 0
}

# Los cuatro nombres falsos de AIW-S7a, tal como estaban antes del rename.
NombresFalsos='
import io
P="crates/sddk-gateway/tests/aiw_s7a_producer_l0.rs"
s=io.open(P,encoding="utf-8").read()
REV={"cognicode_shaped_event_dispatches_registered_rule":"cognicode_finding_e2e",
     "chronos_shaped_crash_dispatches_registered_rule":"chronos_crash_e2e",
     "chronos_shaped_race_produces_no_signal":"chronos_race_e2e",
     "unknown_event_produces_no_signal":"unknown_event_e2e"}
n=0
for a,b in REV.items():
    if s.count("fn %s("%a)==1:
        s=s.replace("fn %s("%a,"fn %s("%b); n+=1
if n!=4: raise SystemExit("se esperaban 4 nombres y se substituyeron %d"%n)
io.open(P,"w",encoding="utf-8").write(s)
'

echo "=== autofalsacion del guard de la regla 4 ==="

BASE="$(run)"
if [[ "$BASE" == "PASS=9 FAIL=0" ]]; then
    ok "base: el guard esta verde con sus nueve comprobaciones ($BASE)"
else
    bad "base: se esperaba PASS=9 FAIL=0 y se obtuvo $BASE -- no se puede falsificar sobre una base no verde"
fi

# M1: los cuatro sufijos prohibidos vuelven. Es la falsificacion que importa: el
# guard tiene que ver el defecto REAL que existia en el repo.
if muta "$F" "$NombresFalsos"; then
    R="$(run)"
    if [[ "$R" != "$BASE" ]]; then
        ok "M1 los cuatro sufijos prohibidos vuelven -> cae ($R)"
    else
        bad "M1 el guard no cae con los cuatro nombres falsos puestos: no vigila el defecto real"
    fi
fi
restore

# M2: la lectura por SUBCADENA. Es el bug que la falsificacion original destapo:
# "PROCESS" es subcadena de "IN_PROCESS", luego la fila se saltaba entera.
if muta "$G" '
import io
P="tests/test_uat_naming_boundary_policy.sh"
s=io.open(P,encoding="utf-8").read()
viejo = "    return bool(boundary_tokens(frontera) & niveles)"
assert s.count(viejo)==1, "no encontrado: %d" % s.count(viejo)
io.open(P,"w",encoding="utf-8").write(s.replace(viejo,"    return any(n in frontera for n in niveles)"))
'; then
    R="$(run)"
    if [[ "$R" != "$BASE" ]]; then
        ok "M2 lectura por subcadena (PROCESS dentro de IN_PROCESS) -> cae ($R)"
    else
        bad "M2 la lectura por subcadena no la detecta nadie: el guard se apaga en silencio"
    fi
fi
restore

# M3: el veto se desconecta. COMPUESTA A PROPOSITO y es la minima necesaria: un
# veto apagado es invisible si no hay nada que vetar. Sin los nombres falsos
# debajo, el guard se queda VERDE con el veto desconectado, porque sus controles
# miden el EXTRACTOR y la CLASIFICACION, no el VETO. La atribucion sigue limpia:
# respecto de M1 lo unico que ha cambiado es la condicion del veto.
if muta "$F" "$NombresFalsos" && muta "$G" '
import io
P="tests/test_uat_naming_boundary_policy.sh"
s=io.open(P,encoding="utf-8").read()
viejo="                if n.endswith(SUFIJOS_PROHIBIDOS):"
assert s.count(viejo)==1, "el veto ya no esta aqui: %d" % s.count(viejo)
io.open(P,"w",encoding="utf-8").write(s.replace(viejo,"                if False:"))
'; then
    R="$(run)"
    if [[ "$R" != "$BASE" ]]; then
        ok "M3 M1 + veto desconectado -> cae ($R): el veto es load-bearing, no decoracion"
    else
        bad "M3 el veto se puede apagar y el guard sigue verde"
    fi
fi
restore

# M4: exigir() deja de mirar los niveles leidos de la autoridad.
if muta "$G" '
import io
P="tests/test_uat_naming_boundary_policy.sh"
s=io.open(P,encoding="utf-8").read()
viejo = "    return bool(boundary_tokens(frontera) & niveles)"
assert s.count(viejo)==1, "no encontrado: %d" % s.count(viejo)
io.open(P,"w",encoding="utf-8").write(s.replace(viejo,"    return bool(boundary_tokens(frontera))"))
'; then
    R="$(run)"
    if [[ "$R" != "$BASE" ]]; then
        ok "M4 exigir() deja de mirar los niveles -> cae ($R)"
    else
        bad "M4 exigir() puede ignorar la autoridad sin que caiga nada"
    fi
fi
restore

# M5: el extractor de nombres deja de encontrar los tests.
if muta "$G" '
import io
P="tests/test_uat_naming_boundary_policy.sh"
s=io.open(P,encoding="utf-8").read()
viejo="    return re.findall(r'"'"'#\\[test\\]\\s*\\n(?:#\\[[^\\]]*\\]\\s*\\n)*fn\\s+([a-z0-9_]+)'"'"',"
assert s.count(viejo)==1, "no encontrado: %d" % s.count(viejo)
io.open(P,"w",encoding="utf-8").write(s.replace(viejo,"    return re.findall(r'"'"'ZZZ_NO_EXISTE_([a-z0-9_]+)'"'"',"))
'; then
    R="$(run)"
    if [[ "$R" != "$BASE" ]]; then
        ok "M5 el extractor de nombres deja de encontrar tests -> cae ($R)"
    else
        bad "M5 un extractor roto que no ve ningun test daria verde siempre"
    fi
fi
restore

# M6: el conjunto exigente deja de LEERSE de la autoridad y se sustituye por uno
# inventado. Es la comprobacion de que la autoridad manda: si el guard puede
# dejar de mirarla, entonces no es una autoridad sino una sugerencia.
if muta "$G" '
import io
P="tests/test_uat_naming_boundary_policy.sh"
s=io.open(P,encoding="utf-8").read()
viejo="    i = spec.find(SEC_HEADING)"
assert s.count(viejo)==1, "no encontrado: %d" % s.count(viejo)
io.open(P,"w",encoding="utf-8").write(s.replace(viejo,"    i = spec.find(\"### Una seccion que no existe\")"))
'; then
    R="$(run)"
    if [[ "$R" != "$BASE" ]]; then
        ok "M6 el conjunto exigente deja de leerse de la autoridad -> cae ($R)"
    else
        bad "M6 el guard puede dejar de mirar la autoridad y seguir verde"
    fi
fi
restore

# M7: la extraccion de tokens deja de cortar en el parentesis, con lo que la
# prosa (`SHA-256`, `BLOCKED`, `PASS`) vuelve a contarse como nivel. Es el bug que
# esta comprobacion nacio para evitar, y la razon de que `boundary_tokens()` este
# en `politica.py` y no en linea dentro de un heredoc.
if muta "$G" '
import io
P="tests/test_uat_naming_boundary_policy.sh"
s=io.open(P,encoding="utf-8").read()
viejo="    cabeza = celda.split(\"(\", 1)[0]"
assert s.count(viejo)==1, "no encontrado: %d" % s.count(viejo)
io.open(P,"w",encoding="utf-8").write(s.replace(viejo,"    cabeza = celda"))
'; then
    R="$(run)"
    if [[ "$R" != "$BASE" ]]; then
        ok "M7 la prosa del parentesis vuelve a contarse como nivel -> cae ($R)"
    else
        bad "M7 extraer del parentesis hacia atras no lo detecta nadie"
    fi
fi
restore

echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ $FAIL -eq 0 ]]; then
    echo "RESULT: PASS -- las 7 mutaciones caen, ninguna se salto, y el guard quedo verde."
else
    echo "RESULT: FAIL -- alguna mutacion no cayo, o no llego a aplicarse."
fi
[[ $FAIL -eq 0 ]] || exit 1

# ── restauracion VERIFICADA, no supuesta ────────────────────────────────────
echo
for que in "$G:$SHA_G" "$F:$SHA_F"; do
    ruta="${que%%:*}"; esperado="${que##*:}"
    actual="$(sha256sum "$ruta" | cut -d' ' -f1)"
    if [[ "$actual" == "$esperado" ]]; then
        ok "restaurado byte-identico: ${ruta#$ROOT/}"
    else
        bad "NO restaurado byte-identico: ${ruta#$ROOT/}"
        exit 1
    fi
done
