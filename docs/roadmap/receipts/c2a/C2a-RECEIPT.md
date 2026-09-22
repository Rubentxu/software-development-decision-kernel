# C2a — RECEIPT

**Cycle:** C2a (Real CogniCode integration, T08–T11)
**Status:** `NOT_EVALUATED_PROVIDER_MISSING`
**Baseline:** `main@5f493ab` (workspace v1.169.142)
**Issued:** 2026-09-22T07:58:30Z
**Issuer:** orchestrator (delegated; evidence-gathering performed directly this session)

## 1. Outcome

C2a could not be executed as `PASS_OBSERVED` because the provider binary the SDDK adapter requires (`cognicode-mcp`) is **not installed** on this host and is **not installable** via `cargo install` (no crates.io match). All four target UAT scenarios (T08, T09, T10, T11) are recorded as `NOT_EVALUATED` per [CERTIFICATIONS.md §3](../../CERTIFICATIONS.md) and the UAT-MATRIX T33 row.

This is **not** a release blocker: per UAT-MATRIX T33, "Ausencia de binario EXT en perfil enhanced → BLOCKED/NOT_RUN; Base puede permanecer válido."

## 2. Evidence

Full evidence in [UAT-EVIDENCE.yaml](UAT-EVIDENCE.yaml). Key OBSERVED commands:

| Command | Output |
|---|---|
| `cognicode --version` | `cognicode 0.97.3` |
| `cognicode serve --help` | `Start the MCP server ... Options: -p, --port <PORT> [default: 8080]` |
| `cognicode doctor` | `❌ cognicode-mcp binary (install: cargo install cognicode)` |
| `cargo search cognicode-mcp` | (empty — no crates.io match) |

Adapter code that targets `cognicode-mcp`:
- `crates/sddk-engine/src/code_intelligence_port_mcp.rs:74-81` (Command::new(binary).arg("--cwd")…)
- `crates/sddk-cli/src/verify_kernel_cmd.rs:64, 172` (literal `cognicode-mcp` in help/error)

## 3. Decision

C2a is parked awaiting operator decision. Two paths forward are documented in `UAT-EVIDENCE.yaml#recovery_action`:

1. **Install `cognicode-mcp` separately** (operator action; artifact not on crates.io). Then re-open the SCOPE-CONTRACT and execute T08–T11 as originally scoped.
2. **Expand C2a scope** to support `cognicode serve` TCP mode (would require an ADR + Adapter contract change). Higher cost; only justified by a second consumer.

Until one of those paths is taken, the historical a6_cognicode_protocol_spike / aiw_s1_cognicode_real tests remain tagged `HISTORICAL` only (per UAT-MATRIX T33 last sentence).

## 4. Side findings (no action taken; documented for next cycle)

- **Adapter/CLI alignment gap**: SDDK adapter was authored against the historical `cognicode-mcp` stdio binary. CogniCode CLI v0.97.3 ships only a TCP MCP server (`serve --port`) plus direct CLI subcommands. This gap is **separate from** the missing binary: even if `cognicode-mcp` were installed, the adapter would not talk to `cognicode serve --port 8080` because the adapter uses `Stdio::piped()` not TCP.
- **User-facing CLI message** at `verify_kernel_cmd.rs:172` hardcodes `cognicode-mcp` in error text, which would mislead operators running on hosts without that binary.
- **Test pinning**: `a6_cognicode_protocol_spike` was pinned to v0.97.1 in Addendum 18; actual binary is v0.97.3. Drift, not bug, but worth noting.

## 5. No code changes

No commits made this WorkItem. Codebase state remains at `main@5f493ab` (workspace v1.169.142), tree clean.

## 6. Acceptance of this RECEIPT

Per CERTIFICATIONS.md §5, this RECEIPT is `status: NOT_EVALUATED` (mapped from `NOT_EVALUATED_PROVIDER_MISSING`). It is NOT a `PASS_OBSERVED`. It is not a `FAIL` either (no test was executed that could fail). It is a documented stop with full evidence and a recovery action owned by the operator.

This RECEIPT satisfies the contract:
- `status` is honest.
- Evidence is real (commands + output verbatim).
- Recovery action is concrete and minimal.
- Source SHA is anchored.

## 7. Pointer reconciliation note

`STATE.yaml.current_sha` field still references `f0f57d4` (last-verified during session-10 close). HEAD has since advanced to `5f493ab` with a single doc commit (`docs(roadmap): persist session-10 close`). The delta is purely a doc-commit that aligned STATE/CURRENT to f0f57d4; HEAD = 5f493ab reflects that alignment being committed. Not a divergence that affects roadmap state.

## 8. Next WorkItem

Per the AUTO initiative and the C2 DAG (`C1 → {C2, C3}`), the next logical WorkItem after C2a is **C2b Chronos** (T12–T14). Same pre-flight rule applies: verify the Chronos binary is present and matches the SDDK adapter before opening a new SCOPE-CONTRACT. The current session will:

1. Verify `chronos` / `chronos-mcp` binary presence and protocol.
2. Emit either a SCOPE-CONTRACT (if viable) or a NOT_EVALUATED RECEIPT (if binary missing).
3. Then move to C2c JCode with the same pre-flight discipline.

C3 (resiliencia/seguridad) stays parallel but does not start this session — its scope (Authority/Storage adversarial) is non-trivial and benefits from dedicated focus without C2 in flight.

J7/J8/J9/R11/X08 remain DEFERRED per ROADMAP.md §C5; no scope re-opening this session.
