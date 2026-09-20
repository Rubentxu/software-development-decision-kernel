# RECEIPT — S5 — Real CogniCode EXT (NOT_EVALUATED)

> **Slice id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s5-ext-real-cognicode`
> **Macro-cycle:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Baseline (released):** `v1.169.95` → `9688ebb` (S1 close)
> **Cycle lead:** orchestrator (this session, auto-run)
> **Status:** **CLOSED — `NOT_EVALUATED`**, push pending release flow.

## §1 Scope adherence

| Hard constraint | Status | Evidence |
|---|---|---|
| C1: no fabrication of live evidence | ✅ | Every row in `UAT-EVIDENCE.yaml` is `NOT_EVALUATED` with reason. No `PASS`/`FAIL` invented for the live EXT. |
| C2: no modification of `code_intelligence_port_mcp` (the live adapter) | ✅ | Unchanged in this slice (and unchanged since CC-S1). |
| C3: closing NOT_EVALUATED here does not lower any UAT bar | ✅ | Every `PR-UAT-C01..C10` and AC10 row remains `NOT_EVALUATED` for the EXT profile. The fake profile is exercised in S1/S2/S3 and remains PASS. |
| C4: workspace test green | ✅ | CC-S1 still 12 PASS / 0 FAIL / 1 ignored (the ignored test is the EXT test gated on `COGNICODE_MCP_BIN` — same as before). |

| STOP condition (per SCOPE §5) | Triggered? |
|---|---|
| `COGNICODE_MCP_BIN` not available in this environment | **Yes — and that is the documented partial path**: stay `NOT_EVALUATED`; do not close as PASS or FAIL. |
| Live CogniCode wire surface drifted from AIW-S1 handshake | **N/A** — no live execution. |
| Live `cognicode-mcp` does not advertise `STATIC_ENHANCED` capabilities consistent with the contract | **N/A** — no live execution. |

Per the operator's session rule, **NOT_EVALUATED is not a STOP** — it
is a documented partial state. The slice closes cleanly in that state.

## §2 Evidence

### 2.1 Environment audit

```
$ env | grep COGNICODE
COGNICODE_MCP_BIN=unset

$ which cognicode-mcp
not found
$ which cognicode
not found
$ ls /usr/local/bin/cogni* 2>/dev/null
no matches
```

### 2.2 Pinned test status (CC-S1, unchanged)

```
$ cargo test -p sddk-engine --offline --test a6_cc_s1_static_graph_completeness
…
running 12 tests
test t_ar_6_ext_real_cognicode_run ... ignored, requires COGNICODE_MCP_BIN pointing to the real CogniCode binary
test result: ok. 12 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

The 1 ignored test is exactly the live EXT pinned by this slice. Its
gate attribute (`#[ignore]`) is the test's own admission that the
absence of `COGNICODE_MCP_BIN` is the honest partial state, not a
fabrication.

### 2.3 UAT matrix rows

See `tests/cycle-artifacts/.../slices/s5-ext-real-cognicode/UAT-EVIDENCE.yaml`
for the durable YAML evidence. All 11 rows (AC10 + `PR-UAT-C01..C10`)
are `NOT_EVALUATED`.

| Profile | Rows | Status |
|---|---|---|
| EXT (live CogniCode) | 11 | `NOT_EVALUATED` |
| Static (fake provider) | 11 | PASS (regression — see S1/S2/S3 receipts) |

## §3 Files changed by this slice

| Path | Δ | Role |
|---|---|---|
| `tests/cycle-artifacts/.../slices/s5-ext-real-cognicode/SCOPE-CONTRACT.md` | nuevo | Slice scope (this file). |
| `tests/cycle-artifacts/.../slices/s5-ext-real-cognicode/UAT-EVIDENCE.yaml` | nuevo | 11 rows, all `NOT_EVALUATED`. |
| `tests/cycle-artifacts/.../slices/s5-ext-real-cognicode/RECEIPT.md` | nuevo | This file. |

**No source file modified. No test added. No dependency changed.**

## §4 Honest limits

1. **No live EXT evidence was produced.** This is the documented
   partial state; it does not invalidate any prior slice. S1, S2, S3
   exercise the same UAT rows against the deterministic fake provider
   with `Partial = true` semantics; the live EXT would replace that
   fake with the real CogniCode binary and assert identical surface
   behaviour. Until a `COGNICODE_MCP_BIN` is available, the EXT rows
   stay `NOT_EVALUATED`.
2. **The `code_intelligence_port_mcp` adapter is ready**, just
   unrunnable in this environment. If the operator points
   `COGNICODE_MCP_BIN` at a real binary in a later session, only
   `t_ar_6_ext_real_cognicode_run` needs to be un-ignored (no code
   change) and this slice can be re-validated to a real PASS row.
3. **This slice does not advance `STATIC_ENHANCED` to Satisfied** in
   the production-readiness sense. The CC-S0/CC-S1 tests still pass;
   the macro-cycle exit criterion remains pending live evidence.

## §5 Next slices

- **S4 — durability** (`PR-UAT-024`). **STOP condition anticipated**
  (see §6). Needs operator alignment before code is written.
- **S6 — fake relocation.** Audit in progress; once complete, either
  proceeds or triggers its own STOP.
- **S7 — closeout integrated report.** Depends on S4/S6 (and on S5
  closing — which it has, in `NOT_EVALUATED` state).

## §6 Note for the macro-cycle closeout (S7)

S5 closing in `NOT_EVALUATED` is acceptable per the macro-cycle
contract. The closeout (S7) should record, in the integrated report:

- every UAT row's last-known status across Static + EXT profiles;
- the environmental gap (no `COGNICODE_MCP_BIN`) as a documented
  follow-up, not as a hidden debt;
- the de-ignore path for the live EXT test so that any future CI
  environment with the binary can flip the test live with **zero
  code change**.
