# A5 — Live Roadmap (post A5-C-RR)

> **Cycle:** `p-63676b11dc0ef88f/a5-current-roadmap-snapshot`
> **Status:** LIVE — replaces the now-invalidated `Milestones remaining`
> section of `ROADMAP-COMPLETION-RECEIPT-2026-09-18-post-A5-4b.md`.
> **Issued:** 2026-09-18 by A5-C-RR (Roadmap Authority Reconciliation).
> **Workspace version at issue:** `1.169.83` (`72dc08e`).

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
A5-C          BASE_PRODUCTION_READY certification                  NOT STARTED    paperwork + acceptance-gate evidence
                                                                          (blocked by all A5-* slices closing)
```

## §3 Mandatory work remaining (A5 → BASE_PRODUCTION_READY)

This is the honest, current, executable backlog. It does **not**
include any M0..M9 work — that baseline is certified.

### P1 — runtime defect signals (ignored tests)

- **R1 — restart-survival**
  `run_survives_restart_with_equivalent_identity_and_provenance`
  (pre-existing v1.89.1 debt; DW-RUNTIME-003 follow-up). Durability /
  restart is G2/G15.

- **R12 — sender-drop (parallel runtime)**
  `parallel_wfr4_par_006_a_namespaced_count_is_n_plus_one`
  `parallel_wfr4_par_006_d_node_runs_v1_has_exactly_one_parent_plus_n_children`
  (sender-drop bug in non-blocking Parallel path; ignored until IR
  setup fixed). Possible runtime defect.

- **Flake observed this session**
  `storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`
  in `crates/sddk-storage/tests/sqlite_storage.rs:850` — concurrent
  SQLite insert hit `DatabaseBusy`. Isolated re-run 1/1 green; flake
  is concurrent-resource contention. Likely same class as R1/R12;
  needs separate cycle with explicit budget.

### P1 — ignored-test OBSOLETE/MUST_CLOSE decision

- `cli_phase_build_remediate_rejects_wrong_phase` — "workflow has no
  transition into REMEDIATING/verify; see cycle-45". Decide OBSOLETE
  or MUST_CLOSE.
- `verify_stream_chain_fails_on_tampered_hash` — "Tampering requires
  trigger bypass; covered by SDDK2-203". Decide OBSOLETE (SDDK2-203
  unreachable → re-target) or MUST_CLOSE.

### Carry-forward from A5-4b (PRE-BASE, NOT POST-BASE)

- **EvidenceAttachmentV1 + compat decoder** — `crates/sddk-engine/src/.../evidence_ref.rs:192` + ratchet exclude + storage migration.

  ```text
  classification:    PRE-BASE
  disposition:       MIGRATE_A5
  risk class:        C2.5
  execution:         separate single-budget slice
  required_before:   A5-C certification
  tracked_in:        docs/architecture/a5/A5-DEBT-DISPOSITION.md §3.5
                     docs/architecture/a5/A5-4b-RECEIPT.md
                       (STOP_NEEDS_SEPARATE_SLICE)
  NOT tracked_in:    docs/architecture/a5/A5-DEFERRED-POST-BASE.md
                     (this item is NOT a POST-BASE item)
  ```

  Source of disposition: `A5-4b-RECEIPT.md` §"Carry-forward" and
  `A5-DEBT-DISPOSITION.md` §3.5 (`EvidenceAttachmentV1 + compat
  decoder → MIGRATE_A5`). Correction recorded in
  `A5-C-RR-ADDENDUM-1.md` §3.3.

### A5-1 follow-up (paperwork, P2)

- `INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH` (open, MEDIUM).

### A5-3 follow-up (paperwork, P3)

- `INC-DEBT-023-lints-advisory-no-expansion-cycle` (open, LOW).

### Deferred POST-BASE (out of A5)

- `multi-region deployment` / `SLA` / `counterfactual planning`
- `CogniCode` integration track (A6)
- `Chronos` integration track (A7)
- `JCode reactive loop` (J0..J9)
- `new providers` (post-A6/A7)
- `new alignment lenses` (post-Base)
- `R11 crate split evaluation` (P3, evidence-driven)

## §4 Acceptance gates for `BASE_PRODUCTION_READY`

Per `docs/SDDK-Production-Readiness-Alignment-2026-09-14/03-PRODUCTION-READY-GATE.md`:

```text
GATE   MEANING                                EVIDENCE STATUS        NOTES
────   ─────────────────────────────────────   ──────────────────     ───────────────────────────────
G0     Baseline semantic conformance          evidenced              09-09-CONFORMANCE-RECEIPT @ 0c2ca56
G1     Ownership and dependency integrity     partial                lint suite (6 of 9 deny; 3 allow-with-reason, machine-pinned)
G2     Knowledge and epistemic integrity       NOT EVIDENCED          R1 restart-survival blocks
G3     Alignment boundary                     NOT EVIDENCED          (no A5 slice targets it)
G4     Verify and DebVerify                   NOT EVIDENCED          (no A5 slice targets it)
G5     Authority, safety and failure          partial                R4-B closed via A6-0..A6-4 (A5-3R0..R4)
G6     Operational recovery / compatibility   partial                release script 14/14 GREEN at v1.169.83
G8     Documentation and release hygiene      partial                current entry point OK; ADRs validated
                                                                  through ADR-0136
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
POST-BASE):

1. **P1 paperwork (cheapest):** ignored-test OBSOLETE/MUST_CLOSE
   decision on `cli_phase_build_remediate_rejects_wrong_phase` and
   `verify_stream_chain_fails_on_tampered_hash`. Zero code; binary
   disposition per item.
2. **P1 runtime:** R12 sender-drop cycle (own budget, runtime defect).
3. **P1 runtime:** R1 restart-survival cycle (own budget, G2/G15).
4. **P1 runtime:** SQLite concurrent-insert flake
   (`storage_insert_gate_receipt_concurrent_allocations_…`).
5. **C2.5 PRE-BASE slice (required before A5-C):**
   `EvidenceAttachmentV1` migration
   (`evidence-migration-v2`). Classification: `MIGRATE_A5 /
   STOP_NEEDS_SEPARATE_SLICE` (per A5-DEBT-DISPOSITION.md §3.5 and
   A5-4b-RECEIPT.md). NOT a POST-BASE item.
6. **A5-C certification:** when A5-* slices are green, run the
   acceptance-gate evidence sweep. G2/G3/G4 must be evidenced
   explicitly; closing the A5-* slices does not automatically
   evidence those gates. Emit `BASE_PRODUCTION_READY` only when
   G0–G6 + G8 are evidenced.

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
