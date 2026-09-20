# SCOPE-CONTRACT — S5 — Real CogniCode EXT (NOT_EVALUATED in this environment)

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s5-ext-real-cognicode`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Status:** **NOT_EVALUATED** — environment lacks `COGNICODE_MCP_BIN`.

## §1 Goal

Execute `t_ar_6_ext_real_cognicode_run` (already drafted in CC-S1, env-gated)
against `cognicode-mcp v0.97.1` on the pinned Git revision. The test
itself exists and is `#[ignore]`-d when `COGNICODE_MCP_BIN` is absent.
The slice closes cleanly when the live run yields a pinned
`Satisfied` `STATIC_ENHANCED` receipt; otherwise it stays
`NOT_EVALUATED` per the macro-cycle rule:

> "Si una prueba externa indispensable queda ignorada o no ejecutada,
> mantener ese requisito como NOT_EVALUATED" — macro-cycle SCOPE §S5.

## §2 UAT rows in scope

| UAT id | Scenario | Expected invariant | Adapter assertion |
|---|---|---|---|
| AC10 + `PR-UAT-C01..C10` (full matrix) | Live `cognicode-mcp` handshake | Adapter surface query returns `available_strategies` and `semantic_classes`; on absence, `Unknown` per M6 (NOT_EVALUATED, not satisfied). | `t_ar_6_ext_real_cognicode_run` (already present in `crates/sddk-engine/tests/a6_cc_s1_static_graph_completeness.rs`) |

## §3 Environment audit (this slice)

```
$ env | grep COGNICODE
COGNICODE_MCP_BIN=unset
$ which cognicode-mcp
not found
$ which cognicode
not found
$ ls /usr/local/bin/cogni*
no such file
```

No env var, no binary on PATH, no `/usr/local/bin/cogni*` artifact. The
EXT cannot be exercised in this session.

Per the test's own gating:

```rust
#[ignore = "requires COGNICODE_MCP_BIN pointing to the real CogniCode binary"]
fn t_ar_6_ext_real_cognicode_run() { ... }
```

This `#[ignore]` was honored in the CC-S1 run recorded earlier
(`12 passed; 0 failed; 1 ignored`). The slice closes in NOT_EVALUATED
state with **zero new tests added** and **zero source changes** —
nothing to validate beyond recording the absence.

## §4 Hard constraints (preserved, not relaxed)

- C1: no fabrication of live evidence.
- C2: no modification of `code_intelligence_port_mcp` (the live
  adapter) — the adapter is ready, only the host binary is missing.
- C3: closing NOT_EVALUATED here does not lower any UAT bar; the
  rows in §2 remain `NOT_EVALUATED` until live evidence exists.

## §5 STOP conditions

Per macro-cycle SCOPE §S5:

| Condition | Action | This slice |
|---|---|---|
| `COGNICODE_MCP_BIN` not available | remain `NOT_EVALUATED`; do not close | **Triggered — recorded as `NOT_EVALUATED`**. |
| Live CogniCode wire surface drifted from AIW-S1 handshake | STOP and report | N/A (no live execution) |
| Live `cognicode-mcp` does not advertise `STATIC_ENHANCED` capabilities consistent with the contract | STOP and report | N/A (no live execution) |

Per the operator's session rule, NOT_EVALUATED is **not** a STOP — it
is a documented partial state, not a failure.

## §6 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../slices/s5-ext-real-cognicode/SCOPE-CONTRACT.md` | ✅ |
| UAT evidence rows | `tests/cycle-artifacts/.../slices/s5-ext-real-cognicode/UAT-EVIDENCE.yaml` | ✅ (every row `NOT_EVALUATED` with reason) |
| RECEIPT | `tests/cycle-artifacts/.../slices/s5-ext-real-cognicode/RECEIPT.md` | ✅ |
| 1 commit `docs(s5)` | — | ✅ |

## §7 Out of scope

- **Durability** (S4): independent slice, requires operator alignment.
- **Fake relocation** (S6): independent slice, audit in progress.
- **Closeout** (S7): depends on S4/S6 and on S5 closing (or remaining
  NOT_EVALUATED by closeout time).

## §8 References

- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/SCOPE-CONTRACT.md` §S5
- `crates/sddk-engine/tests/a6_cc_s1_static_graph_completeness.rs::t_ar_6_ext_real_cognicode_run`
  (the env-gated live test, NOT_EVALUATED without `COGNICODE_MCP_BIN`).
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md`
  (live handshake contract).
