#!/usr/bin/env bash
# uat_ctx_003_durable_deltas.sh — CTX-008 a nivel de CLI: los ContextDelta
# sobreviven entre PROCESOS y un ContextBridge rehidratado los consume.
#
# Contrato (C3j objetivo 5, SPEC-005 CTX-008):
#   0. La superficie del comando existe y expone sus flags.
#   1. Sin binding durable, un delta se RECHAZA con motivo tipado: no se
#      inventa una base.
#   2. Publicar un delta lo persiste en disco con seq monotónica.
#   3. Un drain posterior —otro proceso, otro binario, otra sesión de shell—
#      lo rehidrata y lo aplica (supervivencia real entre procesos).
#   4. La base sólo avanza si el stream realmente avanzó (drain vacío no
#      mueve nada; replay completo es estable).
#   5. Un delta stale (from_revision que no corresponde) se RECHAZA y se
#      REPORTA con su razón, nunca se entrega como contexto válido.
#   6. Un fichero corrupto se SALTA y se REPORTA, nunca se convierte en
#      contexto válido.
#   7. Un delta es siempre advisory: no se convierte en instruction authority
#      (CDD-004).
#
# Uso:
#   bash tests/uat_ctx_003_durable_deltas.sh [--bin <ruta-binario>] [--keep]
#
# Aislamiento: sandbox propio con SDDK_STATE_HOME / XDG_DATA_HOME /
# XDG_CACHE_HOME / HOME. NUNCA toca el ledger real del operador.

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
export SDDK_ACTOR="uat-ctx-003"
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
assert_eq() { # <label> <expected> <actual>
    if [ "$2" = "$3" ]; then
        ok "$1: $3"
    else
        fail "$1: expected $2, got $3"
    fi
}

# Lectores de la salida json, sin eval: los campos son identificadores planos.
field() { # field <json-file> <key>
    python3 -c 'import json,sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)[sys.argv[2]])' "$1" "$2"
}
size() { # size <json-file> <key>
    python3 -c 'import json,sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(len(json.load(handle)[sys.argv[2]]))' "$1" "$2"
}
index() { # index <json-file> <key> <n>
    python3 -c 'import json,sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)[sys.argv[2]][int(sys.argv[3])])' "$1" "$2" "$3"
}

# MEDIDO (session-84 bis 7): `sddk context bootstrap` sale con codigo 4 y
# `status: no_capsule_source` cuando no hay capsule que reconstruir — la
# degradacion honesta de INC-DEBT-042. Exigir 0 aqui, como hacia el guion antes
# de que el runtime la adoptara, hacia fallar por un motivo que ya no existe.
# El binding durable (lo que este fichero mide) se escribe igual en ese camino.
bootstrap_ok() { # bootstrap_ok <session> <out-json> <err-log>
    local session="$1" out="$2" errlog="$3" rc
    set +e
    "$BIN" context bootstrap --root "$WORKTREE" --session "$session" \
        --format json >"$out" 2>>"$errlog"
    rc=$?
    set -e
    if [ "$rc" -ne 0 ] && [ "$rc" -ne 4 ]; then
        fail "bootstrap de $session exited $rc (esperado 0 o 4): $(cat "$errlog")"
        return 1
    fi
    return 0
}

SESSION="uat-ctx-003-session"

# ── 0. La superficie del comando existe y expone sus flags ────────────────
step "0. superficie del comando"
"$BIN" context delta --help >"$SANDBOX/help.txt" 2>&1 || {
    fail "context delta --help no funciona"
    exit 1
}
for flag in --session --publish --add --remove --to-revision; do
    if grep -q -- "$flag" "$SANDBOX/help.txt"; then
        ok "help expone $flag"
    else
        fail "help no expone $flag"
    fi
done

# ── 1. Sin binding, un delta se RECHAZA (no se inventa una base) ───────────
step "1. delta sin binding durable se rechaza"
if "$BIN" context delta --root "$WORKTREE" --session "$SESSION" --publish \
    --add "algo" --to-revision r1 --format json >"$SANDBOX/err0.json" 2>&1; then
    fail "un delta sin binding NO debe tener éxito"
elif grep -q "no durable binding" "$SANDBOX/err0.json"; then
    ok "rechazado con motivo tipado (no durable binding)"
else
    fail "rechazado pero sin motivo tipado: $(cat "$SANDBOX/err0.json")"
fi

# ── 2. bootstrap + publish persiste el delta en disco ─────────────────────
#
# MEDIDO (session-84 bis 7, `sddk 2.11.3`): este paso fallaba con
# `FAIL: bootstrap exited non-zero: ` y stderr VACIO. La causa no era un fallo
# del bootstrap: sin capsule que reconstruir sale con codigo **4** y
# `status: no_capsule_source`, que es la degradacion honesta que introdujo
# INC-DEBT-042 (session-46) — reclamar `complete` sin capsule seria mentir.
# El guion seguia exigiendo 0, se quedo stale el dia que el runtime adopto la
# degradacion, y lleva desde entonces fallando por un motivo que ya no existe.
#
# Lo que este paso necesita es el BINDING durable, y el binding se escribe
# igual en el camino `no_capsule_source` (medido en `uat_ctx_002` paso 1:
# "binding persistido" con exit 4). Por eso el contrato verificado aqui es
# "0 o 4 + binding presente", no "0".
step "2. publish persiste el delta con seq monotónica"
OUT1="$SANDBOX/pub1.json"
bootstrap_ok "$SESSION" "$SANDBOX/boot.json" "$SANDBOX/errb.txt" || exit 1
ok "bootstrap con codigo tipado: $(field "$SANDBOX/boot.json" status)"
PROJECT="$(field "$SANDBOX/boot.json" project_id)"

if ! "$BIN" context delta --root "$WORKTREE" --session "$SESSION" --publish \
    --add "el ledger esta bloqueado" --reason "cambio material" \
    --to-revision r1 --format json >"$OUT1" 2>"$SANDBOX/err1.txt"; then
    fail "publish exited non-zero: $(cat "$SANDBOX/err1.txt")"
    exit 1
fi
assert_eq "operation publish" "published" "$(field "$OUT1" operation)"
assert_eq "seq del primer delta" "1" "$(field "$OUT1" last_seq)"
assert_eq "basis avanzado" "r1" "$(field "$OUT1" basis_revision)"

DELTA_COUNT="$(find "$XDG_DATA_HOME/sddk/projects/$PROJECT" \
    -path "*context/deltas/$SESSION/*" -name 'delta-*.json' -type f | wc -l | tr -d ' ')"
assert_eq "un delta en disco" "1" "$DELTA_COUNT"

# ── 3. Supervivencia ENTRE PROCESOS: un drain nuevo rehidrata y aplica ─────
step "3. drain desde otro proceso (supervivencia CTX-008)"
OUT2="$SANDBOX/drain2.json"
if ! "$BIN" context delta --root "$WORKTREE" --session "$SESSION" --format json \
    >"$OUT2" 2>"$SANDBOX/err2.txt"; then
    fail "drain exited non-zero: $(cat "$SANDBOX/err2.txt")"
    exit 1
fi
assert_eq "operation drain" "drained" "$(field "$OUT2" operation)"
assert_eq "deltas rehidratados" "1" "$(field "$OUT2" applied)"
assert_eq "contenido advisory entregado" "1" "$(field "$OUT2" advisory)"
assert_eq "basis rehidratado" "r1" "$(field "$OUT2" basis_revision)"
assert_eq "sin rechazos" "0" "$(size "$OUT2" rejected)"

# Segundo delta: la secuencia sigue siendo monotónica y el primero NO se pierde.
OUT3="$SANDBOX/pub3.json"
if ! "$BIN" context delta --root "$WORKTREE" --session "$SESSION" --publish \
    --add "el ledger ya no esta bloqueado" --to-revision r2 --format json \
    >"$OUT3" 2>"$SANDBOX/err3.txt"; then
    fail "segundo publish exited non-zero: $(cat "$SANDBOX/err3.txt")"
    exit 1
fi
assert_eq "seq monotónica" "2" "$(field "$OUT3" last_seq)"
assert_eq "ambos cambios acumulados" "2" "$(field "$OUT3" advisory)"
assert_eq "basis al final del stream" "r2" "$(field "$OUT3" basis_revision)"

# ── 4. La base sólo avanza si el stream avanzó ────────────────────────────
step "4. drain idempotente y drain vacío no mueven la base"
OUT4="$SANDBOX/drain4.json"
"$BIN" context delta --root "$WORKTREE" --session "$SESSION" --format json \
    >"$OUT4" 2>"$SANDBOX/err4.txt" || fail "drain repetido exited non-zero"
assert_eq "base estable tras replay completo" "r2" "$(field "$OUT4" basis_revision)"
assert_eq "replay completo" "2" "$(field "$OUT4" applied)"

EMPTY_SESSION="uat-ctx-003-empty"
bootstrap_ok "$EMPTY_SESSION" "$SANDBOX/boot-empty.json" "$SANDBOX/err4.txt" || exit 1
EMPTY_BASIS="$(field "$SANDBOX/boot-empty.json" basis_revision)"
"$BIN" context delta --root "$WORKTREE" --session "$EMPTY_SESSION" --format json \
    >"$SANDBOX/drain-empty.json" 2>>"$SANDBOX/err4.txt" || fail "drain empty session falló"
assert_eq "drain vacío no avanza la base" "$EMPTY_BASIS" \
    "$(field "$SANDBOX/drain-empty.json" basis_revision)"
assert_eq "drain vacío no aplica nada" "0" \
    "$(field "$SANDBOX/drain-empty.json" applied)"

# ── 5. Delta stale: se RECHAZA y se REPORTA, no se entrega ─────────────────
step "5. delta stale rechazado y reportado"
STALE_SESSION="uat-ctx-003-stale"
bootstrap_ok "$STALE_SESSION" "$SANDBOX/boot-stale.json" "$SANDBOX/err5.txt" || exit 1
"$BIN" context delta --root "$WORKTREE" --session "$STALE_SESSION" --publish \
    --add "primer cambio" --to-revision s1 --format json \
    >"$SANDBOX/pub-stale.json" 2>>"$SANDBOX/err5.txt" || fail "publish stale session falló"

STALE_DELTA="$(find "$XDG_DATA_HOME/sddk/projects/$PROJECT" \
    -path "*context/deltas/$STALE_SESSION/delta-*.json" -type f | sort | head -1)"
[ -n "$STALE_DELTA" ] || { fail "no se encontró el stream de $STALE_SESSION"; exit 1; }
cat >"$(dirname "$STALE_DELTA")/delta-000000000002.json" <<'JSON'
{
  "from_revision": "revision-que-nunca-existio",
  "to_revision": "s9",
  "relevance_reason": "delta stale inyectado a mano",
  "additions": ["contenido obsoleto"],
  "deletions": [],
  "seq": 2,
  "advisory_only": true
}
JSON
OUT5="$SANDBOX/drain-stale.json"
if ! "$BIN" context delta --root "$WORKTREE" --session "$STALE_SESSION" --format json \
    >"$OUT5" 2>>"$SANDBOX/err5.txt"; then
    fail "drain con stale exited non-zero: $(cat "$SANDBOX/err5.txt")"
fi
assert_eq "un rechazo" "1" "$(size "$OUT5" rejected)"
assert_eq "seq del rechazo" "2" \
    "$(python3 -c 'import json,sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)["rejected"][0]["seq"])' "$OUT5")"
assert_eq "el delta válido sigue aplicado" "1" "$(field "$OUT5" applied)"
assert_eq "contenido stale NO entregado" "1" "$(field "$OUT5" advisory)"
assert_eq "base intacta tras rechazo" "s1" "$(field "$OUT5" basis_revision)"
REASON="$(python3 -c 'import json,sys
with open(sys.argv[1], encoding="utf-8") as handle:
    print(json.load(handle)["rejected"][0]["reason"])' "$OUT5")"
case "$REASON" in
    "") fail "el rechazo se reportó sin razón" ;;
    *) ok "el rechazo se nombra con su razón: $REASON" ;;
esac

# ── 6. Corrupción: se SALTA y se REPORTA, no se convierte en contexto ─────
step "6. delta corrupto se salta y se reporta"
CORRUPT_SESSION="uat-ctx-003-corrupt"
bootstrap_ok "$CORRUPT_SESSION" "$SANDBOX/boot-corrupt.json" "$SANDBOX/err6.txt" || exit 1
"$BIN" context delta --root "$WORKTREE" --session "$CORRUPT_SESSION" --publish \
    --add "contenido bueno" --to-revision c1 --format json \
    >"$SANDBOX/pub-corrupt.json" 2>>"$SANDBOX/err6.txt" || fail "publish corrupt session falló"
CORRUPT_DELTA="$(find "$XDG_DATA_HOME/sddk/projects/$PROJECT" \
    -path "*context/deltas/$CORRUPT_SESSION/delta-*.json" -type f | sort | head -1)"
[ -n "$CORRUPT_DELTA" ] || { fail "no se encontró el stream de $CORRUPT_SESSION"; exit 1; }
printf '{ esto no es json' >"$(dirname "$CORRUPT_DELTA")/delta-000000000002.json"

OUT6="$SANDBOX/drain-corrupt.json"
if ! "$BIN" context delta --root "$WORKTREE" --session "$CORRUPT_SESSION" --format json \
    >"$OUT6" 2>>"$SANDBOX/err6.txt"; then
    fail "drain con corrupción exited non-zero: $(cat "$SANDBOX/err6.txt")"
fi
assert_eq "fichero corrupto reportado como skipped" "delta-000000000002.json" \
    "$(index "$OUT6" replay_skipped 0)"
assert_eq "el resto del stream se entrega" "1" "$(field "$OUT6" applied)"
assert_eq "contenido corrupto no entregado" "1" "$(field "$OUT6" advisory)"

# ── 7. CDD-004: un delta es advisory, nunca instruction authority ─────────
step "7. los deltas nunca son facts"
assert_eq "facts tras 1 publish" "0" "$(field "$OUT3" facts)"
assert_eq "facts tras replay completo" "0" "$(field "$OUT4" facts)"

printf '\n'
TOTAL=7
if [ "$FAILURES" -eq 0 ]; then
    printf 'UAT CTX-003: PASS (%s/%s escenarios)\n' "$TOTAL" "$TOTAL"
    exit 0
fi
printf 'UAT CTX-003: FAIL (%s fallos)\n' "$FAILURES" >&2
exit 1
