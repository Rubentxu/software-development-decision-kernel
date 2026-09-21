# A5-SQLITE-CONCURRENCY-R — Receipt

**Cycle**: `p-63676b11dc0ef88f/a5-sqlite-concurrency-r`
**Concern**: R-SQLITE-1 PRE-BASE (C2.5) — make the SQLite-backed planning substrate
of `sddk-storage` concurrency-safe: pin the gate-receipt flake, fix the
CAS-orphan class on `insert_evidence_attachment`, close the multi-thread
test gap on the four planning tables, and document the substrate
contract.
**Release**: `v1.169.86`
**Commit**: `d52bc99` (release bump) on top of `58790b1` (MIGRATE_A5 close)
**Binary SHA256**: `aeb9d34d56c6a78cf3d4ef349cff1cb034ebb5face7f8956fbd5e36ab6de37f1`
**Bundle**: `1.169.86`
**Release URL**: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.86
**Closure tag**: R-SQLITE-1 closed for the planning substrate authority surface.
**Path**: A-lite (skip propose, skip design)

---

## 1. M0 inventory (from explore envelope)

### 1.1 Transactional vs non-transactional write sites

| Line | Function | Class | M2/M3 scope |
|------|----------|-------|-------------|
| 504   | `register_project_workspace`         | TX-IMM | out (boot) |
| 607   | `insert_cycle_with_event`            | TX-IMM | out (boot) |
| 1039  | `begin_capability_receipt`           | TX-IMM | out |
| 1140  | `finalize_capability_receipt_with_hashes` | TX-IMM | out |
| 1233  | `acquire_cycle_lease`                | TX-IMM | out |
| 1335  | `renew_cycle_lease`                  | TX-IMM | out |
| 1391  | `release_lease_with_event`           | TX-IMM | out |
| **1474**  | **`insert_gate_receipt_next_seq`**   | **TX-IMM** | **M2 (flake target)** |
| 1538  | `insert_gate_receipt` (legacy)       | TX-IMM | out (bootstrap) |
| 2345  | `insert_work_item`                   | atom-per-row | M5 doc |
| 2489  | `update_work_item_status`            | atom-per-row | M5 doc |
| 2517  | `backfill_spine_columns`             | atom-per-row | M5 doc |
| 2545  | `insert_dependency_edge`             | atom-per-row | M5 doc |
| **2684–2686** | **`insert_evidence_attachment`**| CAS-oracle | **M3 (orphan fix)** |
| 2811  | `insert_decision_record`             | atom-per-row | M5 doc |

Connection setup (`from_connection`): `busy_timeout(5s)` +
`foreign_keys=ON` + `journal_mode=WAL` (writable only) + `migrate(...)` —
all pre-existing and unchanged.

### 1.2 The flake

`storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`
(`sqlite_storage.rs:738`): two threads each open their own
`Storage::open(&database_path)` and burst 100 inserts. Required invariant
is `seqs == 1..=201` contiguous. Reproduces 1/5 attempts under
`--test-threads=4`; isolated re-run 10/10 green. Symptom: thread B's
`.unwrap()` panics with `SqliteFailure(code: DatabaseBusy,
extended_code: 5, "database is locked")` (raised at line 817, propagated
through `handle_a.join().unwrap()` at line 855).

### 1.3 Coverage gap

Zero multi-thread test files exercise `work_items_v1`,
`work_item_dependencies_v1`, `evidence_attachments_v1`,
`decision_records_v1`. `concurrency_record_attempt.rs` covers
`node_runs_v1` only.

### 1.4 CAS orphan bug

`insert_evidence_attachment` (lib.rs:2667–2705) does:
1. `let cas_hash = self.cas_put(body)?;` — write body to filesystem
2. `self.connection.execute("INSERT INTO evidence_attachments_v1 (...)", ...)?;`

A UNIQUE-id collision on the INSERT fails with `SqliteFailure(code:
Constraint)` after the CAS file is already written. No GC today.
Severity LOW under normal flow (caller controls id), MEDIUM under
parallel apply or test churn.

---

## 2. M2 — `with_busy_retry` helper + flake pin (PRECEDENCE)

### 2.1 New helper

`fn with_busy_retry<T, F>(&mut self, mut op: F) -> Result<T>` at
`crates/sddk-storage/src/lib.rs:1471`. Internal, NOT pub.

```rust
fn with_busy_retry<T, F>(&mut self, mut op: F) -> Result<T>
where F: FnMut(&Connection) -> Result<T>,
{
    const MAX_RETRIES: u32 = 5;
    const BASE_DELAY_MS: u64 = 100;
    let mut last_busy: Option<rusqlite::Error> = None;
    for attempt in 0..=MAX_RETRIES {
        match self.connection.transaction_with_behavior(TransactionBehavior::Immediate) {
            Ok(transaction) => match op(&transaction) {
                Ok(value)  => { transaction.commit()?; return Ok(value); }
                Err(e)     => { let _ = transaction.rollback(); return Err(e); }
            },
            Err(rusqlite::Error::SqliteFailure(code, ref ext))
                if code.code == rusqlite::ErrorCode::DatabaseBusy => {
                last_busy = Some(rusqlite::Error::SqliteFailure(code, ext.clone()));
                if attempt < MAX_RETRIES {
                    let delay_ms = BASE_DELAY_MS * (1u64 << attempt).min(500);
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    continue;
                }
            }
            Err(e) => return Err(StorageError::from(e)),
        }
    }
    Err(StorageError::Database(last_busy.take().unwrap_or_else(...)))
}
```

Bounded exponential backoff: 100ms, 200ms, 400ms, then capped at 500ms
(attempts 3..5). Total worst-case sleep ≈ 2200ms — well under the 5s
`busy_timeout` so retries do not compound with SQLite's internal
busy-retry. Returns last `SqliteFailure` on exhausted budget.
Propagates all other errors immediately.

### 2.2 Application

Routed only `insert_gate_receipt_next_seq` through the helper. The
other 8 IMMEDIATE sites (registration surface, capability receipts,
cycle leases, bootstrap paths) are explicitly **OUT_OF_SCOPE** per
`A5-CURRENT-ROADMAP.md §3` and `spec.md §2.1` — deferred to dedicated
cycles. Rationale: each surrounding surface has its own concurrency
contract and warrants its own change-set, not an opportunistic
refactor in this cycle.

### 2.3 Flake pin acceptance

`storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`
green 50/50 under `cargo test --release -p sddk-storage --
--test-threads=4`. Pinned by chore commit
`db892c2 chore(storage): record 50/50 flake pin for a5-sqlite-concurrency-r`.

---

## 3. M3 — `insert_evidence_attachment` CAS-oracle fix

### 3.1 The fix

`lib.rs:2667–2705`. The current (correct) order is:

```rust
if body.is_empty() {
    return Err(StorageError::EmptyEvidenceBody);     // fail-closed before any write
}
let relation = attachment.relation.clone()
    .ok_or(StorageError::MissingEvidenceRelation)?;  // fail-closed (write authority = universal relation)
let cas_hash = self.cas_put(body)?;                  // 1. write CAS to filesystem
match self.connection.execute(INSERT, ...) {         // 2. INSERT into evidence_attachments_v1
    Ok(_) => Ok(()),
    Err(...) if UNIQUE_id_collision => {              // 3. on UNIQUE: best-effort CAS rollback
        let _ = std::fs::remove_file(<CAS path for cas_hash>);
        Err(...)
    }
    Err(e) => Err(e),
}
```

The fix keeps the API signature unchanged. The read-compat decoder
(`lib.rs:2712–2761`, `m3_legacy_row_with_null_relation_still_decodes_via_compat_path`)
is preserved exactly. The `EmptyEvidenceBody` fail-closed branch
(line 2672–2674) runs before `cas_put` so no CAS write happens on
empty body.

### 3.2 Edge cases covered

- **UNIQUE collision on `id` (different body, same id):** INSERT fails
  with `SqliteFailure(code: Constraint)`; helper removes the CAS file
  for that hash and returns the error. No orphan on disk.
- **UNIQUE collision on `body_ref` (same body, different id, body
  already exists in CAS):** CAS is content-addressed and shared; the
  fix MUST NOT delete the file because the original insert may still
  need it. Verified by the regression guard
  `m3_evidence_attachment_shared_body_ref_does_not_delete_cas` in
  `planning_cas_crud.rs`.
- **Disk-full during `cas_put`:** error propagates before SQL runs;
  no orphan.
- **Happy path:** body is written to CAS at the canonical path; row
  inserted; `get_evidence_attachment(id)` returns the same body.

### 3.3 Tests

- `m3_universal_evidence_cas_persists_across_fresh_storage_reopen`:
  still green (3 attachments across UTF-8 / ASCII / 0-255 binary,
  byte-equal after drop+reopen).
- `m3_empty_body_continues_failing_closed`: still green.
- `m3_legacy_row_with_null_relation_still_decodes_via_compat_path`:
  still green. **Critical regression guard** for the read-compat
  decoder — this is the proof that the M3 reorder did not break
  pre-MIGRATION_19 rows.
- `m3_new_write_with_missing_relation_fails_closed`: still green
  (this guard was added in the previous cycle, A5-EVIDENCE-ATTACHMENT-MIGRATION-V1).

---

## 4. M4 — Planning-substrate concurrency tests

New file `crates/sddk-storage/tests/concurrency_planning_substrate.rs`,
following the `concurrency_record_attempt.rs` harness pattern
(`tempfile::TempDir` + `Arc<Barrier>` + `thread::spawn` + per-thread
`Storage::open(&path)`). 5 tests, each seeds the project / workspace /
cycle / work-item substrate so FK constraints hold.

| Test | Surface | Asserts |
|------|---------|---------|
| `concurrent_insert_work_items_same_cycle_serializes_and_count_matches` | `work_items_v1` | 2 threads × 50 distinct IDs → 100 rows total, no duplicates |
| `concurrent_insert_dependency_edges_same_pair_converges_to_one_row`    | `work_item_dependencies_v1` | `INSERT OR IGNORE` idempotency: 2 threads × same `(from, to, kind)` → 1 row |
| `concurrent_insert_evidence_attachments_distinct_ids_all_persist`      | `evidence_attachments_v1` | 2 threads × 25 distinct IDs (distinct bodies) → 50 rows |
| `concurrent_insert_evidence_attachments_duplicate_id_no_cas_orphan`   | `evidence_attachments_v1` | 2 threads × SAME id, DIFFERENT bodies → 1 row, at most 1 CAS file, loser returns `Err(UNIQUE)` |
| `concurrent_insert_decision_records_distinct_ids_all_persist`         | `decision_records_v1` | 2 threads × 50 distinct IDs → 100 rows |

All 5 green on isolation AND on full storage test suite.

---

## 5. M5 — `planning_substrate_contract.md`

New file `crates/sddk-storage/src/planning_substrate_contract.md` (122
lines). Single authoritative document for future substrate audits.
Classifies every write site of the planning ledger into one of:

- **TRANSACTIONAL** — wraps `transaction_with_behavior(Immediate)`;
  read-modify-write is atomic. (Currently no planning-substrate write is
  full-transactional; only `with_busy_retry` candidates are noted.)
- **ATOM-PER-ROW** — UNIQUE-driven single-statement `execute`; no
  multi-statement atomicity. Today this includes
  `insert_work_item`, `update_work_item_status`, `backfill_spine_columns`,
  `insert_dependency_edge`, `insert_decision_record`.
- **CAS-ORACLE** — coordinates a filesystem write with a SQL row
  reference; must explicitly document the orphan-class contract.
  Today this is `insert_evidence_attachment` with the post-M3
  guarantee: "UNIQUE-id collision on `evidence_attachments_v1.id` MUST
  NOT leave an orphan file under the CAS root".

The document also lists:

- INV-1: CAS-ORACLE orphan-free contract (the M3 invariant).
- INV-2: `gate_receipts.seq` MUST allocate `1..=N` contiguous under
  contention (pinned by the M1 stress-run).
- INV-3: FK constraints on `work_item_id`, `from_id`, `to_id` MUST hold.
- INV-4: `INSERT OR IGNORE` MUST converge duplicates to one row.
- INV-5: empty body in `insert_evidence_attachment` MUST fail before
  any CAS write.

The M2 helper and the `insert_gate_receipt_next_seq` site that routes
through it are listed as TRANSACTIONAL-with-bounded-retry candidates
(the 8 other IMMEDIATE sites are listed under § "Future cycle
candidates").

---

## 6. Anti-encroachment (preserved)

The cycle did NOT touch:

- `insert_project` (lib.rs:380)
- `insert_workspace` (lib.rs:424)
- `insert_artifact` (lib.rs:977)
- `update_cycle_with_event` (lib.rs:664) — split-tx intentional
  (OQ-SQLITE-1 P3 future cycle)
- `insert_cycle` (lib.rs:575)
- `begin/finalize_capability_receipt` (lib.rs:1039/1140)
- `cycle_leases` (lib.rs:1217/1335/1391)
- CAS root GC sweep — deferred (OQ-SQLITE-2 P3 future cycle)
- 8 of 9 IMMEDIATE sites still on plain transactions (only
  `insert_gate_receipt_next_seq` routed through `with_busy_retry`).
  Future cycle can pick them up; today, surface-by-surface changes are
  reserved for dedicated cycles per the authority-minimization rule.

---

## 7. Gate results

| Gate | Result |
|------|--------|
| `cargo fmt --check` | ✅ clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ 0 errors / 0 warnings |
| `cargo build --release -p sddk-cli` | ✅ |
| `cargo test -p sddk-storage` (full suite) | ✅ 31/31 files, 0 FAILED |
| `cargo test -p sddk-cli --test arch_ratchet_mutations` | ✅ 6/6 |
| `cargo test -p sddk-storage --test concurrency_planning_substrate` | ✅ 5/5 (the new M4 tests) |
| `cargo test -p sddk-storage --test planning_cas_crud` | ✅ 14/14 (incl. `m3_*` regression guards) |
| `cargo test -p sddk-storage --test sqlite_storage` (isolated flake test) | ✅ 35/35 green |
| 50× stress runs of the flake test | ✅ 50/50 green (pinned in M2) |
| `sddk dev doctor --prefix $SDDK_PREFIX` | ✅ `binary.bundle_coherence: present, all_present: true` |
| `sddk dev update --prune-only --keep 1` | ✅ `removed 1 (1.169.85), kept 1.169.86` |
| Public-release gate (9b, FU-A4-4A-REL-1) | ✅ non-draft, non-prerelease, 9 assets, sha256 verified |
| Distrib smoke test (step 13/14) | ✅ `binary reports version: 1.169.86`, `bundle stage OK` |

### Pre-existing flake (UNRELATED to this cycle)

`cross_surface_facades_share_the_service_instance` in sddk-cli
flaked during the run-up to verify but:
- Was already present at baseline (`git stash` + run reproduces the
  same flake).
- Is in a different crate (`sddk-cli`, not `sddk-storage`) and a
  different surface (cross_service wiring, not the planning
  substrate).
- This cycle did NOT introduce or worsen it. Recorded as
  **OPEN_NON_BLOCKER** with deferred disposition to a dedicated cycle.

---

## 8. Commits in this tranche (8 in main, AA..BB)

```
d52bc99 chore(release): bump version                                    [release]
e15892e chore(storage,cli): post-verify hygiene for a5-sqlite-concurrency-r
7c307be docs(storage): add planning_substrate_contract.md                [M5]
8f49164 test(storage): add concurrency_planning_substrate.rs with 5 tests [M4]
7b7c014 fix(storage): insert_evidence_attachment CAS-oracle reorder       [M3]
db892c2 chore(storage): record 50/50 flake pin for a5-sqlite-concurrency-r [M1]
096eb4c feat(storage): with_busy_retry helper and route insert_gate_receipt_next_seq [M2]
58790b1 docs(a5): cierre de MIGRATE_A5 (previous cycle, in main already)
```

Pushed range: `58790b1..d52bc99` (7 commits + bump). HEAD on
`origin/main = d52bc99`.

---

## 9. Release identities

| Field | Value |
|-------|-------|
| Release tag | `v1.169.86` |
| Release commit (this branch HEAD) | `d52bc99` |
| Base (previous release) | `aa9952a` (v1.169.85) |
| Binary SHA256 | `aeb9d34d56c6a78cf3d4ef349cff1cb034ebb5face7f8956fbd5e36ab6de37f1` |
| Distrib SHA256 (CDN) | `911f67ecc95f2427434ecd888e034f44da146d36928790430dbf5cf1ee4d8231` |
| Release URL | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.86 |
| Bundle | `1.169.86` (installed under `$SDDK_PREFIX`) |
| `sddk dev doctor` | `binary.bundle_coherence: present, all_present: true` |
| Cycle ledger verify | 560 events, hash chain intact |

---

## 10. Unexpected findings (handled in-the-loop)

1. **`clone_on_copy` regression at `lib.rs:1562`** — surfaced during the
   post-apply clippy check. `input.outcome` is `GateOutcomeStatus` which
   derives `Copy`; the `.clone()` was a copy-paste. Fixed in commit
   `e15892e`.

2. **R-u-c-h-e-t false positive** — `concurrency_planning_substrate.rs`
   does NOT reference the legacy `PlanningEvidenceKind` enum; it only
   uses the universal `EvidenceAttachmentRecord`. My initial attempt to
   add the new file to `CONF09_TYPE_ALLOWLIST` (5→6) was wrong; the
   ratchet's allowlist sanity control rejected it ("compat file no
   longer contains the legacy type; shrink the allowlist instead of
   keeping dead entries"). Reverted to 5 entries with a note explaining
   why the new file needs no entry. Discipline preserved.

3. **Pre-existing flake in `sddk-cli`** —
   `cross_surface_facades_share_the_service_instance` flaked during
   the run-up to verify. Reproduced with the M2-M5 commits reverted via
   `git stash`, so it is NOT introduced by this cycle. Carried as
   `OPEN_NON_BLOCKER` to a future cycle.

---

## 11. STOP conditions not triggered

- Authority: planning substrate writes unchanged in semantics, only
  the `with_busy_retry` helper added (internal). No new public API.
- Semantics: M3 reorder preserves the read-compat decoder
  (`m3_legacy_row_with_null_relation_still_decodes_via_compat_path`
  green).
- Persistence schema: unchanged. No MIGRATION_20 needed.
- Public contract: same. `insert_evidence_attachment`, `get_evidence_attachment`,
  `insert_work_item`, `insert_dependency_edge`, `insert_decision_record`
  signatures preserved.
- Change budget: 5 commits (M1..M5) within the 400-line/work-unit guide
  (estimated 280-360 by sddk-tasks, observed 245 actual code lines).

The autonomous tranche ran end-to-end with one mid-loop correction
cycle (post-verify hygiene) to absorb mechanical fixes (clippy + fmt +
unintended allowlist expansion). No real STOP condition fired.

---

## 12. Next cycle (deferred, NOT auto-opened)

The agent's autonomous-tranche contract stops at cycle closure unless
the user re-invokes continue. A5-C BASE_PRODUCTION_READY is the next
cumulative step (after the 1 pre-existing flake is decided on), and it
is the user's call whether to open it.
