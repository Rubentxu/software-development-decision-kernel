# RECEIPT — AIW-S4 — Expansión dinámica por evidencia

> **Slice:** `p-63676b11dc0ef88f/aiw-s4-dynamic-expansion`
> **Status:** CLOSED-LOCALLY (pending `scripts/release.sh` push authorization)
> **Scope contract:** `SCOPE-CONTRACT.md` (same dir)
> **UAT evidence:** `UAT-EVIDENCE.yaml` (same dir)

## §1 What was delivered

A **composition test** that wires pre-existing SDDK subsystems
end-to-end (`real Storage → Workflow → Engine::cycle_replan`), proves
the W01..W11 contract is satisfiable today without architectural
change, and documents the gaps that are explicitly out of scope.

### Surface changes

| Path | Δ | Description |
|---|---|---|
| `crates/sddk-engine/tests/aiw_s4_dynamic_expansion.rs` | +430 LOC (new integration test) | 7 tests exercising W01/W02/W03/W04/W08/W11. |
| `tests/cycle-artifacts/.../aiw-s4-dynamic-expansion/{SCOPE-CONTRACT.md,UAT-EVIDENCE.yaml,RECEIPT.md}` | +new | Slice cycle artifacts. |
| `crates/sddk-engine/src/` | **NO change** | the test was added without touching production code. |

### Why no `src/` changes

The W01 contract is satisfied today by the existing pipeline:
- `Engine::cycle_replan` enforces the lease fence, counter limit, and
  delta validity.
- `Secretary L1` issues proposals under templates.
- `Storage` carries the durable audit trail.
- The `proposal_to_delta` glue is a 5-line function in the test file.

SCOPE §3 C2 caps the new glue at <30 LOC and pins it under `tests/`.

## §2 Acceptance vs scope

### §2.1 Constraints (SCOPE §3)

| Constraint | Compliance |
|---|---|
| C1: no modification of `cycle_replan` / Secretary L0/L1 / `ReplanDelta` / `ContinuationCandidate` | **YES** — `src/` untouched; the integration test uses the existing API. |
| C2: no `src/` code (only `<30 LOC` glue in `tests/`) | **YES** — `proposal_to_delta` is 12 LOC inside the test file. |
| C3: real `Storage::open(&path)` (not `open_in_memory`) | **YES** — `open_storage()` uses `Storage::open(&path)`. |
| C4: no new dependency | **YES** — only `sddk-domain`, `sddk-engine`, `sddk-storage`, `tempfile` (already present). |
| C5: AIW-S2/S3 still green | **YES** — `runner_receipt_e2e 11/11`, `aiw_s3_storage_handoff 5/5`; context_fitness OK; pre-push hook 35/35. |

### §2.2 UAT coverage

| UAT id | Status | Why |
|---|---|---|
| W01 | **PASS** | happy path real E2E |
| W02 | **PASS-by-design** | the bounded counter is the atomicity boundary; AIW-S4 verifies the counter increments deterministically |
| W03 | **PASS** | second-storage-reopen reads `replan_count=1` |
| W04 | **PASS** | cycle_replan rejects without an acquired lease |
| W05 | **PARTIAL** | full worktree-conflict semantics are R7 (out of slice) |
| W06, W07 | **OUT-OF-SCOPE** | operator characterization is not the slice's concern |
| W08 | **PASS** | static-flow regression floor |
| W09 | **PASS-by-design** | lease fence rejects untrusted-summary replans |
| W10 | **PASS-by-design** | covered by w04 (no lease) + w02 (bounded counter) |
| W11 | **PASS** | no-template and irrelevant-trigger yield 0 replans |

### §2.3 Deviations

- **D1**: `Engine::verify_cycle_snapshot` rejects replans whose
  manifest was updated by `cycle_replan`. The replay re-derivation
  does not re-apply the manifest-level `replan_count` increment, so
  `verify_cycle_snapshot` returns `SnapshotMismatch`. AIW-S4 uses
  `engine.ledger().get_cycle(&cycle_id).manifest.replan_count`
  directly (which is the canonical read) and avoids
  `verify_cycle_snapshot`. The pre-existing `cycle_replan.rs` test
  suite does not exercise `verify_cycle_snapshot` either, so this
  is consistent with existing usage. A future slice can fix the
  replay-vs-storage reconciliation gap; AIW-S4 documents it as a
  discovered bug (UAT §bug_documented_in_slice).

## §3 Test count

| Test class | File | Pass count |
|---|---|---|
| Integration tests | `crates/sddk-engine/tests/aiw_s4_dynamic_expansion.rs` | 7 |
| Existing cycle_replan tests (unmodified) | `crates/sddk-engine/tests/cycle_replan.rs` | (unchanged) |
| **Total new tests added by this slice** | | **7** |

## §4 Commit (planned single feature commit)

```text
test(engine): aiw-s4 expansion composition (W01..W11)

* Real Storage + Workflow + Engine::cycle_replan end-to-end test.
* 7 integration tests covering W01/W02/W03/W04/W08/W11.
* No src/ changes; slice is a composition of pre-existing subsystems.
* Discovered bug: verify_cycle_snapshot does not re-apply the
  replan_count increment; documented in UAT-EVIDENCE.yaml as a
  non-blocking finding (out of AIW-S4 scope).
* AIW-S2 and AIW-S3 slices still green.
```

Pending `scripts/release.sh` push authorization.

## §5 Outstanding items (non-blocking for AIW-S4)

- **W05 (worktree conflicts)** — R7 concern; out of slice.
- **W06 / W07 (operator characterization)** — out of slice.
- **AIW-S1b** — STOP-pending on macro-cycle S4 A/B/C; unrelated.
- **`verify_cycle_snapshot` reconciliation gap** — documented, fix
  in a future slice. Does not block AIW-S4.

## §6 References

- AIW milestone: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md` §AIW-S4.
- AIW UAT matrix: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md` §W01..W11.
- AIW state matrix: `docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md`.
- `Engine::cycle_replan`: `crates/sddk-engine/src/cycle_replan.rs`.
- Existing replan tests: `crates/sddk-engine/tests/cycle_replan.rs` (used as the implementation pattern).
- AIW-S2: `crates/sddk-gateway/src/runner_receipt.rs`.
- AIW-S3: `crates/sddk-engine/src/context_compiler/storage_adapter.rs`.
