# A5-EVIDENCE-ATTACHMENT-MIGRATION-V1 — Receipt

**Cycle**: `p-63676b11dc0ef88f-a5-evidence-attachment-migration-v1`
**Concern**: MIGRATE_A5 PRE-BASE (C2.5) — universal `EvidenceRef` becomes the only
productive evidence attachment shape, fail-closed on unknown kinds, and prove
CAS persistence across fresh-storage reopens.
**Release**: `v1.169.85`
**Commit**: `aa9952af1e150896ffd748cc6b27983338546a9e`
**Binary SHA256**: `800d76eab00587e4d9ea451adb6e936590ea71cec90d09a6e8addea78787fe09`
**Bundle**: `1.169.85`
**Release URL**: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.85
**Closure tag**: MIGRATE_A5 closed for the authority surface.

---

## 1. M0 inventory (preflight, before any code)

### 1.1 Two distinct `EvidenceAttachmentV1` authority surfaces

| Location | Role | Status |
|---|---|---|
| `crates/sddk-engine/src/evidence_ref.rs:192` | engine adapter (`kind_string` / `locator` / `payload_bytes`); had `From<...>` with silent `_ => Adhoc` fallback | **DELETED** |
| `crates/sddk-domain/src/planning/mod.rs:352` | domain struct (`#[deprecated]` since 1.168.60); re-exported via `sddk-domain::lib.rs` | **DELETED** |

### 1.2 Production writer

Only `crates/sddk-storage/src/spine_import.rs:384` (`from_universal_relation`) was
the productive entry. CLI `plan.rs:run_evidence` already routed through the
universal resolver.

### 1.3 CAS persistence — already in production path

- `Storage::cas_put` at `crates/sddk-storage/src/lib.rs:2933`
- `insert_evidence_attachment` at line `2641` calls `cas_put` before INSERT
- `get_evidence_attachment` at line `2725` calls `cas_get`

Missing: fresh-storage reopen test. **M3 added it.**

### 1.4 Ratchets (pre-cycle)

- `conf09_universal_evidence_only` (DENY, 4-entry allowlist)
- `conf09_no_planning_evidence_new_writes` (DENY, 2-entry construction allowlist)
- `evidence_kind_v1` lint DENY with 0 hits

### 1.5 Schema

`evidence_attachments_v1` carries `kind` (legacy / read-compat) + `relation`
(MIGRATION_19, production authority).

### 1.6 Anti-encroachment

The only `relation: None` hit outside this scope is `FindingLocation` in
`reducer.rs:334` — out of scope and untouched.

---

## 2. M1 — Engine adapter deletion + typed error

### 2.1 Deleted surfaces

- `EvidenceAttachmentV1` struct in `crates/sddk-engine/src/evidence_ref.rs`
- `From<EvidenceAttachmentV1> for EvidenceRef` (silent fallback)
- 3 adapter tests including `unknown_kind_falls_back_to_adhoc`

### 2.2 Added surfaces

- Typed `UnknownEvidenceKind` error (fail-closed)
- `EvidenceKind::from_domain_tag(tag: &str) -> Result<Self, UnknownEvidenceKind>`
  (previously `-> Option<Self>` with silent `_ => Adhoc` fallback)

### 2.3 Call-site fixes

- `crates/sddk-engine/src/architecture_graph/overlay.rs:625` — `.ok()?` collapse
  to None (callers tolerate "unknown kind = skip")
- `crates/sddk-engine/tests/a4_5c_arch_spec_047_acceptance.rs:917` — test
  rewritten to expect `Err(UnknownEvidenceKind)`

---

## 3. M2 — Domain surface deletion

### 3.1 Deleted

- `sddk_domain::planning::EvidenceAttachmentV1` struct
- `EvidenceAttachmentV1::new()` constructor
- `into_domain` / `from_domain` on `EvidenceAttachmentRecord`
- `evidence_attachment_round_trip` test
- Re-export of `EvidenceAttachmentV1` from `sddk-domain::lib.rs`

### 3.2 Replaced

- `EvidenceAttachmentRecord::into_record(self) -> Self` identity alias (callers
  that still want the "produce a record from a record" path)
- Module-level doc and section header refreshed to reflect that
  `EvidenceAttachmentRecord` is now the only authority at the domain edge.

---

## 4. M3 — Storage fail-closed + CAS reopen proof

### 4.1 New error variant

```rust
StorageError::MissingEvidenceRelation
    code:    STORAGE_MISSING_EVIDENCE_RELATION
    message: "evidence attachment write requires non-empty relation"
    recovery: "construct the record via EvidenceRef::new(...).with_relation(...)"
```

### 4.2 Hardened writer

`insert_evidence_attachment` no longer falls back to `attachment.kind.relation_tag().to_string()`
when `attachment.relation` is `None`. It now does:

```rust
attachment.relation.clone().ok_or(StorageError::MissingEvidenceRelation)?
```

Asymmetry is documented inline: legacy compatibility belongs to the READ
decoder (`from_legacy_kind_tag`), never to the WRITE path.

### 4.3 Four new tests in `planning_cas_crud.rs`

| Test | What it proves |
|---|---|
| `m3_universal_evidence_cas_persists_across_fresh_storage_reopen` | 3 attachments (UTF-8 / ASCII / full 0-255 byte binary); drop storage A; reopen storage B; assert byte-equal content, relation survives, `body_ref == sha256`, actor metadata survives, list-by-work-item consistent |
| `m3_new_write_with_missing_relation_fails_closed` | Record built with `relation = None` is rejected with `MissingEvidenceRelation` |
| `m3_empty_body_continues_failing_closed` | Empty body still rejected before CAS write |
| `m3_legacy_row_with_null_relation_still_decodes_via_compat_path` | INSERT normal record, UPDATE `relation` to NULL, reopen storage, assert `from_legacy_kind_tag` populates relation (READ compatibility stays at the read boundary) |

All 14 tests in `planning_cas_crud` pass (incl. existing
`evidence_attachment_cas_round_trip`).

### 4.4 New dev-dependency

`sddk-storage/Cargo.toml`: `tempfile = { workspace = true }` added to
dev-dependencies for the reopen test.

---

## 5. Ratchet allowlist expansion

`crates/sddk-cli/tests/arch_ratchet_mutations.rs`:

| Allowlist | Before | After | Reason |
|---|---|---|---|
| `CONF09_TYPE_ALLOWLIST` (`conf09_universal_evidence_only`) | 4 entries | **5 entries** | Added `planning_cas_crud.rs` |
| `CONF09_CONSTRUCTION_ALLOWLIST` (`conf09_no_planning_evidence_new_writes`) | 2 entries | **3 entries** | Added the storage test that constructs an `EvidenceAttachmentRecord` to validate the legacy decoder path |

Both ratchet tests (6/6 in `arch_ratchet_mutations`) still pass.

---

## 6. Stale comment fixes (documentation alignment)

- `crates/sddk-engine/src/evidence_relation_mapping.rs` — header doc
- `crates/sddk-storage/src/spine_import.rs` — doc comment about legacy kind
- `crates/sddk-storage/tests/spine_import_idempotent.rs:107` — test docstring

None of these were behaviour changes; they were narrating the migration
already enforced by the code.

---

## 7. Gate results

| Gate | Result |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| `cargo build --workspace` | ✅ |
| `cargo build --release -p sddk-cli --bin sddk` | ✅ (`sddk 1.169.85`) |
| `cargo test --workspace` | ✅ exit 0 |
| `cargo test -p sddk-storage --test planning_cas_crud` | ✅ 14/14 (incl. 4 new M3) |
| `cargo test -p sddk-cli --test arch_ratchet_mutations` | ✅ 6/6 |
| Public-release gate (FU-A4-4A-REL-1) | ✅ (release not draft, not prerelease, 9 assets, sha256 verified from CDN) |
| `sddk dev doctor --prefix $SDDK_PREFIX` | ✅ `binary.bundle_coherence: present, all_present: true` |
| `sddk dev update --prune-only --keep 1` | ✅ `removed 1 stale bundle(s); kept 1.169.85` |

---

## 8. Commits in this tranche

```
aa9952a chore(release): bump version                              (bump + Cargo.lock)
6009b1d chore(cli): expande allowlists conf09 para el nuevo M3
ccf1564 test(storage): demuestra persistencia CAS de evidencia universal tras reapertura
be606be feat(engine,domain): elimina autoridad legacy de EvidenceAttachmentV1 y fail-closed en EvidenceKind
```

Pushed range to `refs/heads/main`: `4597677..aa9952a` (forced update after
amend). Final state: `origin/main = aa9952a`.

---

## 9. Remaining compat-only surfaces (READ boundary)

The only place `EvidenceAttachmentV1` (or its `kind` string) survives is the
**READ path**:

- `crates/sddk-storage/src/lib.rs` — `from_legacy_kind_tag` decoder
- Schema column `evidence_attachments_v1.kind` — kept for read compatibility
  with rows persisted by older binaries

These surfaces are **read-only**. They do not mint new authority. New writes
must always provide a non-empty `relation` or they are rejected.

---

## 10. Falsification

- ❌ "Could a new write accidentally produce a row with `relation = NULL`?"
  No: `insert_evidence_attachment` rejects with `MissingEvidenceRelation`.
- ❌ "Could the silent `_ => Adhoc` fallback resurface?" No: `from_domain_tag`
  now returns `Result`, and the test that used to assert the fallback has
  been replaced with a test that asserts `Err(UnknownEvidenceKind)`.
- ❌ "Could a fresh storage fail to reopen an existing evidence payload?" No:
  M3 test asserts byte-equal content survives drop+reopen via CAS.
- ❌ "Could the legacy decoder break compat?" No: M3 test asserts that a row
  with `relation = NULL` is still decoded via `from_legacy_kind_tag`.

---

## 11. Unexpected findings

- **Two `chore(release):` commits triggered INC-A5-PUSH-RELEASE-MARKER-FRICTION**
  on the pre-push hook. Resolution: amended the second one into the bump
  commit as `chore(build): lock workspace graph for v1.169.85 release` —
  then amended the lock commit into the bump commit so the final push range
  `4597677..aa9952a` contains exactly one version transition.
- **Workspace `target-dir` is redirected to `/var/home/rubentxu/cargo-targets`**
  via `~/.cargo/config.toml`. `cargo build --release` reports "Finished" but
  the binary is at `/var/home/rubentxu/cargo-targets/release/sddk`, not
  `target/release/sddk`. Verified via `sddk --version` returning
  `sddk 1.169.85`.

---

## 12. STOP conditions not triggered

No authority / semantic / persistence / public contract / change-budget
changes were made outside the MIGRATE_A5 scope. SQLite-concurrency,
A5-5 clean-machine, and A5-C BASE_PRODUCTION_READY are POST-BASE concerns and
are NOT auto-opened per the cycle's scope contract.
