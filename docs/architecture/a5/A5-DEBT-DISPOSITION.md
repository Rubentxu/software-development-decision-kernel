# A5 — Debt Disposition

> Cycle: `p-63676b11dc0ef88f/a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only — nothing fixed here)**

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
| `uat_stale_tests::stale_detects_geometry_change` | flaky test | followups ledger (one occurrence, passed on isolated re-run) | **MUST_CLOSE_A5** (P1, test reliability) | A5-5 |
| `INC-A4-RELEASE-VERSION-DRIFT` | pre-bump vs release-tag confusion | `docs/debt/INC-A4-RELEASE-VERSION-DRIFT.md` (open, low); release.sh 1c+9b already mitigate | **MUST_CLOSE_A5** (P1, release governance) | A5-1 |
| `paradigm_lens::evaluate_lens()` | LEGACY_READ_COMPAT facade, NO_RUNTIME_CONSUMER | `A4-MILESTONE-RECEIPT.md` §4 | **MUST_CLOSE_A5** — DELETE (no external consumer found) | A5-4 |
| `INC-A5-PUSH-RELEASE-MARKER-FRICTION` (new) | ceremonial empty `chore(release)` marker | `A5-PUSH-CONTRACT-INVESTIGATION.md` | **MUST_CLOSE_A5** (P2) — independent of the drift INC | A5-1 |

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

## §3.4 Ignored tests (14)

| Test | Documented reason | Disposition |
|---|---|---|
| `cli_phase_build_remediate_rejects_wrong_phase` | "workflow has no transition into REMEDIATING/verify; see cycle-45" | **MUST_CLOSE_A5** or **OBSOLETE** (decide) |
| `regenerate_uat_acceptance_jsonl` | "regenerates uat-acceptance.jsonl; run manually" | **ACCEPTED_RISK** (intentional manual tool) |
| `knowledge::tests::s1_does_not_introduce_new_corenodekind_variants` | "historical anchor only — A3-S2 grew the enums by design" | **OBSOLETE** (delete or re-anchor) |
| `parallel_wfr4_par_006_a_namespaced_count_is_n_plus_one` | "sender-drop bug in non-blocking Parallel path; ignored until IR setup fixed" | **MUST_CLOSE_A5** (P1 — possible runtime defect, risk R12) |
| `parallel_wfr4_par_006_d_node_runs_v1_has_exactly_one_parent_plus_n_children` | same | **MUST_CLOSE_A5** (P1) |
| `dm02_stress_harness` | "T1 diagnostic harness — run manually" | **ACCEPTED_RISK** (manual harness) |
| `verify_stream_chain_fails_on_tampered_hash` | "Tampering requires trigger bypass; covered by SDDK2-203" | **MUST_CLOSE_A5** or **OBSOLETE** (SDDK2-203 unreachable → re-target) |
| `run_survives_restart_with_equivalent_identity_and_provenance` | "pre-existing v1.89.1 debt; DW-RUNTIME-003 follow-up" | **MUST_CLOSE_A5** (P1 — durability/restart is G2/G15) |
| 6 × doc-tests (domain ×4, engine ×1, storage ×1) | rustdoc ignored examples | **ACCEPTED_RISK** (documentation examples) |

**Disposition:** every ignored test is now individually dispositioned
(none invisible). Two pairs point at **runtime** signals (Parallel
sender-drop, restart survival) and are promoted to P1 risks R12/R1.

## §3.5 Legacy / compatibility surfaces

| Surface | Evidence | Disposition |
|---|---|---|
| `paradigm_lens::evaluate_lens()` | `A4-MILESTONE-RECEIPT.md` §4 (tests only) | **MUST_CLOSE_A5** — DELETE |
| `paradigm_lens::LensEvaluation` type | used by `architecture_receipt` (A3 receipt) | **MIGRATE_A5** (route to `alignment_lens`, or justify) |
| `EvidenceAttachmentV1` + compat decoder | `evidence_relation_mapping.rs`; migration deferred | **MIGRATE_A5** |
| old namespaces / schemas in assets | guarded by `asset_*` lints (deny) | **ALREADY_CLOSED** (lint-enforced) |

## §3.6 Post-BASE (not A5)

- `FU-A3-CO-1`/`CO-3`/`S15-4`/`ASC-MA-1` are A5 **only** because they are
  cheap and unblock operator/HX quality. Anything that becomes a *feature*
  is **DEFER_POST_BASE** (see `A5-DEFERRED-POST-BASE.md`).

## Summary

- **Blockers today:** none (0 undisposed P1). Two P1s are *pre-registered*
  from ignored tests (R1/R12) and must be resolved in A5.
- **MUST_CLOSE_A5:** the 8 inherited items + the 2 ignored-test P1s + the
  new push-friction INC.
- **MIGRATE_A5:** 2 compatibility surfaces.
- **ACCEPTED_RISK:** manual harnesses, doc-tests, and the 3 `allow` lints
  (with reasons).
