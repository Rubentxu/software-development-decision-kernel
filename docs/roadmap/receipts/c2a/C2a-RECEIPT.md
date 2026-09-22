# C2a — RECEIPT (session-11 close re-investigation)

**Cycle:** C2a (Real CogniCode integration, T08–T11)
**Status:** `NOT_EVALUATED_PROVIDER_MISSING` (CONFIRMED by re-investigation 2026-09-22T08:49Z)
**Baseline (initial):** `main@5f493ab` (workspace v1.169.142)
**Baseline (re-investigation):** `main@a5f279c` (workspace v1.169.149)
**Issued:** 2026-09-22T07:58:30Z (initial); updated 2026-09-22T08:50:00Z (re-investigation)
**Issuer:** orchestrator (delegated; evidence-gathering performed directly)

## 1. Outcome

C2a could not be executed as `PASS_OBSERVED` because the provider binary the SDDK adapter requires (`cognicode-mcp`) is **not installed** on this host, is **not installable** via `cargo install` (no crates.io match), and the available `cognicode` v0.97.3 CLI **does not expose the MCP server**. All four target UAT scenarios (T08, T09, T10, T11) remain `NOT_EVALUATED` per [CERTIFICATIONS.md §3](../../CERTIFICATIONS.md) and the UAT-MATRIX T33 row.

This is **not** a release blocker: per UAT-MATRIX T33, "Ausencia de binario EXT en perfil enhanced → BLOCKED/NOT_RUN; Base puede permanecer válido."

## 2. Evidence

Full evidence in [UAT-EVIDENCE.yaml](UAT-EVIDENCE.yaml). Original prior-session UAT preserved at
[c2a-prep/UAT-EVIDENCE-prep-2026-09-22T0849.yaml](c2a-prep/UAT-EVIDENCE-prep-2026-09-22T0849.yaml).

Key OBSERVED commands (re-investigation 2026-09-22T08:49Z):

| Command | Output |
|---|---|
| `cognicode --version` | `cognicode 0.97.3` |
| `cognicode serve --help` | `Start the MCP server ... -p, --port <PORT> [default: 8080]` |
| `cognicode serve --port 8080` (5s probe) | `INFO Starting CogniCode CLI v0.97.3 ... Use 'cognicode-mcp' binary to start the MCP server. The MCP server uses stdio transport, not TCP ports. Run: cognicode-mcp --cwd <workspace>` |
| `ss -tlnp \| grep :8080` | (empty — port 8080 not listening while serve runs) |
| `cargo search cognicode-mcp` | (empty — no crates.io match) |
| `cargo search cognicode` | (empty — no crates.io match either) |

Adapter code that targets `cognicode-mcp`:
- `crates/sddk-engine/src/code_intelligence_port_mcp.rs:74-81` (Command::new(binary).arg("--cwd")…)
- `crates/sddk-engine/src/code_intelligence_port_mcp.rs:290` (provider_build string)
- `crates/sddk-cli/src/verify_kernel_cmd.rs:64, 172, 204, 240, 252, 265, 274` (literal `cognicode-mcp` references)

## 3. Re-investigation conclusion (session-11)

Operator instruction §3 ("abrir C2a") triggered a focused re-investigation of the SCOPE-CONTRACT
§9.2 secondary path ("expand C2a scope to support `cognicode serve --port N`"). The hypothesis
was **INVALIDATED** by direct probe:

- `cognicode serve --port 8080` does NOT speak MCP — it errors out with an explicit message
  pointing to the missing `cognicode-mcp` binary.
- The TCP port (8080) is not even listening while `serve` runs (verified via `ss -tlnp`).
- No alternative path was identified: `cargo search` for either `cognicode-mcp` or `cognicode`
  returns empty.

**Decision:** C2a remains `NOT_EVALUATED_PROVIDER_MISSING`. No new SCOPE-CONTRACT C2a.1 is
created because there is no technical path to evaluate C2a against the available binary.

## 4. Recovery actions (concrete, owned by operator)

1. **Install `cognicode-mcp` separately** (operator action; artifact not on crates.io, not
   bundled with cognicode v0.97.3 CLI). CogniCode upstream must publish a separate binary or
   Cargo package that implements stdio JSON-RPC per MCP 2025-03-26.
2. **Approve C2a.1 scope expansion** (ADR required): change the Adapter contract to consume a
   different protocol (e.g. consume `cognicode analyze` output, or talk to the TCP service
   if/when it speaks MCP). NOT justified because:
   - The TCP service exposed by `cognicode serve --port 8080` does NOT speak MCP.
   - No second consumer known for the SDDK adapter.
3. **Defer C2a** for v1.169.149. Rely on T08-T11 historical evidence
   (`a6_cognicode_protocol_spike`, `aiw_s1_cognicode_real`) as HISTORICAL only, not as current
   UAT pass. Profile STATIC_ENHANCED stays NOT_EVALUATED for v1.169.149.

## 5. Side findings (no action taken; documented for next cycle)

- **Adapter/CLI alignment gap** (CONFIRMED): SDDK adapter was authored against the historical
  `cognicode-mcp` stdio binary. CogniCode CLI v0.97.3 ships:
  - `cognicode` direct CLI (analyze/navigate/graph/index/etc.)
  - `cognicode serve --port 8080` TCP service (NOT MCP — verified)
  - No `cognicode-mcp` binary.
- **User-facing CLI message** at `verify_kernel_cmd.rs:172` hardcodes `cognicode-mcp` in error
  text — would mislead operators running on hosts without that binary. This is a **DOC bug**,
  not a functional bug: the adapter still refuses to start without a working binary. Not fixed
  in this cycle (out of scope for an investigation-only WorkItem; would require its own change
  + test cycle).
- **Test pinning**: `a6_cognicode_protocol_spike` was pinned to v0.97.1 in Addendum 18; actual
  binary is v0.97.3. Drift, not bug, but worth noting.

## 6. No code changes

No commits made in `feat(c2a):` style this WorkItem (session-11 close). Codebase state remains
at `main@a5f279c` (workspace v1.169.149), tree clean.

The 22 commits ahead of origin/main (`5f493ab` → `a5f279c`) are session-11 housekeeping
(C3 closeout, docs reconciliation, ceremonial bump, Cargo.lock sync) — not C2a work.

## 7. Acceptance of this RECEIPT

Per CERTIFICATIONS.md §5, this RECEIPT is `status: NOT_EVALUATED` (mapped from
`NOT_EVALUATED_PROVIDER_MISSING`). It is NOT a `PASS_OBSERVED`. It is NOT a `FAIL` either
(no test was executed that could fail). It is a documented stop with full evidence and a
recovery action owned by the operator.

This RECEIPT satisfies the contract:
- `status` is honest and CONFIRMED by fresh re-investigation.
- Evidence is real (commands + output verbatim).
- Recovery action is concrete and minimal.
- Source SHA is anchored to current HEAD (`a5f279c`).
- Prior RECEIPT state (workspace v1.169.142) is preserved in the c2a-prep/ directory.

## 8. Next WorkItem

Per ROADMAP C2 DAG (`C1 → {C2, C3}`), the next viable WorkItem after C2a is **C2b Chronos**
(T12–T14). Same pre-flight rule applies: verify the Chronos binary is present and matches the
SDDK adapter before opening a new SCOPE-CONTRACT. Pre-flight checks before opening C2b:

| Check | Result expected |
|---|---|
| `chronos --version` | present + version string |
| `chronos-mcp --version` | present + version string (or absent → NOT_EVALUATED) |
| `cargo search chronos-mcp` | match (or empty → NOT_EVALUATED) |
| SDDK adapter source | `crates/sddk-engine/src/runtime_evidence_port_mcp.rs` (or similar) — read & document hardcoded refs |

If the binary mismatch pattern repeats (analogous to C2a), the cycle closes with the same
honest `NOT_EVALUATED_PROVIDER_MISSING` status and a recovery_action pointing the operator
to the missing binary.

C3 (resiliencia/seguridad) is **closed** (C3a-f PASS_OBSERVED) — does not re-open this session.
J7/J8/J9/R11/X08 remain DEFERRED per ROADMAP.md §C5; no scope re-opening this session.

## 9. Pre-flight check for next cycle (operator-side optional)

Operator who wants to unblock C2a can run the following verification locally:

```bash
# 1. Confirm the absent binary
ls /home/rubentxu/.cargo/bin/cognicode-mcp 2>&1   # should be absent
cargo search cognicode-mcp                          # should be empty

# 2. Confirm the available CLI's "serve" is NOT MCP
timeout 5 cognicode serve --port 8080 2>&1 | head -10
# Expect: "Use 'cognicode-mcp' binary to start the MCP server..."
```
