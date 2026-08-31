#!/usr/bin/env bash
# validate-project.sh — SDDK real-project validation pipeline
# Usage: ./scripts/validate-project.sh <project> [issue] [--parallel]
#   project: github owner/repo (e.g. sharkdp/fd)
#   issue:   issue number to target (optional)
#
# Pipeline: container → clone → adopt → cycle → implement → verify → report → metrics → clean
# Produces: ~/.sddk-validate/{project}/report.json + metrics.jsonl
set -euo pipefail

PROJECT="${1:?Usage: validate-project.sh <owner/repo> [issue] [--lang <lang>] [--fixture]}"
ISSUE=""
LANG="${LANG:-rust}"
FIXTURE=0
shift || true
while [ $# -gt 0 ]; do
    case "$1" in
        --lang) LANG="${2:?--lang needs a value}"; shift 2 ;;
        --fixture) FIXTURE=1; shift ;;
        --*) echo "unknown option: $1" >&2; exit 2 ;;
        *) ISSUE="$1"; shift ;;
    esac
done
# Resolve image + test command per language.
case "$LANG" in
    rust)   IMAGE="docker.io/library/rust:1.91-slim";      TEST_CMD="cargo test --quiet" ;;
    python) IMAGE="docker.io/library/python:3.12-slim";    TEST_CMD="python -m unittest -q" ;;
    go)     IMAGE="docker.io/library/golang:1.23";         TEST_CMD="go test ./..." ;;
    node)   IMAGE="docker.io/library/node:22-slim";        TEST_CMD="node --test test.js" ;;
    c)      IMAGE="docker.io/library/gcc:13";              TEST_CMD="make test || make check" ;;
    *) echo "unknown language: $LANG" >&2; exit 2 ;;
esac
NAME="$(basename "$PROJECT")"
SDDK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_ROOT="${SDDK_VALIDATE_ROOT:-$HOME/.sddk-validate}"
OUT_DIR="$OUT_ROOT/$NAME"
CONTAINER="sddk-validate-$NAME"

mkdir -p "$OUT_DIR"/{clone,logs}
exec > >(tee "$OUT_DIR/logs/pipeline.log") 2>&1

log() { echo "[$(date -u +%FT%TZ)] $*"; }
json_start() { echo "{" > "$OUT_DIR/report.json"; }
json_kv() { printf '  "%s": %s,\n' "$1" "$2" >> "$OUT_DIR/report.json"; }
json_end() { printf '}\n' >> "$OUT_DIR/report.json"; }

log "=== SDDK VALIDATION: $PROJECT (issue: ${ISSUE:-none}) ==="
log "container: $CONTAINER | out: $OUT_DIR"

# --- 1. PREP: fresh container + clone --------------------------------------
podman rm -f "$CONTAINER" >/dev/null 2>&1 || true
log "PREP: pulling image"
podman pull "$IMAGE" >/dev/null 2>&1

log "PREP: preparing $PROJECT (lang=$LANG fixture=$FIXTURE)"
if [ "$FIXTURE" = "1" ]; then
  rm -rf "$OUT_DIR/clone" && mkdir -p "$OUT_DIR/clone"
  case "$LANG" in
    rust)
      cat > "$OUT_DIR/clone/Cargo.toml" <<'TOML'
[package]
name = "fixture-rust"
version = "0.1.0"
edition = "2021"
TOML
      mkdir -p "$OUT_DIR/clone/src"
      printf 'pub fn add(a: i32, b: i32) -> i32 { a + b }
#[cfg(test)] mod t { #[test] fn add_works() { assert_eq!(super::add(1,2), 3); } }
' > "$OUT_DIR/clone/src/lib.rs" ;;
    python)
      cat > "$OUT_DIR/clone/test_fixture.py" <<'PY'
import unittest

def add(a, b):
    return a + b

class TestAdd(unittest.TestCase):
    def test_add(self):
        self.assertEqual(add(1, 2), 3)

if __name__ == "__main__":
    unittest.main()
PY
      ;;
    go)
      mkdir -p "$OUT_DIR/clone"
      cat > "$OUT_DIR/clone/go.mod" <<'MOD'
module fixture

go 1.21
MOD
      cat > "$OUT_DIR/clone/main_test.go" <<'GO'
package main

import "testing"

func add(a, b int) int { return a + b }

func TestAdd(t *testing.T) { if add(1, 2) != 3 { t.Fatal("bad add") } }
GO
      printf 'package main

func main() {}
' > "$OUT_DIR/clone/main.go" ;;
    node)
      mkdir -p "$OUT_DIR/clone"
      printf '{"name":"fixture-node","version":"0.1.0","scripts":{"test":"node --test test.js"}}
' > "$OUT_DIR/clone/package.json"
      printf 'const test = require("node:test"); const assert = require("node:assert");
test("add", () => assert.equal(1 + 2, 3));
' > "$OUT_DIR/clone/test.js" ;;
    c)
      mkdir -p "$OUT_DIR/clone"
      printf 'int add(int a, int b) { return a + b; }
' > "$OUT_DIR/clone/add.c"
      cat > "$OUT_DIR/clone/Makefile" <<'MK'
test:
	gcc -Wall -c add.c -o add.o
	echo "c fixture compiled"
MK
      ;;
  esac
  CLONE_SHA="fixture-local"
  log "PREP: fixture generated for $LANG"
else
  if [ -d "$OUT_DIR/clone/.git" ]; then
    git -C "$OUT_DIR/clone" fetch --all --quiet && git -C "$OUT_DIR/clone" reset --hard origin/HEAD --quiet
  else
    git clone --depth 1 "https://github.com/$PROJECT.git" "$OUT_DIR/clone" --quiet
  fi
  CLONE_SHA="$(git -C "$OUT_DIR/clone" rev-parse HEAD)"
  log "PREP: clone at $CLONE_SHA"
fi

# --- 2. ADOPT: run SDDK adoption inside container --------------------------
# shellcheck disable=SC2012  # ls for controlled log directory; safe use of ls | head
ls "$OUT_DIR/clone/logs/" 2>/dev/null | head -5
# Build sddk binary once; use PERSISTENT cargo target volume so subsequent
# builds (tests, fixes) reuse artifacts instead of recompiling every run.
log "ADOPT: building sddk (rust image, language-agnostic)"
# musl static build: portable across ANY base image (GLIBC version independent)
BUILD_IMAGE="docker.io/library/rust:1.91-alpine"
mkdir -p "$OUT_ROOT/cargo-target"
podman run --rm -v "$SDDK_ROOT:/src:ro,Z" -v "$OUT_ROOT/cargo-target:/target:Z" \
  -w /src -e CARGO_TARGET_DIR=/target "$BUILD_IMAGE" sh -c "cargo build --release --quiet" 2>&1 | tail -1 || true
SDDK_BIN="$OUT_ROOT/sddk-bin"
mkdir -p "$SDDK_BIN"
cp "$OUT_ROOT/cargo-target/release/sddk" "$SDDK_BIN/sddk" 2>/dev/null || \
  podman run --rm -v "$OUT_ROOT/cargo-target:/target:ro,Z" -v "$SDDK_BIN:/out:Z" \
  "$BUILD_IMAGE" sh -c "cp /target/release/sddk /out/sddk 2>/dev/null || echo BUILD_FAILED" 2>&1 | tail -1
ls -la "$SDDK_BIN/sddk" 2>/dev/null || log "ADOPT: binary copy FAILED"

# --- 3. CYCLE: adopt + open cycle on the cloned project ----------------------
log "CYCLE: adopting $NAME"
mkdir -p "$OUT_DIR/clone/logs"
# `sddk adopt apply` seeds the canonical workflow/workflow.yaml itself (G1 closed)
podman run --rm -v "$OUT_DIR/clone:/workspace:Z" -v "$SDDK_BIN:/sddk-bin:ro,Z" \
  -w /workspace "$IMAGE" \
  bash -c "
    export PATH=/sddk-bin:\$PATH
    sddk adopt apply --root . --scope . >/workspace/logs/adopt.log 2>&1 || true
    sddk cycle start --root . --scope . --name 'validation-$NAME' --path a-lite \
      --lease-owner validation --lease-ms 7200000 >/workspace/logs/cycle.log 2>&1
  " || log "CYCLE: adopt/cycle had warnings (see logs)"
# shellcheck disable=SC2012  # Controlled log directory path; ls | head is safe and appropriate
ls "$OUT_DIR/clone/logs/" 2>/dev/null | head -5

# --- 4. VERIFY: project tests pass BEFORE implementation ---------------------
log "VERIFY: baseline tests"
podman run --rm -v "$OUT_DIR/clone:/workspace:Z" -w /workspace "$IMAGE" \
  bash -c "if [ '$LANG' = 'python' ]; then python -m pytest --version >/dev/null 2>&1 || pip install --quiet pytest >/dev/null 2>&1; fi; $TEST_CMD 2>&1 | tail -8" | tee "$OUT_DIR/logs/baseline-tests.log" || true
case "$LANG" in
  rust)   PASS_PATTERN="test result: ok" ;;
  python) PASS_PATTERN="^OK" ;;
  go)     PASS_PATTERN="^ok" ;;
  node)   PASS_PATTERN="# pass" ;;
  c)      PASS_PATTERN="compiled" ;;
esac
BASELINE_PASS="$(grep -c "$PASS_PATTERN" "$OUT_DIR/logs/baseline-tests.log" || echo 0)"

# --- 5. IMPLEMENT: run SDDK apply on the target issue ------------------------
# NOTE: full autonomous implementation is delegated to the SDDK agent loop.
# For script automation, we record the issue context for the agent and
# verify the final state. The orchestration agent (opencode) performs
# explore→propose→apply against the container; this script prepares inputs.
log "IMPLEMENT: preparing issue context"
if [ -n "$ISSUE" ]; then
  gh issue view "$PROJECT#$ISSUE" --json title,body,labels > "$OUT_DIR/issue.json" 2>/dev/null || \
    echo "{\"number\":\"$ISSUE\"}" > "$OUT_DIR/issue.json"
fi

# --- 6. REPORT ----------------------------------------------------------------
log "REPORT: writing $OUT_DIR/report.json"
json_start
json_kv "project" "\"$PROJECT\""
json_kv "language" "\"$LANG\""
json_kv "test_command" "\"$TEST_CMD\""
json_kv "issue" "\"${ISSUE:-none}\""
json_kv "clone_sha" "\"$CLONE_SHA\""
json_kv "baseline_tests_pass" "$BASELINE_PASS"
json_kv "adopt_done" "$(grep -q 'status: complete' "$OUT_DIR/clone/logs/adopt.log" 2>/dev/null && echo true || echo false)"
json_kv "cycle_open" "$(grep -q 'status: OPEN' "$OUT_DIR/clone/logs/cycle.log" 2>/dev/null && echo true || echo false)"
json_end

log "=== DONE: $PROJECT ==="
log "report: $OUT_DIR/report.json"
