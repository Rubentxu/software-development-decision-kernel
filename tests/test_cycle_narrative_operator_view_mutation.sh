#!/usr/bin/env bash
# Autofalsacion de la vista del operador (`sddk cycle narrative`).
#
# EL DEFECTO
# ----------
# INC-DEBT-067. La superficie que se describe a si misma como *operator view*
# hacia dos afirmaciones que no derivaban de nada: "Cycle completed." como
# literal por defecto de `what_was_done` (sin consultar el `status`), y
# "Nada por ahora." porque `human_action_required` **no tenia productor en
# todo el workspace** — declarado, puesto a `None` por el constructor, leido
# una vez por el pie, y nunca recibio un `Some(..)`.
#
# Peor que una derivacion ausente, MEDIDO sobre el repo real antes del
# arreglo: el comando **nunca abria el store** (su parametro `environment`
# estaba literalmente sin usar), asi que renderizaba "Cycle completed." con
# **exit 0 para un id de ciclo que no existe**. Una vista que fabrica una
# afirmacion de completitud sobre un objeto inexistente no es una vista
# debil: es falsa, y falla abierta en la direccion que esconde trabajo.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Cuatro mutaciones, cada una corrompiendo UN punto de enforcement, y cada
# una exigiendo que caiga el test nominado que lo vigila. Una mutacion
# compuesta no puede decir cual de las comprobaciones cayo por efecto
# colateral, que es como se declara un guard que vigila menos de lo que cree.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`: cada una comprueba
# por sha que el fichero cambio, y la restauracion se comprueba byte a byte.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets}"

ENGINE="$ROOT/crates/sddk-engine/src/cycle_narrative.rs"
CLI="$ROOT/crates/sddk-cli/src/cycle.rs"
ENGINE_TEST="-p sddk-engine --lib cycle_narrative::tests"
CLI_TEST="-p sddk-cli --test cycle_narrative_operator_view"

BAK="$(mktemp -d)"
cp "$ENGINE" "$BAK/engine.rs"
cp "$CLI" "$BAK/cli.rs"
restore() { cp "$BAK/engine.rs" "$ENGINE"; cp "$BAK/cli.rs" "$CLI"; }
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
# que separa "cayo este guard" de "no compilo" — que es la distincion que si
# importa. Fijarse en el texto de una asercion haria depender el falsador
# del ORDEN en que un test se comprueba.
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

echo "== Autofalsacion de la vista del operador =="
echo

# ── M1: la frase vuelve a ser una constante ────────────────────────────────
# El defecto literal que la deuda cita. Sin la rama por status, todo estado
# renderiza la misma frase y un ciclo OPEN vuelve a ser indistinguible de uno
# CLOSED.
echo "-- M1: derive_claims vuelve a devolver una frase constante"
if muta "$ENGINE" \
    '    let what_was_done = match facts.status {' \
    '    let what_was_done = if true { "Cycle completed.".to_string() } else { match facts.status {'; then
    # La rama nueva deja el `match` sin cerrar de forma valida; se cierra.
    ENGINE="$ENGINE" python3 - <<'PY'
import os
p = os.environ["ENGINE"]
s = open(p, encoding="utf-8").read()
marca = '''        ),
    };

    // ── Claim 2'''
reemplazo = '''        ),
    } };

    // ── Claim 2'''
assert marca in s
open(p, "w", encoding="utf-8").write(s.replace(marca, reemplazo, 1))
PY
    if falls "M1" "$ENGINE_TEST" "cycle_narrative::tests::a_live_cycle_is_never_described_as_completed" \
              || falls "M1" "$CLI_TEST" "a_started_cycle_is_never_narrated_as_completed"; then
        ok "M1 detectado: la frase dejo de derivarse del status del ciclo"
    fi
    restore
fi

# ── M2: la lectura del store deja de fallar cerrado ────────────────────────
# Vuelve al comportamiento MEDIDO antes del arreglo: un id de ciclo que no
# existe produce "Cycle completed." con exit 0.
#
# POR QUE MUTA ESTE ENLACE Y NO EL DE LA RESOLUCION
# ----------------------------------------------
# La primera version de esta mutacion rompia `resolve_cycle_context` para que
# aceptara el id crudo, y el test SIGUIO EN VERDE. No porque el store dejara de
# fallar cerrado, sino porque el enlace siguiente -- `get_cycle(cycle_id)?` --
# seguia intacto y seguia rechazando el id inexistente con STORAGE_NOT_FOUND.
# El test pasaba, luego la mutacion no habia roto la garantia: habia roto otra
# cosa. Es el fallo de un falsador escrito desde la narrative del defecto
# ("el comando deja de resolver el ciclo") en vez de desde el punto donde el
# defecto se decidia (el store no encuentra el ciclo y alguien fabrica el
# registro). Un guard que se queda verde cuando se le rompe lo que dice vigilar
# no es un guardDebil: es un guard que no vigila, que es peor porque declara
# una cobertura que no tiene.
#
# El needle incluye el comentario ancla porque `let record = context.storage
# .get_cycle(cycle_id)?;` aparece DOS veces en el fichero (la otra es
# `run_cycle_status`); sin el ancla la mutacion habria caido sobre la otra.
echo "-- M2: un ciclo ausente del ledger se narra en vez de fallar cerrado"
if muta "$CLI" \
    '        // Fails closed with the typed STORAGE_NOT_FOUND when the cycle is
        // not in the ledger. No narrative is emitted for it.
        let record = context.storage.get_cycle(cycle_id)?;' \
    '        // Fails closed with the typed STORAGE_NOT_FOUND when the cycle is
        // not in the ledger. No narrative is emitted for it.
        let record = match context.storage.get_cycle(cycle_id) {
            Ok(record) => record,
            Err(_) => return Ok("Cycle completed.\n\nNada por ahora.".to_string()),
        };'; then
    if falls "M2" "$CLI_TEST" "a_cycle_that_does_not_exist_fails_closed_without_a_narrative"; then
        ok "M2 detectado: la narrativa volvio a narrar con exit 0 un ciclo que no existe"
    fi
    restore
fi

# ── M3: el productor de la accion humana desaparece ────────────────────────
# El defecto central de la deuda: el campo sin productor, luego el pie
# estructuralmente incapaz de decir otra cosa que "Nada por ahora".
echo "-- M3: human_action_required vuelve a no tener productor"
if muta "$ENGINE" \
    '    NarrativeClaims {
        what_was_done,
        human_action_required,
    }' \
    '    let _ = human_action_required;
    NarrativeClaims {
        what_was_done,
        human_action_required: None,
    }'; then
    if falls "M3" "$CLI_TEST" "the_operator_footer_follows_the_lease"; then
        ok "M3 detectado: el pie volvio a no poder decir que hace falta algo"
    fi
    restore
fi

# ── M4: el override del llamante deja de ganar ─────────────────────────────
# El orden de aplicacion es la propiedad: las claims primero y el override
# despues. Al reponer el orden equivocado el override se descarta en
# silencio, que es un fallo de exactamente el mismo genero que el defecto
# original -- una superficie que ignora lo que se le pide y aunasi lo dice.
echo "-- M4: el override del llamante se aplica antes de las claims y se pierde"
if muta "$ENGINE" \
    '    pub fn override_what_was_done(&mut self, text: &str) {
        self.what_was_done = text.to_string();
        self.recompute_digest();
    }' \
    '    pub fn override_what_was_done(&mut self, text: &str) {
        let _ = text;
    }'; then
    if falls "M4" "$CLI_TEST" "an_explicit_override_still_wins_over_the_derived_sentence"; then
        ok "M4 detectado: el override explicito del operador dejaba de ganar"
    fi
    restore
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[[ "$FAIL" -eq 0 ]]
