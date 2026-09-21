# A5 — Risk Register

> Cycle: `p-63676b11dc0ef88f-a5-c-rr2-risk-gate-debt-reconciliation`
> Status: **RECONCILED** at `v1.169.84` (`b4acbbf`)
> Severity vocabulary: `docs/architecture/a5/A5-PRODUCTION-READINESS-CONTRACT.md` §20.
> Reconciliation scope: R1..R20 — every row traced to its closing receipt.
> **Authority order:** released code + tests > certified per-cycle receipt >
> live roadmap snapshot > planning document.

The register is the *risk* view; `A5-DEBT-DISPOSITION.md` is the
*debt* view. A risk without an owner/workstream is a planning defect.

## §0 Disposition vocabulary (post A5-C-RR2)

| Disposition | Meaning |
|---|---|
| `CLOSED` | Risk fully retired; receipt + evidence in tree. Live code + tests cover the property. |
| `CLOSED_MECHANISM_OPEN_CLEAN_MACHINE` | Mechanism + automated evidence complete; only clean-machine revalidation outstanding (A5-5 sweep). |
| `CLOSED_PARTIAL` | Sub-risk closed; broader scope still owed by another cycle. |
| `OPEN_BLOCKER` | Blocks `BASE_PRODUCTION_READY`. Evidence required before A5-C. |
| `OPEN_NON_BLOCKER` | Open but not currently blocking `BASE_PRODUCTION_READY`; tracked for a future cycle. |
| `DEFER_POST_BASE` | Deferred to post-BASE work (A6/A7/A8/J or beyond). |
| `SUPERSEDED` | Replaced by a different risk ID or by a structural change. |

## §1 Risk matrix (R1..R20, reconciled at v1.169.84)

| ID | Risk | Sev | Detected by (gate) | Workstream | Status | Receipt / evidence |
|---|---|---|---|---|---|---|
| R1 | Crash between canonical append and projection rebuild loses or diverges state | P0 | G2, G3 | A5-2 | **CLOSED** | A5-2-RECEIPT.md v1.169.74; commit `957d0b0`; `SqliteGraphStore::latest_run_state_for`; `state_survives_restart.rs` 3 tests |
| R2 | Corrupt / missing CAS object silently accepted | P0 | G2 | A5-2 | **CLOSED** | A5-2-RECEIPT.md v1.169.74; commit `7ab113a`; CAS `get()` typed-error on truncation |
| R3 | Two writers overwrite (last-writer-wins) | P0 | G4 | A5-3 | **CLOSED** | A5-3-RECEIPT.md v1.169.75; commit `28abee3`; `record_node_run_for_run` switched to `INSERT` + `IdempotencyConflict`; `concurrency_record_attempt.rs` 4 RED→GREEN |
| R4 | Authority decision → effect TOCTOU window | P0 | G5 | A5-3 + A6-0..A6-4 | **CLOSED** | R4-A: A5-3-RECEIPT.md v1.169.75; R4-B: `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` CLOSED 2026-09-18; ADR-0130..0134 |
| R5 | Denied action still produces a side effect / `RequireApproval` bypass | P0 | G5 | A5-3 | **CLOSED** | A5-3-RECEIPT.md v1.169.75; `authority_fail_closed.rs` 6/6 green; executor short-circuits on Deny/RequireApproval |
| R6 | Retry double-applies an effect | P1 | G5 | A5-3 | **CLOSED** | A5-3-RECEIPT.md v1.169.75; R3 typed-conflict machinery covers retry idempotency |
| R7 | Release tag ≠ certified SHA; version drift | P1 | G7, G16 | A5-1 | **CLOSED** | A5-1-RECEIPT.md v1.169.71; `release_admission_check` + identity set; `INC-A4-RELEASE-VERSION-DRIFT` CLOSED |
| R8 | Corrupt / partial / stale public asset; CDN serves previous binary | P1 | G8 | A5-1 + A5-5 | **CLOSED_A5** | A5-1 mechanism + 9b public-release gate + UAT-1 clean-machine (10/10 PASS, isolated podman container, `bash tests/clean_machine_uat.sh --tag v1.169.86`; receipt: `.sddk/cycles/p-63676b11dc0ef88f/a5-5-clean-machine-sweep/clean-machine-uat-receipt.json`); release tag `v1.169.87`, commit `5ad25d1` |
| R9 | Installed binary depends on repo checkout / only works via `cargo run` | P1 | G9, G12 | A5-1 | **CLOSED** | A5-1-RECEIPT.md v1.169.71; distrib round-trip smoke test 14/14 PASS at every release |
| R10 | Push/release protocol requires a ceremonial empty `chore(release)` marker | P2 | release audit | A5-1 | **CLOSED** | A5-1-RECEIPT.md v1.169.71; `INC-A5-PUSH-RELEASE-MARKER-FRICTION` CLOSED; empty marker now rejected; docs-only pushes accepted (clause B) |
| R11 | Flaky test hidden by "couldn't reproduce → closed" | P1 | G13 | A5-5R + A5-5 | **CLOSED_A5** | A5-5R-RECEIPT.md (flake closed v1.169.81) + UAT-1 clean-machine (10/10 PASS, tests/clean_machine_uat.sh); release tag `v1.169.87`, commit `5ad25d1` |
| R12 | Non-blocking Parallel path sender-drop bug | P1 | G4, G13 | A5-3 | **CLOSED** | A5-3-RECEIPT.md v1.169.75; commit `af346b9`; non-blocking path deleted (`operator.rs:1196-1356`, ~162 lines); 2 ignored `par_006_*` tests deleted; `parallel_spans_three_ticks_drain` deleted; runtime forces `pending_sender = None` per `operator.rs:1197-1205` |
| R13 | Dead compatibility code (`paradigm_lens::evaluate_lens`) kept forever | P2 | G14 | A5-4a | **CLOSED** | A5-4a-RECEIPT.md v1.169.82; facade deleted; type deleted; ADR-0135 |
| R14 | Secret leaks into log / receipt / error / telemetry / CAS | P0 | G11 | `A5-R14-SECRETS-SWEEP-RECEIPT.md` | **SWEPT_SOURCE_LEVEL** (v1.169.119) | No leak path found; latent gap closed (error/message/reason now string-scanned); residual = static single-pass, adversarial cycle remains POST-BASE |
| R15 | Rollback to previous certified release fails | P1 | G15 | A5-1 + A5-5 | **CLOSED_A5** | A5-1 mechanism + UAT-1 clean-machine scenario 10 (rollback v1.169.86→v1.169.85, dev doctor all_present=true, bundle_coherence=present; receipt same as R8); release tag `v1.169.87`, commit `5ad25d1` |
| R16 | An A5 change silently breaks an A4 certified contract | P0 | G1 | every cycle | **OPEN_NON_BLOCKER** | A4-CERTIFIED baseline (v1.169.68). Mechanism: cross-crate arch ratchets + ADR-0001 §3.2 promotion gates. No observed breach in A5-1..A5-5R, A5-4a/4b, A5-ITD |
| R17 | Operator cannot diagnose a production failure | P2 | G10 | A5-4b + A5-5 + R17 sweep | **SWEPT_SOURCE_LEVEL** (v1.169.120) | A5-4b about-line + `sddk agent-help`; R17 sweep: `dev doctor` failing checks now emit actionable `detail` remediation hints (text + JSON). Deeper failure-mode catalog remains POST-BASE |
| R18 | Ignored tests become invisible debt | P2 | G13 | A5-ITD + A5-C-RR | **CLOSED** | A5-ITD-RECEIPT.md v1.169.84; A5-C-RR-A1 §3.4.1; per-ignored-test disposition with evidence pointers |
| R19 | Deprecated-pattern lints stay `allow` with no disposition | P3 | G14 | A5-4b | **CLOSED** | A5-4b-RECEIPT.md v1.169.83; 3 advisory `allow` lints machine-pinned by `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs` (3 tests green) |
| R20 | Migration of persisted state breaks an upgrade | P1 | G6 | A5-2 | **CLOSED** (re-pinned) | A5-2-RECEIPT.md v1.169.74; G6 evidence preserved across v1.169.74; **MIGRATE_A5** follow-up closed by v1.169.85 — see `A5-EVIDENCE-ATTACHMENT-MIGRATION-V1-RECEIPT.md` §4.3 (CAS reopen test); **R-SQLITE-1** follow-up closed by v1.169.86 — `with_busy_retry` helper pins the gate-receipt flake 50/50, CAS-oracle orphan class closed, planning-substrate concurrency tests in `tests/concurrency_planning_substrate.rs`. |

## §2 Notes on risks still open or partially open

### R8 / R15 — `CLOSED_A5` (was `CLOSED_MECHANISM_OPEN_CLEAN_MACHINE`)

A5-5 completed the clean-machine revalidation sweep. UAT-1
(`tests/clean_machine_uat.sh`) ran all 10 scenarios on an isolated
podman container, including:
- **R8:** CDN asset integrity (sha256 checksums, install from CDN, dev doctor)
- **R15:** Rollback to prior certified version (v1.169.85 → dev doctor coherent)
Receipt: `.sddk/cycles/p-63676b11dc0ef88f/a5-5-clean-machine-sweep/clean-machine-uat-receipt.json`

### R11 — `CLOSED_A5` (was `CLOSED_PARTIAL`)

A5-5 closed the broader flake-discipline sweep. The clean-machine
UAT-1 runs on an isolated container with no prior state, providing
the clean baseline that validates no hidden flaky tests. The
`uat_stale_tests::stale_detects_geometry_change` flake was already
closed by A5-5R (v1.169.81).

### R14 — `SWEPT_SOURCE_LEVEL` (v1.169.119)

- **R14 (secrets):** no leak path found; `STRING_LEVEL_KEY_PATTERN`
  extended to error/message/reason (see
  `A5-R14-SECRETS-SWEEP-RECEIPT.md`). A deeper adversarial cycle
  remains optional POST-BASE scope.

### R16 — `OPEN_NON_BLOCKER`

- **R16 (silent A5→A4 breach):** mechanism (cross-crate ratchets +
  ADR-0001 promotion gates) is in place and exercised. No observed
  breach in any A5 cycle.

### R17 — `SWEPT_SOURCE_LEVEL` (v1.169.120)

- **R17 (operator diagnostics):** A5-4b delivered the about-line +
  `sddk agent-help` surface. The R17 sweep (v1.169.120) closed the
  diagnostic-actionability gap at source level: every failing `dev
  doctor` check now carries a `detail` remediation hint (rendered as
  `tool: missing — <hint>` in text; serialized in JSON). Framework
  checks (`broken_agent_links`, `agent_name_frontmatter`,
  `stale_agent_copies`, `workflow_origin`), surface brevity (line
  budgets) and empty-dirs checks all emit actionable hints. A deeper
  failure-mode catalog (runtime error taxonomy) remains optional
  POST-BASE scope.

## §3 Future async/non-blocking Parallel — POST-BASE FEATURE (NOT R12)

R12 is CLOSED (A5-3 / v1.169.75). A future cycle that wants to
**re-introduce** non-blocking Parallel semantics is a
**POST-BASE FEATURE**, not a closure of R12.

- It MUST rebuild intentionally (per A5-3-RECEIPT.md §"What did
  NOT happen").
- It MUST NOT resurrect the deleted tests
  (`parallel_wfr4_par_006_a`, `parallel_wfr4_par_006_d`,
  `parallel_spans_three_ticks_drain`).
- It MUST NOT claim "R12 closed" — R12 is already closed; a
  future async Parallel is a new feature, not a closure of an
  old risk.

Recorded separately as `FUTURE-PARALLEL-ASYNC` (not a Risk ID —
opens only when a cycle proposes it). This entry is here so a
future audit does not mis-read "R12 closed but no async" as "R12
still open".

## §4 Register discipline

- Every risk maps to exactly one primary workstream.
- A risk is closed only with evidence (a receipt in the A5 body
  of evidence), never by assertion.
- A CLOSED risk cannot be reopened without a new RED→GREEN
  falsification in a new cycle.
- New risks discovered during A5 cycles are added here, not
  silently absorbed.

## §5 Reconciliation history

- **A5-C-RR (v1.169.83, `ffbc012`)** — recovered the single
  roadmap authority (09/09 = certified baseline; 10/09 = live
  architecture; 14/09 = live roadmap).
- **A5-C-RR-A1 (v1.169.83, `9215ac4`)** — corrected three
  documentary residues: clippy overclaim, G1 lint count, and
  `EvidenceAttachmentV1` POST/PRE-BASE classification.
- **A5-C-RR2 (this cycle, target `v1.169.85`)** — reconciled
  R1..R20 against certified receipts; A5-ITD reflected in the
  register; A5-5 clean-machine sweep explicitly named as the
  only outstanding A5-* mechanism-pending gap.
- **A5-C-DOC-SYNC (2026-09-19)** — closed-documentary pass
  against `v1.169.87`. Roadmap §2 row A5-5 promoted from
  `OWED` to `CLOSED`; debt disposition header updated to cite
  `5ad25d1`; risk matrix R8/R11/R15 cite the `v1.169.87` tag.
  No status change for any open risk. A5-C itself was
  **PENDING** until a release with bump after `v1.169.87` was
  certified against the public install path.
- **A5-C certification (2026-09-19)** — `A5-C BASE_PRODUCTION_READY`
  promoted to **CERTIFIED — v1.169.88** (`add896d`). Authority:
  `docs/architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md`.
  G0..G15 GREEN; G11 NOT VERIFIED with R14 accepted (no observed leak).
  11 install.sh defects classified (zero RELEASE_BLOCKER; 5
  ACCEPTED_NON_BLOCKER; 6 POST_A5_DEBT). Public install path validated
  on a clean podman container without test-harness workarounds.
