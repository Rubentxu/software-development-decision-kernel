---
id: arch-spec-044-generic-debverify
title: Generic DebVerify — baseline-challenge and reconciliation kernel
status: implemented
milestone: A4
implemented_by: A4-2 + A4-2M (crates/sddk-engine/src/debverify_kernel, AC5 adapter + ObservationContradictionChallenge; AC4/AC5 convergence via single-execution cores)
depends_on: arch-spec-042-evidence-observation-provenance + arch-spec-043-generic-verify
---

# arch-spec-044 — Generic DebVerify

> **Implemented in A4-2** as the `debverify_kernel` engine module with two
> strategies: `ArchitectureChallengeStrategy` (wrapping AC5's
> `run_debverify_audit`) and `ObservationContradictionChallengeStrategy`
> (consuming `SoftwareObservation` pairs from arch-spec-042).
>
> See `crates/sddk-engine/src/debverify_kernel/` for the working implementation
> and `docs/architecture/adrs/ADR-0123` for the design rationale.

## Purpose

DebVerify challenges the **baseline**, including where nothing changed recently.
It can discover stale or wrong knowledge in areas no recent edit touched.

It is not a "verify everything" mode. **It is a different shape of reasoning.**

```text
Verify     delta-scoped         — driven by ChangeBasis
DebVerify  baseline-challenged  — driven by Baseline + ChallengeStrategies
```

## Shape

```text
ReconciliationScope
   + Baseline
   + ChallengeStrategySet
   + EvidenceSources
        ↓
   ChallengePlan
        ↓
   ReconciliationRun
        ↓
   Findings / Contradictions / Gaps / DebtDelta
        ↓
   ReconciliationSummary
        ↓
   BaselineAssessment (with explicit accepted-debt)
```

No change set is required as input. A baseline-challenge pass **can and must**
run on a project with no recent edits.

## Constraints (REQ-A4S2-001..021)

### Shape constraints

- **Polymorphic by strategy, not by domain.** One kernel; AC5, Knowledge
  freshness, Observation contradiction, and future challenges all run through
  the same shape with the same receipt vocabulary.
- **Reuse `arch-spec-042` observations** as the evidence channel. Strategies
  consume `SoftwareObservation`s; they do not mint a second evidence
  representation.
- **Baseline identity is content-addressed.** Reconciliation identity MUST be
  reproducible from `(Baseline, ChallengeStrategySet, EvidenceSet)` and MUST
  NOT include wall clock, rendering, messages or producer label.
- **Strategy order is irrelevant for the result.** ReconciliationSummary is
  the same regardless of strategy execution order (set semantics).
- **No latest-wins.** When a `SoftwareRelation` has one observation that
  affirms and another that denies, both are kept. Resolution is
  `EvidenceResolution::Conflicted` (A4-0 substrate), never "newest wins" or
  "highest confidence wins".
- **No universal score.** Findings carry closed kinds and severities; no
  numeric weight exists.

### State vocabulary (closed ADT)

ReconciliationSummary is the union of these states. No `bool` + `Option<String>`:

```text
ConfirmedBaseline           — every strategy found no challenge
Contradiction(ContradictionSet) — at least one contradiction reconciled
Staleness(StaleSet)         — at least one stale knowledge / observation
EvidenceGap(GapSet)         — required evidence absent for some subject
AcceptedDebt(DebtDelta)     — known debt present and accepted
NotApplicable               — scope declared but no strategies applicable
```

`Unknown` (missing evidence) is rendered as `EvidenceGap`, never as `ConfirmedBaseline`.

### Strategy protocol

```rust
pub trait ChallengeStrategy: Send + Sync {
    fn name(&self) -> &'static str;
    fn applicable(&self, scope: &ReconciliationScope) -> bool;
    fn challenge(
        &self,
        baseline: &Baseline,
        evidence: &EvidenceSet,
    ) -> Result<ChallengeOutcome, ChallengeError>;
}
```

`ChallengeOutcome` is:

```text
Findings(Vec<ChallengeFinding>) | Contradictions(Vec<Contradiction>) | Gaps(Vec<EvidenceGap>)
```

The kernel composes strategies via a `ChallengeStrategySet` (BTreeMap by name,
deterministic order).

### Debt semantics

```text
DebtItem {
    kind: DebtKind,
    severity: DebtSeverity,
    location: SubjectId,
    description: String,
    decision_ref: Option<DecisionRef>,   // present iff accepted
    revisit_trigger: Option<TriggerCondition>, // required iff decision_ref is Some
}
```

`DebtItem::accepted()` returns `true` iff both `decision_ref` and
`revisit_trigger` are present. Accepted debt is **not** a fix obligation;
the kernel reports it but does not enforce it.

### Strategy registry

`default_strategy_set() -> ChallengeStrategySet` registers the strategies shipped
with A4-2:

- `ArchitectureChallengeStrategy` — wraps `architecture_debverify::run_debverify_audit`.
- `ObservationContradictionChallengeStrategy` — finds pairs of observations on
  the same `SoftwareRelation` where stances conflict (`Affirms` + `Denies`).

Adding a new strategy is a registration-only operation: implement the trait
and add it to `default_strategy_set()`. No kernel code change required.

### Verify/DebVerify separation pin

Both kernels are present and independent. A4-2 pins the invariant that they
are not two modes of one function:

- `verify-kernel` requires `ChangeBasis`. Without one, returns `NotApplicable`.
- `debverify-kernel` does NOT take a `ChangeBasis`. It takes a `Baseline`.

If a future change were to merge them, the `separation_kept` UAT must fail.

## Relationships

### Verify vs. DebVerify

| | Verify | DebVerify |
|---|---|---|
| input | `ChangeBasis` | `Baseline` (+ strategies) |
| scope | delta-reachable | declared `ReconciliationScope` |
| runs with no change? | nothing affected | **yes** |
| result type | `VerificationResult` (5-variant) | `ReconciliationSummary` (6-variant) |
| shared primitives | `EvidenceSource`, `BaselineHash` | same |

The two result types are deliberately distinct. No conversion exists between
them. They share the substrate but not the receipt vocabulary.

### DebVerify vs. Alignment

DebVerify reports **what is stale / contradictory / insufficient**. It does
**not** interpret that against intent. Alignment (A4-3) consumes DebVerify's
output but emits its own advisory vocabulary (`Aligned | Tension | Misaligned`).
A4-2 does NOT emit alignment states.

## Anti-encroachment

`debverify_kernel` does NOT `use`:
- `provider`, `host_sdk`, `agent_host`, `capability`, `effective_instructions`
- `architecture_conformance` (AC4) — Verify is a separate kernel
- `paradigm_profile`, `architecture_mutation`
- any LLM / CogniCode / Chronos / JCode type

Strategies may consume `architecture_debverify` (AC5) and `observation` (A4-0)
because both are substrate, not the other kernel.

## Test invariants (UAT)

The 20 evidence points requested for A4-2 are split into runtime tests (most)
and structural checks (the rest). All are pinned in `crates/sddk-engine/src/debverify_kernel/tests.rs`.

| # | Invariant | Type |
|---|-----------|------|
| 1 | DebVerify takes Baseline, not ChangeBasis | compile-time |
| 2 | Same Baseline → same ReconciliationSummary digest | runtime |
| 3 | Wall clock excluded from baseline identity | runtime |
| 4 | Strategy order does not affect summary | runtime |
| 5 | Strategy registry is open (not hard-coded domain names) | runtime |
| 6 | ArchitectureChallenge produces findings equivalent to AC5 audit | runtime |
| 7 | ObservationContradiction produces findings on real data | runtime |
| 8 | Affirm + Deny on same relation → Conflicted, both kept | runtime |
| 9 | No latest-wins resolution | runtime + structural |
| 10 | Missing evidence → EvidenceGap, not ConfirmedBaseline | runtime |
| 11 | Stale observation does not become Verified | runtime |
| 12 | Accepted debt preserves decision_ref + revisit_trigger | runtime |
| 13 | Verify clean + DebVerify finds drift (and vice versa) | runtime |
| 14 | DebVerify can find a subject with no recent edits | runtime |
| 15 | No universal score field | structural |
| 16 | No canonical mutation from audit | structural |
| 17 | No provider SDK types | structural (grep) |
| 18 | No Alignment→Governance shortcut | structural (grep) |
| 19 | Deterministic serde + ordering | runtime |
| 20 | A4-0 + A4-1 + A3 baselines still green | runtime |

## Open design questions (resolved in A4-2)

- **Q1: closed ADT vs. fully-generic trait?** — closed ADT core + trait for
  strategy pluggability (mirrors A4-1's choice).
- **Q2: baseline identity?** — content hash of `(scope, evidence_set_sha256)`
  with explicit non-inclusion of clock/rendering/messages.
- **Q3: where does the delta scope end and the baseline scope begin?** —
  DebVerify does NOT take a ChangeBasis. Verify does NOT take a Baseline.
  Two entry points, no flag merging.

## Migration plan (NOT in this slice)

A4-2 is the kernel delivery. The cycle `A4-2M` (next, separately scoped)
will demonstrate zero-feature-change convergence of AC4/AC5 onto their
respective generic kernels. That cycle is **not** in scope here.

## Future work (post A4-2, NOT in this slice)

- Knowledge freshness challenge (KMT-based) — extension strategy, separate cycle.
- Decision staleness challenge — extension strategy, separate cycle.
- Static / runtime provider strategies — extension strategies.
- A4-2M AC4/AC5 convergence.
- A4-3 Software Alignment.
