#!/usr/bin/env bash
# uat_ctx_001_adoption_convergence.sh — CTX-UAT-001 reutilizable: bootstrap
# repetido de adopción sobre un proyecto real, aislado del estado del
# operador.
#
# Contrato (C3i objetivo 2, roadmap):
#   1. `sddk adopt apply` ×N sobre el mismo proyecto → `status: complete`
#      en todas las invocaciones.
#   2. El recibo `adoption.json` queda BYTE-ESTABLE desde la primera
#      convergencia (idéntico project/workspace/paths; timestamp/actor
#      congelados salvo drift real de fingerprint).
#   3. La identidad (project_id) extraída del recibo es ÚNICA y estable —
#      pin de runtime: `bootstrap_identity_is_unique_and_stable_across_restarts_and_refresh`.
#
# Uso:
#   bash tests/uat_ctx_001_adoption_convergence.sh \
#       [--bin <ruta-binario>] [--repeats N] [--keep]
#
#   --bin      binario sddk a ejercitar (default: target/release/sddk del repo)
#   --repeats  número total de applies (default: 20; mínimo 3)
#   --keep     no borrar el sandbox temporal (para inspección)
#
# Aislamiento: el sandbox usa SDDK_STATE_HOME / XDG_DATA_HOME / XDG_CACHE_HOME
# propios bajo $(mktemp -d). NUNCA toca el ledger real del operador.
#
# Exit 0 = PASS; exit 1 = FAIL (con diff del recibo que se movió).
# ShellCheck exclusions: sandbox paths are computed at runtime by design.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
# El target dir puede venir de CARGO_TARGET_DIR, de la config de cargo o del
# repo; no hardcodear `target/release` porque el binario puede no estar ahí.
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' \
    || printf '%s/target' "$REPO_ROOT")"
BIN="${SDDK_BIN:-$TARGET_DIR/release/sddk}"
REPEATS=20
KEEP="false"

while [ $# -gt 0 ]; do
    case "$1" in
        --bin)
            BIN="${2:?--bin requires a value}"
            shift 2
            ;;
        --repeats)
            REPEATS="${2:?--repeats requires a value}"
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

if [ "$REPEATS" -lt 3 ]; then
    echo "error: --repeats must be >= 3 (need apply + >=2 replays)" >&2
    exit 2
fi

if [ ! -x "$BIN" ]; then
    echo "error: binary not found or not executable: $BIN" >&2
    echo "hint: cargo build --release -p sddk-cli (or pass --bin <path>)" >&2
    exit 2
fi

SANDBOX="$(mktemp -d "${TMPDIR:-/tmp}/ctx-uat-001.XXXXXX")"
PROJ="$SANDBOX/proj"
mkdir -p "$PROJ"
git -C "$PROJ" init -q
git -C "$PROJ" remote add origin https://example.com/ctx-uat-001/repo.git
# Identidad local al sandbox: el wrapper git del operador firma fail-closed
# en repos no clasificados; un UAT nunca debe tocar identidad global.
git -C "$PROJ" config user.name "ctx-uat-001"
git -C "$PROJ" config user.email "ctx-uat-001@example.invalid"
git -C "$PROJ" commit -q --allow-empty -m "chore: seed"

cleanup() {
    if [ "$KEEP" = "true" ]; then
        echo "sandbox kept: $SANDBOX"
    else
        rm -rf -- "$SANDBOX"
    fi
}
trap cleanup EXIT

export SDDK_STATE_HOME="$SANDBOX/state"
export XDG_DATA_HOME="$SANDBOX/data"
export XDG_CACHE_HOME="$SANDBOX/cache"

COMMON=(--root "$PROJ" --scope . --format json)

echo "== CTX-UAT-001: $REPEATS applies with $BIN =="

"$BIN" adopt apply "${COMMON[@]}" >/dev/null

# Localizar el recibo de adopción dentro del estado aislado.
RECEIPT="$(find "$SANDBOX/data" -name adoption.json -type f | head -1)"
if [ -z "$RECEIPT" ]; then
    echo "FAIL: adoption receipt not found under $SANDBOX/data" >&2
    exit 1
fi

SHA1="$(sha256sum "$RECEIPT" | cut -d' ' -f1)"
PROJECT_ID="$("$BIN" adopt status "${COMMON[@]}" | python3 -c 'import json,sys; print(json.load(sys.stdin)["project_id"])')"
echo "apply #1  complete; receipt=$SHA1 project_id=$PROJECT_ID"

for i in $(seq 2 "$REPEATS"); do
    OUT="$("$BIN" adopt apply "${COMMON[@]}")"
    STATUS="$(echo "$OUT" | python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])')"
    if [ "$STATUS" != "complete" ]; then
        echo "FAIL: apply #$i returned status=$STATUS (expected complete)" >&2
        exit 1
    fi
    SHA_N="$(sha256sum "$RECEIPT" | cut -d' ' -f1)"
    if [ "$SHA_N" != "$SHA1" ]; then
        echo "FAIL: receipt mutated at apply #$i" >&2
        echo "  expected sha256: $SHA1" >&2
        echo "  observed sha256: $SHA_N" >&2
        [ "$KEEP" = "true" ] || cp "$RECEIPT" /tmp/ctx-uat-001-mutated-receipt.json
        exit 1
    fi
done

# Identidad estable: status tras la tormenta de applies sigue reportando el
# mismo project_id que el recibo congelado.
STATUS_JSON="$("$BIN" adopt status "${COMMON[@]}")"
STATUS_ID="$(echo "$STATUS_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["project_id"])')"
if [ "$STATUS_ID" != "$PROJECT_ID" ]; then
    echo "FAIL: project_id moved after $REPEATS applies" >&2
    echo "  expected: $PROJECT_ID" >&2
    echo "  observed: $STATUS_ID" >&2
    exit 1
fi

echo "PASS: $REPEATS/$REPEATS applies complete; receipt byte-stable ($SHA1); identity stable ($PROJECT_ID)"
