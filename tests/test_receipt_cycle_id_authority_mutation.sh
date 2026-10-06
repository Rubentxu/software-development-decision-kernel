#!/usr/bin/env bash
# Autofalsacion del guard de autoridad de `cycle_id` en los recibos.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Cuatro casos, cada uno atacando UN punto, y cada uno exigiendo que caiga (o
# sobreviva) por SU PROPIA comprobacion. Una mutacion compuesta no podria decir
# cual de las comprobaciones cayo por efecto colateral.
#
# M1 y M2 son el DEFECTO en las dos formas que se midieron: un `cycle_id`
# completo que la autoridad nunca emitio, y un campo `**Cycle:**` que no
# contiene un ciclo. M2 importa especialmente porque el arreglo de este bloque
# RENOMBRO ese campo: si el guard solo supiera leer el nombre viejo, el arreglo
# habria dejado el defecto sin vigilar, y el guard estaria verde precisamente
# porque se corrigio lo que vigilaba.
#
# M3 es el fail-closed: sin autoridad no hay veredicto, y "sin veredicto" no es
# verde. Es el control sin el cual M1 y M2 no significarian nada, porque un guard
# que se pone verde cuando el ledger falta detectaria cualquier mentira.
#
# M4 es el control NEGATIVO: prueba que el guard no reacciona a ruido.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

GUARD="tests/test_receipt_cycle_id_authority.sh"
VAULT="docs/roadmap/receipts/cl-vault-graph/RECEIPT.md"
C3H="docs/roadmap/receipts/c3h/RECEIPT.md"

PASS=0
FAIL=0
SKIP=0
ok()   { printf '  [ok]   %s\n' "$*"; PASS=$((PASS + 1)); }
ko()   { printf '  [FAIL] %s\n' "$*"; FAIL=$((FAIL + 1)); }
skip() { printf '  [SKIP] %s — %s\n' "$1" "$2"; SKIP=$((SKIP + 1)); }

BAK="$(mktemp -d)"
cp "$VAULT" "$BAK/vault.md"
cp "$C3H" "$BAK/c3h.md"

# shellcheck disable=SC2329  # invocada por el trap de EXIT, que shellcheck no ve
restore() {
    cp "$BAK/vault.md" "$VAULT"
    cp "$BAK/c3h.md" "$C3H"
    rm -rf "$BAK"
}
trap 'restore' EXIT

sha_de() { sha256sum "$1" | cut -d' ' -f1; }

# ── M1: un cycle_id fabricado en el campo NUEVO ──────────────────────────────
# Ataca el hueco que el propio arreglo abrio: `**Ciclo SDDK:**` tambien declara.
echo "--- M1 un cycle_id fabricado sobrevive en **Ciclo SDDK:**"
S0="$(sha_de "$VAULT")"
if ! grep -q '^\*\*Ciclo SDDK:\*\* ninguno' "$VAULT"; then
    skip "M1" "el recibo no tiene la forma que esta mutacion supone"
else
    python3 - "$VAULT" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
v = "**Ciclo SDDK:** ninguno."
n = "**Ciclo SDDK:** `p-63676b11dc0ef88f/vault-graph`."
assert v in s, "el campo Ciclo SDDK no tiene la forma que esta mutacion supone"
open(p, "w").write(s.replace(v, n, 1))
PY
    if [ "$S0" = "$(sha_de "$VAULT")" ]; then
        skip "M1" "el parche no cambio el fichero: no esta midiendo nada"
    else
        out="$(bash "$GUARD" 2>&1)"; rc=$?
        cp "$BAK/vault.md" "$VAULT"
        if [ "$S0" != "$(sha_de "$VAULT")" ]; then
            ko "M1 — la restauracion no fue byte-identica"
        elif [ "$rc" -eq 0 ]; then
            ko "M1 — el guard PASA con un cycle_id que el ledger no tiene (rc=0). No vigila el campo que el arreglo creo."
        else
            ok "M1 — el guard cae ante un cycle_id fabricado en el campo nuevo ($(printf '%s\n' "$out" | grep -c '\[FAIL\]') FAIL)"
        fi
    fi
fi

# ── M2: el campo **Cycle:** vuelve a existir sin declarar ciclo ───────────────
# La forma "mislabeled" del defecto: el campo promete un ciclo y no lo da.
echo "--- M2 el campo **Cycle:** sin cycle_id"
S0="$(sha_de "$C3H")"
if ! grep -q '^\*\*Sección ROADMAP:\*\* C3h' "$C3H"; then
    skip "M2" "el recibo no tiene la forma que esta mutacion supone"
else
    python3 - "$C3H" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
v = "**Sección ROADMAP:** C3h (Supply-chain audit"
n = "**Cycle:** C3h (Supply-chain audit"
assert v in s, "el campo no tiene la forma que esta mutacion supone"
open(p, "w").write(s.replace(v, n, 1))
PY
    if [ "$S0" = "$(sha_de "$C3H")" ]; then
        skip "M2" "el parche no cambio el fichero: no esta midiendo nada"
    else
        out="$(bash "$GUARD" 2>&1)"; rc=$?
        cp "$BAK/c3h.md" "$C3H"
        if [ "$S0" != "$(sha_de "$C3H")" ]; then
            ko "M2 — la restauracion no fue byte-identica"
        elif [ "$rc" -eq 0 ]; then
            ko "M2 — el guard PASA con un campo **Cycle:** que no declara ciclo (rc=0). Solo mira la forma del id, no la del campo."
        else
            ok "M2 — el guard cae ante un campo Cycle sin cycle_id ($(printf '%s\n' "$out" | grep -c '\[FAIL\]') FAIL)"
        fi
    fi
fi

# ── M3: fail-closed. La autoridad no esta, el guard NO pasa ───────────────────
echo "--- M3 autoridad ausente => FAIL, nunca verde"
out="$(SDDK_STATE_HOME=/tmp/sddk-autoridad-inexistente-$$ XDG_STATE_HOME=/tmp/sddk-autoridad-inexistente-$$ bash "$GUARD" 2>&1)"
rc=$?
if [ "$rc" -eq 0 ]; then
    ko "M3 — el guard PASA sin ledger (rc=0). Un guard que se pone verde cuando la autoridad falta miente como el defecto que vigila."
elif printf '%s\n' "$out" | grep -q 'la autoridad no se pudo leer'; then
    ok "M3 — sin autoridad el guard falla cerrado y lo dice"
else
    ko "M3 — fallo por otra causa: $(printf '%s\n' "$out" | tail -2 | tr '\n' ' ')"
fi

# ── M4: control negativo. Ruido inocuo, veredicto identico ────────────────────
echo "--- M4 comentario inocuo => el veredicto NO cambia"
S0="$(sha_de "$VAULT")"
printf '\n<!-- nota de control negativo: no afirma nada -->\n' >>"$VAULT"
if [ "$S0" = "$(sha_de "$VAULT")" ]; then
    skip "M4" "el parche no cambio el fichero: no esta midiendo nada"
else
    out="$(bash "$GUARD" 2>&1)"; rc=$?
    cp "$BAK/vault.md" "$VAULT"
    if [ "$S0" != "$(sha_de "$VAULT")" ]; then
        ko "M4 — la restauracion no fue byte-identica"
    elif [ "$rc" -eq 0 ]; then
        ok "M4 — el ruido no movio el veredicto: el guard mide lo que dice medir"
    else
        ko "M4 — el guard falla ante un comentario inocuo. Reacciona a ruido."
    fi
fi

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -eq 0 ] && [ "$SKIP" -eq 0 ]; then
    printf 'RESULT: PASS — cada comprobacion cae por su propia causa, incluida la de no-reaccion.\n'
    exit 0
fi
printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
exit 1
