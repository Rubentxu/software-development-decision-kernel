#!/usr/bin/env bash
# ext_provider_gate.sh — C3l.4: external-provider gate where ABSENCE IS NEVER PASS.
#
# Why this exists
# ---------------
# `aiw_s5_chronos_real.rs` used to `return` when `CHRONOS_MCP_BIN` was unset.
# `cargo test` then printed `3 passed` while two of the three tests had never
# executed. This script is the launcher that makes the honest state observable
# from outside the test binary.
#
# Contract
# --------
# Emits exactly one of the five states defined in
# `crates/sddk-engine/src/ext_outcome.rs`:
#
#   pass_observed | fail_observed | blocked_external_dependency | not_run | not_applicable
#
# and one exit code:
#
#   0  pass_observed        — provider resolved AND the contract held
#   1  fail_observed        — provider resolved AND the contract broke
#   2  blocked_external_dependency — provider absent/unusable (NEVER a pass)
#   3  not_run / not_applicable
#
# The `ext_outcome_states_match_the_launcher_contract` test in
# `crates/sddk-engine/src/ext_outcome.rs` fails if this file stops knowing a
# state, so the two halves cannot drift.
#
# Usage
# -----
#   tests/ext_provider_gate.sh chronos   CHRONOS_MCP_BIN
#   tests/ext_provider_gate.sh cognicode COGNICODE_MCP_BIN
#
set -uo pipefail

PROVIDER="${1:-}"
ENV_VAR="${2:-}"

if [[ -z "$PROVIDER" || -z "$ENV_VAR" ]]; then
  echo "usage: $0 <provider> <ENV_VAR>" >&2
  exit 3
fi

# ── receipt dir ────────────────────────────────────────────────────────────
RECEIPTS_DIR="${SDDK_EXT_RECEIPTS_DIR:-tests/receipts/ext}"
mkdir -p "$RECEIPTS_DIR"
RECEIPT="$RECEIPTS_DIR/${PROVIDER}-gate.json"

emit() { # emit <state> <exit_code> <detail>
  local state="$1" code="$2" detail="$3"
  cat > "$RECEIPT" <<JSON
{
  "provider": "${PROVIDER}",
  "env_var": "${ENV_VAR}",
  "state": "${state}",
  "path": "${BIN_PATH:-}",
  "sha256": "${BIN_SHA:-}",
  "version": "${BIN_VERSION:-}",
  "capabilities": ${BIN_CAPABILITIES:-[]},
  "detail": "${detail}"
}
JSON
  echo "== ext gate: ${PROVIDER} -> ${state}"
  echo "   receipt: ${RECEIPT}"
  [[ -n "$detail" ]] && echo "   detail : ${detail}"
  return "$code"
}

# ── resolve ────────────────────────────────────────────────────────────────
BIN_PATH="${!ENV_VAR:-}"
if [[ -z "$BIN_PATH" ]]; then
  if command -v "$PROVIDER" >/dev/null 2>&1; then
    BIN_PATH="$(command -v "$PROVIDER")"
  else
    emit "blocked_external_dependency" 2 \
      "env var ${ENV_VAR} unset/empty and '${PROVIDER}' not on PATH"
  fi
  exit $?
fi

if [[ ! -f "$BIN_PATH" ]]; then
  emit "blocked_external_dependency" 2 "${BIN_PATH} is not a readable file"
  exit $?
fi

BIN_SHA="sha256:$(sha256sum "$BIN_PATH" | cut -d' ' -f1)"

# Best-effort version probe. Absence is recorded as absence, never invented.
BIN_VERSION="$("$BIN_PATH" --version 2>/dev/null | head -1 | tr -d '\n' || true)"
[[ -n "$BIN_VERSION" ]] || BIN_VERSION="unreported"

BIN_CAPABILITIES="[]"

# ── run the EXT profile ────────────────────────────────────────────────────
# `--ignored` is what makes the EXT tests run at all; the ordinary suite leaves
# them ignored precisely so that absence can never read as a pass.
set +e
CARGO_OUTPUT="$(cargo test -p sddk-engine --test "${PROVIDER}_real" -- --ignored --nocapture 2>&1)"
CARGO_EXIT=$?
set -e

if [[ $CARGO_EXIT -eq 0 ]]; then
  # The binary resolved and the profile passed: a real observation.
  BIN_CAPABILITIES="$(printf '%s' "$CARGO_OUTPUT" \
    | grep -o '"capabilities":\[[^]]*\]' | head -1 | cut -d: -f2- || true)"
  [[ -n "$BIN_CAPABILITIES" ]] || BIN_CAPABILITIES="[]"
  emit "pass_observed" 0 "EXT profile passed against ${BIN_PATH} (${BIN_SHA})"
  exit 0
fi

# A resolved provider whose profile fails is a FAIL, not a block. Collapsing
# these two is how a real regression gets reported as an environment problem.
echo "$CARGO_OUTPUT" | tail -25 >&2
emit "fail_observed" 1 "EXT profile failed against ${BIN_PATH} (${BIN_SHA})"
exit 1
