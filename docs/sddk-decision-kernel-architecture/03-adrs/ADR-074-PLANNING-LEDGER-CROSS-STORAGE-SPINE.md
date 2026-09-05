# ADR-074 — Planning Ledger Cross-Storage Provenance Chain + Spine Import

**Canonical title:** `ADR-074-PLANNING-LEDGER-CROSS-STORAGE-SPINE.md`
**Status:** Accepted
**Date:** 2026-09-05
**Decision authority:** User-confirmed architectural locks (Q5–Q9) + apply-phase execution
**Related Work Item:** PLN-LEDGER-003 (order 313, path A-min, horizon H1)
**Direct dependency:** PLN-LEDGER-002 (v1.86.0, `782daa32`)

## 1. Status

Accepted. The five decisions below (Q5–Q9) are locked for PLN-LEDGER-003. This ADR records their architecture authority. The ADR was promoted from draft during apply phase.

## 2. Context

PLN-LEDGER-003 is the third H1 planning-ledger cycle. Its exit gate requires:
1. Cross-storage provenance chain verification (INC-240545): a chain built by one Storage instance must be verifiable by another Storage instance with the same or different CAS root.
2. `EXECUTION-SPINE.yaml` import pipeline: rows from the spine YAML file are upserted into `work_items_v1`, `evidence_attachments_v1`, and `dependency_edges_v1` tables.

PLN-LEDGER-002 (v1.86.0, `782daa32`) shipped the persistence layer, dependency resolution service, provenance chain v1, and `verify_references`. PLN-LEDGER-003 extends the provenance chain with cross-storage identity (schema v2) and builds the spine import pipeline.

The cross-storage verification problem: when a provenance chain is built by Storage A and verified by Storage B, the verifier needs to detect whether the two Storage instances share the same CAS root (aligned) or have different CAS roots (mismatched). Mismatched roots mean the two Storage instances have diverged — this is a critical integrity signal, not a dangling-reference signal. The drift must be detected BEFORE the dangling-reference loop.

## 3. Decision

### 3.1 Schema v2: Producer stamp fields added to `PlanningProvenanceChainV1`

`PlanningProvenanceChainV1` gets two new fields (backward-compatible via `Option`, defaults to `None` for v1 chains):

```rust
pub struct PlanningProvenanceChainV1 {
    // ... existing v1 fields ...
    pub schema_version: u32,               // 1 = v1 chain (no stamp), 2 = v2 chain (with stamp)
    pub producer_cas_root_id: Option<String>, // SHA-256 of producer's canonical CAS root path
    pub producer_signature: Option<String>,    // Reserved for future signature scheme
}
```

| Scenario | `schema_version` | `producer_cas_root_id` | `producer_signature` |
|---|---|---|---|
| Chain built by v2 `build_provenance_chain_v2()` | 2 | `Some(cas_root_id)` | `Some("sddk")` |
| Chain built by v1 `build_provenance_chain()` | 1 (unchanged) | `None` | `None` |
| v1 chain deserialized from existing JSON | 1 | `None` | `None` |

`effective_schema_version()` returns `self.schema_version.max(PLANNING_PROVENANCE_SCHEMA_VERSION)` — effectively 2 for any chain built by a v2-aware system.

### 3.2 `verify_references_with_options`: drift check runs BEFORE dangling-reference loop

The existing `verify_references` method is unchanged. A new `verify_references_with_options(opts: VerifyReferencesOptions)` method adds:

```rust
pub struct VerifyReferencesOptions {
    pub strict_cross_storage: bool, // default: false
}
```

**Drift detection (new, runs first):**

```
IF chain.effective_schema_version() >= 2 AND verifier.cas_root_id() != chain.producer_cas_root_id.get():
    RETURN Err(CrossStorageDrift {
        reason: "cas_root_id_mismatch",
        producer_cas_root_id: chain.producer_cas_root_id,
        verifier_cas_root_id: verifier.cas_root_id(),
        cycle_id: chain.cycle_id.clone(),
    })
```

**Then, dangling-reference loop (existing behavior, unchanged):**

```
FOR each work_item_id IN chain.work_item_ids:
    IF NOT verifier.work_item_exists(work_item_id):
        RETURN Err(DanglingReference(work_item_id))
```

**Strict mode (v1 chains):** When `strict_cross_storage: true` AND the chain has `schema_version == 1` AND `producer_cas_root_id == None`, the drift check also fires if `verifier.cas_root_id()` differs from any previously observed CAS root. This is a conservative mode for cross-storage migration audits.

**Backward compatibility:** `verify_references()` calls `verify_references_with_options(VerifyReferencesOptions::default())`. Default options have `strict_cross_storage: false`, so v1 chains on mismatched roots pass silently — preserving existing behavior.

### 3.3 `Storage` handle identity: `handle_id` and `cas_root_id`

Each `Storage` instance gets two stable identity methods:

- **`Storage::handle_id() → String`**: A stable process-local UUID generated once at `Storage::open()` / `Storage::open_in_memory()`. Different `Storage` instances opened in the same process get different handle IDs. Not persisted. Used only for observability (logging, error messages).

- **`Storage::cas_root_id() → String`**: SHA-256 hash of the **canonical** CAS root path (`Storage::canonical_cas_root()`). Canonical means resolved symlinks and normalized separators. The hash is computed lazily on first call and cached in `OnceLock<String>`. Two Storage instances pointing to the same CAS root (even via different symlinks) produce the same `cas_root_id`.

### 3.4 `PlanningGraphRead` port extended with CAS root identity

The `PlanningGraphRead` trait adds two methods:

```rust
trait PlanningGraphRead {
    // ... existing 5 methods ...
    fn cas_root_id(&self) -> String;
    fn handle_id(&self) -> String;
}
```

`Storage` implements both via the new methods above.

### 3.5 Spine import: `EXECUTION-SPINE.yaml` → SQL

The import pipeline (`crates/sddk-storage/src/spine_import.rs`) performs:

**Phase 1 — Parse and canonicalize:**
1. Read raw bytes
2. `parse_spine_yaml(bytes)` → `ExecutionSpineV1` (validates schema_version: 2, rejects unknown fields, comment-tolerant)
3. `canonicalize_spine_bytes(bytes)` → canonical YAML bytes (sorted keys, normalized list format)

**Phase 2 — Validate:**
4. Self-loop check: any spine item with `depends_on` containing its own `id` → `Err(SelfLoop { item_id })`
5. Unknown-dependency check: any `depends_on` reference to an `id` not in the spine file → `Err(UnknownDependency { item_id, unknown })`

**Phase 3 — Upsert (per row, idempotent via composite-PK):**
6. Compute `body_ref = sha256(canonical_yaml_bytes)` (content-addressable)
7. Upsert `work_items_v1` row: `id = spine.id`, `cycle_id = spine.id` (per Q5), `title = spine.id` (per Q6), `status = mapped(spine.status)`, other fields from defaults
8. Upsert `evidence_attachments_v1` row: `id = spine.id + "-spine-evidence"`, `work_item_id = spine.id`, `body_ref`, `kind = Planning`, `schema_version = 1`
9. Upsert `work_item_dependencies_v1` rows: for each `depends_on` entry, upsert with composite PK `(from_id=spine.id, to_id=dep, kind=Blocks)`

**Phase 4 — Conflict detection:**
10. Before upserting each row, check if an existing row with the same `(id, ...)` has a **different** `objective` or `status` → `Err(ImportConflict { item_id, field, existing, incoming })`

**Conflict detection is per-row hard-error (Q8):** If ANY row conflicts, the entire import aborts. No partial import. The user must resolve the conflict and retry.

### 3.6 Spine status → WorkItemStatus mapping table

Per locked Q7:

| SpineStatus (EXECUTION-SPINE.yaml) | WorkItemStatus |
|---|---|
| `PROPOSED` | `Draft` |
| `READY` | `Draft` |
| `ACTIVE` | `Active` |
| `PARTIAL` | `Active` |
| `BLOCKED` | `Paused` |
| `SHIPPED` | `Done` |
| `ABSORBED` | `Done` |
| `SUPERSEDED` | `Superseded` |

Six of the eight spine statuses map to six of the seven `WorkItemStatus` variants. `Cancelled` is NOT reachable from spine import (per design).

### 3.7 Locked decisions Q5–Q9

| # | Question | Decision |
|---|---|---|
| Q5 | Cycle ownership of spine rows | `work_items_v1.cycle_id = spine.id` — one spine row = one cycle |
| Q6 | `title` field from spine | `work_items_v1.title = spine.id` — spine `id` field becomes the title |
| Q7 | `storage_handles` table | NOT CREATED — `Storage::handle_id()` / `Storage::cas_root_id()` are in-memory only |
| Q8 | Re-import with mutated objective/status | Hard error — `Err(ImportConflict { ... })`, no partial import |
| Q9 | Test scope | Rust-only tests — no shell-based integration tests for spine import |

### 3.8 CLI surface

**`sddk plan import --spine <path> [--format json\|text]`:**
- Reads `EXECUTION-SPINE.yaml` from `<path>`
- Calls `import_spine()` on the current plan's storage
- Outputs `ImportSummary { imported, already_present, conflicts }` in JSON or text format
- Exit 0 on success, 1 on error (parse error, self-loop, unknown dep, conflict, storage error)

**`sddk cycle verify-references --plan <path> [--format json\|text] [--strict-cross-storage]`:**
- Opens storage for plan at `<path>`
- Builds provenance chain v2 for the cycle
- Calls `verify_references_with_options()` with the given options
- Outputs `{verifier_cas_root_id, status, dangling, error}` in JSON or text format
- Exit 0 if verified, 1 if drift or dangling reference

## 4. Consequences

### Positive

- Cross-storage drift is detected before dangling references, preventing false negatives when the verifier has different data because it is a different Storage instance.
- The v2 schema is backward-compatible: v1 chains serialize/deserialize unchanged, and `effective_schema_version()` returns the right value for both.
- Spine import is fully idempotent: two consecutive identical imports report `{imported: 0, already_present: 1, conflicts: 0}`.
- Conflict detection is per-row and hard-failing: no silent data corruption.
- `Storage::handle_id()` / `Storage::cas_root_id()` are zero-cost at runtime (lazy-computed, cached).
- The 8-status → 6-WorkItemStatus mapping table is total and explicit.
- Q7 decision (no `storage_handles` table) keeps the schema additive-only (MIGRATION_14/MIGRATION_15 unchanged).

### Costs and risks

- The drift check fires on **every** `verify_references_with_options` call for v2 chains, even when the two Storage instances are known to be different processes on the same machine. This is intentional — unexpected CAS root mismatches are always a problem.
- The `producer_signature` field is reserved but not used in v1.86.0. A future signature scheme would need a ADR to define the signing key and scheme.
- The spine import conflict detection is per-row; if the user has 100 rows and row 99 conflicts, all 98 prior upserts are rolled back. This is safe but potentially confusing.
- The `cas_root_id` is a SHA-256 of the canonical path — if the CAS root is a symlink that changes target, the CAS root ID changes. This is correct behavior (the physical storage changed), but may surprise users who expect symlinks to be transparent.

### Verification obligation

The implementation MUST deliver ~58 new tests across four layers (per AC-PLN3-12):

- ~21 domain unit tests: 11 cross-storage provenance chain tests, 8 spine parsing tests, 2 spine status mapping tests.
- ~13 storage integration tests: 12 spine import idempotency/conflict tests, 1 graph identity stability test.
- ~6 cross-storage verifier tests using real Storage instances.
- ~6 CLI integration tests: 4 `plan import` tests, 2 `cycle verify-references` tests.

## 5. Alternatives considered

### Q5 — Cycle ownership of spine rows

- **One spine file = one cycle, one row = one sub-cycle**: rejected because `EXECUTION-SPINE.yaml` is a flat list, not a hierarchy. Mapping one spine row to one cycle aligns with the existing `work_items_v1.cycle_id` semantics.
- **`cycle_id = spine.id` (selected)**: aligns with existing PLN-LEDGER-001/002 semantics where `cycle_id` is the work item's identity.
- **`cycle_id = parent spine id or plan_id`**: rejected because it would require inferring parentage from a flat list.

### Q6 — Title field from spine

- **Use `objective` as title**: rejected because `objective` is prose text, potentially long, and not a stable identifier. The spine `id` is the canonical stable identifier.
- **`title = spine.id` (selected)**: the `id` field in EXECUTION-SPINE.yaml is the semantic work-item identifier — it should be the title.

### Q7 — `storage_handles` table

- **Create `storage_handles_v1` table with CAS root and handle ID**: rejected because it introduces mutable schema state that must be kept in sync with the actual Storage instances. The handle ID and CAS root ID are process-local runtime properties, not persistent schema data.
- **In-memory only (selected)**: `Storage::handle_id()` and `Storage::cas_root_id()` are computed once and cached. They are not persisted.

### Q8 — Re-import conflict handling

- **Silent merge (last-write-wins)**: rejected because it silently overwrites user data without any signal.
- **Soft warning**: rejected because a warning would be easy to miss and the conflict is a real data integrity issue.
- **Hard error (selected)**: `Err(ImportConflict { ... })` aborts the entire import. The user must resolve the conflict (either fix the spine YAML or acknowledge the existing data) and retry.

### Q9 — Test scope

- **Shell-based integration tests for spine import**: rejected per Q9 lock. The spine import is tested via Rust unit/integration tests only. CLI behavior is tested via the existing `sddk_cli::run_from` test harness.
- **Rust-only tests (selected)**: all tests are Rust, using `Storage::open_in_memory()` for isolation.

## 6. References

- ADR-068 — Deterministic IR and compiler-boundary invariants.
- ADR-069 — Explicit authority matrix and three-variant `ActorKind` contract.
- ADR-072 — Planning Ledger Domain Model (Shape C hybrid persistence).
- ADR-073 — Planning Ledger Persistence (Q1–Q4 locks).
- PLN-LEDGER-002 cycle — persistence layer shipped at v1.86.0 (`782daa32`).
- `docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml` — spine source file (schema_version: 2).
- `crates/sddk-domain/src/spine.rs` — spine types and parser.
- `crates/sddk-storage/src/spine_import.rs` — import pipeline implementation.
- `crates/sddk-domain/src/planning/mod.rs` — `PlanningProvenanceChainV1` v2 schema and cross-storage verification.
- `crates/sddk-domain/src/planning/cross_storage_tests.rs` — 11 cross-storage unit tests.
- `crates/sddk-storage/src/spine_import_tests.rs` — 13 idempotency/conflict integration tests.
- `crates/sddk-storage/src/cross_storage_verifier_tests.rs` — 6 real-Storage cross-storage tests.
- `crates/sddk-cli/tests/spine_import_cli.rs` — 4 CLI tests.
- `crates/sddk-cli/tests/cycle_verify_references_cli.rs` — 2 CLI tests.

(End of file — total 269 lines)
