#!/usr/bin/env bash
# uat_ctx_004_cycle_inference.sh — CTX-UAT-002 y CTX-UAT-003 (C3i, exit gate
# "ciclo único inferido sin ID explícito" y "múltiples ciclos → ambigüedad
# tipada, nunca adivinar"), automatizados contra el runtime real.
#
# Por qué este script existe: la matriz declaraba CTX-UAT-002 y CTX-UAT-003
# NOT_RUN desde session-40 con la razón "gate humano: la única forma de dejar
# una lease activa es escribir en el ledger real". Esa premisa era incorrecta:
# `sddk cycle lock acquire` y `sddk cycle start` escriben en el MISMO ledger
# aislable que el resto de la suite (SDDK_STATE_HOME / XDG_DATA_HOME / HOME
# bajo mktemp). Este script automatiza el escenario que faltaba sin tocar
# jamás el ledger del operador.
#
# Nomenclatura: el nombre del fichero y sus literales de salida usan
# "CTX-UAT-002/003" (los IDs de la matriz), NO el prefijo numérico del
# fichero. `tests/uat_ctx_002_context_bootstrap.sh` cubre los estados
# `no_active_cycle`/`explicit` y emite el literal "UAT CTX-002"; este fichero
# cubre inferencia por lease. Los dos escenarios son disjuntos.
#
# Uso:
#   bash tests/uat_ctx_004_cycle_inference.sh [--bin <ruta-binario>] [--keep]
#
#   --bin    binario sddk a ejercitar (default: target/release/sddk del repo)
#   --keep   no borrar el sandbox temporal (para inspección)
#
# Exit 0 = PASS; exit 1 = FAIL; exit 2 = error de invocación.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
# El target dir puede venir de CARGO_TARGET_DIR, de la config de cargo o del
# repo; no hardcodear `target/release` porque el binario puede no estar ahí.
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

SANDBOX="$(mktemp -d)"
# shellcheck disable=SC2329  # cleanup se invoca vía trap EXIT.
cleanup() {
    if [ "$KEEP" = "true" ]; then
        echo "sandbox kept: $SANDBOX"
    else
        rm -rf "$SANDBOX"
    fi
}
trap cleanup EXIT

export SDDK_STATE_HOME="$SANDBOX/state"
export XDG_DATA_HOME="$SANDBOX/data"
export XDG_CACHE_HOME="$SANDBOX/cache"
export HOME="$SANDBOX/home"
export SDDK_ACTOR="uat-ctx-004"
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
# assert_eq <label> <expected> <actual>
assert_eq() {
    if [ "$2" = "$3" ]; then
        ok "$1: $3"
    else
        fail "$1: expected $2, got $3"
    fi
}
# assert_contains <label> <needle> <file>
assert_contains() {
    if grep -qF "$2" "$3"; then
        ok "$1"
    else
        fail "$1: '$2' not found in $(basename "$3")"
    fi
}
# assert_absent <label> <needle> <file>
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

# ── CTX-UAT-002: un lease activo ⇒ inferencia sin --cycle ───────────────────
# Dos ciclos existen en el ledger, pero sólo uno tiene lease viva. El
# bootstrap NO recibe --cycle: debe resolver el que la tiene. Si devolviera
# "ambiguous" estaría contando leases, no ciclos, y el exit gate de C3i
# ("ciclo único inferido sin ID explícito") estaría incumplido.
step "CTX-UAT-002: inferencia del ciclo con lease activa, sin --cycle"

CYCLE_LOCKED="uat-lease-cycle"
CYCLE_IDLE="uat-idle-cycle"

# `cycle start` exige el proyecto convergido; el bootstrap lo crea, así que
# primero dejamos que el bootstrap converja la adopción del worktree.
CONVERGE="$SANDBOX/converge.json"
# exit 4 = no_capsule_source: degradación honesta post INC-DEBT-042, no un fallo.
# Lo que este script verifica es la INFERENCIA del ciclo, no la existencia de capsule.
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-converge --format json >"$CONVERGE" 2>"$SANDBOX/converge.err"
RC_CONVERGE=$?
set -e
if [ "$RC_CONVERGE" -ne 0 ] && [ "$RC_CONVERGE" -ne 4 ]; then
    fail "convergence bootstrap exited $RC_CONVERGE (esperado 0 o 4): $(cat "$SANDBOX/converge.err")"
    exit 1
fi
PROJECT_ID="$(jqf "$CONVERGE" "['project_id']")"
assert_eq "adopción convergida antes de crear ciclos" "complete" "$(jqf "$CONVERGE" "['adoption']")"

# `cycle start` deriva el cycle_id del nombre (`CycleId::from_parts(project_id,
# name)`) y NO acepta --cycle. Además ofrece `--lease-owner`: crear el ciclo
# CON lease en una sola operación es la vía nativa del escenario "un lease
# activa", y evita fabricar un estado con dos comandos que podrían dejar
# residuos entre sí.
#
# El primero nace con lease; el segundo SIN lease. Esa asimetría es el
# discriminante real entre "un lease activa" y "dos ciclos existen".
PREFIXED_LOCKED=""
PREFIXED_IDLE=""
for cycle_name in "$CYCLE_LOCKED" "$CYCLE_IDLE"; do
    if [ "$cycle_name" = "$CYCLE_LOCKED" ]; then
        # El array vacío es intencional: el segundo ciclo nace sin lease.
        OWNER_ARGS=(--lease-owner uat-owner-a)
    else
        OWNER_ARGS=()
    fi
    if ! "$BIN" cycle start --root "$WORKTREE" --name "$cycle_name" "${OWNER_ARGS[@]}" \
        --format json >"$SANDBOX/start-$cycle_name.json" 2>"$SANDBOX/start-$cycle_name.err"; then
        fail "cycle start $cycle_name exited non-zero: $(cat "$SANDBOX/start-$cycle_name.err")"
        exit 1
    fi
    CREATED="$(jqf "$SANDBOX/start-$cycle_name.json" "['cycle_id']")"
    if [ "$cycle_name" = "$CYCLE_LOCKED" ]; then
        PREFIXED_LOCKED="$CREATED"
        assert_contains "el ciclo con lease trae su owner" "uat-owner-a" "$SANDBOX/start-$cycle_name.json"
    else
        PREFIXED_IDLE="$CREATED"
    fi
    ok "ciclo creado: $CREATED"
done

# Los identificadores reales los impone el runtime (prefijo de proyecto), así
# que se leen de la salida del comando, nunca se componen a mano: componerlos
# sería asumir un detalle del que el comando es dueño.
assert_eq "ciclo con lease es el primero" "$PROJECT_ID/$CYCLE_LOCKED" "$PREFIXED_LOCKED"
assert_eq "ciclo sin lease es el segundo" "$PROJECT_ID/$CYCLE_IDLE" "$PREFIXED_IDLE"

BOOT2="$SANDBOX/boot2.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-infer-1 --format json >"$BOOT2" 2>"$SANDBOX/boot2.err"
RC2=$?
set -e
if [ "$RC2" -ne 0 ] && [ "$RC2" -ne 4 ]; then
    fail "bootstrap con lease exited $RC2 (esperado 0 o 4): $(cat "$SANDBOX/boot2.err")"
    exit 1
fi
STATE2="$(jqf "$BOOT2" "['cycle']['state']")"
CYCLE2="$(jqf "$BOOT2" "['cycle']['cycle_id']")"
assert_eq "estado con lease única" "resolved" "$STATE2"
assert_eq "ciclo inferido = el que tiene la lease" "$PREFIXED_LOCKED" "$CYCLE2"
assert_absent "no adivina el ciclo sin lease" "$PREFIXED_IDLE" "$BOOT2"

# La capsule del ciclo inferido debe ser la basis: inferir no es sólo saber el
# id, es producir el mismo basis_revision que un --cycle explícito daría.
assert_eq "contexto compilado desde el ciclo inferido" "compiled" "$(jqf "$BOOT2" "['context_source']")"
assert_eq "status honesto con capsule" "complete" "$(jqf "$BOOT2" "['status']")"

# ── CTX-UAT-003: dos leases ⇒ ambigüedad tipada, nunca adivinar ─────────────
step "CTX-UAT-003: dos leases activas ⇒ AmbiguousCycle con candidates"

# La segunda lease se toma sobre el ciclo que antes no la tenía: ahora hay
# DOS ciclos con lease viva, que es el discriminante real de la ambigüedad.
# `--timestamp` es RFC 3339 (no epoch): la validación de `timestamp_ms` rechaza
# un entero con "a character literal was not valid".
NOW_RFC3339="$(python3 -c 'import datetime; print(datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"))')"
if ! "$BIN" cycle lock acquire --root "$WORKTREE" --cycle "$PREFIXED_IDLE" \
    --owner uat-owner-b --lease-ms 3600000 --timestamp "$NOW_RFC3339" \
    --format json >"$SANDBOX/acquire2.json" 2>"$SANDBOX/acquire2.err"; then
    fail "second lock acquire exited non-zero: $(cat "$SANDBOX/acquire2.err")"
    exit 1
fi
assert_contains "segunda lease adquirida" "uat-owner-b" "$SANDBOX/acquire2.json"

BOOT3="$SANDBOX/boot3.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-infer-2 --format json >"$BOOT3" 2>"$SANDBOX/boot3.err"
RC3=$?
set -e
if [ "$RC3" -ne 0 ] && [ "$RC3" -ne 4 ]; then
    fail "bootstrap con dos leases exited $RC3 (esperado 0 o 4): $(cat "$SANDBOX/boot3.err")"
    exit 1
fi
STATE3="$(jqf "$BOOT3" "['cycle']['state']")"
assert_eq "estado con dos leases" "ambiguous" "$STATE3"
assert_contains "candidates lista el ciclo con lease A" "$PREFIXED_LOCKED" "$BOOT3"
assert_contains "candidates lista el ciclo con lease B" "$PREFIXED_IDLE" "$BOOT3"

# Nunca adivinar: en estado ambiguous no puede haber un cycle_id resuelto.
if python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    cycle = json.load(handle)["cycle"]
# Un bootstrap ambiguo declara candidates; jamás cycle_id.
sys.exit(1 if "cycle_id" in cycle else 0)
' "$BOOT3"; then
    ok "estado ambiguous sin cycle_id (no adivina)"
else
    fail "ambiguous state carries cycle_id — the runtime guessed"
fi

# Los candidates deben traer owner y expiración: son accionables por el humano.
if python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    candidates = json.load(handle)["cycle"]["candidates"]
sys.exit(0 if len(candidates) == 2
         and all(c.get("owner") and c.get("expires_at_ms") for c in candidates)
         else 1)
' "$BOOT3"; then
    ok "candidates con owner + expires_at_ms (2/2)"
else
    fail "candidates incomplete: expected 2 with owner+expires_at_ms"
fi

# Recovery: soltar una lease devuelve la inferencia a un único ciclo. Es la
# prueba de que la ambigüedad es estado de runtime, no un fallo permanente.
step "recovery: liberar una lease ⇒ vuelve a inferir sin adivinar"
# El fencing token lo emite el runtime en el acquire; se LEE de esa salida en
# vez de hardcodearlo, porque es un valor que el ledger incrementa en cada
# acquire y que la CLI exige para probar que el que libera es el que la tomó.
FENCING="$(jqf "$SANDBOX/acquire2.json" "['fencing_token']")"
if ! "$BIN" cycle lock release --root "$WORKTREE" --cycle "$PREFIXED_IDLE" \
    --owner uat-owner-b --fencing-token "$FENCING" \
    --format json >"$SANDBOX/release.json" 2>"$SANDBOX/release.err"; then
    fail "lock release exited non-zero: $(cat "$SANDBOX/release.err")"
    exit 1
fi
BOOT4="$SANDBOX/boot4.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-infer-3 --format json >"$BOOT4" 2>"$SANDBOX/boot4.err"
RC4=$?
set -e
if [ "$RC4" -ne 0 ] && [ "$RC4" -ne 4 ]; then
    fail "bootstrap post-release exited $RC4 (esperado 0 o 4): $(cat "$SANDBOX/boot4.err")"
    exit 1
fi
assert_eq "vuelve a resolved con lease única" "resolved" "$(jqf "$BOOT4" "['cycle']['state']")"
assert_eq "vuelve al ciclo con lease" "$PREFIXED_LOCKED" "$(jqf "$BOOT4" "['cycle']['cycle_id']")"

printf '\n'
if [ "$FAILURES" -eq 0 ]; then
    printf 'UAT CTX-UAT-002 + CTX-UAT-003: PASS (todos los escenarios)\n'
    exit 0
fi
printf 'UAT CTX-UAT-002 + CTX-UAT-003: FAIL (%s)\n' "$FAILURES" >&2
exit 1
