# A5-2 Receipt — Durability / rebuild / recovery

> Cycle: `p-63676b11dc0ef88f/a5-2-durability-rebuild-recovery`
> Gates evidence produced:
> - G2 (Durability / crash recovery) — R1 closed; canonical event log becomes authority for `load_run.state`.
> - G3 (Rebuildability) — R1 closure requires only the rowid-monotonic event log; same canonical inputs (record + transition events) rebuild the same `WorkflowRun` read.
> - G6 (Migration / compatibility) — R20 re-baselined; append-only triggers still hold.
> Risks touched: **R1 closed**, **R2 closed**, **R20 re-pinned**.
> Hard constraint honoured: **no new abstraction**.

## Identity (recorded separately)

| Field | Value |
|---|---|
| project_id | `p-63676b11dc0ef88f` |
| workspace_id | `w-2e7853aadc28217a6649e309` |
| cycle HEAD at receipt | `75da372` |
| release HEAD | `31349ab` (v1.169.74) |
| binary sha256 | `0dd9ddc5bf369cb55789031bf02ff30b4c0bdfa0f527223e6ac5dd300d550ec0` |
| cycle commits | `957d0b0` (R1 fix), `7ab113a` (R2 fix), `75da372` (test clippy), `e0cc797` (plan), `06b8937` (prior cycle baseline), `a53b650` (receipt + finding), `31349ab` (release bump) |
| remote tag | `v1.169.74` → `31349ab48cec31daf98569e86c399d31d8bd9290` |
| install path | `~/.local/bin/sddk` (= `/var/home/rubentxu/.local/bin/sddk`) → v1.169.74 |
| doctor | `binary.bundle_coherence: present`, `all_present: true` |
| release pipeline | 14/14 PASS (steps 0–13, including 9b public-release gate, 10 install from URL, 11 doctor, 12 prune, 13 distrib round-trip) |

## §0 Falsification first (RED → GREEN)

| # | Defect (what it really was) | RED (observed) | GREEN (observed) |
|---|---|---|---|
| 1 | **`workflow_runs_v1.state` (snapshot row) was read directly by `load_run`, but only `record_run` wrote it — `record_workflow_run_transition` only wrote the event log. After any transition, `load_run.state` was stale at the initial state.** Pre-fix force of `a5_2_r1_public_state_survives_restart_after_running_transition`: panics with `left: Pending, right: Running` at `crates/sddk-storage/tests/state_survives_restart.rs:142`. | RED captured before any code change. |
| 2 | **CAS `get()` returned whatever bytes happened to live at the path. A truncated / substituted blob was silently handed to the caller.** Pre-fix `a5_2_r2_truncated_blob_get_rejects_with_mismatch` returned `[104, 101, ...]` (the truncated bytes) instead of a typed error. | RED captured. |
| 3 | **The previously-ignored `run_survives_restart_with_equivalent_identity_and_provenance` was not actually RED when run. It was passing for the wrong reason: it asserted `loaded.state == Pending` after the test had already inserted a `running` event, which kept the snapshot stale — the exact bug.** | Reading the test surfaced the bug. |

Falsification note for defect 1: I first attempted an UPDATE within `record_workflow_run_transition`'s transaction. The SQLite triggers raised immediately:

```
thread 'a5_2_r1_chained_public_two_transitions_survive_restart' panicked at
crates/sddk-storage/tests/state_survives_restart.rs:224:14:
transition 1: Database("sync snapshot state: workflow_runs_v1 is append-only")
```

The substrate **rejected the path** I had planned. The fix only landed because the append-only trigger (re-exercised by `migration_append_only_tests::update_on_workflow_runs_v1_fails`) failed closed at test runtime, before I wrote a single line of production code. That's R20 defending R2 transitively.

## §1 What landed

| Surface | Change | Rationale |
|---|---|---|
| `crates/sddk-storage/src/graph_store.rs` | New inherent `SqliteGraphStore::latest_run_state_for(conn, run_id)` reads the canonical event log ordered by `rowid`. Reduced `latest_workflow_run_state` to a 3-line call site. `load_run` now consults the helper, falling back to the snapshot only when no event row exists (a path the substrate does not produce under normal operation). | R1: single source of truth for state; event log is append-only by trigger and durable across restart. |
| `crates/sddk-storage/src/cas.rs::FilesystemCas::get` | After `fs::read`, recompute SHA-256 and compare to the requested key. On mismatch return `CasError::HashMismatch` (already in the enum, never surfaced by any loader until now). | R2: "content addressed" means hash-to-content, not hash-to-path. |
| `crates/sddk-storage/tests/state_survives_restart.rs` (new, 3 tests) | Public-API restart survival, raw-SQL-event restart, chained transitions. | R1 / F1 + F3. |
| `crates/sddk-storage/tests/cas_partial_state.rs` (new, 4 tests) | Truncated blob, substituted bytes, unreadable path (dir in place of file), localised corruption does not poison other blobs. | R2 / F4. |
| `crates/sddk-storage/tests/workflow_run_restart_survival.rs` | Pre-existing ignored test rewritten: removes `#[ignore]`, drops the unrelated clock-skew comment, adds the missing `loaded.state == Running` assertion (the one the docstring said but never executed). | F1 — close the debt. |
| `docs/architecture/a5/A5-2-PLAN.md` | Plan revised in-flight after the substrate rejection in defect 1 RED. | Honest history of the path through option (a)/(b). |

## §2 Gate evidence

| Gate | Evidence | Class |
|---|---|---|
| **G2 — durability / crash recovery** | `state_survives_restart` 3/3; `restart_survival` 2/2 (including the previously-ignored test, now non-ignored). | OBSERVED |
| **G3 — rebuildability** | same canonical inputs (one `record_run` + N transitions on the event log) → identical `load_run` output across process drops. | OBSERVED |
| **G6 — migration / compatibility** | `migration_append_only_tests` 4/4 under `SQLX_OFFLINE=true`. The trigger `workflow_runs_v1_no_update` still aborts the UPDATE attempted in the first F2 attempt. | OBSERVED |
| **R2 closed** | `cas_partial_state` 4/4: truncated, substituted, unreadable path, localised corruption. | OBSERVED |
| **no new abstraction** | diff inspection: zero new trait / port / module. One inherent method, one verification branch in `get`. | OBSERVED |

## §3 Findings discovered by running the cycle

1. **DW-RUNTIME-003 was about clock-skew; the real bug was the snapshot/log split.** The comment that anchored the ignored test for v1.89.1 does not describe what made the test unreliable. The reliable, reproducible assertion was missing. **Disposition**: comment and `#[ignore]` removed; the test now asserts the right invariant. **Cluster**: `CL-DOC-QUALITY`. **Disposition action**: register as `INC-FINDING-A5-2-DW-RUNTIME-003-CLOCK-SKEW` low/P3 with resolution in the cycle (this receipt).
2. **Approach that the substrate defended against.** My first F2 attempt was an in-transaction UPDATE on `workflow_runs_v1`. The trigger made it impossible to ship. The only viable path was the helper route — which respects the substrate decision. **Cluster**: `CL-ARCHITECTURE-ENFORCEMENT`. **Disposition**: A5-2 explicitly honours the substrate (no migration, no new port).
3. **CAS was a write-side contract, not a read-side one.** `put()` verified, `get()` did not. The `HashMismatch` variant existed; no caller could produce it. F4 fixed the loader, not the contract. **Cluster**: `CL-STORAGE-LOADER-CORRECTNESS`. **Disposition**: closed.

## §4 Honesty markers

- The ignored count went **14 → 13** (this cycle). The R1 test left `#[ignore]`; the A5-2 plan documents the 13 still-ignored tests and their reason (sender-drop A5-3, ENVELOPE_GOLDEN harness, chain-verify etc.).
- No commit fixes a Rust warning without a co-located test that pins the contract. No "ceremonial" commits.
- No `git.history_rewrite` rewrites. The manifest drift and `Cargo.lock` of the previous (config-model-v1) cycle is unchanged; the receipt for that cycle recorded 1.169.72 as burned, not as published.

## §5 Hard non-goals still honoured

- No new crate / trait / port. (A5-2 inherits the constraint from A5.)
- No MIGRATION_18 (`event_sequence` column). Option (a) was never taken.
- No REMEDIATING transition design (A5-C).
- No sender-drop fix (A5-3).
- No `proptest` property tests in this cycle.
- No `cargo test --workspace` full run during inner loop; scoped to
  `sddk-storage` tests where the change lives. Profile-wide gate
  reserved for `verify` / release.

## §6 Next actionable cycles

- **A5-3** — concurrency / CAS / authority side-effect races (R12 sender-drop).
- **DW-RUNTIME-003 CL-DOC-QUALITY finding** resolved by this receipt; no further debt.
- **A5-C** — once A5-3 / A5-4 / A5-5 close.

## §7 No new production abstraction

One inherent method (`latest_run_state_for`). One new branch in the existing
CAS `get` impl. No new port, no new schema, no new migration.

## §8 Release evidence (post-merge, on the released tag)

The release was driven from the workspace CWD by `scripts/release.sh` after
the cycle commits were pushed. All 14 steps passed:

| # | Step | Result |
|---|------|--------|
| 0 | preflight (HEAD on `main`, clean tree, version bump commit `chore(release): bump version 1.169.73 -> 1.169.74`, ADR format, vault mirror present) | OK |
| 1 | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` | OK |
| 2 | read version `1.169.74` → tag `v1.169.74` | OK |
| 3 | `cargo build --release --bin sddk` | OK |
| 4 | manifest regenerated + verified (manifest_sha256 `608c6d9c…`) | OK |
| 5 | bundle tarball (`software-development-decision-kernel.tar.gz`, 669 132 bytes) | OK |
| 6 | `BUNDLE.toml` written with `schema_version=2` | OK |
| 7 | unified tarball (`sddk-v1.169.74-sddk-linux-x86_64-musl.tar.gz`, 12 007 275 bytes, exec bit + `BUNDLE.toml` OK) | OK |
| 8 | sha256 + `CHECKSUMS` + CycloneDX 1.5 `sbom.json` | OK |
| 8b | vault ADR mirror sync (36 ADRs, all already present) | OK |
| 9 | `gh release create v1.169.74` (target `main`, actor `system`) | OK |
| 9b | public-release gate: tag SHA anchored to `31349ab…`, `isDraft=false`, `isPrerelease=false`, 9-asset contract, **9/9 canonical assets reachable from public CDN (HTTP 200)** | OK |
| 10 | install from real GH Release URL (`bash scripts/install.sh --version v1.169.74 --editor all`) — CDN served correct binary sha256 after 10 s | OK |
| 11 | `sddk dev doctor --prefix ~/.local/bin` — `binary.bundle_coherence: present`, `all_present: true` | OK |
| 12 | `sddk dev update --prune-only --keep 1` | OK |
| 13 | re-install from URL (distrib round-trip smoke test) | OK |
| 14 | final state: `sddk 1.169.74`, bundle 1.169.74, current symlink → `~/.local/share/sddk/framework/1.169.74` | OK |

Receipt commit (this commit) lands on the released branch so the receipt
pointer and the release tag share the same history.
