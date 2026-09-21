# Session-Close Handoff — 2026-09-16 (A4-2)

## TL;DR

A4-2 **Generic DebVerify kernel** shipped as **v1.169.47** per
`arch-spec-044`. The cycle followed the A-full path: explore → spec →
design (folded into orchestrator) → tasks (folded) → apply → verify →
release → archive.

Local binary: **sddk 1.169.47** at `/home/rubentxu/.local/bin/sddk`.
Bundle: **1.169.47** at `/home/rubentxu/.local/share/sddk/framework/1.169.47/`.
Tag: **v1.169.47** on remote `main`.
Cycle dir + receipts: `.sddk/cycles/p-63676b11dc0ef88f-a4-2-generic-debverify/`.

---

## Current state of the project

- **HEAD**: `4068c4a chore(release): bump version to 1.169.47`
- **Branch**: `main`, clean, in sync with `origin/main`.
- **Working tree**: clean.
- **Binary installed**: `sddk 1.169.47`, source=current.
- **GH releases**: v1.169.42 (FU-A3-CO-2), v1.169.46 (A4-1), **v1.169.47 (A4-2)**.

---

## Cycle — A4-2 Generic DebVerify (SHIPPED)

**Path**: A-full (explore → spec → design‖tasks → apply → verify → release → archive)
**Outcome**: Baseline-challenged sibling kernel to `verify_kernel`. Two strategies ship.

### Implementation summary

**Engine** — `crates/sddk-engine/src/debverify_kernel/` (7 files):

- `mod.rs` — public surface, `DebVerifyKernel::reconcile()`, `default_strategy_set()`
- `types.rs` — closed ADTs: `Baseline`, `BaselineHash`, `ReconciliationScope`,
  `ReconciliationSummary` (6-variant), `ContradictionSet`, `GapSet`,
  `DebtItem`, `DebtItemKind`, `TriggerCondition`, `ChallengeFindingKind`,
  `StaleSet`, `SubjectId` (alias for `SoftwareEntityRef`)
- `strategy.rs` — `ChallengeStrategy` trait + `ChallengeOutcome` + `ChallengeFinding`
- `strategy_set.rs` — `ChallengeStrategySet` with deterministic BTreeMap
- `strategy_architecture.rs` — `ArchitectureChallengeStrategy` (wraps AC5 via
  `audit_to_findings` bridge)
- `strategy_observation_contradiction.rs` — `ObservationContradictionChallengeStrategy`
- `tests.rs` — 20 invariant tests + 8 falsification tests = 28 total

### Two strategies (proves generic, not "AC5 renombrado")

1. **ArchitectureChallengeStrategy** — wraps AC5's
   `architecture_debverify::run_debverify_audit`. The bridge function
   `audit_to_findings(&DebVerifyAudit) -> Vec<ChallengeFinding>` is the
   only place AC5 types cross into the kernel vocabulary. The five
   detectors (`authority_bypass`, `contradiction`, `missing_owner`,
   `shadow_authority`, `stale_compatibility`) are NOT re-implemented
   here; they are called through the AC5 substrate.

2. **ObservationContradictionChallengeStrategy** — consumes arch-spec-042
   `SoftwareObservation` pairs. Groups observations by `RelationId` and
   surfaces `ContradictionSet` for every relation that has both
   `Affirms` and `Denies` observations. Both observation ids are kept
   in the output; **no latest-wins**.

### Shape invariants

| | Verify (A4-1) | DebVerify (A4-2) |
|---|---|---|
| input | `ChangeBasis` | `Baseline` |
| runs with no change? | nothing affected | **yes** |
| result type | `VerificationResult` (5-variant) | `ReconciliationSummary` (6-variant) |
| score field | none | none |
| closure | ConfirmedBaseline / Contradiction / Staleness / EvidenceGap / AcceptedDebt / NotApplicable |

The two result types are deliberately distinct. No conversion exists
between them. They share `SoftwareEntityRef`, `ObservationSet` and
content-addressed identity primitives but not the receipt vocabulary.

### Baseline identity

```text
BaselineHash = sha256(sddk.baseline.id.v1 | scope_name | evidence_sha256)
```

Wall clock, rendering, messages, severity textual, and producer label
are excluded by construction — the `Baseline` struct has no fields that
could vary with time.

### Debt semantics

```text
DebtItem.accepted() == decision_ref.is_some() && revisit_trigger.is_some()
```

`Accepted` debt is reported but not enforced. The kernel never tells
the caller to fix anything; it surfaces what was found.

### 20 invariant tests

1. DebVerify takes Baseline, not ChangeBasis (compile-time)
2. Same Baseline → same ReconciliationSummary (runtime)
3. Wall clock excluded from baseline identity (runtime + structural)
4. Strategy order does not affect summary (runtime)
5. Strategy registry is open (runtime)
6. ArchitectureChallenge produces findings equivalent to AC5 audit (runtime)
7. ObservationContradiction produces findings on real data (runtime)
8. Affirm + Deny on same relation → Conflicted, both kept (runtime)
9. No latest-wins resolution (runtime + structural)
10. Missing evidence → EvidenceGap, not ConfirmedBaseline (runtime)
11. Stale observation does not become Verified (structural)
12. Accepted debt preserves decision_ref + revisit_trigger (runtime)
13. Verify clean + DebVerify finds drift (and vice versa) (runtime)
14. DebVerify can find a subject with no recent edits (runtime)
15. No universal score field (structural)
16. No canonical mutation from audit (structural)
17. No provider SDK types (structural)
18. No Alignment→Governance shortcut (structural)
19. Deterministic serde + ordering (runtime)
20. A4-0 + A4-1 + A3 baselines still green (runtime)

### 8 falsification tests

- baseline identity excludes git diff
- strategy order irrelevant in findings
- newer observation does not overwrite older
- gap does not render as clean
- ArchitectureChallenge does not bypass AC5 (uses audit_to_findings bridge)
- no strategy mutation during evaluation
- baseline identity excludes timestamps
- kernel works for non-architecture (ObservationContradiction runs in
  `non-arch-scope` and still surfaces contradictions)

### Test evidence at v1.169.47

- `sddk-engine::debverify_kernel`: 28/28 pass
- `sddk-engine` (total): 1187 lib tests, 0 failed
- Workspace: 0 FAILED
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --check`: clean

### Release evidence

`gh release view v1.169.47`:
- Tag: `v1.169.47`, published 2026-09-16T10:18:51Z
- URL: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.47
- 9 assets (sddk, unified tarball, bundle tarball, CHECKSUMS, sbom.json, sha256, gh-release-receipt.json)
- `scripts/release.sh`: 14/14 steps passed (exit 0)
- Install from URL round-trip: PASS (binary + bundle coherent after prune)

---

## Cycle artifacts (A4-2)

Written under `.sddk/cycles/p-63676b11dc0ef88f-a4-2-generic-debverify/`:
- `release-receipt.json` — release provenance
- `merge-receipt.json` — linear main merge (2 commits)
- `archive-manifest.json` — phase=release.complete, lessons_learned included

NOTE: `.sddk/cycles/` is in `.gitignore` (cycle state is local-only).

---

## What changed in the codebase

### New files (7)
- `crates/sddk-engine/src/debverify_kernel/{mod,types,strategy,strategy_set,strategy_architecture,strategy_observation_contradiction,tests}.rs`

### Modified files (3)
- `crates/sddk-engine/src/lib.rs` — registered `debverify_kernel` module
- `docs/architecture/specs/arch-spec-044-generic-debverify.md` — status:
  contract-ready → implemented (cycle-time, not deferred)
- `docs/architecture/adrs/ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT.md`
  — added `debverify_kernel` to `implementation_evidence`

### Compat surfaces surviving (for A4-2M)

- `architecture_debverify` (AC5) — used as substrate; not modified, not deleted
- `architecture_debverify::run_debverify_audit` — still the runtime entry;
  A4-2M will route it through DebVerifyKernel
- `DebVerifyAudit` / `DebVerifyFinding` / `DebVerifyFindingKind` /
  `FindingSeverity` — unchanged
- `architecture receipt` / `architecture findings` — unchanged

---

## Lessons learned (for AGENTS.md or future cycles)

1. **Closed ADT + `#[derive(Serialize)]` cascades.** Every nested type in
   `ReconciliationSummary` variant payloads needs its own `Serialize`
   derive or the derive fails. Add derives alongside the type, not at
   release time.
2. **Box<dyn Trait> in a registry requires strategy-by-value**, not
   `&dyn Trait`. `Box::new(&S)` produces `Box<&S>` which cannot coerce
   to `Box<dyn Trait>`. Take the strategy by value; zero-sized structs
   avoid allocation.
3. **Promotion cycle-time.** Promoting arch-spec from contract-ready
   to implemented + updating ADR implementation_evidence *during the
   cycle* (not deferred to release) avoids the fitness-gate-at-release
   pitfall that hit A4-1 (where `no_new_root_level_context_module_without_adr`
   fired only when `scripts/release.sh` ran).
4. **Generic-kernel proof requires heterogeneous strategies.** Shipping
   only the architecture adapter would have been "AC5 renombrado".
   Adding `ObservationContradictionChallengeStrategy` (no architecture
   dependency) demonstrates that `DebVerifyKernel::reconcile` is a real
   kernel that composes strategies.
5. **`Baseline` identity by construction excludes wall clock.** The
   type has no field that varies with time, so the invariant is enforced
   at compile time rather than by convention. This survives any test
   that mutates time.
6. **`DebVerifyKernel::reconcile` deliberately cannot take a
   `ChangeBasis`.** Compile-time separation from `VerifyKernel`
   prevents the "verify --all" anti-pattern that arch-spec-044
   explicitly rejects.

---

## Next steps

### Immediate (next session)

- **A4-2M — AC4/AC5 convergence**: zero-feature-change migration of old
  paths onto generic kernels. Must satisfy:
  - old `architecture receipt` == new `architecture receipt`
  - old `architecture findings` == new `architecture findings`
  - old WHY chain == new WHY chain
  - delete any engine code that became redundant after the cutover

### Roadmap after A4-2M

- A4-3 — Software Alignment domain (arch-spec-045)
- A4-4 — Intent + generic lenses (arch-spec-046)
- A4-5 — Integration / advisory / WHY (arch-spec-047)
- A4 closeout

### Strategy additions (separate cycles)

- KnowledgeFreshnessChallenge (KMT-based, separate slice)
- DecisionStalenessChallenge (separate slice)
- Static / runtime provider strategies (separate slices)

---

## Open questions for the next session

1. Should A4-2M come immediately, or should we first add a third
   strategy to DebVerify (KnowledgeFreshness) to deepen the kernel
   before cutting AC4/AC5 over?
2. The architecture challenge in A4-2 returns no findings when there
   is no overlay/contracts input — should the CLI/integration layer
   always pre-load these, or should the strategy error path be
   surfaced as a finding of `kind=StrategyError`?
3. `ReconciliationSummary::ConfirmedBaseline { strategies_run: usize }`
   exposes a count. Should that be removed (count is information; does
   it belong in a closed ADT that promises no numeric fields)?
4. The two strategies in `default_strategy_set()` produce different
   output kinds (Findings vs Contradictions). The kernel composes both
   into a single `ReconciliationSummary`. Is this composition
   always-safe (e.g., a `Contradiction` always wins over a
   `ConfirmedBaseline` — yes by construction, but should this be
   documented in the spec?).

---

## File pointers for next session

- `docs/architecture/specs/arch-spec-044-generic-debverify.md` — canonical spec
- `crates/sddk-engine/src/debverify_kernel/` — implementation
- `docs/architecture/adrs/ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT.md` — kernel-distinction ADR (now has A4-2 evidence)
- `crates/sddk-engine/src/architecture_debverify/` — AC5 substrate (will be routed through kernel in A4-2M)
- `.sddk/cycles/p-63676b11dc0ef88f-a4-2-generic-debverify/archive-manifest.json` — session summary

---

**Cycle close: A4-2 Generic DebVerify — SHIPPED v1.169.47**

**STOPPED per user instruction.** Next cycle (A4-2M) will be a separate
session, not auto-started.
