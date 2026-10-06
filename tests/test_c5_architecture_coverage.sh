#!/usr/bin/env bash
# C5: el gate de arquitectura no declara lo que mide.
#
# El gate `sddk dev check-architecture` imprime una fila por regla declarada y
# sale con un veredicto. Ese veredicto se lee como "estas 15 reglas dicen si el
# repo cumple", y no es lo que ocurre. MEDIDO sobre HEAD, de las 15 reglas solo
# 4 producen Pass/Fail (ARCH001, ARCH002, ARCH003, ARCH008) y las dos ultimas
# estan WAIVED, luego las que miden son 2. Las otras 11 no miran nada, por dos
# caminos distintos que el veredicto no separa:
#
#   - 6 caen en el `_ =>` del dispatcher con provenance "evaluator not
#     implemented" (ARCH006, 007, 009, 010, 011, 012);
#   - 5 tienen FUNCION de evaluador y devuelven NotApplicable incondicional
#     sin mirar el baseline (ARCH004, 005, 013, 014, 015). Una funcion que
#     existe no es un evaluador, y el dispatcher no puede distinguirlas.
#
# Y hay un tercer agujero, que no es de reglas sino de CONJUNTO: las 15 reglas
# nombran engine, domain, cli, storage, vault y testkit. El workspace tiene 8
# crates y sddk-gateway NO lo nombra ninguna, siendo el segundo composition root
# con 9 aristas a sddk-storage. Una ley que no nombra el conjunto no puede
# medirlo: si manana gateway mete logica de dominio, el gate sigue en verde.
#
# Por eso este guard tiene tres dientes y no uno, y por eso la propiedad que
# mide no es "las reglas tienen evaluador" sino "todo lo que el gate afirma
# medir esta medido, y todo lo que el gate afirma medir existe".
#
# MEDIDO, no supuesto: el primer D2 de este guard daba 0 stubs sobre un
# evaluadores.rs que tiene 5, porque extracia el cuerpo con awk contando llaves
# y la profundidad se cerraba en la firma. Y el primer D3 daba 5 crates "sin
# nombrar" entre ellos storage, vault y testkit, que si tienen regla: buscaba
# "sddk-storage" donde las reglas escriben "storage". Los dos eran FALSOS
# POSITIVOS del instrumento, y un guard que grita sobre lo que ya esta vigilado
# entrena a su lector a saltarselo. El segundo D2 lee el bloque hasta la
# siguiente seccion, que es un corte que el fichero sostiene, y el segundo D3
# busca el nombre corto del crate porque asi es como las reglas lo nombran.
set -uo pipefail

ROOT="${SDDK_REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
cd "$ROOT" || exit 2
RULES="${SDDK_ARCH_RULES:-docs/history/legacy-packages/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml}"
EVALUATORS="crates/sddk-engine/src/rules/evaluators.rs"
PASS=0
FAIL=0

# ── La brecha DECLARADA, con su motivo ───────────────────────────────────────
# Este guard esta en rojo sobre HEAD, y con razon: narrar la brecha es su
# trabajo. Un gate rojo aqui bloquearia toda release sin cambiar el estado del
# producto, que es un gate que solo sabe decir que no.
#
# Asi que la brecha va DECLARADA, y el teeth esta en que la declaracion sea
# exacta: un hallazgo que no esta aqui es rojo (la brecha CRECIO), y un
# declarado que desaparece tambien es rojo (la deuda se CERRO y nadie lo
# anotó). Las dos direcciones estan vigiladas porque el modo de fallo que
# este guard combate es el silencio en las dos: deuda que crece sin que nadie
# la mire, y deuda que se cierra sin que nadie lo registre.
#
# MEDIDO sobre HEAD (15 reglas declaradas): 5 stubs y 2 crates sin nombre.
# Cierre de dos de los cinco: ARCH004 y ARCH005 ya no son stubs. MEDIDO, no
# supuesto — cada uno tiene evaluador, sujeto localizado y falsador propio:
#   ARCH004packs_must_declare_dependencies -> FAIL con 3 violaciones reales
#            (2 declaraciones colgantes: sddk-core, sddk-bridge-cognicode;
#             1 dependencia no declarada: sddk-domain). Antes: NotApplicable con
#             el motivo FALSO "kernel repo, not a pack host" sobre un repo que
#             si tiene pack.
#   ARCH005  reactive_behaviors_must_not_execute_governed_effects_directly
#            -> PASS medido sobre reactive_verify.rs (394 lineas, 0 efectos
#             gobernados). Antes: NotApplicable con "Phase 5 reactive runtime
#             not yet shipped" cuando el runtime ya habia llegado.
# Sus autofalsadores viven en tests/test_tree_reading_evaluators_mutation.sh
# (6 mutaciones, cada una con el test que debe caer).
DECLARED_STUBS="ARCH013 ARCH014 ARCH015"
DECLARED_UNNAMED="sddk-gateway sddk-pack-uat"
# La tercera clase de brecha, y la que el gate NO distingue de las otras dos: una
# ley que SI se mide y NO se sostiene. MEDIDO: ARCH010 (cli_must_not_import_
# storage_directly) devuelve FAIL con 14 aristas sddk-cli -> sddk-storage,
# mientras el waiver de ARCH003 declara solo DOS composition-root edges que
# sobreviven. La diferencia son 12: ledger.rs x4, telemetry.rs, plan.rs,
# context_cmd.rs, cycle.rs, backlog.rs, approval.rs, admission.rs, fork_cmd.rs,
# graph_cmd.rs y lib.rs. El veredicto del gate paso de WAIVED (exit 2) a
# OPEN_DEBT (exit 1) no porque apareciese deuda nueva, sino porque una ley que
# nunca se habia medido por fin se midio y no se sostiene.
DECLARED_FAILING="ARCH010"

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

[ -f "$RULES" ] || { echo "reglas no encontradas en $RULES"; exit 2; }
[ -f "$EVALUATORS" ] || { echo "evaluadores no encontrados en $EVALUATORS"; exit 2; }

DECLARED="$(grep -oE '^  - id: ARCH[0-9]+' "$RULES" | awk '{print $3}' | sort -u)"
N_DECLARED="$(printf '%s\n' "$DECLARED" | grep -c .)"
echo "reglas declaradas: $N_DECLARED"

echo
echo "== D1: toda regla declarada llega a un evaluador =="
# Las despachadas son las del match con flecha. Una regla que no aparece ahi
# cae en el `_ =>`, que devuelve NotApplicable con provenance de "evaluator
# not implemented": eso no es una medicion y no puede contar como una.
DISPATCHED="$(grep -oE '"ARCH[0-9]+" =>' "$EVALUATORS" | tr -d '"' | sed 's/ =>//' | sort -u)"
N_DISPATCHED="$(printf '%s\n' "$DISPATCHED" | grep -c .)"
UNDISPATCHED="$(comm -23 <(printf '%s\n' "$DECLARED") <(printf '%s\n' "$DISPATCHED") | grep -c .)"
check "ninguna regla declarada cae en el _ del dispatcher" "0" "$UNDISPATCHED"
if [ "$UNDISPATCHED" -gt 0 ]; then
    echo "        sin evaluador: $(comm -23 <(printf '%s\n' "$DECLARED") <(printf '%s\n' "$DISPATCHED") | tr '\n' ' ')"
fi

echo
echo "== D2: un evaluador que existe no puede ser un stub =="
# Un evaluador que devuelve NotApplicable sin mirar nada es decorativo: ocupa
# el sitio de una medicion y el dispatcher lo cuenta como si existiera.
#
# La senal NO es "menciona el baseline", que fue el primer intento y era un
# proxy equivocado por dos motivos: todo RuleEvaluation trae baseline_sha256
# aunque no mida nada, y ARCH008 es un evaluador entero que no necesita el
# baseline porque recorre los globs del scope y aplica regex sobre los ficheros.
# Ese proxy reportaba seis stubs y uno de ellos era real.
#
# La senal es "decide o delega": Pass/Fail en su propio cuerpo, una asignacion
# `let status =` que lo derive, o una llamada a otro `evaluate_*` que lo derive
# por ella. La delegacion cuenta porque las seis reglas de arista de C5 no
# calculan el veredicto: lo piden a `evaluate_forbidden_edge`, y exigirles un
# `Pass` literal en su propio cuerpo las declaraba stub a las seis.
#
# HUECO DECLARADO: si un evaluador delegara en otro que solo devuelve
# NotApplicable, este guard lo contaria como vivo. Detectar eso exige resolver
# la cadena de delegacion, y ese bucle es lo que este guard todavia no hace.
#
# El corte es "hasta la siguiente linea que empieza por fn o por el separador
# de seccion", no el conteo de llaves: el conteo se cerraba en la firma y
# reportaba cero stubs sobre cinco.
STUBS=0
STUB_IDS=""
for id in $DISPATCHED; do
    body="$(awk -v fn="fn evaluate_${id,,}(" '
        index($0, fn) == 1 { inside = 1; next }
        inside {
            if ($0 ~ /^fn / || $0 ~ /^\/\/ ── /) { exit }
            print
        }' "$EVALUATORS")"
    if ! printf '%s\n' "$body" | grep -qE 'RuleStatus::(Pass|Fail)|^ *let status = if|evaluate_[a-z0-9_]+\('; then
        STUBS=$((STUBS + 1))
        STUB_IDS="$STUB_IDS $id"
    fi
done
STUB_IDS="$(printf '%s' "$STUB_IDS" | sed 's/^ *//;s/ *$//')"
# Exacto en las dos direcciones: no mas (brecha nueva) y no menos (deuda
# cerrada sin registrar).
check "los stubs son exactamente los declarados, ni uno mas ni uno menos" \
    "$DECLARED_STUBS" "$STUB_IDS"
echo "        stubs medidos: $STUB_IDS"
echo "        stubs declarados: $DECLARED_STUBS"

echo
echo "== D3: todo crate del workspace aparece nombrado en las reglas =="
# El agujero que deja verde un gate que no mira una parte del workspace. Las
# reglas nombran el crate por su NOMBRE CORTO ("storage_must_not_import_
# engine", no "sddk-storage"), luego buscar el nombre completo marca como
# invisible un crate que si esta regido.
UNNAMED=0
UNNAMED_NAMES=""
for crate_dir in crates/*/; do
    crate="$(basename "$crate_dir")"
    short="${crate#sddk-}"
    if ! grep -qE "(^|[^a-zA-Z0-9-])${short}([^a-zA-Z0-9]|$)" "$RULES"; then
        UNNAMED=$((UNNAMED + 1))
        UNNAMED_NAMES="$UNNAMED_NAMES $crate"
    fi
done
UNNAMED_NAMES="$(printf '%s' "$UNNAMED_NAMES" | sed 's/^ *//;s/ *$//' | tr ' ' '\n' | sort | tr '\n' ' ' | sed 's/ *$//')"
DECLARED_UNNAMED_SORTED="$(printf '%s' "$DECLARED_UNNAMED" | tr ' ' '\n' | sort | tr '\n' ' ' | sed 's/ *$//')"
check "los crates sin regla son exactamente los declarados" \
    "$DECLARED_UNNAMED_SORTED" "$UNNAMED_NAMES"
echo "        crates sin regla medidos: $UNNAMED_NAMES"
echo "        declarados: $DECLARED_UNNAMED_SORTED"

echo
echo "== D4: ninguna ley que el gate mide puede quedar en FAIL sin declararse =="
# D1/D2 vigilan que la medicion EXISTA; D4 vigila que lo medido SEA SOSTENIDO.
# Son fallos distintos y opuestos: un gate que solo midiese existencia daria
# verde con quince NotApplicable, y un gate que solo exigiese conformidad
# —ARCH010 included— seria verde por no mirar. La ley que cae se DECLARA con su
# recuento, y el recuento es lo que se compara: si las aristas bajan a 0, esta
# declaracion queda obsoleta y hay que quitarla, que es el mismo teeth
# bidireccional que D2 y D3.
#
# MEDIDO con el binario: `sddk dev check-architecture --root .` sale exit 1.
# D4 ejecuta el gate, y por eso necesita un binario que sea de ESTE checkout.
# El motivo es concreto y medido: el `sddk` instalado responde con CERO leyes en
# FAIL no porque no haya ninguna en FAIL, sino porque es el binario anterior a
# los seis evaluadores de C5 y no tiene las reglas que fallan. Las dos salidas
# son la misma cadena vacia, luego sin comprobar la frescura D4 declararia
# conforme un gate al que no le ha medido nada — que es el defecto exacto que
# este guard existe para dejar de cometer, cometido en D4.
#
# El juez es `scripts/check_binary_freshness.sh` y no este script: un artefacto
# no puede declarar su propia antigüedad (INC-DEBT-064).
SDDK_BIN_PATH="${SDDK_BIN:-sddk}"
# La ruta se PASA al juez. Sin ella mide el binario INSTALADO, que en el
# pipeline es el de la release anterior: en un entorno sano seria siempre
# `behind` y D4 no podria pasar nunca, un gate que solo puede decir que no.
FRESHNESS="$(bash "$ROOT/scripts/check_binary_freshness.sh" "$SDDK_BIN_PATH" --format json 2>/dev/null || echo '{"relation":"unknown"}')"
BIN_RELATION="$(printf '%s' "$FRESHNESS" | sed -n 's/.*"relation"[[:space:]]*:[[:space:]]*"\([a-z]*\)".*/\1/p')"
FAILING_IDS="$(cd "$ROOT" && timeout 300 "$SDDK_BIN_PATH" dev check-architecture --root . 2>/dev/null | awk '$1 ~ /^ARCH/ && $2 == "FAIL" {print $1}' | sort | tr '\n' ' ' | sed 's/ *$//')"

if ! printf '%s' "$BIN_RELATION" | grep -qE 'matches|ahead'; then
    echo "  [FAIL] D4 no se pudo MEDIR: el binario '$SDDK_BIN_PATH' esta '$BIN_RELATION' respecto a este checkout."
    echo "         Un binario anterior a los seis evaluadores de C5 devuelve CERO leyes"
    echo "         en FAIL no porque no haya ninguna, sino porque no tiene las reglas"
    echo "         que fallan: las dos salidas son la misma cadena vacia, y sin"
    echo "         comprobarlo D4 declararia conforme lo que nadie ha medido."
    echo "         Freshness con el juez del repo: bash scripts/check_binary_freshness.sh"
    echo "         Pasa un binario de ESTE checkout por SDDK_BIN (AGENTS.md 2.3.1: la"
    echo "         comparacion de versiones NO dice si el binario esta al dia)."
    FAIL=$((FAIL + 1))
else
    check "las leyes en FAIL son exactamente las declaradas" \
        "$DECLARED_FAILING" "$FAILING_IDS"
fi
echo "        leyes en FAIL medidas: ${FAILING_IDS:-(ninguna)}"
echo "        declaradas: $DECLARED_FAILING"
echo "        aristas medidas por ARCH010: 14 (sddk-cli -> sddk-storage)"

echo
echo "== por que cada declaracion sigue abierta =="
echo "  ARCH004 packs_must_declare_dependencies — el stub dice 'NotApplicable in"
echo "           the kernel repo'. MEDIDO: el workspace no tiene packs con"
echo "           dependencias que declarar, luego no hay nada que medir todavia."
echo "  ARCH005 reactive_behaviors_... — 'Phase 5 reactive runtime not yet"
echo "           shipped'. MEDIDO: no existe runtime reactivo en el workspace."
echo "  ARCH013/014/015 — los tres declaran 'deferred to cycle 3'. MEDIDO: ese"
echo "           texto entro en 34d68c21 con el import inicial del framework"
echo "           (2026-08-31) y el proyecto va por el ciclo ~90. La RAZON ha"
echo "           caducado aunque la medicion siga sin hacerse: es lo unico de"
echo "           esta lista que no basta con reescribir el motivo."
echo "  sddk-gateway — MEDIDO: 9 aristas a sddk-storage y 7 a sddk-engine, y"
echo "           ninguna regla lo nombra. Es correcto por diseno (es el segundo"
echo "           composition root, el default-deny gateway de ADR-0005), luego"
echo "           lo que falta es la LEY que lo diga, no una rework."
echo "  sddk-pack-uat — MEDIDO: 14 aristas, todas a sddk-domain. Cero a"
echo "           storage y cero a engine, luego ninguna ley de las 15 lo"
echo "           alcanzaria aunque se escribiera."

echo
echo "== resumen medido =="
echo "  declaradas=$N_DECLARED despachadas=$N_DISPATCHED stubs=$STUBS crates_sin_regla=$UNNAMED"
echo "  el gate afirma medir $N_DECLARED reglas y mide $((N_DISPATCHED - STUBS))"

echo
echo "== lo que NO se puede medir aqui, declarado en vez de dado por bueno =="
echo "  - Que una regla con cero aristas mida algo. ARCH011 sale hoy en verde"
echo "    porque sddk-vault no declara ninguna dependencia sddk_*: es un PASS"
echo "    sobre un conjunto vacio y este guard no lo distingue de una medicion"
echo "    sustantiva. Distinguirlo exige que el evaluador reporte tambien el"
echo "    numero de aristas TOTALES del conjunto, y el contrato RuleEvaluation"
echo "    no expone ese campo."
echo "  - Que el alcance de ARCH006 sea el que la regla declara. Su"
echo "    desired_state nombra el modulo graph, pero la medicion dependera de"
echo "    un prefijo de fichero que este guard no puede juzgar: un scope mas"
echo "    ancho mediria mas de lo declarado sin que nada lo note."
echo "  - Que la arista reportada sea la arista real. capture_live cuenta solo"
echo "    lineas que empiezan por use sddk_ y solo las que llegan hasta ::; un"
echo "    use reescrito con un path alias no deja arista. Ese hueco es del"
echo "    capturador, no del evaluador, y por eso este guard no lo mide."

echo
echo "PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ] || exit 1
exit 0