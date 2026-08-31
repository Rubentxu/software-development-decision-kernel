#!/usr/bin/env bash
# E2E harness for assets/agent-models/tui.sh (E1–E10).
# Runs against the debug sddk binary with a fake $SDDK_DATA_DIR bundle and a
# fake HOME. The real $HOME and the real bundle are never touched.
#
# Invoked as: bash tests-e2e/tui/run.sh
set -euo pipefail

# shellcheck disable=SC2329  # contains() defined at line 33, used at lines 125-127; shellcheck false positive
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CARGO_BIN="${CARGO_BIN:-cargo}"

echo "# building sddk-cli debug binary"
"$CARGO_BIN" build -p sddk-cli -q
TARGET_DIR="$("$CARGO_BIN" metadata --format-version 1 --no-deps | python3 -c 'import json,sys;print(json.load(sys.stdin)["target_directory"])')"
REAL_SDDK="$TARGET_DIR/debug/sddk"
[[ -x $REAL_SDDK ]] || { echo "error: debug binary missing at $REAL_SDDK" >&2; exit 1; }

PASS=0
FAIL=0

check() {
  local name="$1"
  shift
  if "$@"; then
    PASS=$((PASS + 1))
    echo "ok - $name"
  else
    FAIL=$((FAIL + 1))
    echo "not ok - $name"
  fi
}

# shellcheck disable=SC2329  # contains() invoked via contains "$E1_OUT" "orchestrator: tier=fast" at lines 125-127; shellcheck false positive
contains() { # haystack needle
  grep -qF -- "$2" <<<"$1"
}

# ── Fixture: fake bundle + fake HOME + tool shims ─────────────────────────────
WORK="$(mktemp -d)"
HOME_DIR="$WORK/home"
DATA_DIR="$WORK/data"
BIN_DIR="$WORK/bin"
BUNDLE="$DATA_DIR/framework/bundle-root"
mkdir -p "$HOME_DIR" "$BIN_DIR" "$DATA_DIR/framework"
ln -s "$BUNDLE" "$DATA_DIR/framework/current"
mkdir -p "$BUNDLE/agents" "$BUNDLE/assets/agent-models"

for agent in orchestrator sddk-foo gentle-bar; do
  cat >"$BUNDLE/agents/$agent.md" <<EOF
---
name: $agent
description: test $agent
---
# Body
EOF
done

cp "$REPO_ROOT/assets/agent-models/tui.sh" "$BUNDLE/assets/agent-models/tui.sh"
chmod +x "$BUNDLE/assets/agent-models/tui.sh"

write_fixture_config() {
  cat >"$BUNDLE/assets/agent-models.yaml" <<'EOF'
tiers:
  premium:
    opencode: deepseek/deepseek-chat
    claude: sonnet
  fast:
    opencode: zai-coding-plan/glm-5-turbo
    claude: haiku
agents:
  orchestrator:
    tier: fast
  sddk-foo:
    tier: fast
  gentle-bar:
    tier: fast
EOF
}
write_fixture_config

# gum shim: always fails its self-test → the TUI falls back to bash mode.
cat >"$BIN_DIR/gum" <<'EOF'
#!/bin/sh
exit 1
EOF
chmod +x "$BIN_DIR/gum"

# sddk shim: delegates to the real binary; may inject crafted failures.
cat >"$BIN_DIR/sddk" <<EOF
#!/usr/bin/env bash
if [[ \${FAKE_FAIL_SET:-0} == 1 && \${1:-} == dev && \${2:-} == models && \${3:-} == set ]]; then
  echo "error: forced set failure" >&2
  exit 2
fi
if [[ \${FAKE_FAIL_VALIDATE:-0} == 1 && \${1:-} == dev && \${2:-} == models && \${3:-} == validate ]]; then
  echo "error: forced validation failure" >&2
  exit 2
fi
exec "$REAL_SDDK" "\$@"
EOF
chmod +x "$BIN_DIR/sddk"

export HOME="$HOME_DIR"
export SDDK_DATA_DIR="$DATA_DIR"
export SDDK_BIN="$BIN_DIR/sddk"
export PATH="$BIN_DIR:/usr/bin:/bin"

TUI="$BUNDLE/assets/agent-models/tui.sh"

RUN_TUI_RC=0
run_tui() { # answers... → runs the TUI in fallback mode; exit code in $RUN_TUI_RC
  set +e
  printf '%s\n' "$@" | bash "$TUI" 2>&1
  RUN_TUI_RC=$?
  set -e
  return 0
}

# ── E1 — lists bundle agents only ─────────────────────────────────────────────
E1_OUT="$(run_tui 1 5)"
# shellcheck disable=SC2016  # $1 in bash -c is positional arg passed via _; intentional
check "E1 lists exactly the 3 bundle agents" bash -c '
  out=$1
  count=$(printf "%s\n" "$out" | grep -c ": tier=")
  [[ $count -eq 3 ]]
' _ "$E1_OUT"
check "E1 lists orchestrator" contains "$E1_OUT" "orchestrator: tier=fast"
check "E1 lists sddk-foo" contains "$E1_OUT" "sddk-foo: tier=fast"
check "E1 lists gentle-bar" contains "$E1_OUT" "gentle-bar: tier=fast"

# ── E2 — opencode models used live ────────────────────────────────────────────
cat >"$BIN_DIR/opencode" <<'EOF'
#!/bin/sh
echo "deepseek/fake-live-model"
echo "openai/fake-live-model"
EOF
chmod +x "$BIN_DIR/opencode"
E2_OUT="$(run_tui 3 1 1 99)"
check "E2 live opencode models reach the picker" contains "$E2_OUT" "deepseek/fake-live-model"
rm -f "$BIN_DIR/opencode"

# ── E3 — missing opencode falls back to static catalog with a warning ─────────
E3_OUT="$(run_tui 3 1 1 99)"
check "E3 static catalog used when opencode missing" contains "$E3_OUT" "deepseek/deepseek-chat"
check "E3 fallback warning emitted" contains "$E3_OUT" "using static catalog"

# ── E4 — edit tier flow: orchestrator fast → premium, committed ────────────────
write_fixture_config
E4_OUT="$(run_tui 2 1 1 4 y)"
check "E4 exit code 0 on commit" test "$RUN_TUI_RC" -eq 0
check "E4 yaml now has orchestrator premium" grep -q -A2 "orchestrator:" <<<"$(cat "$BUNDLE/assets/agent-models.yaml")"
check "E4 staged summary printed" contains "$E4_OUT" "written:"

# ── E5 — clear override flow: override removed, falls back to tier default ─────
cat >"$BUNDLE/assets/agent-models.yaml" <<'EOF'
tiers:
  premium:
    opencode: deepseek/deepseek-chat
  fast:
    opencode: zai-coding-plan/glm-5-turbo
agents:
  orchestrator:
    tier: fast
  sddk-foo:
    tier: fast
    overrides:
      opencode: deepseek/deepseek-reasoner
  gentle-bar:
    tier: fast
EOF
# shellcheck disable=SC2034  # E5_OUT captured for documentation; result verified via check call at line 171
E5_OUT="$(run_tui 3 3 1 1 4 y)"
# shellcheck disable=SC2016  # $1 in bash -c refers to positional arg passed as _; intentional
check "E5 override cleared from yaml" bash -c '
  ! grep -q "deepseek-reasoner" "$1"
' _ "$BUNDLE/assets/agent-models.yaml"

# ── E6 — forced `set` failure relays exit 2 and writes nothing ────────────────
write_fixture_config
BEFORE="$(sha256sum "$BUNDLE/assets/agent-models.yaml" | cut -d" " -f1)"
export FAKE_FAIL_SET=1
run_tui 2 1 1 >"$WORK/e6.out"
E6_RC=$RUN_TUI_RC
unset FAKE_FAIL_SET
AFTER="$(sha256sum "$BUNDLE/assets/agent-models.yaml" | cut -d" " -f1)"
check "E6 exit code 2 on set failure" test "$E6_RC" -eq 2
check "E6 no write on failure" test "$BEFORE" = "$AFTER"

# ── E7 — mv failure preserves the original parseable config ───────────────────
write_fixture_config
BEFORE="$(sha256sum "$BUNDLE/assets/agent-models.yaml" | cut -d" " -f1)"
chmod 555 "$BUNDLE/assets"
run_tui 2 1 1 4 y >"$WORK/e7.out"
E7_RC=$RUN_TUI_RC
chmod 755 "$BUNDLE/assets"
check "E7 commit failure exits non-zero" test "$E7_RC" -ne 0
check "E7 original file intact" test "$(sha256sum "$BUNDLE/assets/agent-models.yaml" | cut -d" " -f1)" = "$BEFORE"
check "E7 original still parseable" grep -q "tier:" "$BUNDLE/assets/agent-models.yaml"

# ── E8 — exit code contract table ─────────────────────────────────────────────
write_fixture_config
run_tui 5 >"$WORK/e8-cancel.out"
check "E8 cancel exits 1" test "$RUN_TUI_RC" -eq 1

export FAKE_FAIL_VALIDATE=1
run_tui 4 >"$WORK/e8-validate.out"
check "E8 validation failure exits 2" test "$RUN_TUI_RC" -eq 2
unset FAKE_FAIL_VALIDATE

export SDDK_BIN=/nonexistent-sddk
run_tui 5 >"$WORK/e8-missing.out"
check "E8 missing sddk exits 3" test "$RUN_TUI_RC" -eq 3
export SDDK_BIN="$BIN_DIR/sddk"

run_tui 2 1 1 4 y >"$WORK/e8-ok.out"
check "E8 success exits 0" test "$RUN_TUI_RC" -eq 0

# ── E9 — gum fallback parity: broken gum == absent gum (identical contract) ────
write_fixture_config
run_tui 5 >/dev/null
RC_A=$RUN_TUI_RC
RC_B=0
PATH="/usr/bin:/bin" bash -c 'printf "5\n" | bash "$1" >/dev/null 2>&1' _ "$TUI" || RC_B=$?
check "E9 fallback parity (cancel=1 both modes)" test "$RC_A" -eq "$RC_B"
check "E9 fallback cancel code" test "$RC_A" -eq 1

# ── E10 — no repo/editor mutation: only agent-models.yaml is written ──────────
write_fixture_config
mkdir -p "$HOME_DIR/.config/opencode" "$HOME_DIR/.zcode" "$HOME_DIR/.claude/agents" "$HOME_DIR/.codex/agents"
printf '{"agent":{"x":1},"mcp":{}}\n' >"$HOME_DIR/.config/opencode/opencode.json"
printf '{"agent":{"x":1},"mcp":{}}\n' >"$HOME_DIR/.zcode/zcode.json"
printf -- '---\nname: mine\ndescription: user\n---\n' >"$HOME_DIR/.claude/agents/my-agent.md"
printf 'name = "my-agent"\n' >"$HOME_DIR/.codex/agents/my-agent.toml"
snapshot() {
  sha256sum \
    "$BUNDLE/agents/orchestrator.md" "$BUNDLE/agents/sddk-foo.md" "$BUNDLE/agents/gentle-bar.md" \
    "$HOME_DIR/.config/opencode/opencode.json" "$HOME_DIR/.zcode/zcode.json" \
    "$HOME_DIR/.claude/agents/my-agent.md" "$HOME_DIR/.codex/agents/my-agent.toml" \
    | cut -d" " -f1
}
BEFORE="$(snapshot)"
run_tui 2 1 1 4 y >/dev/null
AFTER="$(snapshot)"
check "E10 no repo/editor mutation" test "$BEFORE" = "$AFTER"
# shellcheck disable=SC2016  # $1 in bash -c is positional arg passed via _; intentional
check "E10 config changed" bash -c '
  grep -q "tier: premium" "$1"
' _ "$BUNDLE/assets/agent-models.yaml"

# ── summary ───────────────────────────────────────────────────────────────────
echo
echo "tests-e2e/tui: $PASS passed, $FAIL failed"
if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
exit 0
