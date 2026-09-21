# SCOPE-CONTRACT — AIW-S6 — A8 correlación estático/runtime sin colapsar

> **Slice id:** `p-63676b11dc0ef88f/aiw-s6-correlation-a8`
> **Macro-cycle (AIW adoption):** `aiw-delivery-complete`
> **Status:** planning + implementation in one session (auto-run mode).

## §1 Goal

Close **AIW-S6 — A8 correlación estático/runtime sin colapsar**:
demonstrate that `compose_intelligence_loop` accepts **two
independent basis anchors** (CogniCode + Chronos) and surfaces the
correlation **without converting it into authority**. The slice
proves the W09..W12 contract on top of pre-existing
`intelligence_loop::compose_intelligence_loop` and
`intelligence_advisory::derive_advisory_context`.

**Important prior context (not re-implemented)**:
- AIW-S1 already produced real CogniCode data
  (`tests/cycle-artifacts/.../aiw-s1-cognicode-real/RECEIPT.md`).
- AIW-S5 already produced real Chronos data
  (`tests/cycle-artifacts/.../aiw-s5-chronos-runtime/RECEIPT.md`).
- `crates/sddk-engine/src/intelligence_loop/mod.rs`
  `compose_intelligence_loop` already composes two bases via
  `knowledge_basis_basis_hash_hex` and `observation_set_canonical_digest`.
- The composition function does NOT call `VerifyKernel`,
  `DebVerifyKernel`, `AlignmentLensKernel`, or `reduce_alignment`
  (per its documented contract).

## §2 UAT rows in scope

Per `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`
§Expansión, externos y empaquetado (P01..P12). AIW-S6 contracts on
P09..P12 specifically:

| UAT id | Class | Scenario | Expected invariant |
|---|---|---|---|
| P09 | E2E | CogniCode + Chronos acuerdan | Dos fuentes visibles; correlación descrita sin autoridad. |
| P10 | E2E | CogniCode + Chronos se contradicen | Ambas se mantienen; Verify refleja conflicto/Unknown. |
| P11 | NEG | dependencia de proveedor para Base | Prohibido. Base UAT sin CogniCode/Chronos. |
| P12 | REG | reconstrucción de proyección combinada | Misma base produce mismos campos/refs, sin store combinado canónico. |

P01..P08 are providers/host concerns (AIW-S4 / S8); out of this slice.

## §3 Hard constraints

- **C1**: No modification of `compose_intelligence_loop`,
  `IntelligenceLoopInputs`, `IntelligenceLoopResult`,
  `KnowledgeBasis`, `ObservationSet`, `ReconciliationSummary`,
  `LensEvaluation`, `AlignmentAssessment` (all pre-existing).
- **C2**: Slice is a composition test that builds **two
  parallel `IntelligenceLoopInputs`** (CogniCode-like and
  Chronos-like) and calls `compose_intelligence_loop` on each.
- **C3**: No new dependency.
- **C4**: No external service required (P11 — Base UAT must be
  green without CogniCode or Chronos installed).
- **C5**: AIW-S2/S3/S4 still green.

## §4 STOP conditions

| Condition | Action |
|---|---|
| `IntelligenceLoopInputs` requires a field not in the existing fixtures | **STOP** — raise the field-construction gap as an upstream concern. |
| `compose_intelligence_loop` is modified | **STOP** — material architectural change. |
| External CogniCode/Chronos installation is required | **STOP** — violates P11. |

## §5 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../aiw-s6-correlation-a8/SCOPE-CONTRACT.md` | ✅ |
| Integration test P09..P12 | `crates/sddk-engine/tests/aiw_s6_correlation_a8.rs` | 🔲 |
| UAT-EVIDENCE | `tests/cycle-artifacts/.../aiw-s6-correlation-a8/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `tests/cycle-artifacts/.../aiw-s6-correlation-a8/RECEIPT.md` | 🔲 |
| STATE-OF-AIW.md update | `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md` | 🔲 |
| 1 commit `test(engine)` | — | 🔲 |

## §6 Out of scope

- **AIW-S7** — Secretary attention (depends on a consumer; not in
  this slice).
- **AIW-S8** — CLI/host evaluative (depends on a JCode jev test
  fork; out of slice).
- **AIW-S1b** — STOP-pending.
- **Modifying `compose_intelligence_loop` or `IntelligenceLoopInputs`** —
  forbidden by C1.

## §7 References

- AIW milestone: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md` §AIW-S6.
- AIW UAT matrix: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md` §P09..P12.
- AIW state matrix: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md`.
- `compose_intelligence_loop`: `crates/sddk-engine/src/intelligence_loop/mod.rs`.
- Existing pattern: `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs` (`happy_inputs` helper).
- AIW-S1: `tests/cycle-artifacts/.../aiw-s1-cognicode-real/RECEIPT.md`.
- AIW-S5: `tests/cycle-artifacts/.../aiw-s5-chronos-runtime/RECEIPT.md`.
- AIW-S2 (test runner): `crates/sddk-gateway/src/runner_receipt.rs`.
- AIW-S3 (storage adapter): `crates/sddk-engine/src/context_compiler/storage_adapter.rs`.
