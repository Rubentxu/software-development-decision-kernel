#!/usr/bin/env bash
# sddk-agent-models — thin UI shell over `sddk dev models` (ADR-0020).
# Zero YAML manipulation in bash: every read/edit/validate/write delegates to
# the sddk CLI over the validated schema. Only write target: the bundle
# agent-models.yaml (staged in a temp file, committed with a single atomic
# rename). Exit codes: 0 success · 1 user cancel · 2 validation error ·
# 3 target/bundle unresolvable.
set -euo pipefail

# ── Static model catalogs (apply-time curated data) ───────────────────────────
OPENCODE_CATALOG=(
  deepseek/deepseek-chat deepseek/deepseek-reasoner deepseek/deepseek-v4-flash
  deepseek/deepseek-v4-pro zai-coding-plan/glm-4.7 zai-coding-plan/glm-5-turbo
  zai-coding-plan/glm-5.2 openai/gpt-5.4 openai/gpt-5.4-fast openai/gpt-5.4-mini
  openai/gpt-5.5 openai/gpt-5.5-fast
)
CLAUDE_CATALOG=(sonnet opus haiku inherit anthropic/claude-sonnet-4-5)
CODEX_CATALOG=(gpt-5.3-codex-spark gpt-5.4 gpt-5.4-fast gpt-5.4-mini gpt-5.5 gpt-5.5-fast)

# ── Feature detection: gum per-subcommand; any missing → whole-session bash
# fallback with the identical state machine and exit-code contract ─────────────
MODE=gum
if ! command -v gum >/dev/null 2>&1; then
  MODE=fallback
else
  for sub in choose filter confirm table input; do
    if ! gum "$sub" --help >/dev/null 2>&1; then
      MODE=fallback
      break
    fi
  done
fi

# ── sddk bridge: exit-code relay (2 → 2, any other non-zero → 3) ──────────────
SDDK="${SDDK_BIN:-}"
if [[ -z $SDDK ]]; then
  SDDK="$(command -v sddk 2>/dev/null || true)"
fi
if [[ -z $SDDK ]]; then
  echo "error: sddk binary not found (set SDDK_BIN)" >&2
  exit 3
fi

sddk_models() {
  local output rc
  set +e
  output="$("$SDDK" dev models "$@" 2>&1)"
  rc=$?
  set -e
  if [[ $rc -eq 2 ]]; then
    printf '%s\n' "$output" >&2
    exit 2
  fi
  if [[ $rc -ne 0 ]]; then
    printf '%s\n' "$output" >&2
    exit 3
  fi
  printf '%s\n' "$output"
}

# ── Target + staged temp file (NoRepoMutation: only agent-models.yaml) ────────
TUI_TARGET=""
target_of() {
  local output
  output="$(sddk_models list --format text)"
  TUI_TARGET="$(printf '%s\n' "$output" | head -n 1)"
  TUI_TARGET="${TUI_TARGET#target: }"
  if [[ -z $TUI_TARGET ]]; then
    echo "error: cannot resolve agent-models.yaml target" >&2
    exit 3
  fi
}

STAGE=""
stage_init() {
  target_of
  STAGE="$(mktemp)"
  if [[ -f $TUI_TARGET ]]; then
    cp "$TUI_TARGET" "$STAGE"
  fi
}
stage_cleanup() {
  [[ -n ${STAGE:-} ]] && rm -f "$STAGE"
  return 0
}
trap stage_cleanup EXIT

# ── UI primitives (single implementation switch at the top) ───────────────────
pick_one() {
  local prompt="$1"
  shift
  if [[ $MODE == gum ]]; then
    gum choose --header "$prompt" --height 12 "$@" || exit 1
  else
    PS3="$prompt "
    local item=""
    select item in "$@"; do
      if [[ -n ${item:-} ]]; then
        printf '%s\n' "$item"
        return 0
      fi
    done
  fi
}

# Read lines from a command's output without stealing the script's stdin
# (a pipeline element would consume the interactive answers).
map_lines() {
  local items=()
  mapfile -t items < <("$@")
  printf '%s\n' "${items[@]}"
}

confirm_yes_no() {
  local prompt="$1"
  if [[ $MODE == gum ]]; then
    gum confirm "$prompt" || exit 1
  else
    local answer=""
    read -r -p "$prompt (y/N): " answer
    if [[ $answer != "y" && $answer != "Y" ]]; then
      exit 1
    fi
  fi
}

# ── Screens ───────────────────────────────────────────────────────────────────
agent_names() {
  sddk_models list --format text | sed -n 's/^  \(.*\): tier=.*/\1/p'
}

list_agents() {
  sddk_models list --format text | sed -n '/^agents:/,$p' | tail -n +2
}

model_catalog() {
  local ide="$1"
  # CLEAR_OVERRIDE first: clearing falls back to the tier default.
  printf 'CLEAR_OVERRIDE\n'
  case $ide in
    opencode) opencode_catalog ;;
    zcode) printf '%s\n' "${OPENCODE_CATALOG[@]}" ;;
    claude) printf '%s\n' "${CLAUDE_CATALOG[@]}" ;;
    codex) printf '%s\n' "${CODEX_CATALOG[@]}" ;;
  esac
}

opencode_catalog() {
  local live="" rc=0
  if command -v opencode >/dev/null 2>&1; then
    set +e
    live="$(timeout 5 opencode models 2>/dev/null)"
    rc=$?
    set -e
    if [[ $rc -ne 0 || -z $live ]]; then
      echo "warning: opencode models unavailable; using static catalog" >&2
    fi
  else
    echo "warning: opencode not found; using static catalog" >&2
  fi
  if [[ -n $live ]]; then
    printf '%s\n' "$live"
  else
    printf '%s\n' "${OPENCODE_CATALOG[@]}"
  fi
}

edit_tier() {
  local agent tier names=()
  mapfile -t names <<<"$(agent_names)"
  agent="$(pick_one "Select agent" "${names[@]}")"
  tier="$(pick_one "Select tier for $agent" premium fast)"
  sddk_models set --file "$STAGE" --agent "$agent" --tier "$tier" >/dev/null
  echo "staged: $agent tier=$tier"
}

edit_override() {
  local agent ide model names=() models=()
  mapfile -t names <<<"$(agent_names)"
  agent="$(pick_one "Select agent" "${names[@]}")"
  ide="$(pick_one "Select IDE" opencode zcode claude codex)"
  mapfile -t models <<<"$(model_catalog "$ide")"
  model="$(pick_one "Select model for $agent ($ide)" "${models[@]}")"
  if [[ $model == "CLEAR_OVERRIDE" ]]; then
    sddk_models set --file "$STAGE" --agent "$agent" --clear-override "$ide" >/dev/null
    echo "staged: cleared override $ide for $agent"
  else
    sddk_models set --file "$STAGE" --agent "$agent" --override "${ide}=${model}" >/dev/null
    echo "staged: $agent $ide=$model"
  fi
}

validate_and_write() {
  sddk_models validate --file "$STAGE" >/dev/null
  echo "staged config (will write to $TUI_TARGET):"
  sddk_models list --file "$STAGE" --format text
  confirm_yes_no "Write this config to $TUI_TARGET?"
  mv "$STAGE" "$TUI_TARGET"
  echo "written: $TUI_TARGET"
  exit 0
}

main_menu() {
  local choice
  while true; do
    echo "sddk agent-models — target: $TUI_TARGET"
    choice="$(pick_one "What do you want to do?" \
      "List agents" \
      "Edit agent tier" \
      "Edit per-IDE override" \
      "Validate & write" \
      "Quit")"
    case $choice in
      "List agents") list_agents ;;
      "Edit agent tier") edit_tier ;;
      "Edit per-IDE override") edit_override ;;
      "Validate & write") validate_and_write ;;
      "Quit") exit 1 ;;
    esac
  done
}

stage_init
main_menu
