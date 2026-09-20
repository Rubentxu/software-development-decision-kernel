# SCOPE-CONTRACT — AIW-S3 — Handoff durable entre DOS tareas

> **Slice id:** `p-63676b11dc0ef88f/aiw-s3-handoff-durable`
> **Macro-cycle (AIW adoption):** `aiw-delivery-complete`
> **Status:** planning + implementation in one session (auto-run mode).

## §1 Goal

Close **AIW-S3 — Handoff durable entre DOS tareas**: a real
`Storage`-backed `ContextAdapter` participates in `ContextCompiler`,
and a second process can re-derive the capsule from persisted state
with no in-memory hand-off, no shared transcript, no `InMemoryStore`
shortcut.

The current `ContextCompiler` (`crates/sddk-engine/src/context_compiler.rs`)
is well-defined but ships with **only fake adapters** in-tree. This slice
materializes the first production adapter pair that read state from
`Storage` (sqlite) so that H01 (E2E + DUR) and H06 (no in-memory shortcut)
are satisfied with evidence on disk.

**Important prior context** (not re-implemented):
- `sddk_engine::context_compiler::ContextCompiler` and the
  `ContextAdapter` trait already exist (M3 / arch-spec-006).
- `sddk_storage::Storage` (sqlite-backed) already provides the read APIs.
- The bounded runner (AIW-S2) provides a content-addressable receipt
  for executable evidence; handoff references those.

## §2 UAT rows in scope

Per `docs/proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`
§Agenda/Secretary/handoff (H01..H09). AIW-S3's contracted slice focuses
on the **durable handoff** kernel; full Secretary attention / advisory
synthesis is AIW-S7's contracted slice.

| UAT id | Scenario | Expected invariant |
|---|---|---|
| H01 | DUR+E2E — dos tareas con handoff tras cerrar proceso | Segunda tarea recupera cápsula desde storage real, sin transcript, sin InMemoryStore. |
| H04 | REG — varias revisiones apuntan a mismo trabajo | Refs WorkItem→PlanRevision→Node→Attempt→Evidence estables, sin duplicar tareas. |
| H06 | IT — disenso de riesgo alto y evidencia obligatoria | Synthesis conserva disposición+refs, no omite silenciosamente. |
| H09 | NEG — capsule compile con dos bases incompatibles | Declarar conflicto o reconstrucción; no presentar snapshot falso. |

H02/H03/H05/H07/H08 are out of the AIW-S3 slice (these need Secretary
consumers and Verify-run gating — AIW-S7 and AIW-S4 follow-ups). They
are referenced in the SCOPE for completeness but **not closed** by this
slice; their T-row status remains "covered by AIW-S7/S4" in UAT-EVIDENCE.

## §3 Hard constraints

- **C1**: No modification of `ContextCompiler` API, `ContextAdapter`
  trait, or `ContextCapsuleV2` struct (pinned contract).
- **C2**: No modification of `Storage` core APIs (the read APIs used
  must be existing public methods).
- **C3**: Adapter must read from a real `Storage` opened via
  `Storage::open(path)` or `Storage::open_in_memory()` — the latter
  uses a private tempfile and is **durable** for H06 (WU-C1.2
  explicitly addresses this: in-memory is isolation, not fake).
- **C4**: No new dependency added.
- **C5**: Workspace test green; AIW-S0/S1/S2/S5 + A6 macro-cycle S1..S3
  remain green.

## §4 STOP conditions

| Condition | Action |
|---|---|
| Required modification of `ContextCompiler` API (e.g. need `async fn`) | **STOP** — material architectural change. Stop, document, raise to operator. |
| Required new write-side API in `Storage` for handoff state | **STOP** — public-contract change. Only pre-existing read APIs. |
| Required schema migration | **STOP** — destructive migration. Use only what exists. |
| Adapter cannot be expressed within the existing `ContextAdapter` trait | **STOP** — re-evaluate the trait, raise to operator. |
| `cargo fmt` / `cargo clippy -p sddk-engine --all-targets -- -D warnings` fails | Fix and continue (not a STOP). |

## §5 Deliverables

| Deliverable | Path | Status |
|---|---|---|
| SCOPE-CONTRACT (this file) | `tests/cycle-artifacts/.../aiw-s3-handoff-durable/SCOPE-CONTRACT.md` | ✅ |
| `StorageLedgerHeadAdapter` | `crates/sddk-engine/src/context_compiler/storage_adapter.rs` | 🔲 |
| Re-export hook in `context_compiler.rs` | `crates/sddk-engine/src/context_compiler.rs` | 🔲 |
| Integration test H01 (DUR) | `crates/sddk-engine/tests/aiw_s3_storage_handoff.rs` | 🔲 |
| Unit tests for adapter (negative cases H02/H09) | `crates/sddk-engine/src/context_compiler/storage_adapter.rs` `#[cfg(test)]` | 🔲 |
| UAT evidence rows | `tests/cycle-artifacts/.../aiw-s3-handoff-durable/UAT-EVIDENCE.yaml` | 🔲 |
| RECEIPT | `tests/cycle-artifacts/.../aiw-s3-handoff-durable/RECEIPT.md` | 🔲 |
| STATE-OF-AIW.md update | `docs/proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md` | 🔲 |
| 1 commit `feat(engine)` | — | 🔲 |

## §6 Out of scope

- **AIW-S4** (Secretary L0/L1/L2 — actual consumer of the capsule) — next slice.
- **AIW-S6/S7/S8** — sequenced after S4.
- **Modify `ContextCompiler` API** — forbidden by C1.
- **Adapter-specific writers** — adapter is read-only; writes stay in
  `Storage` caller's responsibility.
- **Async adapter** — trait is sync; staying sync.

## §7 References

- AIW milestone: `docs/proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md` §AIW-S3.
- AIW UAT matrix: `docs/proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md` §H01..H09.
- AIW state matrix: `docs/proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md`.
- `ContextCompiler` and `ContextAdapter`: `crates/sddk-engine/src/context_compiler.rs`.
- `Storage`: `crates/sddk-storage/src/lib.rs` (read APIs: `project_count`,
  `latest_log_head`, etc.).
- AIW-S2 receipts: `crates/sddk-gateway/src/runner_receipt.rs` (referenced
  by capsule.provenance).
