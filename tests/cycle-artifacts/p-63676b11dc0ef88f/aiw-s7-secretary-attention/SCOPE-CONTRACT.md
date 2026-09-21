# SCOPE-CONTRACT — AIW-S7 — Secretary y atención adaptativa

> **Slice id:** `p-63676b11dc0ef88f/aiw-s7-secretary-attention`
> **Status:** ⚠️ **STOP** — pre-implementation, requires operator
> decision. Cannot be closed in auto-run for the reasons below.

## §1 Goal

Close **AIW-S7 — Secretary y atención adaptativa** by exercising
the 9 UAT rows G01..G09 against the existing
`secretary_l0::SecretaryL0Engine`,
`secretary_l1::SecretaryL1Engine`,
`secretary_l2_replan::SecretaryL2ReplanEngine`, and
`secretary_closed_set::validate_secretary_event` modules in
`crates/sddk-engine/src/`.

## §2 STOP — pre-implementation

AIW-S7 **cannot be closed in auto-run** because:

### S7-STOP-1: G04 requires a real external producer

`G04 (IT): Secretary L0 reacciona a evidencia relevante. Propuesta/consulta observable en ruta productiva, no solo test de módulo; sin añadir WorkItem automático.`

This row requires **L0 to observe a producer emitting real evidence
into a real persistence substrate**. Today, the only producers
delivered are:
- CogniCode MCP (real, AIW-S1) — emits evidence into a one-shot CLI
  run that goes to stdout; it does **not** stream into L0.
- Chronos MCP (real, AIW-S5) — same: vertical, not streamed.
- The `secretary_l0.rs` module declares its `AttentionFrame` inputs
  but no consumer has been wired that converts a real producer
  stream into `AttentionFrame`s.

Wiring this consumer is **out of scope for AIW-S7** (it would
require either a streaming adapter in `crates/sddk-gateway/src/`
or a new MCP-side plumbing change). Doing it inside this slice
would be a material architectural change (gateway producer
adapter).

### S7-STOP-2: G01 depends on PARTIAL consumer

`G01 (IT): snapshot Planning reconciliado, A bloquea B.`

This row is the **consumer of AIW-S3's PARTIAL side** (planning
reconciliation from `StorageSnapshot`). AIW-S3 was deliberately
shipped with H02/H03/H05 PARTIAL for the same reason:
- The StorageSnapshot adapter exists (`storage_adapter.rs`)
- But the consumer that reads the snapshot and feeds
  `SecretaryL1Engine::next_planning_step` is not wired end-to-end.

Without that wiring, G01 cannot be exercised as a real integration
test, only as a unit test (which is explicitly forbidden by G01's
"observable en ruta productiva, no solo test de módulo").

### S7-STOP-3: G05 + G07 require live permission grant

`G05 (SEC): Secretary intenta release/gate/lease/receipt fuera closed-set.`
`G07 (NEG): autoridad de leer documento/capability no otorgada.`

These rows test Secretary's authority boundaries end-to-end. Today
`secretary_closed_set.rs` has unit tests for the closed-set validator
and `is_secretary(actor)`, but the **end-to-end denial**
(`SecretaryL1Engine::propose` blocked by `AuthorityContext` lacking
the right grant) is not exercised in any integration test.

Writing this integration test requires a working
`AuthorityContext::grant_table` fixture with **negative-grant
scenarios** — this is more substantial than a single integration
test file and would inflate the slice beyond what one session can
defensibly close.

## §3 What CAN be done in auto-run

The Secretary **subsystems themselves** are tested by their
respective module-level tests. The slice is **NOT_STARTED at the
integration level** (the rows are about end-to-end behavior, not
unit-level invariants). The module-level coverage is already
green; see:

- `crates/sddk-engine/src/secretary_l0.rs` — `SecretaryL0Engine::observe`
  unit tests
- `crates/sddk-engine/src/secretary_l1.rs` — `SecretaryL1Engine::propose`
  unit tests
- `crates/sddk-engine/src/secretary_l2_replan.rs` — `SecretaryL2ReplanEngine::plan_replan`
  unit tests
- `crates/sddk-engine/src/secretary_closed_set.rs` — `validate_secretary_event`
  unit tests (closed-set enforcement)

None of these close AIW-S7's rows because the rows are
**integration-level** (not module-level).

## §4 Operator decision needed

| Decision | Effect |
|---|---|
| **Wire a producer→L0 stream adapter in a follow-up slice** | Unblocks G04 + G06. |
| **Wire a StorageSnapshot→SecretaryL1 consumer** | Unblocks G01 + G03. |
| **Add AuthorityContext negative-grant integration tests** | Unblocks G05 + G07. |
| **Accept AIW-S7 as NOT_STARTED in delivery** | No code change. AIW-S7 stays open. |

Without at least the first decision, G04 cannot be honestly
exercised. Without the second, G01 cannot. Without the third,
G05 cannot. **All three are required to close AIW-S7 in this
session.**

## §5 Disposition

- **Status**: ⚠️ **NOT_STARTED** (auto-run-blocked).
- **AIW roadmap**: AIW-S7 remains in the live roadmap.
- **STATE-OF-AIW**: row marked NOT_STARTED with reason = S7-STOP-1..3.
- **No commit to close AIW-S7 in this slice.** The slice directory
  contains only this SCOPE-CONTRACT (no UAT-EVIDENCE, no RECEIPT)
  to make the STOP-pending state explicit and traceable.

## §6 References

- AIW milestone: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md` §AIW-S7.
- AIW UAT matrix: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md` §G01..G09.
- `secretary_l0.rs` / `secretary_l1.rs` / `secretary_l2_replan.rs`:
  existing module-level coverage (NOT a substitute for the G-rows).
- AIW-S3 (StorageSnapshot adapter): `crates/sddk-engine/src/context_compiler/storage_adapter.rs` —
  the producer side exists but the consumer side (Secretary)
  is not wired.
- AIW-S1 (CogniCode real) + AIW-S5 (Chronos real): producer
  capability exists but stream-to-L0 wiring is missing.
