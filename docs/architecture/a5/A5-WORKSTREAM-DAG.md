# A5 — Workstream DAG & Cycle Roadmap

> Cycle: `p-63676b11dc0ef88f/a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only)**
> Authority: `docs/architecture/README.md` remains the **living roadmap**.

This document proposes the A5 cycle decomposition. It is *not* a second
roadmap authority; the living roadmap row for A5 points here.

## Principle

**One major risk class / change budget per cycle.** Cycles are grouped by
change budget and blast radius, not by name.

## Proposed cycles (validated against the risk register)

The starting hypothesis (§18) was validated with adjustments:

| Cycle | Change budget | Risk classes | Gates | Notes |
|---|---|---|---|---|
| **A5-1** | Release / distribution / version governance | R7, R8, R10, R15 | G7, G8, G16, G15 | resolves `INC-A4-RELEASE-VERSION-DRIFT` + `INC-A5-PUSH-RELEASE-MARKER-FRICTION`; hardens *around* `PublicReleaseGate` (does not redesign it without evidence) |
| **A5-2** | Durability / rebuild / recovery | R1, R2, R20 | G2, G3, G6 | crash-point falsification; CAS corrupt/missing; migration |
| **A5-3** | Concurrency / CAS / authority side-effect races | R3, R4, R5, R6, R12 | G4, G5 | includes the Parallel sender-drop investigation; names the decision→effect atomicity boundary |
| **A5-4** | Compatibility / deprecation / lints / operator UX | R13, R17, R19 | G10, G14 | deletes `evaluate_lens`; disposes lints; `--help`; fitness/doctor design |
| **A5-5** | Reliability / clean-machine / security / operational UAT | R9, R11, R14, R18 | G9, G11, G12, G13 | flake resolution; ignored-test closure; secrets matrix; fresh-environment distribution UAT |
| **A5-C** | BASE_PRODUCTION_READY certification | all gates | G1..G16 | exact-revision certification; milestone receipt |

Adjustments to the hypothesis:

- **Split accepted as proposed** — blast radii are genuinely orthogonal
  (release tooling vs durability vs concurrency vs compat/UX vs
  reliability/security).
- **A5-1 first is required**, because the clean-machine distribution UAT
  (A5-5) and the certification (A5-C) both depend on a trustworthy
  release/push protocol.
- **A5-4 is independent** and could run in parallel with A5-2 in time, but
  the repo convention is linear cycles; keep the order shown.
- **A5-C is not a feature cycle** — it is an audit/certification cycle
  (like A4-CLOSEOUT) that also produces the durable milestone receipt.

## Dependency DAG (§19)

```text
A5-1 release/distribution/version governance
   │
   ├──► A5-5 clean-machine distribution UAT ──┐
   │                                          │
A5-2 durability/rebuild/recovery ─────────────┤
   │                                          │
A5-3 concurrency/CAS/authority races ─────────┤
   │                                          │
A5-4 compat/deprecation/lints/operator UX ────┤
                                              ▼
                                        A5-C BASE_PRODUCTION_READY certification
                                              │
                                              ▼
                                        (POST_BASE)
```

Hard ordering:

- `A5-1 → A5-5` (clean-machine UAT needs a trustworthy release protocol).
- `A5-2 → A5-3` (concurrency stress presumes a working recovery substrate).
- `A5-1, A5-2, A5-3, A5-4, A5-5 → A5-C` (certification consumes all gate
  evidence).
- No cycle may reorder to start A5-C before its prerequisites.

## Per-cycle contract (applies to every A5 cycle)

1. **Single change budget**, named explicitly at cycle open.
2. **A4 regression gate** (§22): the A4 milestone corpus +
   `a4_closeout_milestone_audit` + relevant A5 tests must stay green;
   `A4_CERTIFIED` must not break silently.
3. **Falsification first** where the cycle targets a failure mode: write
   the failing probe before the fix.
4. **Receipts**: each gate produces durable evidence a third party can
   re-run.
5. **STOP rule**: discovering an A4 semantic defect → STOP → corrective
   semantic slice + recertification impact analysis (never "fix inside
   hardening").
6. **Release**: a semantic release only when the cycle changes runtime
   behaviour; a docs/planning-only cycle does **not** publish a
   production release (see the A5-PLAN release policy).

## Feature freeze (§21)

Prohibited in A5: CogniCode, Chronos, JCode reactive loop, new AI
provider, new Alignment lens, new intelligence source, new graph engine,
new control plane, new workflow feature, new plugin system. Good ideas →
`A5-DEFERRED-POST-BASE.md`.

## Roadmap Delta (at A5-PLAN close)

```text
A4-CLOSEOUT  CLOSED v1.169.68
A4           CLOSED / CERTIFIED
A5-PLAN      CLOSED v1.169.69 (planning)
A5-1         NEXT (not auto-opened)
A5-2..A5-5   blocked_by A5-1 (per DAG)
A5-C         blocked_by A5-1..A5-5
A5           blocked_by A5-C
```
