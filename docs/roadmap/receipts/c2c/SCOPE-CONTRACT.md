# C2c — SCOPE-CONTRACT: Real JCode host integration (T15–T18)

**Cycle:** C2c (Agentic API path)
**Baseline:** `main@<post-c2b-commit>` (workspace v1.169.142)
**Opened:** 2026-09-22T08:03:00Z
**Owner:** orchestrator
**Pre-flight finding:** `jcode` v0.86.0 binary IS present, exposes `acp` (Agent Client Protocol) adapter. However, **no SDK crate (`jcode-sdk` or equivalent) is present on crates.io**, and **no SDDK adapter targeting JCode exists in this repo**.

## 1. Objective

Demonstrate T15–T18 (UAT-MATRIX) with real JCode host evidence, executing against the actual host with adapter + SDK.

## 2. T15 contract requirement (falsable)

T15 demands: "Adapter separado con SDK público, host real, connect + attach". Compile outside the SDDK internal repo.

Observed status:

| Check | Result |
|---|---|
| `jcode --version` | `jcode v0.86.0 (e589cbe5a)` |
| `jcode acp --help` | exposes ACP adapter backed by daemon |
| `cargo search jcode-sdk` | empty |
| `grep "jcode" crates/*/Cargo.toml` | no dependency on any jcode crate |
| Adapter in `crates/sddk-engine/src/` | no JCode adapter found |

## 3. Why this maps to NOT_EVALUATED rather than PASS_BY_CODE_READING

- `PASS_BY_CODE_READING` is forbidden per [CERTIFICATIONS.md §3](../../CERTIFICATIONS.md).
- T15 requires a real adapter that compiles against an external SDK; the SDK is not located.
- T16–T18 require observable host behavior under burst/reorder/stale events — without adapter+SDK, those scenarios cannot be exercised.
- An honest stop is mandated by [UAT-MATRIX T33](../../UAT-MATRIX.md): "Ausencia de binario EXT en perfil enhanced → BLOCKED/NOT_RUN; Base puede permanecer válido."

## 4. Proposed recovery actions

1. **Locate the public JCode SDK** (operator action): is it on crates.io, GitHub, or distributed via JCode itself (`jcode sdk --help`)? If found, add it as a workspace dependency under a new crate (e.g. `sddk-jcode-adapter`) and proceed with T15–T18.
2. **Confirm the JCode adapter lives in another repo** (cited in session-10 finding). If yes, link or vendor; do NOT duplicate.
3. **Accept C2c as DEFERRED** with explicit decision: SDDK does not own the JCode adapter; downstream product must consume it from its canonical source.

## 5. STOP conditions

- Until (1), (2) or (3) is decided, do not implement a JCode adapter in this repo without an ADR.
- Do not declare JCode work `DONE` based on "we could write one".

## 6. Out-of-scope

- No new dependency on crates.io for an unverified jcode crate.
- No ACP client implementation without ADR.
- No release.

## 7. Acceptance

This SCOPE-CONTRACT + UAT-EVIDENCE + RECEIPT are committed, with status `NOT_EVALUATED_ADAPTER_MISSING` (parallel to C2a's category shape). The session-journal records the finding so the next cycle has the diagnostic trail.
