# A5 — Live Roadmap (post A5-C, sync at v1.169.88)

> **Cycle:** `p-63676b11dc0ef88f/a5-c-base-production-ready-certification`
> **Status:** LIVE — reconciled against certified per-cycle receipts.
> **Issued:** 2026-09-19 by A5-C closure pass.
> **Workspace version at issue:** `1.169.88` (`add896d`) — released.
> **A5-C = CERTIFIED** at `v1.169.88`. Authority:
>   `docs/architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md`.

This is the live execution roadmap. It is the **single source of
truth** for what remains until `BASE_PRODUCTION_READY` and beyond.

## §0 Roadmap authority (declared once)

Three documents jointly govern the SDDK roadmap. Each has a
distinct role; mixing them is the drift this snapshot fixes.

```text
CERTIFIED HISTORICAL BASELINE          (not a live execution roadmap)
   docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/
   - 72 documents
   - M0..M9 closed by C0..C7 conformance at:
       09-09-CONFORMANCE-RECEIPT.md (PASS, 100% conformance)
       commit:    0c2ca56
       workspace: SDDK 1.169.19
   - Do NOT re-open M0..M9 as live work. They are baseline, not backlog.

CANONICAL ARCHITECTURE (live)
   docs/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/
   - Bounded context map
   - Target source layout
   - Canonical roadmap R0..R11 (this is the design intent)
   - Supersedes (historical): prior competing roadmaps in its
     05-INTEGRATION/SUPERSESSION.md

CANONICAL EXECUTION ROADMAP (live)
   docs/SDDK-Production-Readiness-Alignment-2026-09-14/
   - 02-MINI-ROADMAP.md (A0..A5 BASE_READY → A6/A7/A8 + J0..J9)
   - 03-PRODUCTION-READY-GATE.md (G0..G6 + G8 acceptance gates)
   - This package IS the executable roadmap; the 10/09 is its design.

THIS FILE
   docs/architecture/a5/A5-CURRENT-ROADMAP.md
   - Snapshot of the 14/09 mini-roadmap as it stands at v1.169.83.
   - Single source for "what's next" until A5 closes.
```

## §1 Live roadmap (A0..A8 + J0..J9)

Status as of `v1.169.83`. Evidence column points to the receipt
or release tag that closed the slot, where applicable.

```text
SLOT    MEANING                                          STATUS                 EVIDENCE
─────   ──────────────────────────────────────────────   ────────────────────   ─────────────────────────────────────────────
A0      authority / drift closeout                        CLOSED                v1.169.50 (per 14/09 mini-roadmap)
A1      C7 baseline conformance                           CLOSED                09-09-CONFORMANCE-RECEIPT @ 0c2ca56
A2      R0 + R1 (context boundary / core)                 CLOSED                v1.169.55 (per 14/09 mini-roadmap)
A3      R2 + R3 (knowledge + advisory)                    CLOSED / CERTIFIED    v1.169.66 (A4-3R)
A4      R4 + R5 + R6 (Alignment/Verify/DebVerify)         CLOSED / CERTIFIED    v1.169.68 (A4-CLOSEOUT)
A5      BASE_PRODUCTION_READY hardening                   IN PROGRESS           See §2 below
A6      CogniCode / STATIC_ENHANCED                       NOT STARTED           reserved (NOT the historical a6-0..a6-4)
A7      Chronos / RUNTIME_ENHANCED                        NOT STARTED           reserved
A8      FULLY_ENHANCED                                    blocked_by A6 + A7
J2..J6  JCODE_CORE_GA                                     NOT STARTED           parallel track, P1
R11     crate split evaluation                            NOT STARTED           P3, evidence-driven
```

## §2 A5 status — what remains for BASE_PRODUCTION_READY

A5 is the live cycle. The work below is the actual backlog.

```text
SLICE         SCOPE                                                STATUS         EVIDENCE
─────────     ───────────────────────────────────────────────     ────────────   ───────────────────────────────────
A5-1          release / distribution / version governance          CLOSED         v1.169.71 (A5-1-RECEIPT.md)
A5-2          durability / rebuild / recovery                      CLOSED         v1.169.74 (A5-2-RECEIPT.md)
A5-3          concurrency / CAS / authority side-effect races      CLOSED/PARTIAL R4-B slice closed via A6-0..A6-4
                                                                       (historical a6-0..a6-4 attribution
                                                                        = A5-3R0..A5-3R4; see README appendix)
A5-4a         remove paradigm_lens legacy facade                   CLOSED         v1.169.82 (A5-4a-RECEIPT.md, ADR-0135)
A5-4b         compat / lints / operator UX                         CLOSED         v1.169.83 (A5-4b-RECEIPT.md, ADR-0136)
A5-5R         stale Playwright flake                               CLOSED         v1.169.81 (A5-5R-RECEIPT.md)
A5-5          clean-machine sweep (G8/G9/G12/G15 + R8/R15/R11)    CLOSED         v1.169.87 (A5-5-CLEAN-MACHINE-SWEEP-RECEIPT.md, `5ad25d1`)
A5-C          BASE_PRODUCTION_READY certification                  CERTIFIED      v1.169.88 (A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md, `add896d`)
                                                                          G0..G15 GREEN; G11 NOT VERIFIED with R14 accepted (see cert §11).
                                                                          Public install path validated without test-harness workarounds.
```

## §3 Mandatory work remaining (A5 → BASE_PRODUCTION_READY)

This is the honest, current, executable backlog. It does **not**
include any M0..M9 work — that baseline is certified.

> **A5-C-RR2 reconciliation note (v1.169.84 → `b4acbbf`):** The
> prior version of this section listed `R1 — restart-survival` and
> `R12 — sender-drop (parallel runtime)` as P1 open. Both are
> **CLOSED** by certified per-cycle receipts:
>
> - **R1** closed by A5-2-RECEIPT.md (v1.169.74, commit `957d0b0`):
>   `SqliteGraphStore::latest_run_state_for` makes the canonical
>   event log the authority for `load_run.state`. The previously-
>   ignored test `run_survives_restart_with_equivalent_identity_and_
>   provenance` was **corrected** (not merely un-ignored) — it
>   was passing for the wrong reason, asserting `loaded.state ==
>   Pending` after a `running` event was inserted, which kept the
>   snapshot stale. Now it asserts `Running` (A5-2 §0 row 3).
>
> - **R12** closed by A5-3-RECEIPT.md (v1.169.75, commit
>   `af346b9`): the non-blocking Parallel path was deleted
>   (`operator.rs:1196-1356`, ~162 lines); the 2 ignored `par_006`
>   tests were deleted; `parallel_spans_three_ticks_drain` was
>   deleted. The runtime now forces `pending_sender = None` per
>   `operator.rs:1197-1205`. Any future async/non-blocking
>   Parallel is a POST-BASE feature (see
>   `A5-RISK-REGISTER.md §3`), NOT a closure of R12.
>
> - **A5-ITD** (v1.169.84) closed the two ignored tests previously
>   listed as "P1 paperwork": both OBSOLETE → DELETED.

### PRE-BASE — closed by v1.169.85 (no remaining items)

`EvidenceAttachmentV1 + compat decoder` was the only MIGRATE_A5 entry; it
is closed by `v1.169.85`. See
`docs/architecture/a5/A5-EVIDENCE-ATTACHMENT-MIGRATION-V1-RECEIPT.md`.

The READ-compat decoder (`from_legacy_kind_tag`) remains at the read
boundary as a documented compat surface; it does not mint new authority
and is not a PRE-BASE concern.

### P1 candidate (NOT yet classified — needs reproduction cycle)

- **`storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`**
  in `crates/sddk-storage/tests/sqlite_storage.rs:738` —
  concurrent SQLite insert hit `DatabaseBusy` during release.sh
  runs. **CLOSED** by `v1.169.86`
  (cycle `A5-SQLITE-CONCURRENCY-R`):
  - flake pinned 50/50 under `--test-threads=4` via the new
    `Storage::with_busy_retry` helper (M2, `lib.rs:1471`)
  - CAS-orphan class on `insert_evidence_attachment` closed (M3)
  - 5 new multi-thread tests for the planning substrate in
    `tests/concurrency_planning_substrate.rs` (M4)
  - substrate contract documented in
    `crates/sddk-storage/src/planning_substrate_contract.md` (M5)
  - receipt: `docs/architecture/a5/A5-SQLITE-CONCURRENCY-R-RECEIPT.md`

  **Remaining scope deferred to a future cycle:** apply the
  `with_busy_retry` helper to the other 8 IMMEDIATE sites
  (registration, capability receipts, cycle leases) — out-of-scope
  for R-SQLITE-1 by design (each surface warrants its own change-set).

### A5-5 — CLOSED at v1.169.87 (`5ad25d1`)

A5-5 clean-machine sweep (UAT-1, `tests/clean_machine_uat.sh`,
10/10 scenarios PASS on isolated podman container against v1.169.86)
closed the mechanism-pending gap on G8/G15 + R8/R15 + the broader
R11. Receipt: `docs/architecture/a5/A5-5-CLEAN-MACHINE-SWEEP-RECEIPT.md`.
What A5-5 did NOT certify: the public install path *without the
UAT harness's internal workarounds*. Those 11 install.sh workarounds
are tracked as **installation defects pending classification** in the
A5-C admission audit (see `A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md`
when emitted). They are not closed by A5-5 alone.

### A5-C — CERTIFIED at v1.169.88 (`add896d`)

See `docs/architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md`
for the G0..G16 matrix, the 11 install.sh defect classifications,
and the §10 release checkpoint.

- G0..G15 GREEN.
- G11 NOT VERIFIED with R14 accepted (no observed leak; future
  dedicated security cycle owed).
- Public install path passes end-to-end on a clean podman container
  against the released artifact `v1.169.88`, without test-harness
  workarounds.
- 11 `install.sh` defects: zero RELEASE_BLOCKER; 5 ACCEPTED_NON_BLOCKER;
  6 POST_A5_DEBT (test-harness bookkeeping).

`BASE_PRODUCTION_READY` reached. Post-BASE backlog lives in the
roadmap's deferred sections (A6/A7/A8, J2..J6, R17 deeper sweep,
the 8 IMMEDIATE sites not yet routed through `with_busy_retry`, etc.).

### Post-A5 (not yet started)

After `BASE_PRODUCTION_READY` is certified, the roadmap reverts to
the deferred-post-base queue:

- **A6 CogniCode / STATIC_ENHANCED** (NOT to be confused with the
  historical `a6-0..a6-4` cycle IDs which were A5/G5 Authority
  hardening — see `docs/architecture/README.md` Roadmap
  Reconciliation Appendix).
- **A7 Chronos / RUNTIME_ENHANCED**.
- **A8 FULLY_ENHANCED** (blocked_by A6 + A7).
- **J2..J6 JCODE_CORE_GA** (parallel track, P1).
- **R14 dedicated security cycle** (secrets matrix) — source-level sweep
  CLOSED by `A5-R14-SECRETS-SWEEP-RECEIPT.md` (v1.169.119);
  adversarial-level review remains optional POST-BASE scope.
- **R17 deeper operator diagnostics sweep**.
- The 8 IMMEDIATE sites not yet routed through `with_busy_retry`
  (out of R-SQLITE-1 by design).
- Future async/non-blocking Parallel (a POST-BASE feature, NOT R12).
- R11 crate split evaluation (P3).

A6 is not auto-opened by A5-C. The user will decide its scope
after reviewing the accepted debt (§3 G11 + the 5
ACCEPTED_NON_BLOCKER install.sh defects + INC P2/P3 paperwork).

### OPEN_NON_BLOCKER (no current blocker)

- **R14 (secret leak path):** no observed leak; tracked for A5-5
  or a future security cycle.
- **R16 (silent A5→A4 contract breach):** mechanism (cross-crate
  ratchets + ADR-0001 promotion gates) in place; no observed
  breach in any A5 cycle.
- **R17 (operator diagnostics):** partial mitigation in A5-4b;
  deeper sweep pending A5-5 / A5-C.

### A5-1 / A5-3 follow-up (paperwork)

- `INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH` (open, MEDIUM, P2).
- `INC-DEBT-023-lints-advisory-no-expansion-cycle` (open, LOW, P3).

### Deferred POST-BASE (out of A5)

- **A6 CogniCode STATIC_ENHANCED** (NOT to be confused with the
  historical `a6-0..a6-4` cycle IDs which were A5/G5 Authority
  hardening — see `docs/architecture/README.md` Roadmap
  Reconciliation Appendix).
- **A7 Chronos RUNTIME_ENHANCED**.
- **A8 FULLY_ENHANCED** (blocked_by A6 + A7).
- **J2..J6 JCODE_CORE_GA** (parallel track, P1).
- **Future async/non-blocking Parallel** (NOT R12; POST-BASE feature).
- **R11 crate split evaluation** (P3, evidence-driven).
- `multi-region deployment` / `SLA` / `counterfactual planning`.

## §4 Acceptance gates for `BASE_PRODUCTION_READY`

Per `docs/SDDK-Production-Readiness-Alignment-2026-09-14/03-PRODUCTION-READY-GATE.md`
and reconciled by A5-C-RR2 (v1.169.84 → `b4acbbf`):

```text
GATE   MEANING                                AUTHORITY / RECEIPT           STATUS                  NOTES
────   ─────────────────────────────────────   ───────────────────────────   ─────────────────────   ────────────────────────────────────────────
G0     Baseline semantic conformance          09-09-CONFORMANCE-RECEIPT    EVIDENCED               @ 0c2ca56 / SDDK 1.169.19; 100% conformance
                                              (certified historical)
G1     Ownership and dependency integrity     A5-4b / A5-ITD               EVIDENCED               lint suite (6 of 9 deny; 3 allow-with-reason,
                                                                                                  machine-pinned by a5_4b_lint_disposition_pin.rs)
G2     Knowledge and epistemic integrity       A5-2                          EVIDENCED               R1 closed; canonical event log is authority
                                              (durability / crash recovery)
G3     Rebuildability                         A5-2                          EVIDENCED               R1 closure requires only the rowid-monotonic
                                                                                                  event log; same canonical inputs rebuild the
                                                                                                  same WorkflowRun read
G4     Verify and DebVerify                   A5-3                          EVIDENCED               R3 (typed conflict) + R6 (retry idempotency)
                                              (concurrency correctness)                            + R12 (non-blocking Parallel dead-path removal);
                                                                                                  4 + 6 = 10 RED→GREEN tests
G5     Authority, safety and failure          A5-3 (R4-A) + A6-0..A6-4      EVIDENCED               R4-A: deny ⇒ zero side effect (A5-3);
                                              (R4-B: AuthorityTicketService)                       R4-B: AuthorityAdmissionTicket primitive
                                                                                                  + migration pattern (ADR-0130..0134);
                                                                                                  INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY
                                                                                                  CLOSED 2026-09-18
G6     Operational recovery / compatibility   A5-1 + A5-2                   EVIDENCED               release pipeline 14/14 GREEN at every release
                                              (R7, R8, R9, R10, R15, R20)                          since v1.169.71; A5-2 R20 re-pinned (G6)
G8     Documentation and release hygiene      A5-1 + A5-2 + A5-4a + A5-4b   EVIDENCED               current entry point OK; ADRs validated
                                                                                                  through ADR-0136; A5-2 R1/R2 evidence in
                                                                                                  state_survives_restart.rs; A5-4a/4b lint
                                                                                                  suite machine-pinned
G11    Secrets / leak paths                   (R14 OPEN_NON_BLOCKER)        OPEN_NON_BLOCKER        no observed leak path; tracked for A5-5 or
                                                                                                  a future security cycle
G13    Test reliability / ignored discipline  A5-5R + A5-ITD + A5-C-RR-A1   EVIDENCED (R18)         R18 closed (per-ignored-test disposition);
                                              (R11 CLOSED_PARTIAL,                                  R11 broader flake-discipline sweep owed
                                               R18 CLOSED)                                          by A5-5
G15    Rollback                               A5-1 mechanism + clean-mach   CLOSED_MECHANISM        mechanism + automated evidence complete;
                                              validation pending A5-5        OPEN_CLEAN_MACHINE     clean-machine revalidation owed by A5-5
```

## §5 Cold-start pickup for the next session

The next session should read:

1. `docs/architecture/README.md` (current normative architecture)
2. `docs/architecture/a5/A5-CURRENT-ROADMAP.md` (this file)
3. `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`
4. `docs/SDDK-Production-Readiness-Alignment-2026-09-14/03-PRODUCTION-READY-GATE.md`
5. `docs/handoff/HANDOFF-2026-09-18-a5-4b-session-close.md`
6. `docs/architecture/a5/A5-DEBT-DISPOSITION.md` §3.7 (current state)
7. `docs/architecture/a5/A5-DEFERRED-POST-BASE.md`

Then pick the next single item with a single change budget.
The recommended order, by cheapest first, **PRE-BASE work required
before A5-C certification** (EvidenceAttachmentV1 is PRE-BASE, not
POST-BASE; R1/R12 are CLOSED and not in the active backlog):

1. **C2.5 PRE-BASE slice (required before A5-C):**
   `EvidenceAttachmentV1` migration
   (`evidence-migration-v2`). Classification: `MIGRATE_A5 /
   STOP_NEEDS_SEPARATE_SLICE` (per A5-DEBT-DISPOSITION.md §3.5 and
   A5-4b-RECEIPT.md). NOT a POST-BASE item.
2. **P1 candidate cycle:** SQLite concurrent-insert flake
   (`storage_insert_gate_receipt_concurrent_allocations_…`). **DONE**
   by `A5-SQLITE-CONCURRENCY-R` (v1.169.86). See receipt
   `docs/architecture/a5/A5-SQLITE-CONCURRENCY-R-RECEIPT.md`.
3. **A5-5 clean-machine sweep:** G8 / G15 / R11 broader
   flake-discipline. Mechanism + automated evidence complete
   (release pipeline 14/14 PASS at every release); only the
   clean-machine revalidation is outstanding.
4. **A5-C certification:** when items 1–3 are green, run the
   final evidence sweep. Per the reconciled §4, G0..G6 + G8 are
   already EVIDENCED; only G11 (secrets, OPEN_NON_BLOCKER),
   G13 (R11 partial) and G15 (clean-machine) remain. Emit
   `BASE_PRODUCTION_READY` only when these are either closed or
   explicitly accepted as OPEN_NON_BLOCKER.

After A5-C closes:

- **A6 CogniCode / STATIC_ENHANCED** (NOT to be confused with the
  historical `a6-0..a6-4` cycle IDs which were A5/G5 Authority
  hardening — see `docs/architecture/README.md` Roadmap
  Reconciliation Appendix).
- **A7 Chronos / RUNTIME_ENHANCED**.
- **A8 FULLY_ENHANCED** (blocked_by A6 + A7).
- **J2..J6 JCODE_CORE_GA** (parallel track, P1).

None of the above opens M0..M9, R0..R11, A0..A4 without explicit
user green-light. M0..M9 are baseline-cert-closed (C0..C7 PASS at
`0c2ca56` / SDDK `1.169.19`); R0..R11 lives in
`docs/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/`
as design intent; A0..A4 are declared closed in §1 above.
