# A5 — Deferred Post-BASE

> Cycle: `p-63676b11dc0ef88f/a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only)**

A5 is hardening. Anything that is a *feature* rather than a *risk closure*
is deferred here, so the A5 cycles keep a single change budget.

## Feature freeze (§21) — prohibited in A5

If any of the following appears during A5, it goes to this list, **not**
into an A5 cycle:

- CogniCode integration
- Chronos integration
- JCode reactive loop
- any new AI provider
- any new Alignment lens
- any new intelligence source
- any new graph engine
- any new control plane
- any new workflow feature
- any new plugin system

Rationale: A5 must certify that the *existing* base can operate reliably
in production. Adding capability while hardening invalidates the
certification.

## Deferred list

| Item | Source | Why deferred |
|---|---|---|
| Multi-region deployment / horizontal scaling | non-goal in the readiness contract | beyond BASE |
| SLA / capacity planning | non-goal | beyond BASE |
| `evidence-kernel-v2`-style payload encoding *feature* work beyond `FU-A3-CO-1`'s minimal close | debt disposition | feature, not risk closure |
| Workbook / control-tower surfaces | A4 "explicitly out" | feature |
| Counterfactual planning | A4 "explicitly out" | feature |
| Proof-carrying changes | A4 "explicitly out" | feature |
| LLM alignment evaluator | A4 "explicitly out" | feature |
| New providers (CogniCode/Chronos/JCode) | A4 "explicitly out" | feature |
| `evaluate_lens` external-consumer support (if one is ever found) | debt disposition | would be a migration feature; A5 plans to *delete* the facade |

## Promotion rule

An item leaves this list only when a new milestone (POST_BASE) is opened
with its own change budget and acceptance. It does not get pulled into A5
opportunistically.
