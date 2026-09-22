# C2b — RECEIPT

**Cycle:** C2b (Real Chronos runtime integration, T12–T14)
**Status:** `NOT_EVALUATED_PROVIDER_MISSING`
**Baseline:** `main@3c81239` (workspace v1.169.142)
**Issued:** 2026-09-22T08:02:30Z

## 1. Outcome

Same shape as C2a: the runtime provider binary (`chronos-mcp`) is absent and not installable from crates.io. All three target UAT scenarios (T12, T13, T14) are `NOT_EVALUATED`. The historical AIW-S5 evidence is preserved as `HISTORICAL` (per UAT-MATRIX T33 last sentence).

## 2. Evidence

Full evidence in [UAT-EVIDENCE.yaml](UAT-EVIDENCE.yaml). Key OBSERVED commands:

| Command | Output |
|---|---|
| `which chronos-mcp` | (not found) |
| `cargo search chronos-mcp` | (empty — no crates.io match) |
| `ls ~/.cargo/bin/chron*` | empty |

Adapter code that targets `chronos-mcp`:
- `crates/sddk-engine/src/runtime_evidence_port_mcp.rs:53-67` (`Command::new(binary).stdin(Stdio::piped())…`)
- `crates/sddk-cli/src/verify_kernel_cmd.rs:327` (literal `chronos-mcp` in error message)

## 3. Decision

C2b parked awaiting operator decision, parallel to C2a. Recovery actions identical in shape: provide `chronos-mcp` binary, or accept DEFERRED.

## 4. Parallel finding with C2a

C2a (cognicode-mcp) and C2b (chronos-mcp) fail for the same root cause class: the SDDK adapters were authored against provider binaries that ship separately from the public CLI and are not on crates.io. This is **systemic** — not a per-cycle accident. The roadmap C2 contract needs an explicit decision: either the operator provides both binaries for certification cycles, or the SDDK adapters need an alternative transport (TCP fallback to `cognicode serve`, eBPF/ptrace fallback for runtime evidence, ACP for JCode).

This is recorded as a candidate ADR for the next governance cycle.

## 5. No code changes

No commits to source. Codebase at `main@3c81239` (post-C2a docs commit), tree clean.

## 6. Acceptance of this RECEIPT

Honest `NOT_EVALUATED` with full evidence and concrete recovery. Source SHA anchored. Pointer reconcilable.

## 7. Next WorkItem

C2c JCode next, then C3 cycle decision.
