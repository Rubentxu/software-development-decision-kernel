# Proposal: JCode Anti-Corruption Adapter (J2)

**Status:** PROPOSAL — not opened as a cycle. Awaiting operator
decision.

**Author:** SDDK orchestrator session, 2026-09-19.

**Predecessor:** SEC-1 (closed 2026-09-19, commits
`2c63aff` + `1374951` + receipt at `f6e1906`).

## Why this proposal exists

JCode (the Jcode host runtime that consumes SDDK as a library)
needs to invoke capabilities via the gateway and read back
`CapabilityReceipt` results. If JCode reaches into `serde_json::Value`
or directly into the gateway's internal `CapabilityReceipt`
struct, two failure modes emerge:

1. JCode could read raw stdout/stderr before SEC-1 redaction
   (if a future path bypasses `Gateway::finish_effect`).
2. JCode could attempt to construct a denied capability by
   hand-rolling a `CapabilityPlanInput` that the gateway would
   reject — but the rejection error path may leak the original
   args (including the secret).

An **anti-corruption layer** between the SDDK core and the JCode
host gives the JCode side a single, narrow, versioned API that
the SDDK side controls. The adapter is the natural follow-up
boundary after SEC-1 closed the inner one.

## What this proposal defines (not yet implemented)

ONE trait + ONE view type, both versioned:

```rust
/// The single JCode-facing surface. Implementations live in
/// sddk-gateway; JCode hosts depend on this trait only.
pub trait JCodeHostAdapter {
    /// Cycle identifier this adapter instance is bound to.
    fn cycle_id(&self) -> &str;

    /// Request a capability effect. Returns the persisted
    /// receipt view, NOT the full CapabilityReceipt. The
    /// adapter applies SEC-1 redaction before returning.
    fn request_effect(
        &mut self,
        capability: &str,
        args: JCodeArgs,
        reason: &str,
    ) -> Result<JCodeReceiptView, JCodeAdapterError>;

    /// Read the masked stdout of a previously issued capability.
    fn masked_stdout(&self, receipt_id: &str) -> Option<String>;

    /// Read the masked stderr of a previously issued capability.
    fn masked_stderr(&self, receipt_id: &str) -> Option<String>;
}

/// A narrow, JCode-facing view of a capability receipt. Does
/// NOT expose serde_json::Value, the request payload, or any
/// field that would allow the host to reconstruct a denied
/// capability.
#[derive(Debug, Clone)]
pub struct JCodeReceiptView {
    pub receipt_id: String,
    pub capability: String,
    pub status: JCodeStatus,            // Succeeded | Failed | Denied
    pub exit_status: Option<i32>,
    pub stdout_len: usize,              // byte length after redaction
    pub stderr_len: usize,
    pub redactions: usize,              // how many <redacted:N> tokens
    pub diagnostic_prefix: String,      // first 64 bytes verbatim
}
```

The two views (success vs. denied) both flow through the same
typed surface; the host never sees a `serde_json::Value` blob.

## Falsification battery

If this proposal becomes a cycle (`sec-2-jcode-boundary`), the
test battery MUST include:

1. **Host cannot bypass the adapter.** A test that constructs
   a JCode host, then attempts to call into `sddk_gateway::Gateway`
   directly. Compile-time error: the JCode `Cargo.toml` only
   depends on `sddk-gateway` via the adapter module path, not
   the gateway crate root.
2. **Adapter preserves SEC-1 redaction.** A test that issues
   `request_effect` with a canary via stdout, then asserts the
   returned `JCodeReceiptView.stdout_len` matches the masked
   length and `masked_stdout` does not contain the canary.
3. **Adapter redacts before returning.** Same as (2) but for
   stderr, and asserts `redactions >= 1`.
4. **Denied capability does not leak the original args.** A
   test that issues a denied capability (e.g. `git.delete_branch`
   without approval) and asserts the `JCodeAdapterError::Denied`
   variant does NOT include the `args` field. Only the
   capability name, the policy verdict code, and the recovery
   hint.
5. **Adapter is `serde` free at the boundary.** A test that
   derives `serde::Serialize` on `JCodeReceiptView` is forbidden
   (the type has no `Serialize` impl in its definition). The
   test compiles only if the type definition is unchanged.
6. **Diagnostic prefix is preserved.** A test that asserts the
   first 64 bytes of stdout round-trip verbatim through the
   adapter, except for the masked segments.

## MUST (acceptance)

- M1. `JCodeReceiptView` does NOT contain `serde_json::Value`,
  `serde_json::Map`, or any `serde::Serialize` impl.
- M2. `JCodeAdapterError::Denied` does NOT contain the original
  capability args (only the capability name and policy verdict).
- M3. SEC-1 redaction is observable at the adapter boundary
  (`redactions`, `stdout_len` fields).
- M4. The JCode host cannot depend on `sddk_gateway::*` types
  directly; only on the `JCodeHostAdapter` trait.
- M5. All 6 falsification tests GREEN.

## MUST NOT

- N1. No new secret manager / provider / log system.
- N2. No new capability taxonomy; the adapter does not invent
  capabilities.
- N3. No `serde_json::Value` leakage at the adapter boundary.
- N4. No dependency on JCode's internal trait tree; the
  adapter lives in `sddk-gateway`, not in JCode's repo.
- N5. No retroactive change to A5-C certification or to the
  SEC-1 contract.

## Out of scope (kept on record)

- Adapter for CogniCode (separate surface, separate cycle).
- Adapter for `mcp__*` hosts.
- Multi-tenant adapter scoping.
- Async streaming (current gateway returns final receipt;
  streaming would require a separate ADR).

## Estimated scope

Small cycle. Roughly:

- 1 new trait + 1 new view type + 1 new error type in
  `crates/sddk-gateway/src/jcode_adapter.rs` (~150 lines).
- 6 falsification tests in
  `crates/sddk-gateway/tests/sec2_jcode_adapter.rs`.
- 1 scope contract + 1 receipt.

## Why not open this cycle now

The user mandate was "SEC-1 only". This proposal is the
forward step but requires explicit operator authorisation.
If authorised, the cycle id would be `sec-2-jcode-boundary`
with parent identity `p-63676b11dc0ef88f`.

## Status

PROPOSAL. Awaiting operator decision.

A5-C cert, SEC-1 fix, and the workspace release path are
unaffected by this proposal.
