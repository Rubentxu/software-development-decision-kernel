# A5-3 Receipt — Concurrency / CAS / authority side-effect races

> Cycle: `p-63676b11dc0ef88f/a5-3-concurrency-cas-authority-races`
> Gates evidence produced:
> - G4 (concurrency correctness) — R3 closed; R6 closed.
> - G5 (authority fail-closed) — R5 closed; R4 partial (R4-A closed; R4-B open, see INC-R4).
> Risks touched: **R3 closed**, **R5 closed**, **R6 closed**, **R12 closed**, **R4 partial** (R4-A closed; R4-B opened in INC-R4).
> Hard constraint honoured: **no new abstraction**.

## Identity (recorded separately)

| Field | Value |
|---|---|
| project_id | `p-63676b11dc0ef88f` |
| workspace_id | `w-2e7853aadc28217a6649e309` |
| cycle HEAD at receipt | `6a60f83` |
| release HEAD | `6a60f83` (v1.169.75) |
| binary sha256 | `76a22e83b5ea4194747fbcd5a64c7b531bc3dde1cc59a79a638ea134b9b61583` |
| cycle commits | `28abee3` (R3+R6 fix + tests + plan), `af346b9` (R12 deletion + restart regression fix), `bd8d9cc` (R5 tests + R4 incident), `bdd16a7` (receipt + finding), `6a60f83` (release bump) |
| remote tag | `v1.169.75` → `6a60f834e6fa4d88d9c5e534cc21437754a67031` |
| install path | `~/.local/bin/sddk` (= `/home/rubentxu/.local/bin/sddk`) → v1.169.75 |
| doctor | `binary.bundle_coherence: present`, `all_present: true` |
| release pipeline | 14/14 PASS (steps 0–13, including 9b public-release gate, 10 install from URL, 11 doctor, 12 prune, 13 distrib round-trip) |

## §0 Falsification first (RED → GREEN)

| # | Defect (what it really was) | RED (observed) | GREEN (observed) |
|---|---|---|---|
| 1 | **`record_node_run_for_run` used `INSERT OR REPLACE`, which silently overwrote any existing row without tripping the `node_runs_v1_no_delete` append-only trigger. Two writers on the same `(run_id, node_id)` pair would observe last-writer-wins.** | Pre-fix `a5_3_r3_first_writer_wins_second_writer_typed_conflict_on_node_run`: a second write returned `Ok` instead of a typed `IdempotencyConflict`. | Post-fix: typed `Err(StorageError::IdempotencyConflict { key })`. |
| 2 | **Concurrent writers on the same node_id: `INSERT OR REPLACE` lost one of the writes silently.** | Pre-fix `a5_3_r3_concurrent_writers_serialize_through_append_only_substrate`: spawning 8 writers on `node-shared` produced 8 rows claiming to be "first", last-write-wins. | Post-fix: exactly 1 row with `attempt_seq=1`, all-but-one writers receive `IdempotencyConflict`. |
| 3 | **An identical retry of a CAS attempt must be a no-op (idempotent). A divergent retry (different payload, same key) must surface a typed conflict.** | `a5_3_r6_identical_retry_returns_ok_no_double_apply` and `a5_3_r6_divergent_retry_returns_typed_idempotency_conflict` both depend on the same path. | Both GREEN. |
| 4 | **Two ignored tests in `parallel_spec_scenarios.rs` (lines 864, 984) test dead code: the non-blocking Parallel path is already side-stepped at `workflow_runtime.rs:1079` ("Delta-4: Parallel operators now use the blocking path").** | Reading the IR (`make_ir()` returns 0 operators) + the runtime comment made it impossible to un-ignore by fixing the IR alone. | Deleted the dead path (operator.rs:1196-1356, 162 lines) + the 2 ignored tests + 1 redundant `parallel_spans_three_ticks_drain` test in `parallel_concurrency_tests.rs`. |
| 5 | **Deny / RequireApproval must short-circuit `DagExecutor` before any handler is dispatched — zero side effects.** | Reading the executor (`executor.rs:209-217` for Deny, `188-208` for RequireApproval) showed the path was already correct, but no test pinned it. | New `authority_fail_closed.rs`: 6 tests, all GREEN. |

Falsification note for defect 1: A5-3's first attempt was to keep `INSERT OR REPLACE` and add an in-Python-style "compare-and-swap" guard around it. The substrate rejects that approach because it cannot atomically compare-then-swap in user-space — the CAS must move into the database (the `UNIQUE` constraint on `(run_id, node_id)`). The fix is the substrate-native one: `INSERT` + handle `UNIQUE` as `IdempotencyConflict`. The substrate defended itself.

## §1 What landed

| Surface | Change | Rationale |
|---|---|---|
| `crates/sddk-storage/src/graph_store.rs::record_node_run_for_run` | `INSERT OR REPLACE` → `INSERT`; map `ConstraintViolation` on `node_runs_v1_node_id_run_id_unique` to `Err(StorageError::IdempotencyConflict { key, attempt_seq: 0 })`. | R3: silent overwrite is gone. |
| `crates/sddk-storage/tests/concurrency_record_attempt.rs` (new, 4 tests) | r3 first-writer-wins, r3 concurrent writers, r6 identical retry, r6 divergent retry. | R3/R6 pinning. |
| `crates/sddk-engine/src/operator.rs` | Non-blocking Parallel supervisor branch deleted (~162 lines, 1196-1356). Replaced with explanatory comment naming Delta-4 + deletion rationale. | R12: dead code removal. |
| `crates/sddk-engine/src/workflow_runtime.rs` | (a) Delta-4 comment updated to reference A5-3 R12. (b) `apply_outcomes_to_state`: `Err(StorageError::IdempotencyConflict)` from `record_node_run_for_run` is now a safe no-op (mirrors the existing `record_attempt` pattern in operator.rs:925). | R12: production-boundary pinning. R3 regression fix: Sequence evaluates step-by-step; without the no-op handler, the 2nd+ tick on the seq-root panics. |
| `crates/sddk-engine/tests/parallel_spec_scenarios.rs` | 2 ignored tests deleted (par_006_a, par_006_d). S-PAR-007a test-count assertion 23 → 21. | R12: ignored count 14 → 12. |
| `crates/sddk-engine/tests/parallel_concurrency_tests.rs` | `parallel_spans_three_ticks_drain` test deleted. Obsolete `BTreeMap` + `RunId` imports dropped. | R12. |
| `crates/sddk-engine/tests/authority_fail_closed.rs` (new, 6 tests) | Deny short-circuits walk (zero executions, downstream skipped), RequireApproval short-circuits walk, --allow-high-band promotion sanity, system-actor Write allowed, user-actor Write requires approval, user-actor Read admitted. | R5 pinning. |
| `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` (new) | R4-B (decide-then-act TOCTOU) opened as P2 follow-up. | R4 partial closure. |
| `docs/architecture/a5/A5-3-PLAN.md` (new) | Scope budget + R12 disposition. | Audit trail. |

## §2 Gate evidence

| Gate / Risk | Evidence | Class |
|---|---|---|
| **G4 (concurrency correctness) — R3** | `concurrency_record_attempt::a5_3_r3_*` 2/2 GREEN. | OBSERVED |
| **G4 — R6** | `concurrency_record_attempt::a5_3_r6_*` 2/2 GREEN. | OBSERVED |
| **G5 (authority fail-closed) — R5** | `authority_fail_closed::*` 6/6 GREEN. | OBSERVED |
| **G5 — R4 partial** | R4-A closed (overlaps R5, pinned). R4-B opened as `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` with closure criteria. | DOCUMENTED |
| **R12 dead-code removal** | non-blocking path deleted; ignored count 14 → 12; `parallel_spans_three_ticks_drain` removed. | OBSERVED |
| **Restart regression caught + fixed** | `restart_survival_end_to_end` 3/3 GREEN after `apply_outcomes_to_state` handles `IdempotencyConflict` as no-op. | OBSERVED |
| **Full engine regression sweep** | `parallel_concurrency_tests` 21/21; `parallel_spec_scenarios` 22/22; `restart_survival_end_to_end` 3/3; `authority_fail_closed` 6/6. | OBSERVED |
| **Full storage regression sweep** | `sqlite_storage` 35/35; `concurrency_record_attempt` 4/4. | OBSERVED |
| **clippy** | `cargo clippy --workspace --all-targets -- -D warnings` → 0 errors. | OBSERVED |

## §3 Findings discovered by running the cycle

1. **`INSERT OR REPLACE` was hiding a substrate bypass.** The trigger `node_runs_v1_no_delete` enforces append-only at the SQL boundary. `INSERT OR REPLACE` was a workaround in the call site that turned the trigger into a no-op. **Disposition**: closed by changing the call site to `INSERT` + typed conflict. **Cluster**: `CL-CAS-SUBSTRATE-BYPASS`. **Finding**: `INC-FINDING-A5-3-DELTA-4-NON-BLOCKING-PATH-ABANDONED` (medium/P3, this receipt).
2. **The non-blocking Parallel path was abandoned in Delta-4 but the code remained.** Two ignored tests + the dead path are now gone. **Cluster**: `CL-DEAD-CODE-IGNORED-TESTS`. **Disposition**: deletion is the resolution; the next cycle that needs non-blocking semantics must build it intentionally.
3. **R4 (decide-then-act TOCTOU) was conflated with R5 (denied ⇒ no effect).** R5 is closed; R4-B (the decide-then-act half) is real and unmitigated. **Disposition**: open `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY` (medium/P2) for a future cycle outside A5.
4. **Sequence::evaluate produces Running per child per tick.** This means the runtime calls `record_node_run_for_run(seq-root, Running)` repeatedly until all children complete. With the R3 fix, the 2nd+ call returns `IdempotencyConflict`. The runtime must treat that as a safe no-op — same pattern that already existed for `record_attempt`. **Disposition**: runtime-side fix at `apply_outcomes_to_state`. **Cluster**: `CL-IDEMPOTENT-WRITE-CONFLICT-HANDLING`.

## §4 Honesty markers

- The ignored count went **14 → 12** (this cycle). 2 ignored tests removed from `parallel_spec_scenarios`, 1 from `parallel_concurrency_tests`. The remaining 12 are documented in A5-2-RECEIPT (chain-verify, ENVELOPE_GOLDEN manual harness, etc.).
- R4 is **partially closed**, not closed. The honest statement is "R4-A closed; R4-B open". A future cycle must choose between Option A (transactional side effects), B (fenced admission tickets), or C (high-band-only migration) and ship one.
- No commit fixes a Rust warning without a co-located test. No `git.history_rewrite` rewrites.
- The R3 fix surfaced a real substrate bypass that A5-1 / A5-2 did not catch. The fix is small (5 lines + 4 tests) because the substrate was already correct; the call site was the bug.

## §5 Hard non-goals still honoured

- No new crate / trait / port. (A5-3 inherits the constraint from A5.)
- No new AdmissionTicket / FencedAdmission substrate — that is the R4-B follow-up's job, and it lives outside A5.
- No rewriting of pre-A5 commit history.

## §6 Release pipeline (14/14 PASS)

| # | Step | Gate / verifier | Result |
|---|---|---|---|
| 0 | Preflight | `gh auth status`, branch `main`, tree clean, HEAD `chore(release): bump version`, `jq` on PATH | PASS |
| 1 | Workspace green | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` | PASS |
| 2 | Read version | `Cargo.toml` `[workspace.package] version` = `1.169.75` | PASS |
| 3 | Build binary | `cargo build --release --bin sddk` | PASS |
| 4 | Manifest | `sddk dev manifest --root .` + `--verify` (RDI) | PASS |
| 5 | Bundle tarball | `tar czf` with prefix `software-development-decision-kernel/` | PASS |
| 6 | BUNDLE.toml (v2) | `schema_version=2`, manifest_sha256 pinned | PASS |
| 7 | Unified tarball | `bin/sddk` + `framework/`, `chmod 0755` defensivo | PASS |
| 8 | sha256 + CHECKSUMS + sbom | CycloneDX 1.5 | PASS |
| 9 | `gh release create` | 9 assets in one command | PASS |
| 9b | Public-release gate | tag SHA anchored via `git ls-remote origin $TAG`, `isDraft=false`, `isPrerelease=false`, all 6 HTTP-download URLs return 200, binary SHA `76a22e83...` matches the in-tarball binary | PASS |
| 10 | Install from URL real | `bash scripts/install.sh --version v1.169.75 --editor none` | PASS |
| 11 | `sddk dev doctor` | `binary.bundle_coherence: present, all_present: true` | PASS |
| 12 | `sddk dev update --prune-only --keep 1` | removed 1.169.74, kept 1.169.75 | PASS |
| 13 | Distrib smoke test | re-install from URL after prune; binary reports 1.169.75; bundle coherent | PASS |
| 14 | Final state | `binary: sddk 1.169.75`, `bundle: 1.169.75`, `current -> 1.169.75` | PASS |

## §7 What this cycle leaves for the next one

- **R4-B (decide-then-act TOCTOU) is open.** `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY` (medium/P2) is the durable reminder. A future cycle outside A5 must:
  - Choose between Option A (transactional side effects), Option B (fenced admission tickets), Option C (high-band-only migration).
  - Add a substrate primitive if Option B is chosen — which is the only one that fits the existing architecture.
  - Migrate High-band unguarded surfaces (`framework_bundle`, `github_releases`) first; pin the "Allow ticket still Allow at effect time" contract with integration tests.
- **Ignored count: 12.** The remaining `#[ignore]` tests are documented in `A5-2-RECEIPT` and `A5-3-RECEIPT` (chain-verify, ENVELOPE_GOLDEN manual harness, etc.).
- **Recovered `parallel_spans_three_ticks_drain` is gone forever.** If a future cycle needs non-blocking Parallel semantics, it MUST rebuild intentionally.

## §8 Cargo.lock propagation (follow-up)

The `chore(release): bump version 1.169.74 -> 1.169.75` commit (`6a60f83`) updated `Cargo.toml` but did not propagate the version bump to `Cargo.lock`. The propagation landed via this follow-up commit (paired with this docs paragraph per the pre-push hook's allowlist rules — Cargo.lock alone is not authority; a paired docs change is). The release binary itself was unaffected (`sddk` package version is read from `Cargo.toml` at build time, not from `Cargo.lock`).
