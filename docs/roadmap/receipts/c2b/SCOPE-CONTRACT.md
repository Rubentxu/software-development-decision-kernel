# C2b — SCOPE-CONTRACT: Real Chronos runtime integration (T12–T14)

**Cycle:** C2b (Runtime Enhanced path)
**Baseline:** `main@3c81239` (post C2a docs commit; workspace v1.169.142)
**Opened:** 2026-09-22T08:02:00Z
**Owner:** orchestrator
**Pre-flight finding:** `chronos-mcp` binary NOT present on host; not installable via `cargo install` (no crates.io match).

## 1. Objective

Demonstrate T12–T14 (UAT-MATRIX) with real Chronos runtime evidence, executing against the actual provider binary.

## 2. Reasons this scope is gated as NOT_EVALUATED (not yet opened for apply)

| Check | Result |
|---|---|
| `which chronos-mcp` | not found |
| `which chronos` | not found |
| `ls ~/.cargo/bin/chron*` | empty |
| `cargo search chronos-mcp` | empty |
| Adapter target (`crates/sddk-engine/src/runtime_evidence_port_mcp.rs:58-67`) | expects `chronos-mcp` stdio JSON-RPC binary |

The historical cycle `p-63676b11dc0ef88f/aiw-s5-chronos-runtime` (AIW-S5) at `runtime_evidence_port_mcp.rs:3` already documented S5 DISCOVERY (protocol 2025-03-26, probe_start/get_execution_summary semantics). That evidence is **HISTORICAL**, not current.

## 3. Proposed apply path (deferred until preconditions met)

If and when `chronos-mcp` becomes installable or is provided by the operator:

1. Spawn `chronos-mcp` via the existing adapter (`runtime_evidence_port_mcp.rs:53-67`).
2. Execute T12 (capture >0 events, claim non-satisfied → real verdict).
3. Execute T13 (capture=0, probe failure, timeout, restart → UNKNOWN/ERROR typed).
4. Execute T14 (two programs, resource/protocol budgets → per-program evidence).
5. Emit `UAT-EVIDENCE.yaml` with command+output+SHA per scenario.
6. Emit `C2b-RECEIPT.md` with PASS_OBSERVED or BLOCKED status (real, not PASS_BY_CODE_READING).

## 4. STOP conditions

- If `chronos-mcp` remains absent → do NOT open SCOPE-CONTRACT for apply. Emit C2b-RECEIPT with `status: NOT_EVALUATED_PROVIDER_MISSING` and the same shape as C2a-RECEIPT.

## 5. Out-of-scope

- No code changes to the runtime evidence adapter.
- No JCode or CogniCode work.
- No release.

## 6. Acceptance

This SCOPE-CONTRACT is accepted when:
- It is committed to the roadmap (yes, this file).
- The corresponding C2b-RECEIPT.md is committed with the correct NOT_EVALUATED status.
- The session-journal entry documents the parallel finding with C2a.
