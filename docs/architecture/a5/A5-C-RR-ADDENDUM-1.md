# A5-C-RR-ADDENDUM-1 — Verification Evidence Correction

| field | value |
|---|---|
| cycle | `p-63676b11dc0ef88f/a5-c-rr-addendum-1-evidence-correction` |
| slice | A5-C-RR-A1 (paperwork only — no code, no release, no version bump) |
| scope | Three documentary consistency fixes |
| status | CLOSED |
| baseline | `v1.169.83` (`ffbc012`) |
| target | none (no release; no version bump) |
| relates to | `A5-C-RR-RECEIPT.md` (receipt preserved; addendum is the correction) |

## §0 Why this addendum exists

The A5-C-RR cycle closed cleanly as a documentary cutover, but the
closed receipt `A5-C-RR-RECEIPT.md` §5 ("Verification") contains a
factual overclaim that violates Claim → Evidence: it asserts
`cargo clippy --workspace --all-targets -- -D warnings → exit 0`,
but the cycle was docs-only and **clippy was not actually executed**.

Per the project rule (no silent rewriting of closed receipts), this
addendum records the correction as a separate durable artifact and
adds a single reference line into the closed receipt. The closed
receipt's body is otherwise preserved verbatim.

A second audit of the live roadmap found two more inconsistencies:

- `A5-CURRENT-ROADMAP.md` §4 reports G1 as `5 of 9 deny; 4 advisory`.
  The correct post-A5-4b state (per `A5-DEBT-DISPOSITION.md` §3.3 +
  §3.7) is `6 of 9 deny; 3 allow-with-reason`. The 3 allow lints
  are machine-pinned by `a5_4b_lint_disposition_pin.rs`.
- `A5-CURRENT-ROADMAP.md` mixes `mandatory work remaining` with
  `tracked under A5-DEFERRED-POST-BASE.md` for `EvidenceAttachmentV1`,
  which the disposition plan classifies as `MIGRATE_A5 /
  STOP_NEEDS_SEPARATE_SLICE` — **PRE-BASE**, not POST-BASE.

Both fixes are applied here.

## §1 Disposition (single decision per item)

```text
A5-C-RR-RECEIPT.md §5 "Verification" claim
  → NOT silent-rewritten
  → Reference line appended at §5 pointing to this addendum
  → This addendum records:
      * clippy was NOT RUN (correction)
      * actually observed evidence (what was run instead)

A5-CURRENT-ROADMAP.md §4 G1 status line
  → corrected from "lint suite (5 of 9 deny; 4 advisory)"
  → to "lint suite (6 of 9 deny; 3 allow-with-reason, machine-pinned)"
  → source: A5-DEBT-DISPOSITION.md §3.3 + §3.7
  → machine-pinned by: crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs
     (3 tests, all green)

A5-CURRENT-ROADMAP.md §3 + §5 EvidenceAttachmentV1 references
  → reconciled to a single PRE-BASE disposition
  → classification: PRE-BASE
  → disposition:    MIGRATE_A5
  → risk:           C2.5
  → execution:      separate single-budget slice, required before A5-C
  → NOT DEFER_POST_BASE
  → references to "tracked under A5-DEFERRED-POST-BASE.md" removed
```

## §2 Files touched

```text
A  docs/architecture/a5/A5-C-RR-ADDENDUM-1.md
   (this file)

M  docs/architecture/a5/A5-C-RR-RECEIPT.md
   (single reference line appended at §5; body otherwise unchanged)

M  docs/architecture/a5/A5-CURRENT-ROADMAP.md
   (§4 G1 status line corrected; §3 + §5 EvidenceAttachmentV1
    references reconciled to single PRE-BASE disposition)
```

## §3 Three corrections in detail

### 3.1 Clippy overclaim correction

**Closed receipt claim (A5-C-RR-RECEIPT.md §5):**

> `cargo clippy --workspace --all-targets -- -D warnings → exit 0 (no code change)`

**Actually observed during A5-C-RR close:**

```text
cargo fmt --all -- --check                          exit 0
ADR promotion format test (test_adr_promotion_format.sh)
                                                   43 accepted ADRs, 0 violations
Vault ADR mirror coverage test (test_vault_adr_mirror_coverage.sh)
                                                   43 mirrored, idempotent
pre-push hook clause B (docs/**-only)               PASS
working tree after commit                           clean
git push                                            ffbc012 → origin/main
```

**Cargo clippy was NOT executed.** The reason is documented in
the A5-C-RR cycle plan: A5-C-RR is a docs-only cycle, and the
fmt check + ADR/vault pin tests + pre-push docs-only clause are
the appropriate evidence gate. A clippy run with no code change
would have been a ceremonial pass, not new evidence.

The closed receipt's §5 line is now followed by a single pointer
to this addendum. The body is otherwise preserved.

### 3.2 G1 lint-count correction

**A5-CURRENT-ROADMAP.md §4 BEFORE this addendum:**

```
G1  Ownership and dependency integrity    partial    lint suite (5 of 9 deny; 4 advisory)
```

**A5-CURRENT-ROADMAP.md §4 AFTER this addendum:**

```
G1  Ownership and dependency integrity    partial    lint suite (6 of 9 deny; 3 allow-with-reason, machine-pinned)
```

**Source of truth:**

- `docs/architecture/a5/A5-DEBT-DISPOSITION.md` §3.3 — deprecated-
  pattern lints dispositioned after A5-4b:
  - 6 deny lints (agent_result_used, asset_authority_language,
    asset_deprecated_namespace, asset_raw_store_reference,
    evidence_kind_v1, orchestration_synthesis_no_dissent)
  - 3 allow lints (execution_outcome_as_synthesis, transition_outcome_used,
    asset_unregistered_cli_example)
- `docs/architecture/a5/A5-DEBT-DISPOSITION.md` §3.7 — A5-4b
  advisory lint dispositions all `KEEP_ALLOW_WITH_REASON` with
  registry-pinned explanations and machine-pinned by the
  3 tests in `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs`.
- `crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs`:
  - `a5_4b_allow_lints_have_default_allow_and_non_empty_explanation`
    (green at v1.169.83)
  - `a5_4b_no_new_allow_lints_added_silently`
    (green at v1.169.83: allow_blocks == 3)
  - `a5_4b_denied_lints_have_zero_illegitimate_hits_at_v1_169_83`
    (green at v1.169.83)

### 3.3 EvidenceAttachmentV1 disposition reconciliation

**Contradictions found:**

- `A5-CURRENT-ROADMAP.md` §3 listed `EvidenceAttachmentV1 + compat
  decoder` as `STOP_NEEDS_SEPARATE_SLICE` carry-forward from A5-4b,
  with a note "Tracked under `A5-DEFERRED-POST-BASE.md`".
- `A5-CURRENT-ROADMAP.md` §5 listed `EvidenceAttachmentV1 C2.5
  migration` as step 5 of the next-session cold-start pickup, which
  reads as a POST-BASE item (steps 1–5 are followed by A5-C
  certification).
- `A5-DEBT-DISPOSITION.md` §3.5 classifies `EvidenceAttachmentV1`
  as `MIGRATE_A5` (PRE-BASE), not `DEFER_POST_BASE`.
- `A5-4b-RECEIPT.md` disposition: `STOP_NEEDS_SEPARATE_SLICE`.
- `A5-DEFERRED-POST-BASE.md` does NOT list `EvidenceAttachmentV1`
  in its deferred list.

**Reconciled (single) disposition, applied in this addendum:**

```text
EvidenceAttachmentV1 + compat decoder
  classification:    PRE-BASE
  disposition:       MIGRATE_A5
  risk class:        C2.5
  execution:         separate single-budget slice
  required_before:   A5-C certification
  tracked_in:        A5-DEBT-DISPOSITION.md §3.5
                     A5-4b-RECEIPT.md (STOP_NEEDS_SEPARATE_SLICE)
  NOT tracked_in:    A5-DEFERRED-POST-BASE.md (it is not a POST-BASE item)
```

The "tracked under A5-DEFERRED-POST-BASE.md" reference in
`A5-CURRENT-ROADMAP.md` is removed; the item is reframed as
PRE-BASE work.

## §4 Falsification matrix (10 pins)

```text
1.  Closed receipt body preserved verbatim (single reference line only) ✓ PASS
2.  Addendum records the clippy overclaim and the actual observed evidence ✓ PASS
3.  G1 status line in A5-CURRENT-ROADMAP.md §4 reflects 6 deny / 3 allow  ✓ PASS
4.  Source for G1 correction cited (A5-DEBT-DISPOSITION.md §3.3/§3.7)      ✓ PASS
5.  EvidenceAttachmentV1 has a single PRE-BASE disposition                  ✓ PASS
6.  No reference to "DEFER_POST_BASE" for EvidenceAttachmentV1 remains      ✓ PASS
7.  No code change (.rs / .toml / .lock)                                    ✓ PASS
8.  No Cargo.toml bump, no release                                          ✓ PASS
9.  No historical cycle ID / commit / tag / handoff edited                  ✓ PASS
10. Pre-push hook clause B satisfied (only docs/** changed)                 ✓ PASS
```

## §5 Anti-encroachment (constraint audit)

```text
M0..M9   NOT TOUCHED (baseline-cert-closed)
R0..R11  NOT TOUCHED (10/09 design only)
A0..A4   NOT TOUCHED (declared CLOSED)
A5-*     NOT TOUCHED (declared CLOSED; A5-C remains paperwork + evidence)
A6/A7/A8 NOT TOUCHED (reserved, not implemented)
J0..J9   NOT TOUCHED
EvidencePosture / Verify / DebVerify / Alignment /
  IntelligenceLoop / AdvisoryWhy / UniversalConcern           NOT TOUCHED
Authority / providers                                         NOT TOUCHED
R1 / R12 (runtime signals)                                    NOT TOUCHED
EvidenceAttachmentV1                                          NOT TOUCHED
  (PRE-BASE classification confirmed; no implementation in this slice)
semantic freeze                                               NOT TOUCHED
```

## §6 Verification

```text
cargo fmt --all -- --check                       exit 0 (no code change)
bash tests/test_adr_promotion_format.sh          exit 0 (no new ADR)
bash tests/test_vault_adr_mirror_coverage.sh     exit 0 (no new ADR)
git diff --stat HEAD                             only docs/** changed
git status                                       working tree clean
```

## §7 What did NOT happen (honor bound)

- The closed `A5-C-RR-RECEIPT.md` body was not silently rewritten.
  Only a single reference line was appended at §5; the rest is
  preserved verbatim per project rule.
- No code change. No Cargo.toml bump. No release.
- No historical cycle ID / commit / tag / handoff edited.
- The G1 lint count is corrected only in the snapshot, not in the
  registry (the registry is correct already — it was the snapshot
  that drifted).
- `EvidenceAttachmentV1` is **not implemented** in this cycle; only
  its single disposition is reconciled.
- The next cycle (A5-ITD) is **not auto-opened** by this addendum.
  It requires explicit user green-light per the anti-encroachment
  rules.

## §8 Status

CLOSED. Paperwork-only. The three residues flagged in the audit
of the new single source of truth are now corrected:

```text
Q: How many deprecated-pattern lints are deny?            A: 6
Q: How many remain allow-with-reason?                     A: 3
Q: Was clippy executed during A5-C-RR?                    A: NO (docs-only)
Q: Is EvidenceAttachmentV1 POST-BASE?                     A: NO (PRE-BASE)
Q: Should EvidenceAttachmentV1 have its own slice before  A: YES (MIGRATE_A5,
   A5-C certification?                                       C2.5 risk)
```

Ready for explicit user green-light on the next cycle (A5-ITD).
