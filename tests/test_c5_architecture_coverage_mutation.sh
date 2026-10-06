#!/usr/bin/env bash
# Autofalsacion de tests/c5-coverage-guard.sh
#
# Un guard con tres dientes puede tener uno flojo y seguir verde por el
# efecto colateral de los otros dos. Cada mutacion rompe UNA cosa y exige que
# el diente que la cubre caiga Y SOLO ese: si M1 (stub) tambien rompe D3, no
# se sabe que D2 tenga dientes, y eso es exactamente lo que paso al construir
# este guard en su primera version, donde D3 decia que sddk-storage no lo
# nombraba ninguna regla siendo que las reglas lo nombran por su nombre corto.
#
# Regla de conteo, la misma del resto del repo: una mutacion que NO se
# aplica es SKIP con su motivo, jamas PASS. Un parche que no cambio nada y se
# cuenta como deteccion hace que la suite certifique un guard que nadie ha
# falsificado nunca.
set -uo pipefail

ROOT="${SDDK_REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
GUARD="${SDDK_C5_GUARD:-$ROOT/tests/test_c5_architecture_coverage.sh}"
EVALUATORS="$ROOT/crates/sddk-engine/src/rules/evaluators.rs"
RULES_REL="docs/history/legacy-packages/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml"
RULES="$ROOT/$RULES_REL"
WORK="$(mktemp -d)"
PASS=0
FAIL=0
SKIP=0

dispose() {
    if command -v mavis-trash >/dev/null 2>&1; then
        mavis-trash -- "$1" >/dev/null 2>&1 || rm -rf "$1"
    else
        rm -rf "$1"
    fi
}
trap 'dispose "$WORK"' EXIT

[ -f "$GUARD" ] || { echo "no existe $GUARD"; exit 2; }
[ -f "$EVALUATORS" ] || { echo "no existe $EVALUATORS"; exit 2; }

GUARD_GUARD="$GUARD"
cp "$GUARD_GUARD" "$WORK/guard.sh.orig"
cp "$EVALUATORS" "$WORK/evaluators.rs.orig"
cp "$RULES"      "$WORK/rules.yaml.orig"
EVAL_SHA_BEFORE="$(sha256sum "$EVALUATORS" | awk '{print $1}')"
RULES_SHA_BEFORE="$(sha256sum "$RULES" | awk '{print $1}')"
GUARD_SHA_BEFORE="$(sha256sum "$GUARD" | awk '{print $1}')"

# Ejecuta el guard y extrae los veredictos de D1, D2 y D3 por separado.
run_guard() {
    SDDK_REPO_ROOT="$ROOT" bash "$GUARD" 2>&1 | tr -d '\r'
}

# La deteccion se mide CONTRA LA BASE, no contra el cero.
#
# La primera version exigia que los otros dos dientes estuvieran en 0, y eso
# es insatisfacible por construccion: el guard esta legitimamente en rojo en
# HEAD, porque narrar la brecha es su trabajo. Con D2 y D3 ya caidos, ninguna
# mutacion podria Attributionarse un solo diente y las tres se declaraban no
# detectadas con el detalle "cayo por otro diente" que en realidad era el suyo
# mas la brecha de fondo.
#
# Lo que hay que exigir es mas fuerte que "este diente cae": este diente TIENE
# QUE CAER MAS que en la base, y los otros dos tienen que quedar EXACTAMENTE
# como estaban. Si una mutacion rompe algo de mas, el recuento la delata.
diente_caio() {
    local out="$1" esperado="$2"
    local n1 n2 n3
    # Se compara el NUMERO de hallazgos, no el boolean rojo/verde. El guard
    # narra la brecha: D2 esta en rojo en la base porque hay 5 stubs, y una
    # mutacion que sube 5 a 6 deja el "rojo" igual de rojo. Contar lineas
    # [FAIL] —que es 0 o 1— hacia que el falsador no viera ninguna de las
    # mutaciones, que si se aplicaban.
    n1="$(printf '%s\n' "$out" | sed -n 's/.*sin evaluador: //p' | wc -w)"
    n2="$(printf '%s\n' "$out" | sed -n 's/.*stubs medidos: *//p' | wc -w)"
    n3="$(printf '%s\n' "$out" | sed -n 's/.*crates sin regla medidos: *//p' | wc -w)"
    printf 'sin_evaluador=%s stubs=%s crates_sin_regla=%s (base %s/%s/%s)\n' \
        "$n1" "$n2" "$n3" "$BASE_N1" "$BASE_N2" "$BASE_N3"
    case "$esperado" in
        D1) [ "$n1" -gt "$BASE_N1" ] && [ "$n2" = "$BASE_N2" ] && [ "$n3" = "$BASE_N3" ] ;;
        D2) [ "$n2" -gt "$BASE_N2" ] && [ "$n1" = "$BASE_N1" ] && [ "$n3" = "$BASE_N3" ] ;;
        D3) [ "$n3" -gt "$BASE_N3" ] && [ "$n1" = "$BASE_N1" ] && [ "$n2" = "$BASE_N2" ] ;;
    esac
}

echo "== base: el guard sobre el sujeto real =="
BASE_OUT="$(run_guard)"
BASE_N1="$(printf '%s\n' "$BASE_OUT" | sed -n 's/.*sin evaluador: //p' | wc -w)"
BASE_N2="$(printf '%s\n' "$BASE_OUT" | sed -n 's/.*stubs medidos: *//p' | wc -w)"
BASE_N3="$(printf '%s\n' "$BASE_OUT" | sed -n 's/.*crates sin regla medidos: *//p' | wc -w)"
echo "  brecha medida en el sujeto: sin_evaluador=$BASE_N1 stubs=$BASE_N2 crates_sin_regla=$BASE_N3"
echo "  (se espera >=1: el guard NARRA la brecha, no la tapa)"

# Restaura por copia propia. `git checkout --` resolveria igual sobre un arbol
# limpio y DESCARTA todo el trabajo sin commitear del sujeto sobre un arbol
# sucio, que es el caso normal: el falsador se corre DESPUES de implementar y
# antes de commitear.
restore() {
    cp "$WORK/evaluators.rs.orig" "$EVALUATORS"
    cp "$WORK/rules.yaml.orig"     "$RULES"
    cp "$WORK/guard.sh.orig"       "$GUARD_GUARD"
}

# ── M1: un stub nuevo, en una regla que hoy si mide ─────────────────────────
# Se convierte ARCH001 (que devuelve Pass/Fail) en un stub NotApplicable. Si
# D2 tiene dientes, cae. Si D2 midiese "menciona el baseline", no caeria: el
# cuerpo seguiria citando baseline_sha256. Esa es la razon de esta mutacion
# sobre el resto.
echo
echo "== M1: ARCH001 pasa a devolver NotApplicable sin mirar =="
restore
python3 - "$EVALUATORS" <<'PY'
import io,re,sys
p=sys.argv[1]
s=io.open(p,encoding='utf-8').read()
start=s.index("fn evaluate_arch001(")
end=s.index("// ── ARCH002", start)
body=s[start:end]
# Hay que quitar la DERIVACION entera, no solo sus ramas: cambiar Pass y Fail
# por NotApplicable dejaba vivo al evaluador (`let status = if ...` seguia
# decidiendo) y el diente no caia, correctamente.
patron = re.compile(r"let status = if violating\.is_empty\(\) \{\s*RuleStatus::\w+\s*\} else \{\s*RuleStatus::\w+\s*\};", re.S)
mutado, n = patron.subn("let status = RuleStatus::NotApplicable;", body)
assert n == 1, f"M1 no aplico: {n} sustituciones (se esperaba 1)"
mutado = mutado.replace("baseline.cross_crate_imports", "Vec::new()")
assert mutado != body
io.open(p,'w',encoding='utf-8').write(s[:start]+mutado+s[end:])
PY
if [ "$(sha256sum "$EVALUATORS" | awk '{print $1}')" = "$EVAL_SHA_BEFORE" ]; then
    echo "  [SKIP] M1 no aplico: el fichero quedo byte-identico"
    SKIP=$((SKIP + 1))
else
    OUT="$(run_guard)"
    echo "  dientes: $(diente_caio "$OUT" D2)"
    if diente_caio "$OUT" D2 >/dev/null; then
        echo "  [ok]   M1 detectado, por D2 y solo por D2"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] M1 NO detectado por D2 (o caiu por otro diente)"
        FAIL=$((FAIL + 1))
    fi
fi
restore

# ── M2: una regla que hoy tiene evaluador pierde su despacho ────────────────
echo
echo "== M2: ARCH001 sale del match del dispatcher =="
restore
sed -i 's|^                "ARCH001" => evaluate_arch001(rule, baseline, evaluated_at),||' "$EVALUATORS"
if cmp -s "$EVALUATORS" "$WORK/evaluators.rs.orig" ; then
    echo "  [SKIP] M2 no aplico: el fichero quedo byte-identico"
    SKIP=$((SKIP + 1))
else
    OUT="$(run_guard)"
    echo "  dientes: $(diente_caio "$OUT" D1)"
    if diente_caio "$OUT" D1 >/dev/null; then
        echo "  [ok]   M2 detectado, por D1 y solo por D1"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] M2 NO detectado por D1 (o caiu por otro diente)"
        FAIL=$((FAIL + 1))
    fi
fi
restore

# ── M3: un crate nuevo que ninguna regla nombra ─────────────────────────────
# El workspace tiene 8 crates y dos no los nombra ya. Esta mutacion no anade un
# crate real: retira el nombre de vault de la regla que lo nombra, que es la
# forma de fingir que el hueco se ha cerrado sin tocar la topologia.
echo
echo "== M3: las reglas dejan de nombrar sddk-vault =="
restore
sed -i 's|vault_must_not_import_storage|boundary_must_not_import_storage|' "$RULES"
if cmp -s "$RULES" "$WORK/rules.yaml.orig" ; then
    echo "  [SKIP] M3 no aplico: el fichero quedo byte-identico"
    SKIP=$((SKIP + 1))
else
    OUT="$(run_guard)"
    echo "  dientes: $(diente_caio "$OUT" D3)"
    if diente_caio "$OUT" D3 >/dev/null; then
        echo "  [ok]   M3 detectado, por D3 y solo por D3"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] M3 NO detectado por D3 (o caiu por otro diente)"
        FAIL=$((FAIL + 1))
    fi
fi
restore

# ── M4: la declaracion de stubs se reduce sin que la deuda se cierre ────────
# El segundo diente de D2. Con el guard en verde por declaracion, quitar un
# stub de DECLARED_STUBS sin cerrarlo es falsear la declaracion: si esto
# pasara, bastaria con borrar una linea para que una deuda real dejara de
# contarse y nadie lo notaria.
echo
echo "== M4: la declaracion pierde un stub que sigue existiendo =="
restore
# La lista se lee del propio guard y se acorta por el ULTIMO elemento, en vez de
# escribirla entera aqui. Escribiendola fija, cada cierre de deuda (que cambia
# la lista) dejaba esta mutacion sin aplicar: SKIP, que no es PASS, luego la
# segunda mitad de D2 se dejaba de ejercitar sola y sin que nadie lo dijera.
CUR_STUBS="$(sed -n 's|^DECLARED_STUBS="\(.*\)"$|\1|p' "$GUARD_GUARD" | head -1)"
if [ -z "$CUR_STUBS" ]; then
    echo "  [FAIL] M4 no pudo leer DECLARED_STUBS: el guard cambio de forma"
    FAIL=$((FAIL + 1))
    restore
else
    # tr separa la lista por espacios en vez de confiar en el word splitting
    # de un $VAR sin comillas: mismo resultado, sin riesgo de globbing.
    SHORT_STUBS="$(printf '%s\n' "$CUR_STUBS" | tr ' ' '\n' | head -n -1 | tr '\n' ' ' | sed 's/ *$//')"
    sed -i "s|^DECLARED_STUBS=\"$CUR_STUBS\"\$|DECLARED_STUBS=\"$SHORT_STUBS\"|" "$GUARD_GUARD"
fi
if cmp -s "$GUARD_GUARD" "$WORK/guard.sh.orig"; then
    echo "  [SKIP] M4 no aplico: el guard quedo byte-identico"
    SKIP=$((SKIP + 1))
else
    OUT="$(run_guard)"
    if printf '%s\n' "$OUT" | grep -q 'ni uno mas ni uno menos.*\[FAIL\]\|\[FAIL\] los stubs son exactamente'; then
        echo "  [ok]   M4 detectado: declarar menos de lo que existe es rojo"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] M4 NO detectado: una declaracion puede encogerse en silencio"
        FAIL=$((FAIL + 1))
    fi
fi
restore

# ── M5: la declaracion de crates gana un crate que si esta regido ───────────
echo
echo "== M5: la declaracion gana un crate que las reglas si nombran =="
restore
sed -i 's|^DECLARED_UNNAMED="sddk-gateway sddk-pack-uat"$|DECLARED_UNNAMED="sddk-gateway sddk-pack-uat sddk-storage"|' "$GUARD_GUARD"
if cmp -s "$GUARD_GUARD" "$WORK/guard.sh.orig"; then
    echo "  [SKIP] M5 no aplico: el guard quedo byte-identico"
    SKIP=$((SKIP + 1))
else
    OUT="$(run_guard)"
    if printf '%s\n' "$OUT" | grep -q '\[FAIL\] los crates sin regla son exactamente'; then
        echo "  [ok]   M5 detectado: declarar de mas es rojo"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] M5 NO detectado: una excepcion puede crecer sin que la regla exista"
        FAIL=$((FAIL + 1))
    fi
fi
restore

# ── M6: control negativo ────────────────────────────────────────────────────
# Con todo restaurado, el guard tiene que volver EXACTAMENTE a su veredicto de
# HEAD. Si una mutacion dejo el arbol movido, este control lo canta. Sin el, un
# guard que se auto-repara entre mutaciones contaria como verde.
echo
echo "== M6: control de restauracion byte-identica =="
if [ "$(sha256sum "$EVALUATORS" | awk '{print $1}')" = "$EVAL_SHA_BEFORE" ] \
   && [ "$(sha256sum "$RULES" | awk '{print $1}')" = "$RULES_SHA_BEFORE" ] \
   && [ "$(sha256sum "$GUARD_GUARD" | awk '{print $1}')" = "$GUARD_SHA_BEFORE" ]; then
    echo "  [ok]   evaluadores.rs, el yaml de reglas y el guard byte-identicos"
    PASS=$((PASS + 1))
else
    echo "  [FAIL] una mutacion dejo el arbol movido: el verde posterior no significa nada"
    FAIL=$((FAIL + 1))
fi

# ── M8: un evaluador deja de poder decir que mira ────────────────────────────
# La propiedad de D5. Quitarle el campo a una ley que hoy lo tiene la convierte
# en verde sin sujeto otra vez, y eso tiene que ser rojo por D5 y SOLO por D5:
# si lo detectara D2 seria porque la ley paso a contarse como stub, que es otro
# defecto, y el guard no debe tapar uno con el otro.
echo
echo "== M8: ARCH011 deja de reportar el tamano de su sujeto =="
restore
python3 - "$EVALUATORS" <<'PY'
import sys, re
p = sys.argv[1]
s = open(p).read()
# ARCH011 delega en el helper compartido, asi que el campo se quita del helper
# y se mide si D5 lo detecta ahi. Un recorte que solo afectara a ARCH011 seria
# mas fiel al titulo, pero exigiria partir el helper en dos y el defecto que
# importa es el mismo: una ley de arista que no puede decir cuanto miro.
before = s.count('"subject_edges": subject_edges')
assert before >= 1, "M8 no aplica: no hay subject_edges que quitar"
s = s.replace('"subject_edges": subject_edges,\n            "measured_nothing": subject_edges == 0,', '')
open(p, 'w').write(s)
PY
if cmp -s "$EVALUATORS" "$WORK/evals.rs.orig"; then
    echo "  [SKIP] M8 no aplico: el evaluador quedo byte-identico"
    SKIP=$((SKIP + 1))
else
    OUT="$(run_guard)"
    if printf '%s\n' "$OUT" | grep -q '\[FAIL\] las leyes sin medicion de sujeto'; then
        if printf '%s\n' "$OUT" | grep -q '\[FAIL\] los stubs son exactamente'; then
            echo "  [FAIL] M8 detectado, pero tapado por D2: un defecto cubierto por dos dientes"
            FAIL=$((FAIL + 1))
        else
            echo "  [ok]   M8 detectado, por D5 y solo por D5"
            PASS=$((PASS + 1))
        fi
    else
        echo "  [FAIL] M8 NO detectado: un evaluador puede dejar de decir que mira"
        FAIL=$((FAIL + 1))
    fi
fi
restore

# ── M5: control de no-vacuidad ──────────────────────────────────────────────
# El guard tiene que DETECTAR la brecha en HEAD, no solo saber contarla. Si
# sobre HEAD saliera todo en verde, su PASS seria la afirmacion de que el gate
# cumple, que es justo lo falso que este guard existe para decir.
echo
echo "== M7: el guard mide una brecha declarada real =="
if [ "$((BASE_N1 + BASE_N2 + BASE_N3))" -ge 1 ]; then
    echo "  [ok]   el guard declara una brecha real (sin_evaluador=$BASE_N1 stubs=$BASE_N2 crates_sin_regla=$BASE_N3)"
    PASS=$((PASS + 1))
else
    echo "  [FAIL] el guard declara cero brecha: la del sujeto no se estaria midiendo"
    FAIL=$((FAIL + 1))
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[ "$FAIL" -eq 0 ] || exit 1
exit 0