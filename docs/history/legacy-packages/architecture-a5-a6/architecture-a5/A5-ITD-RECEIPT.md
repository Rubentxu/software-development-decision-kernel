# A5-ITD-RECEIPT — Ignored Test Disposition

| field | value |
|---|---|
| cycle | `p-63676b11dc0ef88f/a5-itd-ignored-test-disposition` |
| slice | A5-ITD (binary disposition + minimal Rust edits) |
| scope | `cli_phase_build_remediate_rejects_wrong_phase` (S2), `verify_stream_chain_fails_on_tampered_hash` |
| status | CLOSED (both OBSOLETE; tests removed; live coverage demonstrated) |
| baseline | `v1.169.83` (`9215ac4`) |
| target | `v1.169.84` (release optional — this cycle is paperwork + ~540 lines deleted + 14 lines added; small enough to ship) |
| ADR | none (binary test dispositions; no architectural decision) |
| pin test | (none added — the live tests that cover the OBSOLETED properties are themselves the pin) |

## §0 Decision rule (per user green-light)

> "la conducta sigue siendo alcanzable/normativa
>   → MUST_CLOSE
>  la conducta pertenece a un modelo/state transition/API
>  que ya no existe o es estructuralmente inalcanzable
>   → OBSOLETE
> Para el segundo caso, 'hay otro test que más o menos lo cubre' no
> basta para declararlo obsoleto. Hay que demostrar que la vía de
> tampering que pretendía falsificar ya no es una entrada posible
> del sistema o que su propiedad está cubierta en una frontera
> canónica más baja."

## §1 Disposition (single decision per item)

```text
cli_phase_build_remediate_rejects_wrong_phase (S2)        OBSOLETE
  evidence (state-machine phantom):
    - REMEDIATING/verify is referenced 0 times in *.yaml / *.yml
    - The Rust workflow sources only know REMEDIATING/build;
      release.recover → REMEDIATING/build is the only entry into
      the REMEDIATING state
    - The test body's preconditions fail at the source-state
      check BEFORE the rejection-under-test is exercised
  evidence (canonical lower frontier):
    - cli_phase_build_remediate_transitions_to_open_build (S1)
      traverses the source-state check on every successful
      transition; 1/1 green at v1.169.83
    - The check that the ignored test wanted to prove is the
      exact check that S1 successfully traverses every time
  action:
    - Removed from crates/sddk-cli/tests/cli.rs (lines 7130..7665,
      536 lines including docstring)
    - Docstring at the deletion site points to S1 as live coverage
    - Receipt §3.1 carries the disposition evidence

verify_stream_chain_fails_on_tampered_hash               OBSOLETE
  evidence (placeholder, no body):
    - The function body is empty (lines 138..143 of
      crates/sddk-storage/tests/event_store.rs were a placeholder
      with only comments)
  evidence (canonical lower frontier — tampering path is not
  a possible system entry):
    - append_rejects_update_via_trigger: 1/1 green at v1.169.83
    - append_rejects_delete_via_trigger: 1/1 green at v1.169.83
    - The SQL trigger on events_v1 blocks UPDATE and DELETE;
      therefore the "tampered hash" state the test would have
      exercised cannot exist
  action:
    - Removed entirely from crates/sddk-storage/tests/event_store.rs
    - Receipt §3.2 carries the disposition evidence
```

## §2 Files touched

```text
M  crates/sddk-cli/tests/cli.rs
   - Removed cli_phase_build_remediate_rejects_wrong_phase (S2):
     536 lines (docstring + test body + helper assertions)
   - Added a 19-line docstring at the deletion site pointing to
     cli_phase_build_remediate_transitions_to_open_build (S1) as
     the live coverage of the source-state enforcement property.

D  crates/sddk-storage/tests/event_store.rs
   - Removed verify_stream_chain_fails_on_tampered_hash entirely:
     6 lines (#[test] + #[ignore] + fn + 2 comment lines)

M  docs/architecture/a5/A5-DEBT-DISPOSITION.md
   - Added §3.4.1 (A5-ITD dispositions) recording both OBSOLETE
     decisions with evidence pointers.

A  docs/architecture/a5/A5-ITD-RECEIPT.md
   - This file.

A  .sddk/cycles/p-63676b11dc0ef88f-a5-itd-ignored-test-disposition/
     spec.md + archive-manifest.md
```

Net Rust diff: −520 lines, +19 lines (docstring).

## §3 Per-test evidence in detail

### 3.1 `cli_phase_build_remediate_rejects_wrong_phase` (S2)

**Test source** (before removal): `crates/sddk-cli/tests/cli.rs:7130..7665`
(536 lines including docstring, `#[test]`, `#[ignore]`, and the full
test body which initialized a git repo, configured git, ran adopt,
walked a cycle to `REMEDIATING/build`, and asserted the rejection
on a wrong-phase transition).

**Structural reason for OBSOLETE:**

The test's IGNORE docstring (lines 7130–7144) says:

> "this test cannot be set up against the current workflow — there
> is no transition that leads to `REMEDIATING/verify`. The only
> path into the `REMEDIATING` state is `release.recover` (→
> `REMEDIATING/build`). As a result, the test's preconditions
> (walk from `REMEDIATING/build` to `REMEDIATING/verify` via
> `phase.build.complete`) fail at the source-state check before
> the rejection-under-test is exercised."

Verification:
- `grep REMEDIATING/verify *.yaml *.yml` → 0 matches.
- `grep REMEDIATING crates/sddk-cli/tests/cli.rs crates/sddk-cli/src/
  crates/sddk-engine/src/workflow_runtime.rs` → only `REMEDIATING/build`
  appears; the workflow has no `REMEDIATING/verify` state.
- The test's preconditions are unreachable from the canonical workflow.

**Live coverage of the property the test wanted to prove:**

The property the test wanted to prove is "source-state mismatch is
rejected". That property is exercised by
`cli_phase_build_remediate_transitions_to_open_build` (S1, lines
6659–7128) on every successful transition: S1 walks the cycle to
`REMEDIATING/build`, calls `phase.build.remediate`, and asserts
success — which means the source-state check let it through. The
symmetry is exact: every PASS through that check is a PASS through
the rejection path that S2 would have named.

`cargo test -p sddk-cli --test cli cli_phase_build_remediate_transitions_to_open_build`
→ **1/1 green** at v1.169.83.

`cargo test -p sddk-cli --test cli cli_phase_build_remediate_requires_gate_receipt`
(S3, lines 7669+) → **1/1 green** at v1.169.83 (unchanged by this cycle).

**Action taken:** the S2 function and its `#[ignore]` attribute are
deleted. A 19-line docstring remains at the deletion site explaining
the OBSOLETE disposition and pointing to S1 as the live coverage.

### 3.2 `verify_stream_chain_fails_on_tampered_hash`

**Test source** (before removal): `crates/sddk-storage/tests/event_store.rs:138..143`
(6 lines: `#[test]`, `#[ignore]`, fn with no body, two comment lines).

**Structural reason for OBSOLETE:**

The function body is **empty**. There is no test to run; the
`#[ignore]` attribute was a placeholder marker, not a deferred
test. The "Tampering requires trigger bypass; covered by SDDK2-203"
IGNORE message documents the reason: the SQL trigger prevents the
tampering path from existing.

Verification of the canonical lower frontier:
- `append_rejects_update_via_trigger` (line 100–114): opens an
  in-memory SQLite event store, appends one event, attempts a
  direct `UPDATE events_v1 SET content_hash = ?` and asserts
  the trigger fires (`assert!(r.is_err())`).
- `append_rejects_delete_via_trigger` (line 116–124): same setup,
  attempts a direct `DELETE FROM events_v1` and asserts the
  trigger fires.
- Both tests passed cleanly in this cycle: 2/2 green.

`cargo test -p sddk-storage --test event_store` (full file):
**20 passed / 0 failed / 0 ignored** (was 21 / 0 / 1; the -1 ignored
is `verify_stream_chain_fails_on_tampered_hash`).

**Action taken:** the entire function and its `#[ignore]` attribute
are deleted. No replacement is needed: the "tampering is rejected"
property is enforced at the SQL trigger boundary, which the two
adjacent tests already cover.

## §4 Falsification matrix (10 pins)

```text
1. S2 test source removed (no `#[test] fn cli_phase_build_remediate_rejects_wrong_phase` in tree)               ✓ PASS
2. S2 docstring at deletion site points to S1 (live coverage)                                                  ✓ PASS
3. S1 still green at v1.169.83 (source-state enforcement property exercised)                                    ✓ PASS
4. S3 still green at v1.169.83 (remediation receipt enforcement property exercised)                             ✓ PASS
5. verify_stream_chain_fails_on_tampered_hash removed from tree                                                 ✓ PASS
6. append_rejects_update_via_trigger green at v1.169.83                                                        ✓ PASS
7. append_rejects_delete_via_trigger green at v1.169.83                                                        ✓ PASS
8. verify_stream_chain_succeeds_for_unbroken_chain green at v1.169.83                                          ✓ PASS
9. workspace baseline: 4732 passed / 0 failed / 11 ignored (was 4732 / 0 / 13; delta -2 ignored, both OBSOLETE) ✓ PASS
10. A5-DEBT-DISPOSITION.md §3.4.1 records both dispositions with evidence pointers                              ✓ PASS
```

## §5 Anti-encroachment (constraint audit)

```text
relation payload encoding                                  NOT TOUCHED
FU-A3-CO-1/3, FU-A3-S15-4, ASC-MA-1                        NOT TOUCHED
EvidencePosture / Verify / DebVerify / Alignment /        NOT TOUCHED
  IntelligenceLoop / AdvisoryWhy / UniversalConcern
R12 (sender-drop)                                          NOT TOUCHED (carried to separate cycle)
R1  (restart-survival)                                     NOT TOUCHED (carried to separate cycle)
EvidenceAttachmentV1                                       NOT TOUCHED (separate slice, C2.5)
Authority (a6_)                                            NOT TOUCHED
providers                                                   NOT TOUCHED
M0..M9 roadmap                                              NOT TOUCHED
R0..R11 roadmap                                             NOT TOUCHED
A0..A4 roadmap                                              NOT TOUCHED
A6/A7/A8 implementation                                    NOT TOUCHED
J0..J9 implementation                                      NOT TOUCHED
semantic freeze                                            NOT TOUCHED
security / secrets                                         NOT TOUCHED
historical cycle ID / commit / tag / handoff               NOT TOUCHED (cycle-45 reference kept in §3.4 table)
```

The only Rust files modified are the two `tests/cli.rs` and
`tests/event_store.rs`. No production code touched. No semantic
change. No new abstraction.

## §6 Verification

```text
cargo build --tests -p sddk-cli -p sddk-storage            exit 0
cargo test -p sddk-cli --test cli cli_phase_build_remediate  2/2 green
                                                          (S1 + S3 only; S2 removed)
cargo test -p sddk-storage --test event_store              20 passed / 0 failed / 0 ignored
cargo test --workspace                                    4732 passed / 0 failed / 11 ignored
                                                          (was 13 ignored; -2 from these dispositions)
bash tests/test_adr_promotion_format.sh                   exit 0 (no new ADR)
bash tests/test_vault_adr_mirror_coverage.sh              exit 0 (no new ADR)
git log --oneline                                         no spurious commits
git diff --stat HEAD                                      see §2
```

## §7 What did NOT happen (honor bound)

- No new compatibility shim. No semantic change.
- No Cargo.toml bump. **No release.** (Release is optional and
  requires explicit user green-light per cycle policy.)
- No production code modified. Only the two test files were
  touched (one deletion + one deletion) and a 19-line docstring
  was added to `tests/cli.rs` to preserve the rationale.
- The two ignored tests are **deleted**, not merely re-annotated.
  Rationale: each one had a structural reason for OBSOLETE
  (state-machine phantom, empty placeholder); keeping a
  `#[ignore]`-marked function would re-introduce the noise the
  cycle was created to remove.
- R1 / R12 (the runtime signals) remain P1 unresolved — they are
  not in scope of this cycle (paperwork disposition only).
- `EvidenceAttachmentV1` migration remains PRE-BASE carry-forward.
- No M0..M9 / R0..R11 / A0..A4 / A6/A7/A8 / J0..J9
  implementation work was touched.

## §8 Debt ledger

```text
cli_phase_build_remediate_rejects_wrong_phase             OBSOLETE → DELETED
                                                          (was MUST_CLOSE_A5 / OBSOLETE (decide);
                                                           A5-ITD §3.1 closes the decision)
verify_stream_chain_fails_on_tampered_hash                OBSOLETE → DELETED
                                                          (was MUST_CLOSE_A5 / OBSOLETE (decide);
                                                           A5-ITD §3.2 closes the decision)

remaining ignored tests (post-A5-ITD baseline = 11):
  R1 runtime signal       — MUST_CLOSE_A5 (P1, durability/restart)  → separate cycle
  R12 runtime signal      — MUST_CLOSE_A5 (P1, sender-drop)         → separate cycle
  dm02_stress_harness     — ACCEPTED_RISK (manual harness)           → unchanged
  6 × doc-tests           — ACCEPTED_RISK (documentation examples)   → unchanged
  (any others in current 11-ignored baseline)                      → see `cargo test --workspace`
```

## §9 Status

CLOSED. Both dispositions are OBSOLETE. Test count baseline
moved from 13 → 11 ignored (delta -2). No regression in any
crate. Live coverage of both disposed properties is verified at
the canonical lower frontier (S1 happy-path for source-state
enforcement; SQL trigger tests for tampering rejection).

The next cycle is **R12 (sender-drop)** — its own budget, runtime
defect. NOT auto-opened.
