#!/usr/bin/env bash
# uat_ctx_007_context_expand.sh — CTX-UAT-015 (+ la mitad observable de
# CTX-UAT-014) sobre la superficie CLI real.
#
# Criterio de la matriz:
#   CTX-UAT-015: "Expand evidence/decision ref → contenido correcto +
#                 ContextReadRecord actualizado".
#   CTX-UAT-014: "Context exceeds inline budget → capsule contiene
#                 refs/progressive disclosure, no dump ilimitado".
#
# Qué demuestra este script y qué no:
#   - La disclosure es PROGRESIVA de verdad: el envelope del bootstrap NO
#     lleva el contenido de la capsule (solo su basis revision); el único
#     camino hacia el contenido es expandir UNA referencia cada vez.
#   - El contenido expandido viene del LEDGER (la autoridad), no de la prosa
#     de la capsule: el work item se crea con `sddk change` (superficie del
#     producto) y el expand devuelve su título y estado reales.
#   - Cada lectura queda registrada en un ContextReadRecord durable por
#     sesión, y el log CRECE entre invocaciones ("actualizado").
#   - Una referencia que la capsule no contiene es un error TIPADO que lista
#     las refs disponibles (el patrón candidates), nunca contenido inventado.
#
# Uso:
#   bash tests/uat_ctx_007_context_expand.sh [--bin <ruta>] [--keep]
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

echo "CTX-UAT-015 — binario: $BIN ($("$BIN" --version))"

SANDBOX="$(mktemp -d)"
# shellcheck disable=SC2329  # cleanup se invoca vía trap EXIT.
# El trap NO puede alterar el código de salida del script: si la limpieza
# falla, el exit observado sería el de `rm`, no el del veredicto. Se guarda
# el status y se restaura, y se devuelve HOME a su valor real antes de borrar
# (este script lo sobreescribe con el sandbox).
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
export SDDK_ACTOR="uat-ctx-015"
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
        fail "$1: '$2' unexpectedly present in $(basename "$3")"
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

# ── 0. Convergencia: una sesión bindeada a PROYECTO no puede expandir ──────
# Fail-closed antes de que exista ciclo: sin binding de ciclo no hay capsule
# que expandir, y el comando lo dice en vez de inventar contexto.
step "fail-closed: sesión sin ciclo ⇒ error tipado"
CONVERGE="$SANDBOX/converge.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-015-proj --format json \
    >"$CONVERGE" 2>"$SANDBOX/converge.err"
RC_CONVERGE=$?
set -e
if [ "$RC_CONVERGE" -ne 0 ] && [ "$RC_CONVERGE" -ne 4 ]; then
    fail "convergence bootstrap exited $RC_CONVERGE (esperado 0 o 4): $(cat "$SANDBOX/converge.err")"
    exit 1
fi
PROJECT_ID="$(jqf "$CONVERGE" "['project_id']")"
ok "project_id=$PROJECT_ID"

MISSING_CYCLE="$SANDBOX/nocycle"
set +e
"$BIN" context expand --root "$WORKTREE" --session uat-015-proj \
    --ref "cycle:x" --format json >"$MISSING_CYCLE.json" 2>"$MISSING_CYCLE.err"
RC_NOCYCLE=$?
set -e
if [ "$RC_NOCYCLE" -ne 0 ]; then
    ok "expand sin ciclo sale no-cero: $RC_NOCYCLE"
else
    fail "expand en sesión bindeada a proyecto debe fallar, devolvió exit 0"
fi
assert_contains "el error explica que no hay capsule de ciclo" "no capsule" "$MISSING_CYCLE.err"

# ── 1. Ciclo real con lease + work item por la superficie del producto ─────
step "ciclo real + work item creado con sddk change"
CYCLE="uat-015-expand"
if ! "$BIN" cycle start --root "$WORKTREE" --name "$CYCLE" --lease-owner owner-015 \
    --format json >"$SANDBOX/start.json" 2>"$SANDBOX/start.err"; then
    fail "cycle start exited non-zero: $(cat "$SANDBOX/start.err")"
    exit 1
fi
CYCLE_ID="$(jqf "$SANDBOX/start.json" "['cycle_id']")"
ok "ciclo creado: $CYCLE_ID"

WI_TITLE="cerrar CTX-UAT-015 con contenido real"
# `sddk change` (fachada M6.1) resuelve el proyecto desde el CWD (root="."
# hardcodeado): la invocación se hace DENTRO del worktree del sandbox. Las
# variables de entorno del sandbox siguen exportadas, así que el ledger
# sigue siendo el aislado.
if ! (cd "$WORKTREE" && "$BIN" change --cycle-id "$CYCLE_ID" \
    --title "$WI_TITLE" --description "el expand debe devolver este titulo desde el ledger" \
    --format json) >"$SANDBOX/change.json" 2>"$SANDBOX/change.err"; then
    fail "sddk change exited non-zero: $(cat "$SANDBOX/change.err")"
    exit 1
fi
if grep -qF '"work_item_id"' "$SANDBOX/change.json"; then
    WORK_ITEM_ID="$(jqf "$SANDBOX/change.json" "['work_item_id']")"
elif grep -qF '"id"' "$SANDBOX/change.json"; then
    WORK_ITEM_ID="$(jqf "$SANDBOX/change.json" "['id']")"
else
    fail "sddk change no devolvió el id del work item: $(cat "$SANDBOX/change.json")"
    exit 1
fi
ok "work item creado: $WORK_ITEM_ID"

# ── 2. Bootstrap: binding de ciclo + capsule compilada desde facts ─────────
step "bootstrap bindea la sesión y compila la capsule"
BOOT="$SANDBOX/boot.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-015-bound \
    --cycle "$CYCLE_ID" --format json >"$BOOT" 2>"$SANDBOX/boot.err"
RC_BOOT=$?
set -e
if [ "$RC_BOOT" -ne 0 ]; then
    fail "bootstrap exited $RC_BOOT: $(cat "$SANDBOX/boot.err")"
    exit 1
fi
assert_eq "binding a ciclo resuelto" "explicit" "$(jqf "$BOOT" "['cycle']['state']")"
assert_eq "capsule compilada" "compiled" "$(jqf "$BOOT" "['context_source']")"

# CTX-UAT-014 (mitad observable): la disclosure es progresiva — el envelope
# del bootstrap NO vuelca la capsule. Si el contenido viajara inline, este
# escenario estaría resolviendo el budget con un dump, no con refs.
step "CTX-UAT-014: el envelope del bootstrap no vuelca la capsule"
assert_absent "bootstrap no contiene must_read" "must_read" "$BOOT"
assert_absent "bootstrap no contiene content de capsule" "\"content\"" "$BOOT"
assert_absent "bootstrap no contiene work-item:" "work-item:" "$BOOT"

# ── 3. Expand del ciclo: contenido correcto ────────────────────────────────
step "expand cycle:<id> devuelve el ciclo del ledger"
EXP_CYCLE="$SANDBOX/expand-cycle"
set +e
"$BIN" context expand --root "$WORKTREE" --session uat-015-bound \
    --ref "cycle:$CYCLE_ID" --format json >"$EXP_CYCLE.json" 2>"$EXP_CYCLE.err"
RC_EC=$?
set -e
if [ "$RC_EC" -ne 0 ]; then
    fail "expand cycle exited $RC_EC: $(cat "$EXP_CYCLE.err")"
else
    assert_eq "kind = cycle" "cycle" "$(jqf "$EXP_CYCLE.json" "['kind']")"
    assert_contains "el contenido nombra el ciclo" "$CYCLE_ID" "$EXP_CYCLE.json"
    CAPSULE_ID="$(jqf "$EXP_CYCLE.json" "['capsule_id']")"
    if [ "$CAPSULE_ID" = "None" ] || [ -z "$CAPSULE_ID" ]; then
        fail "expand no reporta la capsule de la que salió el ref"
    else
        ok "capsule_id=$CAPSULE_ID"
    fi
fi

# ── 4. Expand del work item: contenido LEDGER, no prosa de capsule ─────────
step "expand work-item:<id> devuelve el registro real del ledger"
EXP_WI="$SANDBOX/expand-wi"
set +e
"$BIN" context expand --root "$WORKTREE" --session uat-015-bound \
    --ref "work-item:$WORK_ITEM_ID" --format json >"$EXP_WI.json" 2>"$EXP_WI.err"
RC_EW=$?
set -e
if [ "$RC_EW" -ne 0 ]; then
    fail "expand work-item exited $RC_EW: $(cat "$EXP_WI.err")"
else
    assert_eq "kind = work-item" "work-item" "$(jqf "$EXP_WI.json" "['kind']")"
    assert_contains "el contenido trae el título del ledger" "$WI_TITLE" "$EXP_WI.json"
    assert_contains "el contenido trae la descripción del ledger" \
        "el expand debe devolver este titulo" "$EXP_WI.json"
    SHA_WI="$(jqf "$EXP_WI.json" "['content_sha256']")"
    if [ "$SHA_WI" = "None" ] || [ -z "$SHA_WI" ]; then
        fail "expand sin content_sha256: la lectura no es auditable"
    else
        ok "content_sha256 presente"
    fi
fi

# ── 5. ContextReadRecord actualizado: el log crece por sesión ──────────────
step "ContextReadRecord: una fila por expand, persistida y creciente"
READS_FILE="$XDG_DATA_HOME/sddk/projects/$PROJECT_ID/context/reads/uat-015-bound.json"
if [ -f "$READS_FILE" ]; then
    N_READS="$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))))" "$READS_FILE")"
    if [ "$N_READS" -ge 2 ]; then
        ok "read log acumula $N_READS lecturas (cycle + work-item)"
    else
        fail "read log con $N_READS lecturas; esperadas >= 2"
    fi
    if grep -qF "\"work-item:$WORK_ITEM_ID\"" "$READS_FILE"; then
        ok "el log nombra el ref expandido"
    else
        fail "el log no nombra work-item:$WORK_ITEM_ID"
    fi
    if grep -qF "$SHA_WI" "$READS_FILE"; then
        ok "el log lleva el hash del contenido leído"
    else
        fail "el log no lleva el content_sha256 del expand"
    fi
else
    fail "no existe el read log de la sesión: $READS_FILE"
fi

# ── 6. Ref desconocida: error tipado con las refs disponibles ──────────────
step "ref desconocida ⇒ error tipado que lista las disponibles"
EXP_BAD="$SANDBOX/expand-bad"
set +e
"$BIN" context expand --root "$WORKTREE" --session uat-015-bound \
    --ref "work-item:no-existe" --format json >"$EXP_BAD.json" 2>"$EXP_BAD.err"
RC_BAD=$?
set -e
if [ "$RC_BAD" -ne 0 ]; then
    ok "expand de ref desconocida sale no-cero: $RC_BAD"
else
    fail "una ref desconocida debe fallar, devolvió exit 0"
fi
assert_contains "el error nombra la ref pedida" "work-item:no-existe" "$EXP_BAD.err"
assert_contains "el error lista la ref disponible del ciclo" "cycle:$CYCLE_ID" "$EXP_BAD.err"
assert_contains "el error lista el work item disponible" "work-item:$WORK_ITEM_ID" "$EXP_BAD.err"
# Un fallo no emite envelope en stdout.
if [ -s "$EXP_BAD.json" ]; then
    fail "la ref desconocida emitió stdout: $(head -c 120 "$EXP_BAD.json")"
else
    ok "sin envelope en stdout ante ref desconocida"
fi

# ── 7. Sin binding: el mismo fail-closed que el delta ──────────────────────
step "sesión sin binding ⇒ error tipado que nombra la sesión"
EXP_NOBIND="$SANDBOX/expand-nobind"
set +e
"$BIN" context expand --root "$WORKTREE" --session uat-015-ghost \
    --ref "cycle:x" --format json >"$EXP_NOBIND.json" 2>"$EXP_NOBIND.err"
RC_NB=$?
set -e
if [ "$RC_NB" -ne 0 ]; then
    ok "expand sin binding sale no-cero: $RC_NB"
else
    fail "expand sin binding debe fallar, devolvió exit 0"
fi
assert_contains "el error nombra la sesión" "uat-015-ghost" "$EXP_NOBIND.err"

printf '\n'
if [ "$FAILURES" -eq 0 ]; then
    printf 'UAT CTX-UAT-015: PASS (todos los escenarios)\n'
    exit 0
fi
printf 'UAT CTX-UAT-015: FAIL (%s)\n' "$FAILURES" >&2
exit 1
