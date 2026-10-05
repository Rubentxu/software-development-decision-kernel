#!/usr/bin/env bash
# uat_ctx_002_context_bootstrap.sh — CTX-UAT-002/003/004 reutilizables:
# `sddk context bootstrap` como operación de alto nivel, aislado del estado
# del operador.
#
# Contrato (C3j objetivo 3, SPEC-005 CTX-003/004/005/011):
#   1. Sin ciclo activo → estado TIPADO `no_active_cycle` (no error), y el
#      binding de sesión se persiste igual (project scope).
#   2. Replay de la misma sesión → NO reescribe el binding (byte-stable) y
#      mantiene project_id/workspace_id/basis idénticos.
#   3. `--cycle <id>` explícito → estado `explicit`, binding al target Run, y
#      la capsule de ese ciclo se recupera como basis (read-reuse por digest).
#   4. El binding persistido NO contiene transcript: sólo semantic refs.
#
# Uso:
#   bash tests/uat_ctx_002_context_bootstrap.sh [--bin <ruta-binario>] [--keep]
#
#   --bin    binario sddk a ejercitar (default: target/release/sddk del repo)
#   --keep   no borrar el sandbox temporal (para inspección)
#
# Aislamiento: el sandbox usa SDDK_STATE_HOME / XDG_DATA_HOME / XDG_CACHE_HOME
# propios bajo $(mktemp -d). NUNCA toca el ledger real del operador.
#
# Exit 0 = PASS; exit 1 = FAIL. ShellCheck exclusions: los paths del sandbox
# se calculan en runtime por diseño.

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
# shellcheck disable=SC2329  # cleanup se invoca via trap EXIT.
# MEDIDO (session-84 bis 5): este `cleanup` no preservaba `$?` y no restauraba
# `HOME` antes de borrar, con lo que el codigo de salida del guard lo decidia el
# borrado y no lo que el guard midio. MEDIDO con el binario real:
#   uat_ctx_001 imprime `PASS: 3/3 applies complete` y salia 1, con
#   `mavis-trash: failed to trash '/tmp/ctx-uat-001...'` como ultima linea;
#   uat_ctx_004 imprime `UAT CTX-UAT-002 + CTX-UAT-003: PASS` y salia 64.
# Y con `--keep`, que salta el borrado, los dos salen 0: luego el veredicto era
# correcto y solo la limpieza lo destruia.
#
# LA CAUSA, MEDIDA: los cuatro mueven `HOME` DENTRO del sandbox y nunca lo
# restauran, luego el borrado corre con la casa dentro del directorio que va a
# borrar y no puede resolver donde dejar la papelera. `uat_ctx_005`, `006` y
# `007` ya lo hacen bien con `REAL_HOME` -- son la referencia, y se copia su
# patron en vez de inventar uno. MEDIDO el alcance: de los siete, cuatro con el
# defecto y tres correctos, y la division coincide EXACTAMENTE con quien
# declara `REAL_HOME`.
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
export SDDK_ACTOR="uat-ctx-002"
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

jqf() { # jqf <json-file> <jq-filter>
    python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)
print(eval("data" + sys.argv[2]))' "$1" "$2"
}

# ── 1. Bootstrap sin ciclo activo → no_active_cycle + binding persistido ─────
# Desde INC-DEBT-042 (session-46) el comando sale con **código 4** cuando no
# hay capsule que reconstruir: `status: no_capsule_source` es la degradación
# honesta (no reclamar `complete` sin capsule) y el JSON sigue siendo válido.
# Antes de ese cambio el script asumía exit 0 y quedó stale: el escenario pasó a
# FAIL en cuanto el runtime adoptó la degradación. El contrato verificado aquí
# es "exit 4 + estado tipado + binding persistido", no "exit 0".
step "1. bootstrap sin ciclo activo (CTX-004 estado 0, exit 4 = no_capsule_source)"
OUT1="$SANDBOX/out1.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-s1 --format json >"$OUT1" 2>"$SANDBOX/err1.txt"
RC1=$?
set -e
if [ "$RC1" -ne 0 ] && [ "$RC1" -ne 4 ]; then
    fail "bootstrap sin ciclo exited $RC1 (esperado 0 o 4): $(cat "$SANDBOX/err1.txt")"
    exit 1
fi
ok "código de salida tipado: $RC1"
assert_eq "status honesto sin capsule" "no_capsule_source" "$(jqf "$OUT1" "['status']")"
STATE1="$(jqf "$OUT1" "['cycle']['state']")"
assert_eq "estado tipado sin ciclo" "no_active_cycle" "$STATE1"

PROJECT1="$(jqf "$OUT1" "['project_id']")"
WORKSPACE1="$(jqf "$OUT1" "['workspace_id']")"
BASIS1="$(jqf "$OUT1" "['basis_revision']")"
CONTEXT1="$(jqf "$OUT1" "['context_source']")"
ADOPTION1="$(jqf "$OUT1" "['adoption']")"
assert_eq "adopción convergida" "complete" "$ADOPTION1"
assert_eq "basis fresh (sin capsule)" "fresh" "$CONTEXT1"
assert_eq "basis revision vacía" "empty" "$BASIS1"

# El binding vive bajo project_data (nivel proyecto) y el recibo de adopción
# bajo workspace_data (nivel workspace): se LOCALIZAN, no se adivinan.
BINDING="$(find "$XDG_DATA_HOME/sddk/projects/$PROJECT1" -path '*context/bindings/uat-s1.json' -type f | head -1)"
[ -n "$BINDING" ] || fail "no binding files under $XDG_DATA_HOME/sddk/projects/$PROJECT1"
if [ -f "$BINDING" ]; then ok "binding persistido: $BINDING"; else fail "binding missing at $BINDING"; fi

# CTX-011 / ASB-004: el transcript del host nunca se persiste.
if grep -qiE 'transcript|"messages"|"turns"' "$BINDING" 2>/dev/null; then
    fail "binding contains transcript-like content"
else
    ok "binding sin transcript (sólo refs semánticas)"
fi

# ── 2. Replay idempotente (CTX-005) ────────────────────────────────────────
step "2. replay idempotente (CTX-005 no-op semántico)"
HASH_BEFORE="$(sha256sum "$BINDING" | cut -d' ' -f1)"
OUT2="$SANDBOX/out2.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-s1 --format json >"$OUT2" 2>"$SANDBOX/err2.txt"
RC2=$?
set -e
if [ "$RC2" -ne 0 ] && [ "$RC2" -ne 4 ]; then
    fail "replay exited $RC2 (esperado 0 o 4): $(cat "$SANDBOX/err2.txt")"
    exit 1
fi
HASH_AFTER="$(sha256sum "$BINDING" | cut -d' ' -f1)"
assert_eq "binding byte-stable en replay" "$HASH_BEFORE" "$HASH_AFTER"

WRITTEN2="$(jqf "$OUT2" "['binding_written']")"
assert_eq "binding_written en replay" "False" "$WRITTEN2"

assert_eq "project_id estable" "$PROJECT1" "$(jqf "$OUT2" "['project_id']")"
assert_eq "workspace_id estable" "$WORKSPACE1" "$(jqf "$OUT2" "['workspace_id']")"

# ── 3. Ciclo explícito → estado explicit + binding Run (CTX-004) ───────────
#
# MEDIDO (session-84 bis 7, `sddk 2.11.3`): este paso fallaba 1/1 con
# `error: cycle not found: uat-cycle-1`. El guion pedia un ciclo que NUNCA
# creo, y ademas usaba un id que el runtime no puede producir: los ids reales
# los impone el runtime con prefijo de proyecto (`p-<hash>/uat-cycle-1`), de
# modo que `--cycle uat-cycle-1` a pelo no puede existir. El mismo patron que
# lo resuelve ya esta escrito en `uat_ctx_005` (que si pasa): crear el ciclo y
# LEER el id de la salida, nunca componerlo a mano.
step "3. ciclo explícito (CTX-003 paso 3 + CTX-004 explícito)"
CYCLE_NAME="uat-cycle-1"
set +e
"$BIN" cycle start --root "$WORKTREE" --name "$CYCLE_NAME" \
    --lease-owner "owner-$CYCLE_NAME" --format json \
    >"$SANDBOX/cycle-start.json" 2>"$SANDBOX/cycle-start.err"
RC_START=$?
set -e
if [ "$RC_START" -ne 0 ]; then
    fail "cycle start exited $RC_START: $(cat "$SANDBOX/cycle-start.err")"
    exit 1
fi
CYCLE_ID="$(jqf "$SANDBOX/cycle-start.json" "['cycle_id']")"
if [ -z "$CYCLE_ID" ] || [ "$CYCLE_ID" = "None" ]; then
    fail "cycle start no devolvio cycle_id"
    exit 1
fi
ok "ciclo creado con el id que impone el runtime: $CYCLE_ID"

OUT3="$SANDBOX/out3.json"
set +e
"$BIN" context bootstrap --root "$WORKTREE" --session uat-s2 --cycle "$CYCLE_ID" --format json >"$OUT3" 2>"$SANDBOX/err3.txt"
RC3=$?
set -e
if [ "$RC3" -ne 0 ] && [ "$RC3" -ne 4 ]; then
    fail "explicit-cycle bootstrap exited $RC3 (esperado 0 o 4): $(cat "$SANDBOX/err3.txt")"
    exit 1
fi
STATE3="$(jqf "$OUT3" "['cycle']['state']")"
CYCLE3="$(jqf "$OUT3" "['cycle']['cycle_id']")"
assert_eq "estado explícito" "explicit" "$STATE3"
assert_eq "ciclo explícito" "$CYCLE_ID" "$CYCLE3"

BINDING2="$(find "$XDG_DATA_HOME/sddk/projects/$PROJECT1" -path '*context/bindings/uat-s2.json' -type f | head -1)"
[ -n "$BINDING2" ] || fail "no explicit-cycle binding under project data"
[ -f "$BINDING2" ] || fail "explicit-cycle binding missing at $BINDING2"
if CYCLE_ID="$CYCLE_ID" python3 -c '
import json, os, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    binding = json.load(handle)
target = binding.get("target", {})
sys.exit(0 if target.get("kind") == "run"
         and target.get("run_ref", {}).get("RunRef") == os.environ["CYCLE_ID"] else 1)
' "$BINDING2" 2>/dev/null; then
    ok "binding con target = run:$CYCLE_ID"
else
    # El nombre de la variante serde puede variar; comprobamos el contenido.
    if grep -q "$CYCLE_ID" "$BINDING2" && grep -q '"kind": *"run"' "$BINDING2"; then
        ok "binding con target run:$CYCLE_ID"
    else
        fail "binding target is not run:$CYCLE_ID"
    fi
fi

# ── 4. Convergencia de adopción observable en disco (CTX-005) ──────────────
step "4. convergencia de adopción observable"
RECEIPT="$(find "$XDG_DATA_HOME/sddk/projects/$PROJECT1" -name adoption.json -type f | head -1)"
[ -n "$RECEIPT" ] || fail "no adoption receipt under project data"
if [ -f "$RECEIPT" ]; then
    ok "recibo de adopción escrito por el bootstrap"
else
    fail "adoption receipt missing at $RECEIPT"
fi

printf '\n'
if [ "$FAILURES" -eq 0 ]; then
    printf 'UAT context bootstrap: PASS (4/4 escenarios, contrato exit 0/4 post INC-DEBT-042)\n'
    exit 0
fi
printf 'UAT context bootstrap: FAIL (%s)\n' "$FAILURES" >&2
exit 1
