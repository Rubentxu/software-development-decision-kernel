# ADR-039 — Risk- and Evidence-Driven Adaptive Verification

**Status:** Proposed

## Context
Running the same review/debt-verification depth for every change wastes budget; running too little risks regressions. Verification intensity should follow actual risk, change impact and missing evidence.

## Decision
Verification becomes a `CONVERGE` capability set chosen from deterministic signals plus bounded cognitive judgment.

Signals include:
- touched architecture boundaries/dependencies;
- security/auth/data sensitivity;
- API/schema compatibility;
- user-observable behavior;
- change size/novelty/irreversibility;
- test/evidence gaps;
- graph architecture delta;
- previous failures/remediation;
- historical reliability of route/workflow class.

Always run cheap deterministic checks first. Add specialist/verifier agents only when triggered. UAT is activated when behavior/risk policy requires it.

Convergence repeats BUILD/VERIFY until PASS, max rounds or escalation.

## Consequences
- Less fixed review overhead.
- Verification decisions become explainable events.
- Requires strict minimum invariants and bounded max rounds.
