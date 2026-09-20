# SCOPE-CONTRACT — AIW-S4 — Expansión dinámica por evidencia

> **Slice id:** `p-63676b11dc0ef88f/aiw-s4-dynamic-expansion`
> **Macro-cycle (AIW adoption):** `aiw-delivery-complete`
> **Status:** planning + implementation in one session (auto-run mode).

## §1 Goal

Close **AIW-S4 — Expansión dinámica por evidencia**: end-to-end, real
`Storage`, real `Workflow`, real `Secretary L0/L1` proposal, real
`cycle_replan` commit. The slice is a **composition** of pre-existing
SDDK subsystems, not an architectural change:

1. A real `Storage` accumulates observations (`emit_canonical_event`).
2. Secretary L0 surface (defined in `crates/sddk-engine/src/secretary_l0.rs`)
   filters the events into a set of interest.
3. Secretary L1 (`secretary_l1.rs`) issues a `SecretaryProposal` under a
   registered template, with the proposal's `evidence_refs`
   pinned to event envelopes from storage.
4. The proposal is **converted** to a `ReplanDelta` (changed_files +
   reason) by a thin adapter. This adapter is the one new piece of
   glue the slice adds, and it lives in tests/, not in src/.
5. `Engine::cycle_replan` (authority + admission + durable commit +
   audit-trail event) admits the proposal, increments the counter,
   writes `replan-receipt.json`, and emits ledger events.
6. A second `Storage` reopen reads the replayed ledger and recovers
   the parent + child revision (`h04_multiple_revisions_use_stable_adapter_ids`
   reuse, plus a `cycle_replay` API call).

**Important prior context (not re-implemented)**:
- `sddk_engine::Engine::cycle_replan` already enforces the W01
  authority/lease/idempotency gates (`bounded_runner_contract.rs`-style).
- `secretary_l0::observe_and_match` is the existing L0 rule evaluator.
- `secretary_l1::SecretaryL1Engine::propose` is the existing L1
  proposer with templates and bounded uses.
- The 4 existing `cycle_replan` tests cover individual error paths;
  AIW-S4 closes the **integration** row.

## §2 UAT rows in scope

Per `docs/proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`
§Expansión, externos y empaquetado (W01..W11):

| UAT id | Class | Scenario | Expected invariant | In/Out of this slice |
|---|---|---|---|---|
| W01 | E2E | evidencia real+claim gap produce propuesta | Orchestrator decide, Authority admite, compiler+validator añaden Task ejecutable, versión padre+hija persistente. | **IN — this slice** |
| W02 | NEG | trigger duplicado, proposal reenvío | Una expansión y sin doble efecto. | **IN — covered by `cycle_replan` counter + adapter idempotency** |
| W03 | DUR | reinicio entre proposal/commit y tras commit | Recuperación determina exactamente una revisión adoptada; completed nodes preservados. | **IN — second-storage-reopen test** |
| W04 | NEG | permiso denegado/budget agotado/capability ausente | No mutación IR ni ejecución; motivo trazable. | PARTIAL — `cycle_replan` already has `LeaseConflict`; AIW-S4 verifies the trigger path with no lease. |
| W05 | NEG | basis stale, parent revision cambiado, conflicto worktree | Rechazo/rebase autorizado; no apply parcial. | PARTIAL — out of scope (worktree conflict is R7). |
| W06 | IT | operador no construido `Loop/Wait/Join/...` | Rechazo explícito. | OUT — operator-characterization is not the slice's concern; this slice uses an existing operator. |
| W07 | IT | `Choice` guard sin fallback en fallo | El test constata semántica REAL. | OUT — `Choice` semantics is an Operator concern. |
| W08 | REG | workflow SDD estático | Sigue funcionando sin selección dinámica. | **IN — static-flow preservation test** |
| W09 | NEG | evento del proveedor con prompt injection | Se trata como input no fiable. | **IN — fuzzed event payload test** |
| W10 | CONC | dos propuestas simultáneas con mismo parent | Control de concurrencia sobre versión y no pérdida/duplicación. | PARTIAL — `cycle_replan` lease fence already handles this; AIW-S4 verifies it. |
| W11 | PERF | trigger irrelevante | 0 llamadas cognitivas y 0 nuevas tareas. | **IN — irrelevant trigger yields zero replans** |

## §3 Hard constraints

- **C1**: No modification of `Engine::cycle_replan`, `Secretary L0/L1`,
  `ReplanDelta`, or `continuation_candidate` APIs.
- **C2**: No new production code in `crates/sddk-engine/src/` other
  than a strictly optional **adapter trait** that lives in `tests/`
  (a glue function mapping `(SecretaryProposal, event_refs)` to a
  `ReplanDelta`). If we cannot express the adapter as `<30 LOC` of
  pure-function glue, **STOP** and raise — no perma-fixtures in `src/`.
- **C3**: The integration test must use **real `Storage` file paths**,
  not `open_in_memory`. (Per WU-C1.2 + W03.)
- **C4**: No new dependency added.
- **C5**: Workspace test green; AIW-S2/S3 still green.

## §4 STOP conditions

| Condition | Action |
|---|---|
| Conversion `SecretaryProposal → ReplanDelta` requires more than 30 LOC of glue | **STOP** — the abstraction is leaky, raise. |
| `Engine::cycle_replan` requires a new field or argument | **STOP** — material architectural change. |
| Concurrency test (W10) requires new lock surfaces | **STOP** — out of slice scope. |
| `cargo fmt` / `cargo clippy` fails | Fix and continue (not STOP). |
| New dep needed | **STOP** — STOP. |

## §5 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../aiw-s4-dynamic-expansion/SCOPE-CONTRACT.md` | ✅ |
| Integration test W01 | `crates/sddk-engine/tests/aiw_s4_dynamic_expansion.rs` | 🔲 |
| UAT evidence rows W01/W03/W08/W09/W11 | `tests/cycle-artifacts/.../aiw-s4-dynamic-expansion/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `tests/cycle-artifacts/.../aiw-s4-dynamic-expansion/RECEIPT.md` | 🔲 |
| STATE-OF-AIW.md update | `docs/proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md` | 🔲 |
| 1 commit `test(engine)` | — | 🔲 |

## §6 Out of scope

- **AIW-S6/S7/S8** — sequenced after S4.
- **AIW-S1b** still STOP-pending on S4 A/B/C (different S4 — that
  S4 is the macro-cycle slice; AIW-S4 here is the AIW adoption slice).
- **Choice/Loop/Wait operator characterization** — out; AIW-S4 reuses
  the existing operators the way the rest of the engine does.
- **Worktree conflicts (W05 full)** — out; only the lease-fence path
  is touched, and that is already covered by `cycle_replan` itself.

## §7 References

- AIW milestone: `docs/proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md` §AIW-S4.
- AIW UAT matrix: `docs/proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md` §W01..W11.
- AIW state matrix: `docs/proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md`.
- `Engine::cycle_replan`: `crates/sddk-engine/src/cycle_replan.rs` + tests in `crates/sddk-engine/tests/cycle_replan.rs`.
- Secretary L0/L1: `crates/sddk-engine/src/secretary_l0.rs`, `crates/sddk-engine/src/secretary_l1.rs`.
- `ContinuationCandidate`: `crates/sddk-engine/src/continuation_candidate.rs`.
- AIW-S3 storage adapter (input to W01): `crates/sddk-engine/src/context_compiler/storage_adapter.rs`.
- AIW-S2 runner receipt (proxy for "Task ejecutable"): `crates/sddk-gateway/src/runner_receipt.rs`.
