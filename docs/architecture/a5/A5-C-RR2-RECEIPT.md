# A5-C-RR2 Receipt — Risk / Gate / Debt Reconciliation

> Cycle: `p-63676b11dc0ef88f/a5-c-rr2-risk-gate-debt-reconciliation`
> Status: **CLOSED — docs-only reconciliation, no code touched**
> Workspace version at close: `1.169.84` (`b4acbbf`)
> Risk class: **C0 (paperwork)**
> ADR required: **no** (reconciliation is not an architectural decision)

## §0 Why this cycle exists

The live roadmap snapshot (`A5-CURRENT-ROADMAP.md` v1.169.83 /
`72dc08e`) listed R1 — restart-survival and R12 — non-blocking
Parallel sender-drop as **P1 open**, and §4 stamped **G2, G3, G4**
as **NOT EVIDENCED**. The certified per-cycle receipts contradict
both:

- **A5-2-RECEIPT.md** line 5: *"G2 (Durability / crash recovery) —
  R1 closed"*; line 8: *"Risks touched: R1 closed, R2 closed, R20
  re-pinned"* (v1.169.74, commit `957d0b0`).
- **A5-3-RECEIPT.md** line 5: *"G4 (concurrency correctness) — R3
  closed; R6 closed"*; line 7: *"Risks touched: R3 closed, R5
  closed, R6 closed, R12 closed, R4 partial"* (v1.169.75, commit
  `af346b9`).

A5-C-RR (`v1.169.83`) had fixed the roadmap **authority** (single
source of truth) but not the per-row **content**. A5-C-RR-A1 had
fixed three residual documentary defects (clippy overclaim, G1 lint
count, EvidenceAttachmentV1 PRE/POST-BASE classification). Neither
touched the contradiction between the live roadmap/risk register/
debt disposition and the receipts.

A5-C-RR2 is the single-budget reconciliation pass that closes that
contradiction.

## §1 Authority order (declared once, used throughout)

```text
1. released code + tests                                    (in tree at v1.169.84)
2. certified per-cycle receipt                             (A5-2-RECEIPT.md, A5-3-RECEIPT.md, …)
3. live roadmap snapshot                                    (A5-CURRENT-ROADMAP.md)
4. planning document / risk register / debt disposition    (A5-PLAN, A5-RISK-REGISTER, A5-DEBT-DISPOSITION)
```

A higher-numbered source **cannot contradict** a lower-numbered
source. When it does, the higher-numbered source is wrong and must
be reconciled — never the other way around.

This is the same order used by A5-C-RR and A5-C-RR-A1.

## §2 Per-row reconciliation (R1..R20)

Closing-receipt citations, SHAs, and live-evidence pointers.
Updated mirrors: `A5-RISK-REGISTER.md §1` and
`A5-DEBT-DISPOSITION.md §3.4.2`.

| ID | Risk | Status | Closing cycle | Commit | Live evidence |
|---|---|---|---|---|---|
| R1 | Restart survival / crash recovery | **CLOSED** | A5-2 / v1.169.74 | `957d0b0` | `sqlite_graph_store::latest_run_state_for` + 3 tests in `state_survives_restart.rs` |
| R2 | Corrupt / missing CAS object | **CLOSED** | A5-2 / v1.169.74 | `7ab113a` | CAS `get()` typed-error on truncation |
| R3 | Two writers overwrite (last-writer-wins) | **CLOSED** | A5-3 / v1.169.75 | `28abee3` | `INSERT` + `IdempotencyConflict`; 4 RED→GREEN tests in `concurrency_record_attempt.rs` |
| R4 | Authority decision → effect TOCTOU | **CLOSED** | A5-3 (R4-A) + A6-0..A6-4 (R4-B) / v1.169.80 | ADR-0130..0134 | `INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` CLOSED 2026-09-18 |
| R5 | Denied action still produces a side effect | **CLOSED** | A5-3 / v1.169.75 | — | `authority_fail_closed.rs` 6/6 green |
| R6 | Retry double-applies an effect | **CLOSED** | A5-3 / v1.169.75 | — | R3 typed-conflict machinery covers retry idempotency |
| R7 | Release tag ≠ certified SHA | **CLOSED** | A5-1 / v1.169.71 | — | `release_admission_check`; `INC-A4-RELEASE-VERSION-DRIFT` CLOSED |
| R8 | Corrupt / partial / stale public asset | **CLOSED_MECHANISM_OPEN_CLEAN_MACHINE** | A5-1 mechanism + A5-5 sweep owed | — | release pipeline 14/14 PASS; 9b public-release gate sha256-anchored; clean-machine revalidation pending A5-5 |
| R9 | Installed binary depends on repo checkout | **CLOSED** | A5-1 / v1.169.71 | — | distrib round-trip smoke test 14/14 PASS at every release |
| R10 | Empty `chore(release)` marker required | **CLOSED** | A5-1 / v1.169.71 | — | empty marker now rejected; docs-only pushes accepted (clause B) |
| R11 | Flaky test hidden by "couldn't reproduce" | **CLOSED_PARTIAL** | A5-5R / v1.169.81 (specific flake) | — | broader sweep owed by A5-5 |
| R12 | Non-blocking Parallel sender-drop | **CLOSED** | A5-3 / v1.169.75 | `af346b9` | non-blocking path deleted; 2 `par_006_*` tests deleted; `parallel_spans_three_ticks_drain` deleted |
| R13 | Dead compatibility code (`paradigm_lens::evaluate_lens`) | **CLOSED** | A5-4a / v1.169.82 | — | facade + type deleted; ADR-0135 |
| R14 | Secret leaks | **OPEN_NON_BLOCKER** | — | — | no observed leak; tracked for A5-5 / security cycle |
| R15 | Rollback to previous certified release | **CLOSED_MECHANISM_OPEN_CLEAN_MACHINE** | A5-1 mechanism + A5-5 owed | — | release pipeline 14/14 PASS |
| R16 | A5 change silently breaks A4 contract | **OPEN_NON_BLOCKER** | — | — | cross-crate ratchets + ADR-0001 promotion gates; no observed breach |
| R17 | Operator cannot diagnose production failure | **OPEN_NON_BLOCKER** | partial A5-4b / v1.169.83 | — | `sddk agent-help`; deeper sweep pending A5-5 / A5-C |
| R18 | Ignored tests become invisible debt | **CLOSED** | A5-ITD / v1.169.84 | `b4acbbf` | A5-C-RR-A1 §3.4.1 + per-ignored-test disposition |
| R19 | Deprecated-pattern lints stay `allow` with no disposition | **CLOSED** | A5-4b / v1.169.83 | — | `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs` 3/3 green |
| R20 | Migration of persisted state breaks upgrade | **CLOSED** (re-pinned) | A5-2 / v1.169.74 | — | G6 evidence preserved across v1.169.74 |

## §3 Disposition vocabulary additions

Added to `A5-RISK-REGISTER.md §0`:

| Disposition | Meaning |
|---|---|
| `CLOSED_MECHANISM_OPEN_CLEAN_MACHINE` | Mechanism + automated evidence complete; only clean-machine revalidation outstanding (A5-5 sweep). |
| `CLOSED_PARTIAL` | Sub-risk closed; broader scope still owed by another cycle. |

Existing vocabulary (`CLOSED`, `OPEN_BLOCKER`, `OPEN_NON_BLOCKER`,
`DEFER_POST_BASE`, `SUPERSEDED`) preserved.

## §4 Acceptance gates reconciliation

`A5-CURRENT-ROADMAP.md §4` updated with explicit authority
attribution and EVIDENCED status where the closing receipts
support it. Summary:

| Gate | Prior status (v1.169.83) | Reconciled status (v1.169.84) |
|---|---|---|
| G0 | EVIDENCED | EVIDENCED (unchanged) |
| G1 | partial | EVIDENCED (lint pin machine-verified at v1.169.83) |
| G2 | NOT EVIDENCED | **EVIDENCED** (A5-2 / R1 closed) |
| G3 | NOT EVIDENCED | **EVIDENCED** (A5-2 / R1 closure implies rebuildability) |
| G4 | NOT EVIDENCED | **EVIDENCED** (A5-3 / R3 + R6 + R12) |
| G5 | partial | **EVIDENCED** (A5-3 R4-A + A6-0..A6-4 R4-B) |
| G6 | partial | **EVIDENCED** (A5-1 + A5-2 R20) |
| G8 | partial | **EVIDENCED** (A5-1 + A5-2 + A5-4a + A5-4b) |
| G11 | (not previously listed) | OPEN_NON_BLOCKER (R14) |
| G13 | (not previously listed) | EVIDENCED (R18); R11 broader sweep owed |
| G15 | (not previously listed) | CLOSED_MECHANISM_OPEN_CLEAN_MACHINE (R15) |

No claim of `BASE_PRODUCTION_READY` is made by this reconciliation.
That claim requires either closing or explicitly accepting the
OPEN_NON_BLOCKER rows (R14, R15, R16, R17).

## §5 Files modified

| File | Change | Authority preserved |
|---|---|---|
| `docs/architecture/a5/A5-RISK-REGISTER.md` | REWRITE — risk matrix reconciled R1..R20; disposition vocabulary expanded; future async Parallel noted as POST-BASE | closed receipts untouched |
| `docs/architecture/a5/A5-CURRENT-ROADMAP.md` | §3 reordered (R1/R12 out, A5-ITD noted, SQLite-concurrency as next P1 candidate); §4 gate matrix EVIDENCED; §5 cold-start reordered | closed receipts untouched |
| `docs/architecture/a5/A5-DEBT-DISPOSITION.md` | §3.4 split into §3.4.0 (historical, preserved) + §3.4.1 (live) + §3.4.2 (R1/R12 closure addendum) | historical rows preserved verbatim; CLOSED A5-2 / CLOSED A5-3 suffix added |

No code, no Cargo bump, no release, no `docs/adr/` change.

## §6 Anti-encroachment — what was NOT touched

- `crates/**` — untouched.
- `Cargo.toml` — untouched (pre-push clause B applies: docs-only range).
- `docs/architecture/a5/A5-2-RECEIPT.md` — untouched (verbatim, A5-2 is the closing receipt for R1).
- `docs/architecture/a5/A5-3-RECEIPT.md` — untouched (verbatim, A5-3 is the closing receipt for R12).
- `docs/architecture/a5/A5-4a-RECEIPT.md` — untouched.
- `docs/architecture/a5/A5-4b-RECEIPT.md` — untouched.
- `docs/architecture/a5/A5-ITD-RECEIPT.md` — untouched.
- `docs/architecture/a5/A5-5R-RECEIPT.md` — untouched.
- `docs/architecture/a5/A5-1-RECEIPT.md` — untouched.
- `docs/debt/INC-R4-DECISION-EFFECT-ATOMICITY-BOUNDARY.md` — untouched.
- New ADRs — none created (reconciliation is not an architectural decision).
- EvidenceAttachmentV1 migration — NOT in scope (separate cycle budget).

## §7 Verification

| Check | Result | When |
|---|---|---|
| `cargo fmt --check` | PASS | after docs-only edits, before commit |
| Working tree | clean (only the 3 docs + this receipt) | before commit |
| Pre-push hook clause B (docs-only) | applies (no `crates/**` touched) | pending `git push` |
| No ADR-XXXX created | verified | n/a |
| Closed receipts verbatim | verified (no diff in git history for A5-2/A5-3/A5-4a/A5-4b/A5-ITD) | n/a |

## §8 Falsification (per spec §10)

**Q1: Is R1 still open?**
A: No. Closed by A5-2-RECEIPT.md / v1.169.74 / commit `957d0b0`.
Live code (`sqlite_graph_store::latest_run_state_for`) and live
tests (`state_survives_restart.rs`) back the closure. The
previously-ignored test was *corrected*, not merely un-ignored.

**Q2: Is R12 still open?**
A: No. Closed by A5-3-RECEIPT.md / v1.169.75 / commit `af346b9`.
The non-blocking path was deleted; 2 ignored `par_006_*` tests
deleted; runtime forces `pending_sender = None`. A future async
Parallel is a POST-BASE feature, NOT a closure of R12.

**Q3: Does `A5-CURRENT-ROADMAP.md` contradict the receipts?**
A: No. §3 no longer lists R1/R12 as P1; §4 lists G2/G3/G4 as
EVIDENCED with A5-2/A5-3 attribution; §5 cold-start reflects
SQLite-concurrency as the next P1 candidate, not R1/R12.

**Q4: Does `A5-RISK-REGISTER.md` contradict the receipts?**
A: No. §1 rows R1/R12 read CLOSED with closing-cycle, commit,
and live-evidence pointers.

## §9 What this cycle is NOT

- **Not** a claim of `BASE_PRODUCTION_READY`.
- **Not** a closure of OPEN_NON_BLOCKER rows (R14, R16, R17) —
  those stay open with current disposition.
- **Not** a removal of the SQLite-concurrency flake (P1 candidate
  awaiting its own cycle).
- **Not** an EvidenceAttachmentV1 migration (separate cycle budget).
- **Not** an A5-5 clean-machine sweep (A5-5 itself owes that).
- **Not** a resurrection of the deleted Parallel non-blocking path.

## §10 What comes next

STOP. The next cycle is **A5-SQLITE-CONCURRENCY-R** (proposed) or
the EvidenceAttachmentV1 MIGRATE_A5 slice, depending on the
operator's pick — both are **single-budget slices** with no
auto-promotion. A5-C-RR2 explicitly does NOT open the next cycle.

## §11 Receipt authority

- **Closing cycle:** A5-C-RR2.
- **Closing commit:** `<this cycle's commit>`.
- **Authority chain:** A5-C-RR2-RECEIPT.md (this file) →
  `A5-RISK-REGISTER.md §1` (live matrix) →
  `A5-CURRENT-ROADMAP.md §4` (gate status) →
  `A5-DEBT-DISPOSITION.md §3.4.2` (debt view).
- **Prior receipts (untouched, verbatim):**
  - A5-2-RECEIPT.md — closes R1/R2/R20.
  - A5-3-RECEIPT.md — closes R3/R4-A/R5/R6/R12.
  - A5-4a-RECEIPT.md — closes R13.
  - A5-4b-RECEIPT.md — closes R19; partial R17.
  - A5-ITD-RECEIPT.md — closes R18; closes `cli_phase_build_remediate_rejects_wrong_phase`, `verify_stream_chain_fails_on_tampered_hash` (OBSOLETE).
  - A5-5R-RECEIPT.md — partial R11 closure.
  - A5-1-RECEIPT.md — closes R7/R9/R10; partial R8/R15 mechanism.
  - A5-C-RR-A1-RECEIPT.md — fixes three residual documentary defects from A5-C-RR.
