#!/usr/bin/env bash
# Autofalsacion del conjunto cerrado de `backlog discard`.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Antes de este bloque, el conjunto cerrado de razones de descarte no era una
# propiedad del ledger. Se mediaron cuatro capas y ninguna lo-era:
#
#   - el dominio declaraba `InvalidDiscardReason` y NUNCA lo construia;
#   - `BacklogEvent::Discarded.reason` era `String`, y los propios tests del
#     storage escribian "won't fix", "done" y "x";
#   - el schema exigia que `reason` fuera string, no que estuviera en el
#     conjunto;
#   - `append_event` no valida contra el schema.
#
# Lo unico que exigia la pertenencia era el `ValueEnum` de clap: una copia
# privada en la capa CLI. Un conjunto cerrado que solo impone un parser de
# argumentos no es una propiedad del ledger.
#
# DOS REGLAS, Y SON EL MOTIVO DE QUE ESTE SCRIPT EXISTA
# ----------------------------------------------------
# 1. Cada mutacion corrompe UN solo punto de enforcement, y hay que ver caer al
#    test nominado que lo vigila. Una mutacion compuesta no puede decir cual de
#    las comprobaciones cayo por efecto colateral, que es como se declara un
#    guard que vigila menos de lo que cree.
# 2. UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`. Cada mutacion
#    comprueba por sha que el fichero cambio antes de exigir nada, y la
#    restauracion se comprueba byte a byte al final.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets}"

DOMAIN="$ROOT/crates/sddk-domain/src/backlog.rs"
SCHEMAS="$ROOT/crates/sddk-domain/src/event_registry/schemas.rs"
STORE="$ROOT/crates/sddk-storage/src/backlog_store.rs"
CLIBL="$ROOT/crates/sddk-cli/src/backlog.rs"

BAK="$(mktemp -d)"
cp "$DOMAIN" "$BAK/domain.rs"
cp "$SCHEMAS" "$BAK/schemas.rs"
cp "$STORE" "$BAK/store.rs"
cp "$CLIBL" "$BAK/clibl.rs"

restore() {
    cp "$BAK/domain.rs" "$DOMAIN"
    cp "$BAK/schemas.rs" "$SCHEMAS"
    cp "$BAK/store.rs" "$STORE"
    cp "$BAK/clibl.rs" "$CLIBL"
}
trap 'restore; rm -rf "$BAK"' EXIT

PASS=0
FAIL=0
SKIP=0

ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skp() { echo "  [SKIP] $1"; SKIP=$((SKIP + 1)); }

# Aplica una mutacion y EXIGE que el fichero haya cambiado. Sin esta comprobacion
# un texto que ya no existe hace que la mutacion no haga nada, el guard no caiga
# por su culpa, y el paso se reporte como deteccion.
muta() {  # muta <ruta> <viejo> <nuevo>
    local ruta="$1" viejo="$2" nuevo="$3" antes despues
    antes="$(sha256sum "$ruta" | cut -d' ' -f1)"
    RUTA="$ruta" VIEJO="$viejo" NUEVO="$nuevo" python3 - <<'PY'
import os, sys
ruta, viejo, nuevo = os.environ["RUTA"], os.environ["VIEJO"], os.environ["NUEVO"]
with open(ruta, encoding="utf-8") as fh:
    src = fh.read()
if viejo not in src:
    sys.exit(3)
with open(ruta, "w", encoding="utf-8") as fh:
    fh.write(src.replace(viejo, nuevo, 1))
PY
    if [[ $? -ne 0 ]]; then
        skp "la mutacion no encontro su texto en ${ruta##*/}; NO cuenta como deteccion"
        return 2
    fi
    despues="$(sha256sum "$ruta" | cut -d' ' -f1)"
    if [[ "$antes" == "$despues" ]]; then
        skp "la mutacion sobre ${ruta##*/} no cambio el fichero; NO cuenta como deteccion"
        return 2
    fi
    return 0
}

# Corre un test y dice si FALLO. El segundo argumento, si se da, es un texto que
# debe aparecer en la salida: asi una mutacion que rompe el codigo por otra
# causa no se contabiliza como si hubieracaido el guard previsto.
# Corre un test y dice si FALLO. El segundo argumento, si se da, es el NOMBRE del
# test que tiene que caer. La granularidad es deliberada: lo que se exige es que
# el guard nominado caiga, no cual de sus aserciones Habria sido un error
#needle: al medir por el texto exacto de una asercion, esta mutacion cayo por la
# comprobacion de grafia (que dispara antes) y el falsador la declaro "fallo por
# una causa distinta" -- cuando en realidad la habia detectado bien. Fijarse en
# la asercion hace que el falsador dependa del ORDEN en que un test se comprueba, que
# no es la propiedad que se quiere vigilar. El nombre del test separa "cayo este
# guard" de "no compilo", que es la distincion que si importa.
falls() {  # falls <etiqueta> <filtro-cargo> [nombre-del-test-que-debe-caer]
    local etiqueta="$1" filtro="$2" needle="${3:-}" salida
    # El filtro se expande SIN comillas a proposito: son varios argumentos de
    # cargo (`-p crate --lib nombre`). Comillarlo los convierte en un unico
    # argumento, cargo lo toma por nombre de paquete, y el guard "cae" por una
    # causa que no es la suya — exactamente la confusion que este script existe
    # para no repetir.
    salida="$(cargo test $filtro 2>&1)"
    if [[ $? -eq 0 ]]; then
        bad "$etiqueta: el test SIGUE EN VERDE tras la mutacion (no tiene dientes)"
        return 1
    fi
    if [[ -n "$needle" ]] && ! grep -qF -- "$needle" <<<"$salida"; then
        bad "$etiqueta: fallo por una causa distinta a la esperada; no cuenta como deteccion"
        printf '%s\n' "$salida" | tail -20
        return 1
    fi
    return 0
}

echo "== Autofalsacion del conjunto cerrado de backlog discard =="
echo

# ── M1: el parser del dominio deja de admitir un miembro ─────────────────────
# El dominio es la autoridad. Si `resolved` desaparece del parser, el mensaje de
# error lo sigue anunciando (se renderiza desde ALL) y el schema --que usa el
# mismo parser-- lo rechaza. Cae el round-trip del dominio.
echo "-- M1: el parser del dominio deja de admitir 'resolved'"
if muta "$DOMAIN" \
    '            "resolved" => Ok(Self::Resolved),' \
    '            "resolved-not-actually" => Ok(Self::Resolved),'; then
    if falls "M1" "-p sddk-domain --lib discard_reason_round_trip_for_every_member" \
              "discard_reason_round_trip_for_every_member" ; then
        ok "M1 detectado: el dominio exigia al parser lo que el parser ya no daba"
    fi
    restore
fi

# ── M2: el schema vuelve a exigir presencia, no pertenencia ──────────────────
# Es el gate de los payloads que llegan al log sin pasar por el evento tipado:
# un replay, una importacion, una fila escrita a mano.
echo "-- M2: el schema vuelve a aceptar cualquier string en 'reason'"
if muta "$SCHEMAS" \
    '            // Membership, not mere presence. This schema used to accept
            // any string for `reason`, so the closed set lived only in
            // the CLI'"'"'s argument parser. Membership is read from
            // `BacklogDiscardReason` so the schema and the type cannot
            // describe different sets.
            && p.get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.parse::<crate::backlog::BacklogDiscardReason>().is_ok())
                .unwrap_or(false)' \
    ''; then
    if falls "M2" "-p sddk-domain --lib discarded_schema_admits_every_member" \
              "discarded_schema_admits_every_member"; then
        ok "M2 detectado: el schema habia vuelto a ser una comprobacion de presencia"
    fi
    restore
fi

# ── M3: el campo vuelve a ser un String pelado ───────────────────────────────
# Esta es la garantia que se movio de runtime a tiempo de compilacion, y por eso
# necesitaba un guard: una suite que solo compila es indistinguible de una que no
# se ejecuto nunca. El rasgo sellado hace que la regresion no compile, y aqui se
# cuenta como caida precisamente porque lo que se busca es que no compile.
echo "-- M3: el campo 'reason' vuelve a ser String"
if muta "$STORE" \
    '        reason: BacklogDiscardReason,' \
    '        reason: String,'; then
    salida_m3="$(cargo test -p sddk-storage --lib the_discarded_reason_field_is_not_a_bare_string 2>&1)"
    if grep -qF "NotABareString" <<<"$salida_m3"; then
        ok "M3 detectado: el rasgo sellado impidio que el campo volviera a String"
    elif grep -q "error\[" <<<"$salida_m3"; then
        skp "M3: la mutacion rompio la compilacion por otra causa; NO cuenta como deteccion"
        printf '%s\n' "$salida_m3" | tail -20
    else
        bad "M3: el campo volvio a String y la suite siguio en verde (no tiene dientes)"
    fi
    restore
fi

# ── M4: la CLI colapsa dos motivos en un mismo miembro del dominio ───────────
# La primera version de esta mutacion quitaba `Resolved` del enum de la CLI. Caia
# --pero por un error de compilacion, porque el propio guard referencia la
# variante--, asi que no llegaba a ejercitar la logica del guard: cualquier
# borrado de variante caeria igual, y el guard podria no comprobar nada. Medir
# "cayo" sin mirar por que cayo es como se declara un guard que vigila menos de
# lo que cree. Esta mutacion SI compila y SI exige la asercion de inyectividad.
echo "-- M4: la CLI mapea dos motivos al mismo miembro del dominio"
if muta "$CLIBL" \
    '            CliDiscardReason::Resolved => D::Resolved,' \
    '            CliDiscardReason::Resolved => D::Wontfix,'; then
    if falls "M4" "-p sddk-cli --lib discard_reason_maps_bijectively_onto_the_domain_set" \
              "discard_reason_maps_bijectively_onto_the_domain_set"; then
        ok "M4 detectado: dos miembros de la CLI colapsaban en uno solo del dominio"
    fi
    restore
fi

# ── M5: la regla de linaje se aplica a todos los motivos ─────────────────────
# Si todo motivo exigiera sucesor, `resolved` seria inutilizable: habria que
# inventar un sucesor para cerrar algo que no fue reemplazado por nada.
echo "-- M5: requires_successor devuelve true para todos los miembros"
if muta "$DOMAIN" \
    '        matches!(self, Self::Superseded)' \
    '        true'; then
    if falls "M5" "-p sddk-domain --lib only_superseded_requires_a_successor" \
              "only_superseded_requires_a_successor"; then
        ok "M5 detectado: la regla de linaje se habia extendido a todos los motivos"
    fi
    restore
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[[ "$FAIL" -eq 0 ]]
