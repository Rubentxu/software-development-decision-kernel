#!/usr/bin/env bash
# uat_ctx_005_explicit_cycle_migration.sh — MIG-UAT-001 (C3i).
#
# Criterio de la matriz: "Migration: skill vieja vs nueva con el mismo ledger →
# misma identidad y ciclo; sin pérdida de contexto". Definido en el paquete
# hypermedia como "Caller legacy pasa cycle ID explícito → explicit vence
# inference; comportamiento compatible".
#
# Por qué este script existe: la fila estuvo NOT_RUN desde session-36 con la
# razón "requiere dos versiones de skill conviviendo; sin release nuevo". Esa
# premisa era incorrecta en dos puntos, verificados antes de escribir una línea:
#
#   1. El enunciado de origen NO es "dos skills conviviendo": es que un caller
#      legacy que pasa `--cycle` explícito debe prevalecer sobre la inferencia.
#      Eso es un contrato del RUNTIME, no del texto de la skill, y se ejercita
#      contra un solo binario y un solo ledger.
#   2. Ahora hay release publicado (v2.3.2), así que el binario real ya existe.
#
# El escenario que de verdad discrimina: DOS leases activas (estado `ambiguous`,
# donde la inferencia está bloqueada por contrato) + un `--cycle` explícito. Si
# el runtime resolviera por inferencia, devolvería `ambiguous`; si honora al
# caller explícito, resuelve al ciclo nombrado. La ambigüedad es el experimento que
# separa "vence" de "da igual": sin ella, un resolved por inferencia y un
# resolved por ciclo explícito darían la misma salida.
#
# Uso:
#   bash tests/uat_ctx_005_explicit_cycle_migration.sh [--bin <ruta>] [--keep]
#
# Exit 0 = PASS; exit 1 = FAIL; exit 2 = error de invocación.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' \
    || printf '%s/target' "$REPO_ROOT")"
BIN="${SDDK_BIN:-$TARGET_DIR/release/sddk}"
KEEP="false"

while [ $# -gt 0 ]; do
    case "$1" in
        --bin)
            BIN="${2:?--bin requires a value}"
            shift 2
            ;;
        --keep)
            KEEP="true"
            shift
            ;;
        *)
            echo "error: unknown flag $1" >&2
            exit 2
            ;;
    esac
done

if [ ! -x "$BIN" ]; then
    echo "error: sddk binary not found or not executable: $BIN" >&2
    echo "hint: cargo build --release --bin sddk" >&2
    exit 2
fi

echo "MIG-UAT-001 — binario: $BIN ($("$BIN" --version))"

SANDBOX="$(mktemp -d)"
# El trap NO puede alterar el código de salida del script: si la limpieza
# falla, el exit que se observa sería el de `rm`, no el del veredicto — una
# medición contaminada que puede reportar PASS con FAILURES>0 o al revés. Se
# guarda el status y se restaura. Además se restaura HOME antes de borrar:
# este script lo sobreescribe con el sandbox, y un gestor de archivos que
# resuelve "home" desde $HOME se negaría a trastar el directorio que contiene
# el home que el propio script creó.
# shellcheck disable=SC2329  # cleanup se invoca vía trap EXIT.
cleanup() {
    local rc=$?
    export HOME="${REAL_HOME:-$HOME}"
    if [ "$KEEP" = "true" ]; then
        echo "sandbox kept: $SANDBOX"
    else
        rm -rf "$SANDBOX" || true
    fi
    return "$rc"
}
trap cleanup EXIT
REAL_HOME="$HOME"

export SDDK_STATE_HOME="$SANDBOX/state"
export XDG_DATA_HOME="$SANDBOX/data"
export XDG_CACHE_HOME="$SANDBOX/cache"
export HOME="$SANDBOX/home"
export SDDK_ACTOR="uat-mig-001"
mkdir -p "$SDDK_STATE_HOME" "$XDG_DATA_HOME" "$XDG_CACHE_HOME" "$HOME"

WORKTREE="$SANDBOX/worktree"
mkdir -p "$WORKTREE"
git -C "$REPO_ROOT" init -q "$WORKTREE"

FAILURES=0
step() { printf '\n=== %s ===\n' "$1"; }
fail() {
    printf 'FAIL: %s\n' "$1" >&2
    FAILURES=$((FAILURES + 1))
}
ok() { printf 'ok: %s\n' "$1"; }
assert_eq() {
    if [ "$2" = "$3" ]; then
        ok "$1: $3"
    else
        fail "$1: expected $2, got $3"
    fi
}
assert_contains() {
    if grep -qF "$2" "$3"; then
        ok "$1"
    else
        fail "$1: '$2' not found in $(basename "$3")"
    fi
}
assert_absent() {
    if grep -qF "$2" "$3"; then
        fail "$1: '$2' unexpectedly present"
    else
        ok "$1"
    fi
}

jqf() { # jqf <json-file> <jq-filter>
    python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)
print(eval("data" + sys.argv[2]))' "$1" "$2"
}

# ── 0. Convergencia de adopción: fija la identidad del proyecto ─────────────
# exit 4 = no_capsule_source: degradación honesta post INC-DEBT-042, no un fallo.
step "convergencia de adopción (identidad del proyecto)"
CONVERGE="$SANDBOX/converge.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-mig-converge --format json \
    >"$CONVERGE" 2>"$SANDBOX/converge.err"
RC_CONVERGE=$?
set -e
if [ "$RC_CONVERGE" -ne 0 ] && [ "$RC_CONVERGE" -ne 4 ]; then
    fail "convergence bootstrap exited $RC_CONVERGE (esperado 0 o 4): $(cat "$SANDBOX/converge.err")"
    exit 1
fi
PROJECT_ID="$(jqf "$CONVERGE" "['project_id']")"
WORKSPACE_ID="$(jqf "$CONVERGE" "['workspace_id']")"
assert_eq "adopción convergida" "complete" "$(jqf "$CONVERGE" "['adoption']")"
ok "project_id=$PROJECT_ID workspace_id=$WORKSPACE_ID"

# ── 1. Dos ciclos, DOS leases ⇒ la inferencia queda bloqueada ───────────────
# Este es el estado que hace significativa la prueba: con ambigüedad, NINGÚN
# caller sin `--cycle` puede resolver. Por eso un `resolved` posterior sólo
# puede venir del ciclo explícito.
step "dos ciclos con lease ⇒ estado ambiguous (la inferencia no puede resolver)"
CYCLE_TARGET="mig-target"
CYCLE_OTHER="mig-other"
PREFIXED_TARGET=""
PREFIXED_OTHER=""
for cycle_name in "$CYCLE_TARGET" "$CYCLE_OTHER"; do
    if ! "$BIN" cycle start --root "$WORKTREE" --name "$cycle_name" \
        --lease-owner "owner-$cycle_name" \
        --format json >"$SANDBOX/start-$cycle_name.json" 2>"$SANDBOX/start-$cycle_name.err"; then
        fail "cycle start $cycle_name exited non-zero: $(cat "$SANDBOX/start-$cycle_name.err")"
        exit 1
    fi
    CREATED="$(jqf "$SANDBOX/start-$cycle_name.json" "['cycle_id']")"
    if [ "$cycle_name" = "$CYCLE_TARGET" ]; then
        PREFIXED_TARGET="$CREATED"
    else
        PREFIXED_OTHER="$CREATED"
    fi
    ok "ciclo creado: $CREATED"
done
# Los ids reales los impone el runtime (prefijo de proyecto): se leen de la
# salida, nunca se componen a mano.
assert_eq "ciclo objetivo bajo el prefijo del proyecto" "$PROJECT_ID/$CYCLE_TARGET" "$PREFIXED_TARGET"
assert_eq "ciclo distractor bajo el prefijo del proyecto" "$PROJECT_ID/$CYCLE_OTHER" "$PREFIXED_OTHER"

AMBIG="$SANDBOX/ambig.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-mig-ambig --format json \
    >"$AMBIG" 2>"$SANDBOX/ambig.err"
RC_AMBIG=$?
set -e
if [ "$RC_AMBIG" -ne 0 ] && [ "$RC_AMBIG" -ne 4 ]; then
    fail "bootstrap ambiguo exited $RC_AMBIG (esperado 0 o 4): $(cat "$SANDBOX/ambig.err")"
    exit 1
fi
assert_eq "sin --cycle la inferencia está bloqueada" "ambiguous" "$(jqf "$AMBIG" "['cycle']['state']")"
assert_contains "ambiguous lista el ciclo objetivo" "$PREFIXED_TARGET" "$AMBIG"
assert_contains "ambiguous lista el ciclo distractor" "$PREFIXED_OTHER" "$AMBIG"
# Nunca adivinar: en `ambiguous` no puede haber cycle_id resuelto.
if python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    cycle = json.load(handle)["cycle"]
sys.exit(1 if "cycle_id" in cycle else 0)
' "$AMBIG"; then
    ok "ambiguous sin cycle_id (no adivina)"
else
    fail "ambiguous state carries cycle_id — the runtime guessed"
fi

# ── 2. El caller legacy pasa --cycle ⇒ el explícito vence ───────────────────
# Mismo ledger, mismo binario, misma sesión de conocimiento. Lo único que
# cambia respecto al paso 1 es el argumento explícito.
step "caller legacy con --cycle explícito ⇒ resuelve al ciclo nombrado"
EXPLICIT="$SANDBOX/explicit.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-mig-explicit \
    --cycle "$PREFIXED_TARGET" --format json >"$EXPLICIT" 2>"$SANDBOX/explicit.err"
RC_EXPLICIT=$?
set -e
if [ "$RC_EXPLICIT" -ne 0 ]; then
    fail "bootstrap explícito exited $RC_EXPLICIT: $(cat "$SANDBOX/explicit.err")"
    exit 1
fi
assert_eq "el explícito gana a la ambigüedad" "explicit" "$(jqf "$EXPLICIT" "['cycle']['state']")"
assert_eq "resuelve al ciclo nombrado, no al otro" "$PREFIXED_TARGET" "$(jqf "$EXPLICIT" "['cycle']['cycle_id']")"
assert_absent "no cae al ciclo distractor" "$PREFIXED_OTHER" "$EXPLICIT"

# ── 3. Misma identidad: la migración no cambia de proyecto ─────────────────
step "identidad estable entre el caller ambiguo y el explícito"
assert_eq "project_id idéntico" "$PROJECT_ID" "$(jqf "$EXPLICIT" "['project_id']")"
assert_eq "workspace_id idéntico" "$WORKSPACE_ID" "$(jqf "$EXPLICIT" "['workspace_id']")"
assert_eq "adopción no se re-ritualiza" "complete" "$(jqf "$EXPLICIT" "['adoption']")"

# ── 4. Sin pérdida de contexto ─────────────────────────────────────────────
# Este es el criterio literal de la matriz. "Sin pérdida" no puede significar
# "igual de vacío": el camino explícito debe entregar MÁS que la degradación
# honesta del camino ambiguo, nunca menos. Se exige la capsule compilada desde
# los facts reales del ciclo (ADR-0147 D2), que es la prueba de que el
# contexto se reconstruyó y no se devolvió un placeholder.
step "el camino explícito reconstruye el contexto (sin pérdida)"
assert_eq "status completo" "complete" "$(jqf "$EXPLICIT" "['status']")"
assert_eq "contexto compilado desde el ledger" "compiled" "$(jqf "$EXPLICIT" "['context_source']")"
CAPSULE_ID="$(jqf "$EXPLICIT" "['capsule_id']")"
if [ "$CAPSULE_ID" = "None" ] || [ -z "$CAPSULE_ID" ]; then
    fail "el camino explícito no entregó capsule: sin capsule no hay contexto que no se pierda"
else
    ok "capsule_id=$CAPSULE_ID"
fi
assert_eq "la basis es la capsule entregada" "$CAPSULE_ID" "$(jqf "$EXPLICIT" "['basis_revision']")"
assert_eq "la capsule está anclada al ciclo explícito" "cycle-$PREFIXED_TARGET" \
    "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["capsule_id"].rsplit(":",2)[0])' "$EXPLICIT")"
assert_eq "binding persistido" "True" "$(jqf "$EXPLICIT" "['binding_written']")"

# Comparación explícita con la ruta degradada: el explícito no puede tener
# menos contexto que el ambiguo.
AMBIG_CAPSULE="$(jqf "$AMBIG" "['capsule_id']")"
if [ "$AMBIG_CAPSULE" = "None" ] && [ "$CAPSULE_ID" != "None" ]; then
    ok "el explícito entrega capsule donde la ambigüedad no entrega ninguna (no hay pérdida)"
else
    fail "esperado: ambiguo sin capsule y explícito con capsule; obtenido '$AMBIG_CAPSULE' / '$CAPSULE_ID'"
fi

# ── 5. La referencia es una referencia: un id que no existe no se ata ──────
# El otro lado de "explicit vence": vencer no es "aceptar cualquier cosa". Si
# el runtime se comiera un id inexistente, volvería el fail-open que esta UAT
# cierra — un envelope con `state: explicit` apuntando a una ficción. Debe
# fallar cerrado, como ya hace `sddk cycle status --cycle <desconocido>`.
step "un --cycle que no nombra ningún ciclo falla cerrado"
MISSING="$SANDBOX/missing"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-mig-missing \
    --cycle "$PROJECT_ID/does-not-exist" --format json >"$MISSING.json" 2>"$MISSING.err"
RC_MISSING=$?
set -e
if [ "$RC_MISSING" -ne 0 ]; then
    ok "exit no-cero ante referencia rota: $RC_MISSING"
else
    fail "una referencia inexistente debe fallar cerrado, devolvió exit 0"
fi
assert_contains "el error nombra la referencia rota" "does-not-exist" "$MISSING.err"
assert_contains "el error es tipado (no una ambigüedad)" "cycle not found" "$MISSING.err"
# Un fallo no debe renderizar un envelope resuelto. Se comprueba stdout de
# forma explícita en vez de intentar parsearlo a ciegas: cuando el comando
# falla, stdout está VACÍO, y `json.load("")` lanza excepción — un `if` que
# tratara ese error como "envelope presente" invertiría el resultado y
# declararía FAIL justo en el caso que queremos PASS.
if [ -s "$MISSING.json" ]; then
    if python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)
cycle = data.get("cycle") or {}
sys.exit(1 if cycle.get("state") == "explicit" else 0)
' "$MISSING.json"; then
        fail "la referencia rota emitió stdout con un envelope que NO es explicit; debe fallar sin envelope"
    else
        fail "la referencia rota produjo un envelope con state=explicit"
    fi
else
    ok "sin envelope en stdout ante la referencia rota"
fi
# Y no debe haberbinding durable a la ficción.
BINDING_FILE="$XDG_DATA_HOME/sddk/projects/$PROJECT_ID/context/bindings/uat-mig-missing.json"
if [ -f "$BINDING_FILE" ]; then
    fail "se persistió un binding a un ciclo inexistente: $BINDING_FILE"
else
    ok "sin binding durable al ciclo inexistente"
fi

# ── 6. Compatibilidad de los callers legacy (superficie de la skill) ───────
# La skill (skills/sddk-cycle-resume/SKILL.md) reconstruye estado con estos dos
# comandos en su camino explícito. Un caller legacy que los usa debe seguir
# obteniendo el mismo ciclo por las dos vías.
step "comandos legacy de la skill con ciclo explícito"
set +e
"$BIN" cycle status --root "$WORKTREE" --scope . --cycle "$PREFIXED_TARGET" --format json \
    >"$SANDBOX/cycle-status.json" 2>"$SANDBOX/cycle-status.err"
RC_STATUS=$?
set -e
assert_eq "cycle status --cycle resuelve" "0" "$RC_STATUS"
assert_eq "cycle status devuelve el ciclo nombrado" "$PREFIXED_TARGET" "$(jqf "$SANDBOX/cycle-status.json" "['cycle_id']")"

set +e
"$BIN" cycle artifacts-dir --root "$WORKTREE" --scope . --cycle "$PREFIXED_TARGET" --format json \
    >"$SANDBOX/artifacts.json" 2>"$SANDBOX/artifacts.err"
RC_ARTIFACTS=$?
set -e
assert_eq "cycle artifacts-dir --cycle resuelve" "0" "$RC_ARTIFACTS"
assert_eq "artifacts-dir devuelve el ciclo nombrado" "$PREFIXED_TARGET" "$(jqf "$SANDBOX/artifacts.json" "['cycle_id']")"
if grep -qF "$PREFIXED_TARGET" "$SANDBOX/artifacts.json"; then
    ok "el path de artefactos pertenece al ciclo nombrado"
else
    fail "artifacts-dir no apunta al ciclo nombrado"
fi

# Las dos superficies (bootstrap y cycle status) deben coincidir sobre el mismo
# identificador: es el invariante que la skill asume al documentarlos juntos.
assert_eq "bootstrap y cycle status coinciden en el ciclo" \
    "$(jqf "$EXPLICIT" "['cycle']['cycle_id']")" "$(jqf "$SANDBOX/cycle-status.json" "['cycle_id']")"

printf '\n'
if [ "$FAILURES" -eq 0 ]; then
    printf 'UAT MIG-UAT-001: PASS (todos los escenarios)\n'
    exit 0
fi
printf 'UAT MIG-UAT-001: FAIL (%s)\n' "$FAILURES" >&2
exit 1
