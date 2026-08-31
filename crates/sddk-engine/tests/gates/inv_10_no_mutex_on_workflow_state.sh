#!/usr/bin/env bash
# INV-10: No Mutex/RwLock on workflow state
# Cycle-21 adds the shell-level gate that cycle-20 verify flagged as missing.
# Excludes INV-10 permitted exceptions per ADR-0054 + ADR-0050 §Permitted Exceptions:
#   - Arc<Mutex<NodeRun>>                (P1 closure, ADR-0054)
#   - Arc<Mutex<dyn GraphStore + Send>>  (per-child scratch store)
#   - Receiver<ChildResult>              (mpsc, not a Mutex but grep avoids false positives)
#   - CountingSemaphore internal Mutex<usize> (ADR-0055: backpressure primitive, not workflow state lock)
set -euo pipefail

# Find workspace root
WS_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
SRC_DIR="${WS_ROOT}/crates/sddk-engine/src"

if [[ ! -d "${SRC_DIR}" ]]; then
  echo "ERROR: ${SRC_DIR} not found"
  exit 2
fi

# Search for std::sync::Mutex, parking_lot::Mutex, RwLock — but exclude permitted patterns
# Excludes:
#   - node_run / store / EventStore / Receiver< : ADR-0054 permitted exceptions
#   - CountingSemaphore / condvar / permits     : ADR-0055 backpressure primitive (not workflow-state lock)
#   - // INV-10 comment lines (contain keyword but are not violations)
matches=$(grep -rn \
  -E '(std::sync::Mutex|parking_lot::Mutex|tokio::sync::Mutex|RwLock)' \
  "${SRC_DIR}" 2>/dev/null \
  | grep -v -E '(node_run|store:|EventStore|Receiver<|CountingSemaphore|condvar|permits|// INV-10)' \
  || true)

if [[ -n "${matches}" ]]; then
  echo "INV-10 VIOLATION: Mutex/RwLock found in sddk-engine workflow state"
  echo "${matches}"
  exit 1
fi

echo "INV-10 OK: no Mutex/RwLock on workflow state in sddk-engine"
exit 0