---
id: arch-spec-044-generic-debverify
title: Generic DebVerify — baseline challenge and reconciliation
status: contract-ready
milestone: A4
implemented_by: pending (A4-2)
depends_on: arch-spec-042-evidence-observation-provenance + arch-spec-043-generic-verify
---

# arch-spec-044 — Generic DebVerify

> **Contract-ready, NOT implemented.**

## Purpose

DebVerify challenges the **baseline**, including where nothing changed recently.
It can discover stale or wrong knowledge in areas no recent edit touched.

## The distinction is the whole point

```text
Verify     delta-scoped
DebVerify  baseline-challenge-scoped
```

**They are not two modes of one function.** `verify --all` is explicitly not
DebVerify: that would inherit Verify's change-shaped reasoning and miss precisely
the staleness DebVerify exists to find.

## Shape

```text
ReconciliationScope + Baseline + ChallengeStrategy + EvidenceSources
  → ContradictionSet + DebtDelta
```

No change set is required as input.

## Challenge strategies (extensible, one kernel)

```text
DebVerify
 ├── Architecture challenge        (AC5 — already delivered early, A3-S7)
 ├── Knowledge freshness challenge
 ├── Decision staleness challenge
 ├── Contradiction reconciliation  (consumes arch-spec-042)
 └── future static/runtime challenge
```

AC5's five detectors become **strategies** behind this kernel, not a separate
engine.

## Constraints

- Contradictions are first-class and reconciled, never `latest-wins`.
- No universal score; findings carry closed kinds and severities.
- Reuses `arch-spec-042` observations as its evidence channel.
