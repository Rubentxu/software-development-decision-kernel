# A5 — Debt Disposition

> Cycle: `p-63676b11dc0ef88f/a5-c-rr2-risk-gate-debt-reconciliation`
> Status: **RECONCILED at v1.169.84** (`b4acbbf`) — R1, R12 closures
> propagated from certified per-cycle receipts into §3.4 / §3.4.2.
> Last live update: A5-C-RR2 (v1.169.84) — closed-receipt
> cross-references added; historical §3.4.0 preserved.
> Last A5-cycle update: A5-5 (v1.169.86) — clean-machine UAT (UAT-1)
>   10/10 scenarios PASS on isolated container. R8, R15 → CLOSED_A5;
>   R11 → CLOSED_A5 (broader flake-discipline sweep done).

Disposition vocabulary (§3):

`BLOCKS_BASE_PRODUCTION_READY` · `MUST_CLOSE_A5` · `MIGRATE_A5` ·
`ACCEPTED_RISK` · `DEFER_POST_BASE` · `OBSOLETE` · `ALREADY_CLOSED`

`open` does **not** mean blocker. Each disposition carries evidence.

## §3.1 Inherited from A4

| Item | What it is | Evidence | Disposition | Owner |
|---|---|---|---|---|
| `FU-A3-CO-1` | Relation payload encoding convention | `docs/debt/` + followups ledger; A4-4 closed the lens shape, encoding issue independent | **MUST_CLOSE_A5** (P2) | A5-4 |
| `FU-A3-CO-3` | Remaining rename / shape cleanup (post-CO-2) | followups ledger | **MUST_CLOSE_A5** (P3) | A5-4 |
| `FU-A3-S15-4` | Fitness rule / CLI lint / doctor conversion | followups ledger | **MUST_CLOSE_A5** (P2) → design in A5-4 | A5-4 |
| `ASC-MA-1` | `sddk --help` UX pass | followups ledger | **MUST_CLOSE_A5** (P3) | A5-4 |
| `uat_stale_tests::stale_detects_geometry_change` | flaky test | followups ledger (one occurrence, passed on isolated re-run) | **CLOSED_A5** (A5-5R + A5-5; clean-machine UAT validates) | A5-5 ✓ |
| `INC-A4-RELEASE-VERSION-DRIFT` | pre-bump vs release-tag confusion | `docs/debt/INC-A4-RELEASE-VERSION-DRIFT.md` (open, low); release.sh 1c+9b already mitigate | **MUST_CLOSE_A5** (P1, release governance) | A5-1 |
| `paradigm_lens::evaluate_lens()` | LEGACY_READ_COMPAT facade, NO_RUNTIME_CONSUMER | `A4-MILESTONE-RECEIPT.md` §4 → **`A5-4a-RECEIPT.md` CLOSED / DELETED** | **CLOSED_A5** (A5-4a) | A5-4a ✓ |
| `paradigm_lens::LensEvaluation` | Obsolete wrapper around `LensAssessment` | **`A5-4a-RECEIPT.md` OBSOLETE → DELETE; receipt composer field narrowed to `&[LensAssessment]`** | **CLOSED_A5** (A5-4a) | A5-4a ✓ |
| `INC-A5-PUSH-RELEASE-MARKER-FRICTION` (new) | ceremonial empty `chore(release)` marker | `A5-PUSH-CONTRACT-INVESTIGATION.md` | **CLOSED_A5** (A5-5; clean-machine UAT exercises rollback path) | A5-5 ✓ |

## §3.2 Live debt (`docs/debt/`)

- 47 files; the great majority `status: closed`.
- **Open:** `INC-A4-RELEASE-VERSION-DRIFT` (low), `INC-DEBT-023`
  (lints advisory, low). `INCIDENCE-TEMPLATE.md` is a template.
- `INC-HX-AUTH-001/003/004` are `status: closed` (they were `open` at an
  earlier audit; the ledger shows closure).
- **Disposition:** open low-severity records → **MUST_CLOSE_A5** if cheap,
  else **ACCEPTED_RISK** with reason. No open debt record is a
  `BASE_PRODUCTION_READY` blocker today.

## §3.3 Deprecated-pattern lints (9)

| Lint | Default | Disposition |
|---|---|---|
| `agent_result_used` | deny | **ALREADY_CLOSED** (0 hits; keep deny) |
| `evidence_kind_v1` | deny | **ALREADY_CLOSED** (0 hits after exclude_paths) |
| `orchestration_synthesis_no_dissent` | deny | **ALREADY_CLOSED** (0 hits) |
| `asset_deprecated_namespace` | deny | **ALREADY_CLOSED** (0 hits) |
| `asset_raw_store_reference` | deny | **ALREADY_CLOSED** (0 hits) |
| `asset_authority_language` | deny | **ALREADY_CLOSED** (0 hits) |
| `execution_outcome_as_synthesis` | allow | **KEEP_ALLOW_WITH_REASON** (medium-confidence regex; false-positive risk) — revisit if corpus grows |
| `transition_outcome_used` | allow | **KEEP_ALLOW_WITH_REASON** (24 legitimate uses; re-categorized `state_machine`) |
| `asset_unregistered_cli_example` | allow | **KEEP_ALLOW_WITH_REASON** (regex unsafe-by-design) |

**Disposition:** A5-4 must re-justify each `allow` (one machine-readable
source → doctor/release presentations), and either promote or keep with
reason. No lint is a blocker today.

## §3.4 Ignored tests

> **A5-C-RR2 reconciliation note (v1.169.84 → `b4acbbf`):**
> The original §3.4 listed 14 ignored tests and called out R1 / R12
> as P1 unresolved. **Both are CLOSED by certified per-cycle
> receipts**, and **the historical 14-row table is no longer
> current**. The live baseline at v1.169.84 has **11 ignored**
> tests (per `cargo test --workspace`); per-test disposition
> lives in §3.4.1 (live) below. The historical §3.4 table is
> preserved for context; **the live authority is §3.4.1**.

### §3.4.0 Historical table (preserved for context)

| Test | Documented reason | Original disposition |
|---|---|---|
| `cli_phase_build_remediate_rejects_wrong_phase` | "workflow has no transition into REMEDIATING/verify; see cycle-45" | **MUST_CLOSE_A5** or **OBSOLETE** (decide) |
| `regenerate_uat_acceptance_jsonl` | "regenerates uat-acceptance.jsonl; run manually" | **ACCEPTED_RISK** (intentional manual tool) |
| `knowledge::tests::s1_does_not_introduce_new_corenodekind_variants` | "historical anchor only — A3-S2 grew the enums by design" | **OBSOLETE** (delete or re-anchor) |
| `parallel_wfr4_par_006_a_namespaced_count_is_n_plus_one` | "sender-drop bug in non-blocking Parallel path; ignored until IR setup fixed" | **MUST_CLOSE_A5** (P1 — possible runtime defect, risk R12) — **CLOSED A5-3** |
| `parallel_wfr4_par_006_d_node_runs_v1_has_exactly_one_parent_plus_n_children` | same | **MUST_CLOSE_A5** (P1) — **CLOSED A5-3** |
| `dm02_stress_harness` | "T1 diagnostic harness — run manually" | **ACCEPTED_RISK** (manual harness) |
| `verify_stream_chain_fails_on_tampered_hash` | "Tampering requires trigger bypass; covered by SDDK2-203" | **MUST_CLOSE_A5** or **OBSOLETE** (SDDK2-203 unreachable → re-target) |
| `run_survives_restart_with_equivalent_identity_and_provenance` | "pre-existing v1.89.1 debt; DW-RUNTIME-003 follow-up" | **MUST_CLOSE_A5** (P1 — durability/restart is G2/G15) — **CLOSED A5-2** |
| 6 × doc-tests (domain ×4, engine ×1, storage ×1) | rustdoc ignored examples | **ACCEPTED_RISK** (documentation examples) |

**Disposition (historical):** every ignored test was individually
dispositioned (none invisible). Two pairs historically pointed at
**runtime** signals (Parallel sender-drop, restart survival) and
were promoted to P1 risks R12/R1 — both now **CLOSED** by certified
per-cycle receipts (A5-3 / A5-2). See §3.4.1 for the live
disposition table.

### §3.4.1 A5-ITD dispositions (post-v1.169.83)

Two ignored tests disposed in `p-63676b11dc0ef88f/a5-itd-ignored-test-disposition`:

| Test | Original reason | Disposition | Evidence |
|---|---|---|---|
| `cli_phase_build_remediate_rejects_wrong_phase` (S2) | "workflow has no transition into REMEDIATING/verify; see cycle-45" | **OBSOLETE** | `REMEDIATING/verify` is a state-machine phantom: 0 references in YAML; the workflow only knows `REMEDIATING/build` (`release.recover`). The property the test wanted to prove (source-state mismatch rejected) is enforced at the canonical lower frontier: `cli_phase_build_remediate_transitions_to_open_build` (S1, green) traverses the same source-state check on every successful transition. Test removed; docstring at the deletion site points to S1 as live coverage. Receipt: `A5-ITD-RECEIPT.md §3.1`. |
| `verify_stream_chain_fails_on_tampered_hash` | "Tampering requires trigger bypass; covered by SDDK2-203" | **OBSOLETE** | Test body is empty (placeholder). The tampering path (mutating `events_v1.content_hash`) is not a possible system entry: the SQL trigger on `events_v1` blocks UPDATE and DELETE, demonstrated by `append_rejects_update_via_trigger` and `append_rejects_delete_via_trigger` (both 2/2 green at v1.169.83). The "tampering is rejected" property is enforced at the canonical lower frontier (the SQL trigger), not at `verify_stream_chain`. Test removed. Receipt: `A5-ITD-RECEIPT.md §3.2`. |

Post-A5-ITD baseline: 11 ignored tests (was 13; -2 from these
OBSOLETE dispositions). Both runtime signals that originally
pointed at R1 and R12 are now **CLOSED** by certified
per-cycle receipts (A5-2 for R1, A5-3 for R12) — see
`docs/architecture/a5/A5-RISK-REGISTER.md §1`. The 6 doc-tests
remain `ACCEPTED_RISK`.

### §3.4.2 R1 / R12 closure addendum (A5-C-RR2)

Cross-reference for any future audit that finds the historical
"remain P1 unresolved" wording above:

| Risk | Closing cycle | Closing commit | Live evidence | Why it's closed (one line) |
|---|---|---|---|---|
| **R1** (restart survival) | A5-2 / v1.169.74 | `957d0b0` | `crates/sddk-engine/src/sqlite_graph_store.rs::latest_run_state_for` + 3 tests in `crates/sddk-engine/tests/state_survives_restart.rs` | Canonical event log is the authority for `load_run.state`; the previously-ignored test was *corrected* (not merely un-ignored) to assert the right post-restart state |
| **R12** (non-blocking Parallel sender-drop) | A5-3 / v1.169.75 | `af346b9` | `crates/sddk-engine/src/operator.rs:1197-1205` (forces `pending_sender = None`) | Non-blocking path deleted (operator.rs:1196-1356, ~162 lines); 2 ignored `par_006_*` tests deleted; `parallel_spans_three_ticks_drain` deleted |

The closed receipts (`A5-2-RECEIPT.md`, `A5-3-RECEIPT.md`) remain
verbatim — no silent rewriting. The historical §3.4.0 rows above
keep the original disposition; the **CLOSED A5-2** / **CLOSED A5-3**
suffix is the addendum.

A future async/non-blocking Parallel is a **POST-BASE feature**,
NOT a closure of R12. See `A5-RISK-REGISTER.md §3`.

## §3.5 Legacy / compatibility surfaces

| Surface | Evidence | Disposition |
|---|---|---|
| `paradigm_lens::evaluate_lens()` | `A4-MILESTONE-RECEIPT.md` §4 → **`A5-4a-RECEIPT.md`** | **CLOSED_A5** (A5-4a) — DELETED |
| `paradigm_lens::LensEvaluation` type | used by `architecture_receipt` (A3 receipt) → **narrowed to `&[LensAssessment]`** | **CLOSED_A5** (A5-4a) — DELETED |
| `EvidenceAttachmentV1` + compat decoder | `evidence_relation_mapping.rs`; migration deferred | **CLOSED_A5** (v1.169.85) — see `A5-EVIDENCE-ATTACHMENT-MIGRATION-V1-RECEIPT.md`; READ decoder retained as documented compat, no new authority minted |
| old namespaces / schemas in assets | guarded by `asset_*` lints (deny) | **ALREADY_CLOSED** (lint-enforced) |

## §3.6 Post-BASE (not A5)

- `FU-A3-CO-1`/`CO-3`/`S15-4`/`ASC-MA-1` are A5 **only** because they are
  cheap and unblock operator/HX quality. Anything that becomes a *feature*
  is **DEFER_POST_BASE** (see `A5-DEFERRED-POST-BASE.md`).

## §3.8 SQLite concurrency authority surface (planning substrate)

| Surface | Evidence | Disposition |
|---|---|---|
| `Storage::insert_evidence_attachment` (lib.rs:2667) CAS-oracle orphan on UNIQUE collision | `M3` in spec; pre-cycle `cas_put` precedes SQL INSERT with no rollback | **CLOSED_A5** (v1.169.86) — see `A5-SQLITE-CONCURRENCY-R-RECEIPT.md` §3 |
| Gate-receipt concurrent_allocations flake (`storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`, sqlite_storage.rs:738) | pre-cycle 1/5 reproductions under `--test-threads=4` | **CLOSED_A5** (v1.169.86) — pinned 50/50 via `with_busy_retry` helper (M2, `lib.rs:1471`) |
| Planning substrate concurrency coverage gap (0 multi-thread tests on `work_items_v1` / `work_item_dependencies_v1` / `evidence_attachments_v1` / `decision_records_v1`) | pre-cycle inventory by explore envelope `8585666997…6e11c8fc0875e8da1c52c667d05a8aa576bb` | **CLOSED_A5** (v1.169.86) — 5 new tests in `tests/concurrency_planning_substrate.rs` |
| Pre-existing flake `cross_surface_facades_share_the_service_instance` in `sddk-cli` | reproducible with `git stash` of this cycle's commits | **OPEN_NON_BLOCKER** — different crate (`sddk-cli`), different surface (cross-service wiring); recorded for a future dedicated cycle |
| 8 IMMEDIATE sites not routed through `with_busy_retry` | `lib.rs:504, 607, 1039, 1140, 1233, 1335, 1391, 1538` | **DEFER_POST_BASE** — each surface warrants its own change-set; the helper is available and pinned for opportunistic adoption |
| `update_cycle_with_event` (lib.rs:664) split-transaction design | intentional (event-first fail-closed) | **DEFERRED** (OQ-SQLITE-1, P3 future cycle) |
| CAS root GC sweep for orphans | pre-cycle no GC; `DEFAULT_CAS_ROOT = "cas/"` | **DEFERRED** (OQ-SQLITE-2, P3 future cycle) |

## §3.7 A5-4b dispositions (v1.169.83)

> Cycle: `p-63676b11dc0ef88f/a5-4b-bounded-compat-lints-operator-ux`
> Released: `v1.169.83`
> Receipt: `docs/architecture/a5/A5-4b-RECEIPT.md`
> Test pins: `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs`

A5-4b closes 4 MUST_CLOSE_A5 items, pins 3 advisory lints as
`KEEP_ALLOW_WITH_REASON`, and carries 1 forward to a separate slice.

### MUST_CLOSE_A5 items disposed

| Item | Disposition | Evidence |
|---|---|---|
| `FU-A3-CO-1` (relation payload encoding) | **KEEP_WITH_REASON** | `KnowledgePayload::Relation` exists in `crates/sddk-knowledge/src/...` with zero production writers; A4-4 already closed the lens-shape concern independently. No alternate encoding to close. Recorded here so a future cycle does not re-open it as orphaned work. |
| `FU-A3-CO-3` (rename/shape cleanup) | **CLOSED_BY_PRIOR_WORK** | `SpecifiedBy` is the one canonical relation post-A4-S15R (v1.169.65); `VerifiedBy` was repointed to `EvidenceRef` in the same cycle. No residual rename/shape to perform. |
| `FU-A3-S15-4` (fitness rule / CLI lint / doctor conversion) | **CLOSED_BY_PRIOR_WORK** | TOML-driven lint at `docs/architecture/lints/deprecated_patterns.toml` + implementation in `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs`. Zero duplicate implementations across lint / doctor / release (one source, many presentations). |
| `ASC-MA-1` (`sddk --help` UX pass) | **CLOSE_BY_HELP** | `crates/sddk-cli/src/lib.rs:189` about-line changed from `"First-class commands: status, plan, run, ship, recover, memory"` (5/6 M6.1 legacy facades) to `"uses \`sddk agent-help\` for the operator-facing surface"`. Two pin tests verify BOTH the absence of the legacy substring AND the presence of the `sddk agent-help` substring (`crates/sddk-cli/tests/first_class_commands.rs` and `crates/sddk-cli/tests/cli_first_class_help.rs`). Golden fixtures regenerated: `docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-help.txt` (54→58 lines) and `crates/sddk-cli/tests/fixtures/cli/help-top-level.txt` (byte-comparison stderr snapshot, 4148 bytes). |

### Advisory lints (`default = "allow"`) dispositioned as KEEP_ALLOW_WITH_REASON

| Lint | Hits | Reason |
|---|---|---|
| `execution_outcome_as_synthesis` | 0 | audit-cleared; corpus expansion organic; registry carries `explanation = """ ... """` body > 10 tokens. |
| `transition_outcome_used` | 24 | state-machine regression guard, M9.2 closed; registry explanation body > 10 tokens. |
| `asset_unregistered_cli_example` | 0 | regex unsafe-by-design (CLI examples do not always carry asset registry IDs); AX-S1 pin; explanation body > 10 tokens. |

These three remain `allow` because none of them satisfies the
ADR-0001 §3.2 acceptance gates (deterministic detector, false-positive
audit, migration complete, 0 illegitimate hits). The promotion
denial is **machine-pinned** by
`crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs`:

* `a5_4b_allow_lints_have_default_allow_and_non_empty_explanation`
  walks the registry and asserts each allow entry has
  `default = "allow"` plus a registry-pinned explanation body
  of >10 non-whitespace tokens.
* `a5_4b_no_new_allow_lints_added_silently` enforces the
  invariant `allow_blocks_in_registry == ALLOW_LINTS.len()`. If
  a future cycle adds a 4th allow lint without updating this
  pin, the test fails immediately.
* `a5_4b_denied_lints_have_zero_illegitimate_hits_at_v1_169_83`
  freezes the deny-lint corridor.

If any of these three should be promoted in the future, the path
is: add the lint ID to the deny-side registry, run the corpus
audit, and update `ALLOW_LINTS` here.

### Carry-forward (not A5-4b scope)

| Item | Disposition |
|---|---|
| `EvidenceAttachmentV1` + compat decoder | **STOP_NEEDS_SEPARATE_SLICE** |

The compat decoder in `crates/sddk-engine/src/.../evidence_ref.rs:192`
plus ratchet exclude plus the storage migration are **C2.5** risk
class. Anti-encroachment for A5-4b prohibits EvidencePosture, Verify,
DebVerify, Alignment, IntelligenceLoop, AdvisoryWhy, and UniversalConcern
semantics changes. The compat decoder is exactly that — a migration
of how attachments are stored — and a single-risk A5-4b cannot
absorb it. See `A5-DEFERRED-POST-BASE.md` for the follow-up.

### What A5-4b did NOT touch

* No `EvidencePosture`, `Verify`, `DebVerify`, `Alignment`,
  `IntelligenceLoop`, `AdvisoryWhy`, or `UniversalConcern`
  semantic-freeze changes.
* No Authority / providers / M0–M9 restructuring.
* No security / secrets.
* No resurrection of deleted symbols (`evaluate_lens` is gone;
  pin tests assert both absence of the legacy substring AND
  presence of the new `sddk agent-help` substring).

## Summary

- **Blockers today:** none (0 undisposed P1). Two P1s are *pre-registered*
  from ignored tests (R1/R12) and must be resolved in A5.
- **MUST_CLOSE_A5:** the 8 inherited items + the 2 ignored-test P1s + the
  new push-friction INC — of which 4 (`FU-A3-CO-1`, `FU-A3-CO-3`,
  `FU-A3-S15-4`, `ASC-MA-1`) are disposed by A5-4b (v1.169.83).
- **MIGRATE_A5:** `EvidenceAttachmentV1` + compat decoder carried forward
  to a separate slice (STOP_NEEDS_SEPARATE_SLICE); the other compat
  surface (`paradigm_lens::evaluate_lens`) is CLOSED in A5-4a.
- **ACCEPTED_RISK:** manual harnesses, doc-tests, and the 3 `allow` lints
  (with reasons — registry-pinned and machine-enforced by
  `a5_4b_lint_disposition_pin.rs`).


