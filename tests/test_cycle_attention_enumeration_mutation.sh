#!/usr/bin/env bash
# Autofalsacion de la enumeracion de "esto necesita mi decision".
#
# EL DEFECTO
# ----------
# `sddk cycle list` es la UNICA superficie que enumera los ciclos de un
# proyecto, y no decia nada de lo que un operador necesita para responder
# "que necesita mi decision?".
#
# MEDIDO sobre el ledger real de `p-63676b11dc0ef88f` antes del arreglo:
# la salida tenia 662 lineas y **0** mencionaban `runtime_state` o
# `approval`. Los ciclos bloqueados por una persona eran byte-identicos a los
# que no necesitaban nada: mismo `status: OPEN`, misma `phase`, mismas fechas.
# Entre 29 ciclos OPEN, descubrir el bloqueado exigia saber de antemano su id.
#
# Y el fallo abierto tiene una forma particularmente insidiosa: cuando la
# derivacion no puede hacerse, el renderizado natural es la CADENA VACIA, que se
# lee como "nada pendiente". Es la misma mentira de INC-DEBT-067 un nivel mas
# abajo: el dato falta y el render presenta la ausencia como una afirmacion
# positiva.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Cinco mutaciones, cada una corrompiendo UN punto de enforcement, y cada una
# exigiendo que caiga el test NOMBRADO que lo vigila. Una mutacion compuesta
# no puede decir cual de las comprobaciones cayo por efecto colateral.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets}"

CLI="$ROOT/crates/sddk-cli/src/cycle.rs"
ENGINE="$ROOT/crates/sddk-engine/src/cycle_summary.rs"
CLI_TEST="-p sddk-cli --test cycle_attention_enumeration"

BAK="$(mktemp -d)"
cp "$CLI" "$BAK/cli.rs"
cp "$ENGINE" "$BAK/engine.rs"
restore() { cp "$BAK/cli.rs" "$CLI"; cp "$BAK/engine.rs" "$ENGINE"; }
trap 'restore; rm -rf "$BAK"' EXIT

PASS=0
FAIL=0
SKIP=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skp() { echo "  [SKIP] $1"; SKIP=$((SKIP + 1)); }

# Aplica una mutacion y EXIGE que el fichero haya cambiado.
muta() {  # muta <ruta> <viejo> <nuevo>
    local ruta="$1" viejo="$2" nuevo="$3" antes despues
    antes="$(sha256sum "$ruta" | cut -d' ' -f1)"
    RUTA="$ruta" VIEJO="$viejo" NUEVO="$nuevo" python3 - <<'PY' || return 2
import os, sys
ruta, viejo, nuevo = os.environ["RUTA"], os.environ["VIEJO"], os.environ["NUEVO"]
with open(ruta, encoding="utf-8") as fh:
    src = fh.read()
if viejo not in src:
    sys.exit(3)
with open(ruta, "w", encoding="utf-8") as fh:
    fh.write(src.replace(viejo, nuevo, 1))
PY
    despues="$(sha256sum "$ruta" | cut -d' ' -f1)"
    if [[ "$antes" == "$despues" ]]; then
        skp "la mutacion sobre ${ruta##*/} no cambio el fichero; NO cuenta como deteccion"
        return 2
    fi
    return 0
}

# Corre un test y dice si CAE. Lo que se exige es que caiga el test NOMBRADO,
# que separa "cayo este guard" de "no compilo".
falls() {  # falls <etiqueta> <filtro-cargo> <test-que-debe-caer>
    local etiqueta="$1" filtro="$2" nombre="$3" salida
    local -a args
    read -r -a args <<<"$filtro"
    salida="$(cargo test "${args[@]}" -- --exact "$nombre" 2>&1)"
    if grep -qE "^test result: ok" <<<"$salida"; then
        bad "$etiqueta: el test SIGUE EN VERDE tras la mutacion (no tiene dientes)"
        return 1
    fi
    if ! grep -qF "$nombre" <<<"$salida"; then
        bad "$etiqueta: fallo por una causa distinta; el test nominado no aparece en la salida"
        return 1
    fi
    return 0
}

echo "== Autofalsacion de la enumeracion de decisiones pendientes =="
echo

# ── M1: la enumeracion deja de preguntar por sus propias filas ─────────────
# El fallo literal de quienCablea mal la derivacion: la lista de ids que se
# entrega al productor se vacia, el mapa vuelve vacio y TODAS las filas caen en
# la rama "no se pudo derivar". Es la forma que toma el defecto cuando nadie
# lo escribe a proposito: la enumeracion no dice "nada pendiente", dice
# "nada se", que es peor y mas facil de no ver.
echo "-- M1: la enumeracion pregunta por cero ciclos"
if muta "$CLI" \
    'derive_all_cycle_facts(&context.storage, cycle_ids.iter().map(String::as_str))' \
    'derive_all_cycle_facts(&context.storage, std::iter::empty::<&str>())'; then
    if falls "M1" "$CLI_TEST" "a_cycle_awaiting_a_decision_is_distinguishable_in_the_enumeration"; then
        ok "M1 detectado: la enumeracion dejo de derivar el estado que espera a una decision"
    fi
    restore
fi

# ── M2: el estado no derivable se declara derivado ─────────────────────────
# El fallo abierto. La cadena vacia es el renderizado natural de un fallo, y se
# lee como "nada pendiente". Esta mutacion lo impone al POR DE TODO: forzar el
# indicador a conocido, que es como volveria el defecto sin que nadie escribiera
# la cadena vacia. Cae por tres aserciones independientes del mismo test
# nombradO: la palabra `unknown`, el contador aparte, y el indicador de la fila.
echo "-- M2: una fila que no se puede leer se declara conocida"
if muta "$CLI" \
    '                let runtime_state_known = facts.is_some() && c.manifest_readable;' \
    '                let runtime_state_known = true;'; then
    if falls "M2" "$CLI_TEST" "a_cycle_whose_state_cannot_be_derived_says_so_and_is_counted_apart"; then
        ok "M2 detectado: un ciclo que no se puede leer volvio a decir que no necesita nada"
    fi
    restore
fi

# ── M3: el contador declarado se desincroniza de las filas ─────────────────
# Un numero que nada cruza es una DECLARACION, no evidencia. Esta mutacion
# hace que el contador diga otra cosa que las filas que lo respaldan, que es
# la forma que toma "la cifra se pudre".
echo "-- M3: el contador declarado deja de contar las filas"
if muta "$CLI" \
    '                if !pending_approvals.is_empty() {
                    pending_human_decisions += 1;
                }' \
    '                let _ = &pending_approvals;'; then
    if falls "M3" "$CLI_TEST" "the_declared_counter_agrees_with_the_rows_it_counts"; then
        ok "M3 detectado: el contador dejo de estar atado a las filas que cuenta"
    fi
    restore
fi

# ── M4: las capacidades pendientes ya no se nombran ────────────────────────
# Un contador sin el nombre de la capacidad responde "cuantos" y no "cual",
# que es lo que permite actuar. La fila seria contable y accionable=False.
echo "-- M4: la capacidad pendiente deja de nombrarse"
if muta "$CLI" \
    '        for capability in &entry.pending_approvals {
            out.push_str(&format!("  pending_approval: {capability}\n"));
        }' \
    '        let _ = &entry.pending_approvals;'; then
    if falls "M4" "$CLI_TEST" "a_cycle_awaiting_a_decision_is_distinguishable_in_the_enumeration"; then
        ok "M4 detectado: la fila dejo de decir QUE hay que aprobar"
    fi
    restore
fi

# ── M5: el productor solo responde por los ciclos que tienen eventos ───────
# Esta es la tentacion del motor: devolver una entrada solo cuando hay eventos,
# porque "sin eventos, sin nada pendiente" se deduce. Se deduce SI, pero el
# consumidor tendria que reconstruir el argumento para no leer una ausencia
# como "no se pudo derivar". El contrato es una entrada por ciclo pedido, y este
# es el unico sitio donde se rompe sin que nadie lo note: la fila que no tiene
# eventos pasa de "tranquila y conocida" a "desconocida", y la enumeracion
# mezcla los dos silencios.
#
# HISTORIA DE ESTA MUTACION, que es el motivo de que apunte a un test propio:
# la primera vez se lanzo contra el test de la enumeracion y el suite entero
# quedo EN VERDE — la mutacion SI se aplico, pero no tenia dientes, porque en el
# sandbox TODOS los ciclos habian sido arrancados con `cycle start`, y arrancar
# un ciclo emite su propio evento inicial. No existia ni un solo ciclo sin
# eventos, luego no habia nada que M5 pudiera romper. Un guard que no puede ver
# el defecto que nombra compra cobertura en el papel; por eso el sujeto se planta
# y por eso M5 exige caer un test que lo describa.
echo "-- M5: el productor omite los ciclos sin eventos"
if muta "$ENGINE" \
    '        out.insert(cycle_id.to_owned(), derive_runtime_facts(&cycle_events));' \
    '        if !cycle_events.is_empty() {
            out.insert(cycle_id.to_owned(), derive_runtime_facts(&cycle_events));
        }'; then
    if falls "M5" "$CLI_TEST" "a_cycle_with_no_events_is_quiet_and_known_rather_than_undetermined"; then
        ok "M5 detectado: un ciclo sin eventos dejo de ser 'tranquilo y conocido'"
    fi
    restore
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[[ "$FAIL" -eq 0 ]]
