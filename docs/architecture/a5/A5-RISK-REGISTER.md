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
| R8 | Corrupt / partial / stale public asset; CDN serves previous binary | P1 | G8 | A5-1 + A5-5 | **CLOSED_MECHANISM_OPEN_CLEAN_MACHINE** | A5-1 mechanism + 9b public-release gate (sha256-anchored, 10 scenarios, HTTP-200 budget). Clean-machine revalidation pending A5-5 |
| R9 | Installed binary depends on repo checkout / only works via `cargo run` | P1 | G9, G12 | A5-1 | **CLOSED** | A5-1-RECEIPT.md v1.169.71; distrib round-trip smoke test 14/14 PASS at every release |
| R10 | Push/release protocol requires a ceremonial empty `chore(release)` marker | P2 | release audit | A5-1 | **CLOSED** | A5-1-RECEIPT.md v1.169.71; `INC-A5-PUSH-RELEASE-MARKER-FRICTION` CLOSED; empty marker now rejected; docs-only pushes accepted (clause B) |
| R11 | Flaky test hidden by "couldn't reproduce → closed" | P1 | G13 | A5-5R + A5-5 | **CLOSED_PARTIAL** | A5-5R-RECEIPT.md (specific flake `stale_detects_geometry_change` closed v1.169.81); broader flake-discipline sweep owed by A5-5 |
| R12 | Non-blocking Parallel path sender-drop bug | P1 | G4, G13 | A5-3 | **CLOSED** | A5-3-RECEIPT.md v1.169.75; commit `af346b9`; non-blocking path deleted (`operator.rs:1196-1356`, ~162 lines); 2 ignored `par_006_*` tests deleted; `parallel_spans_three_ticks_drain` deleted; runtime forces `pending_sender = None` per `operator.rs:1197-1205` |
| R13 | Dead compatibility code (`paradigm_lens::evaluate_lens`) kept forever | P2 | G14 | A5-4a | **CLOSED** | A5-4a-RECEIPT.md v1.169.82; facade deleted; type deleted; ADR-0135 |
| R14 | Secret leaks into log / receipt / error / telemetry / CAS | P0 | G11 | — | **OPEN_NON_BLOCKER** | No observed leak path; not addressed in A5 yet. Tracked for A5-5 or a future security cycle |
| R15 | Rollback to previous certified release fails | P1 | G15 | A5-1 + A5-5 | **CLOSED_MECHANISM_OPEN_CLEAN_MACHINE** | A5-1 mechanism + release pipeline 14/14 PASS. Clean-machine revalidation pending A5-5 |
| R16 | An A5 change silently breaks an A4 certified contract | P0 | G1 | every cycle | **OPEN_NON_BLOCKER** | A4-CERTIFIED baseline (v1.169.68). Mechanism: cross-crate arch ratchets + ADR-0001 §3.2 promotion gates. No observed breach in A5-1..A5-5R, A5-4a/4b, A5-ITD |
| R17 | Operator cannot diagnose a production failure | P2 | G10 | A5-4b + A5-5 | **OPEN_NON_BLOCKER** | A5-4b partial: about-line + `sddk agent-help` pointer. Deeper diagnostics sweep pending A5-5 / A5-C |
| R18 | Ignored tests become invisible debt | P2 | G13 | A5-ITD + A5-C-RR | **CLOSED** | A5-ITD-RECEIPT.md v1.169.84; A5-C-RR-A1 §3.4.1; per-ignored-test disposition with evidence pointers |
| R19 | Deprecated-pattern lints stay `allow` with no disposition | P3 | G14 | A5-4b | **CLOSED** | A5-4b-RECEIPT.md v1.169.83; 3 advisory `allow` lints machine-pinned by `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs` (3 tests green) |
| R20 | Migration of persisted state breaks an upgrade | P1 | G6 | A5-2 | **CLOSED** (re-pinned) | A5-2-RECEIPT.md v1.169.74; G6 evidence preserved across v1.169.74 |

## §2 Notes on risks still open or partially open

### R8 / R15 — `CLOSED_MECHANISM_OPEN_CLEAN_MACHINE`

The mechanism is complete and observed (release pipeline 14/14 PASS
at every release since A5-1, including the public-release gate 9b
sha256-anchored to the GH tag). The remaining artefact is the
**clean-machine revalidation** sweep — a fresh host running the
release end-to-end and asserting install + doctor + round-trip.
That sweep is owed by **A5-5**; it does not require code changes,
only an isolated machine run.

### R11 — `CLOSED_PARTIAL`

A5-5R (`v1.169.81`) closed the specific flake
`uat_stale_tests::stale_detects_geometry_change`. The **broader
flake-discipline** sweep (which A5-5 was supposed to perform
systematically) is owed by **A5-5**.

### R14 / R16 / R17 — `OPEN_NON_BLOCKER`

These are tracked, not blocking `BASE_PRODUCTION_READY`:

- **R14 (secrets):** no observed leak path; out of scope for any
  current cycle. Requires a dedicated security review cycle.
- **R16 (silent A5→A4 breach):** mechanism (cross-crate ratchets +
  ADR-0001 promotion gates) is in place and exercised. No observed
  breach in any A5 cycle.
- **R17 (operator diagnostics):** partial mitigation in A5-4b.
  Deeper sweep pending A5-5 or A5-C.

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
