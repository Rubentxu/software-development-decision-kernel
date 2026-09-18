# Planning Substrate Concurrency Contract

> **Authority:** `crates/sddk-storage/src/planning_substrate_contract.md`
> **Cycle:** `p-63676b11dc0ef88f/a5-sqlite-concurrency-r`
> **Status:** Active

This document classifies every write site on the SDDK planning substrate into exactly one
concurrency class, and documents the invariants that must hold under concurrent writers.

## Write-Site Classification

| Method | Class | Schema surface | Concurrency guarantee |
|--------|-------|---------------|----------------------|
| `insert_work_item` | **ATOM-PER-ROW** | `work_items_v1` (UNIQUE `id`) | SQLite row-level atomicity; concurrent inserts with distinct ids each succeed |
| `update_work_item_status` | **ATOM-PER-ROW** | `work_items_v1` (single-row UPDATE) | SQLite row-level atomicity; UPDATE is inherently serialized by row lock |
| `insert_dependency_edge` | **ATOM-PER-ROW** | `work_item_dependencies_v1` (composite PK: from_id, to_id, kind) | `INSERT OR IGNORE`; concurrent inserts with identical `(from_id, to_id, kind)` converge to exactly one row (INV-4) |
| `insert_evidence_attachment` | **CAS-ORACLE** | `evidence_attachments_v1` + CAS filesystem | SQL INSERT executes first; CAS file written only after INSERT succeeds. UNIQUE collision on `id` returns error immediately; CAS is never written (INV-1) |
| `insert_decision_record` | **ATOM-PER-ROW** | `decision_records_v1` (UNIQUE `id`) | SQLite row-level atomicity; concurrent inserts with distinct ids each succeed |
| `backfill_spine_columns` | **ATOM-PER-ROW** | `work_items_v1` (AC-PLN4-04 guarded) | SQLite row-level atomicity; guarded by AC-PLN4-04 lifecycle guard |
| `with_busy_retry` (helper) | **TRANSACTIONAL** | Wraps `transaction_with_behavior(Immediate)` | Bounded retry (max 5 attempts, 100–500ms exponential backoff) on `SQLITE_BUSY` (extended code 5); all other errors propagate immediately |
| `insert_gate_receipt_next_seq` | **TRANSACTIONAL** | `gate_receipts` (via `with_busy_retry`) | Routes through `with_busy_retry`; bounded retry on `DatabaseBusy` |

## Class Definitions

### ATOM-PER-ROW

A write that relies on SQLite's row-level atomicity for a single `execute` or `INSERT`.
No multi-statement atomicity. No multi-statement transaction needed for correctness
of the individual row.

**Guarantees:**
- A single row insert/update is atomic at the SQLite level.
- `INSERT OR IGNORE` paths converge duplicate composite PKs to exactly one row.
- FK constraints on `work_item_id`, `from_id`, `to_id` are preserved under concurrent writers.

**FK constraints:**
- `evidence_attachments_v1.work_item_id` → `work_items_v1.id`
- `work_item_dependencies_v1.from_id` → `work_items_v1.id`
- `work_item_dependencies_v1.to_id` → `work_items_v1.id`
- `decision_records_v1.work_item_id` → `work_items_v1.id`

### CAS-ORACLE

A write that coordinates a filesystem write (CAS file) with a SQL row reference.
The orphan class must be explicitly managed: a SQL UNIQUE collision on `id`
must not leave an orphaned CAS file.

**Invariant INV-1 (mandatory):**
```
UNIQUE-id collision in evidence_attachments_v1 MUST NOT leave an orphan CAS file.
```

**Implementation rule:** SQL INSERT executes first. CAS is written only after
INSERT succeeds. This makes the orphan class impossible by construction — the
reordered path never writes CAS before the DB row exists.

**Guarantees:**
- Exactly one row for a given `id` (UNIQUE constraint).
- At most one CAS file per distinct body hash (idempotent `cas_put`).
- Zero orphan CAS files on UNIQUE collision (by construction, not cleanup).

### TRANSACTIONAL

A write wrapped in `transaction_with_behavior(Immediate)` with bounded retry on
`SQLITE_BUSY` (extended code 5). Full multi-statement atomicity.

**Bounded-retry contract (`with_busy_retry`):**
- Max 5 retry attempts with exponential backoff: 100ms, 200ms, 400ms, 500ms, 500ms.
- Retries only on `rusqlite::Error::SqliteFailure(code: DatabaseBusy, extended_code: 5)`.
- All other errors propagate immediately (no retry).
- Fits inside the connection's existing `busy_timeout(5s)` budget.

## IMMEDIATE Transaction Sites (9 total)

SQLite's `IMMEDIATE` transaction behavior acquires a write lock at the start of
the transaction (not at commit time), which serializes concurrent writers.
The following methods open `transaction_with_behavior(Immediate)`:

| Line | Method | Routing |
|------|--------|---------|
| ~504 | `register_project_workspace` | Out of scope (registration surface) |
| ~607 | `insert_cycle` | Out of scope (bootstrap path, runs inside `insert_cycle_with_event`'s IMMEDIATE) |
| ~1039 | `begin_capability_receipt` | Out of scope (capability-receipt substrate) |
| ~1140 | `finalize_capability_receipt` | Out of scope (capability-receipt substrate) |
| ~1233 | `acquire_cycle_lease` | Out of scope (already fully transactional, lease semantics) |
| ~1335 | `renew_cycle_lease` | Out of scope (already fully transactional) |
| ~1391 | `release_cycle_lease` | Out of scope (already fully transactional) |
| ~1474 | `insert_gate_receipt_next_seq` | Routes through `with_busy_retry` (bounded retry) |
| ~1538 | `insert_gate_receipt` | Out of scope (bootstrap, caller-supplied seq) |

## Planning Substrate Write Sites (6 total)

These are the methods that write to the four planning tables:

| Table | Methods |
|-------|---------|
| `work_items_v1` | `insert_work_item`, `update_work_item_status`, `backfill_spine_columns` |
| `work_item_dependencies_v1` | `insert_dependency_edge` |
| `evidence_attachments_v1` | `insert_evidence_attachment` |
| `decision_records_v1` | `insert_decision_record` |

## Derived Invariants

| ID | Invariant | Verified by |
|----|-----------|-------------|
| INV-1 | UNIQUE-id collision in `evidence_attachments_v1` MUST NOT leave an orphan CAS file. | M3 scenarios + M4 test #4 |
| INV-2 | `gate_receipts.seq` allocation MUST produce `1..=N` contiguous, gap-free, distinct values under 2-thread contention. | M1 stress-run + `storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq` |
| INV-3 | FK constraints on `work_item_id`, `from_id`, `to_id` MUST be preserved under concurrent writers. | M4 tests #1, #2, #5 |
| INV-4 | `INSERT OR IGNORE` for `insert_dependency_edge` MUST converge duplicate `(from_id, to_id, kind)` to exactly one row. | M4 test #2 |
| INV-5 | Empty body in `insert_evidence_attachment` MUST fail before any CAS write. | M3 scenario + `m3_empty_body_continues_failing_closed` in `planning_cas_crud.rs` |

## Out of Scope

| Site | Reason |
|------|--------|
| `insert_project` (lib.rs) | Separate registration surface; runs inside `register_project_workspace`'s transaction |
| `insert_workspace` (lib.rs) | Same registration surface as above |
| `insert_artifact` (lib.rs) | Separate evidence-of-artifacts substrate; distinct write pattern |
| `update_cycle_with_event` (lib.rs) | Split-transaction design is intentional (event-first for fail-closed semantics); audit deferred to OQ-SQLITE-1 (P3) |
| `insert_cycle` (lib.rs) | Bootstrap path; runs inside `insert_cycle_with_event`'s IMMEDIATE transaction in production |
| `begin/finalize_capability_receipt` (lib.rs) | Separate capability-receipt substrate, already fully transactional |
| `cycle_leases` (lib.rs) | Already transactional; no flake observed |
