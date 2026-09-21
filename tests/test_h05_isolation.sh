#!/usr/bin/env bash
# tests/test_h05_isolation.sh — H05 isolation: the seam
# `set_process_service_for_tests` must NOT be exported by the
# production rlib or release binary of `sddk-engine`.
#
# Cycle: p-63676b11dc0ef88f/c1-h05-seam-isolation
#
# ROADMAP §2 C1 H05: "test-seam set_process_service_for_tests con
# semántica real y aislamiento de producción". The companion Rust
# integration test (tests/h05_seam_test_only.rs) pins the production
# API surface; THIS shell test pins the binary-isolation contract.
#
# What we assert:
#   1. `nm target/release/libsddk_engine.rlib` MUST NOT contain a
#      defined symbol `set_process_service_for_tests`. (`--defined-only`
#      would be enough but `nm` defaults are fine for this check.)
#   2. `nm target/release/sddk` MUST NOT reference the symbol.
#
# If `cargo build --release -p sddk-engine` is not present, we
# silently skip the binary check with a message; CI runs this script
# AFTER `cargo build --release`, so the check is meaningful.
#
# Exit codes:
#   0  PASS — seam absent from both rlib and binary (or build artefacts
#             missing and we explicitly skip).
#   1  FAIL — seam symbol is present in either artefact.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE/.."

REPO_ROOT="$(pwd)"
# Honour CARGO_TARGET_DIR (the sandbox may redirect it). When unset,
# `cargo build --release` writes to `target/release/`.
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
RLIB="$CARGO_TARGET_DIR/release/libsddk_engine.rlib"
BIN="$CARGO_TARGET_DIR/release/sddk"

PASS=0
FAIL=0

note() { printf '  %s\n' "$*" ; }
ok() { printf 'PASS  %s\n' "$1"; PASS=$((PASS+1)) ; }
ko() { printf 'FAIL  %s\n' "$1"; FAIL=$((FAIL+1)) ; }

# --- 1) rlib inspection ---
if [[ -f "$RLIB" ]]; then
    if nm "$RLIB" 2>/dev/null | grep -q 'set_process_service_for_tests'; then
        ko "rlib export leaks set_process_service_for_tests"
        note "  found $(nm "$RLIB" | grep -c 'set_process_service_for_tests') occurrence(s)"
    else
        ok "rlib does not export set_process_service_for_tests"
    fi
else
    note "skip: $RLIB not present (run cargo build --release -p sddk-engine first)"
fi

# --- 2) release binary inspection ---
if [[ -f "$BIN" ]]; then
    if nm "$BIN" 2>/dev/null | grep -q 'set_process_service_for_tests'; then
        ko "binary leak: set_process_service_for_tests is exported"
        note "  found in $(nm "$BIN" | grep -c 'set_process_service_for_tests') line(s)"
    else
        ok "release binary does not export set_process_service_for_tests"
    fi
else
    note "skip: $BIN not present (run cargo build --release -p sddk-cli first)"
fi

echo ""
echo "=== matrix result: PASS=$PASS FAIL=$FAIL ==="
[[ $FAIL -eq 0 ]] || exit 1
exit 0
