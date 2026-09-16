# A4-2M Cycle — AC4/AC5 Convergence (v1.169.48)

> Cycle: `p-63676b11dc0ef88f/a4-2m-ac4-ac5-convergence`
> Baseline: `v1.169.47` (post A4-2)
> Released: **v1.169.48** on 2026-09-16
> Path: **A-lite** (zero-feature-change migration)
> Final state: 4-way coherent (workspace=binary=bundle=tag=v1.169.48, HEAD = origin/main = `c846048`)

## What A4-2M did

Convergence slice. No new features, no finding types, no public surface
changes. The single execution of AC4 and AC5 now lives in one place per
side, and the generic kernels own the call paths.

### AC4 (Architecture Verify) — `verify_kernel` convergence

Before A4-2M, `ArchitectureVerificationDomain::evaluate` was a stub that
returned `Unknown`. The CLI called `compute_conformance_delta` directly.
There were **two parallel paths**: one in `architecture_conformance`, one
in `verify_kernel`. They shared nothing.

After A4-2M, there is **one core**:

```
crates/sddk-engine/src/architecture_conformance/compute.rs
    compute_conformance_delta_core()  ← single AC4 execution surface
```

Both `compute_conformance_delta` (legacy DTO, kept for receipt shape)
and `ArchitectureVerificationDomain::evaluate_with_context` (new path,
returns `VerificationResult`) call into the same core.

The change is structural, not behavioural: the byte-level delta output
is identical (verified by tests against fixtures, receipts, and the
`ac5_not_verify_full` / `ac6_ac4_witness_bridge` / `ac8_full_chain_receipt`
integration suites).

### AC5 (Architecture DebVerify) — `debverify_kernel` convergence

`ArchitectureChallengeStrategy` (A4-2) shipped with a stub
`challenge()` that returned `Findings(vec![])` when no overlay/contracts
were passed. That was a **false-clean trap**: the kernel would promote
it to `ReconciliationSummary::ConfirmedBaseline` and the caller would
conclude "all good" when in fact nothing was checked.

After A4-2M, the strategy returns `Gaps(...)` when audit inputs are
missing. The kernel's composition rule promotes that to
`ReconciliationSummary::EvidenceGap(...)`. This closes the false-clean
trap at the kernel level.

The single execution of the five detectors remains in
`architecture_debverify::run_debverify_audit`. The strategy is a
**port**; the CLI / integration layer is the **executor**. The
`audit_to_findings` bridge converts the audit's output into the
generic kernel vocabulary.

## Hard invariant — preserved

```text
old architecture receipt    == new architecture receipt
old architecture findings   == new architecture findings
old why architecture        == new why architecture
```

Verified by:
- 47 existing E2E CLI tests in `crates/sddk-cli/tests/architecture_*.rs`
  — all green, no fixture drift.
- 7 integration tests in `crates/sddk-engine/tests/ac*` exercising the
  `compute_conformance_delta` surface — all green.
- New `crates/sddk-engine/src/verify_kernel/adapter_architecture.rs`
  A4-2M tests:
  - `a4_2m_legacy_and_context_paths_agree_on_empty_inputs` — both paths
    agree on `Unknown` when the contract set is empty.
  - `a4_2m_trait_evaluate_without_context_returns_unknown` — the trait
    port returns `Unknown`, never `Verified`/`Contradicted`, without
    context.
  - `a4_2m_legacy_is_thin_wrapper_over_core` — the legacy DTO and the
    core produce byte-identical `id`, `plan_digest`, `contract_set_digest`,
    `graph_digest`, `evaluated_at`, `vector`, and matching `affected` /
    `claims` sizes.

## False-clean guard

`ArchitectureChallengeStrategy::challenge` (in
`crates/sddk-engine/src/debverify_kernel/strategy_architecture.rs`)
now returns `Gaps(...)` when the audit cannot run. The kernel's
composition rule (Contradiction > EvidenceGap > Staleness > AcceptedDebt
> ConfirmedBaseline) promotes this to `ReconciliationSummary::EvidenceGap(...)`.

The existing A4-2 test
`debverify_kernel::tests::verify_clean_debverify_drift` was updated: it
previously asserted `ConfirmedBaseline` when the architecture strategy
had no inputs, which was the false-clean trap encoded in a test. It now
asserts `EvidenceGap` to match the corrected behaviour.

## Files touched

| File | What changed |
|---|---|
| `architecture_conformance/compute.rs` | Extracted core: `compute_conformance_delta_core` |
| `architecture_conformance/mod.rs` | Re-exported `compute_conformance_delta_core` |
| `verify_kernel/adapter_architecture.rs` | Added `evaluate_with_context`; replaced stub; added 3 A4-2M tests |
| `debverify_kernel/strategy_architecture.rs` | `challenge()` returns `Gaps(...)` when inputs missing (false-clean guard) |
| `debverify_kernel/tests.rs` | Updated `verify_clean_debverify_drift` to assert `EvidenceGap` |
| `docs/architecture/specs/arch-spec-043-generic-verify.md` | `implemented_by` updated to A4-1 + A4-2M |
| `docs/architecture/specs/arch-spec-044-generic-debverify.md` | `implemented_by` updated to A4-2 + A4-2M |

CLI code (`architecture_cmd.rs`) is **unchanged**. The legacy DTO
contract (`ArchitectureConformanceDelta`) is **unchanged**. The
strategy API is **unchanged** (`ChallengeStrategy` trait still takes
`Baseline + ObservationSet`). What changed is the **execution
ownership**: the audit is called once at the call site, the strategy
is the port, and missing inputs produce `EvidenceGap` instead of
`ConfirmedBaseline`.

## Release

- 9 commits delta from `v1.169.47`:
  - `c846048 chore(release): bump version to 1.169.48`
  - `7adb4e8 feat(engine): A4-2M Architecture Verify/DebVerify convergence (arch-spec-043 + 044)`
  - `0230530 fix(coherence): revert post-handoff workspace bump to 1.169.47` *(A4-2 closeout carry-over)*
- 4-way coherence: workspace=`1.169.48`, binary=`1.169.48`,
  bundle=`1.169.48`, tag=`v1.169.48` (points at `c846048`).
- `scripts/release.sh`: 14/14 PASS, exit 0.
- `cargo test --workspace`: 0 FAILED.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- 1 transient flaky UAT/Playwright test (`uat_stale_tests::stale_detects_geometry_change`) failed first run, passed on re-run isolated. Not a regression; infra flake.

## What's NOT in scope (per user mandate)

- KnowledgeFreshnessChallenge
- DecisionStalenessChallenge
- Software Alignment (A4-3)
- New lenses / paradigms
- CogniCode / Chronos / JCode
- New CLI commands
- New finding kinds / severities
- Public receipt format changes

## Next session — STOP before A4-3

Per user instruction, this cycle ends here. **A4-3 (Alignment core,
arch-spec-045, 7 closed states) is NOT auto-started.**

State to resume from:
- HEAD = origin/main = `c846048`
- binary=bundle=tag=**v1.169.48**
- Cycle `p-63676b11dc0ef88f/a4-2m-ac4-ac5-convergence` will need
  lease re-acquired (was lost across session boundary)
- Roadmap pinned: A4-3 → A4-4 → A4-5 → A4 closeout → A5 BASE_PRODUCTION_READY

## See also

- Spec: `.sddk/cycles/p-63676b11dc0ef88f-a4-2m-ac4-ac5-convergence/spec.md`
- ADR-0123 implementation_evidence updated with A4-2M anchors
- Previous handoff: `docs/handoff/HANDOFF-2026-09-16-a4-2-generic-debverify-and-v1.169.47-release.md`
