# UAT master

## Boundary UAT

- **BC-01** Alignment module cannot import Governance implementation.
- **BC-02** Knowledge cannot import CogniCode/Chronos types.
- **BC-03** advisory context never appears in EffectiveInstructions.
- **BC-04** Workbooks rebuild after projection DB deletion.

## Knowledge/KMT

- **KMT-01** comment-only change leaves dependency fingerprint unchanged.
- **KMT-02** new import invalidates dependency-derived assertions only.
- **KMT-03** analyzer-version change marks affected assertions stale with reason.
- **KMT-04** overlay impact yields POSSIBLY_STALE, not automatic false contradiction.
- **KMT-05** historical `as_of` reconstructs previous current assertion set.

## Verify

- **VER-01** 2 changed files in 100-file fixture do not trigger full deep review.
- **VER-02** impact closure includes dependent module from provider/base graph.
- **VER-03** previous card is supplied before raw source.
- **VER-04** provider missing -> EvidenceGap/NOT_EVALUATED, command still succeeds when Preferred.

## DebVerify

- **DEB-01** finds seeded debt in untouched file.
- **DEB-02** invalid Tradeoff assumption becomes REVIEW_DUE, decision unchanged.
- **DEB-03** risk-weighted deepening avoids sending every file to LLM.
- **DEB-04** emits new baseline + delta from previous.

## Alignment

- **ALI-01** OO lens on pure-functional fixture may return NOT_APPLICABLE without penalty.
- **ALI-02** same universal concern evaluated by FP and OO lenses may disagree; both preserved.
- **ALI-03** heuristic tension cannot block release without explicit Governance policy.
- **ALI-04** explicit MUST_NOT dependency can produce ContractViolation with proof.
- **ALI-05** accepted tension with DecisionRef is not rediscovered as mandatory remediation.

## Providers

- **PRO-01** Base mode with no services.
- **PRO-02** CogniCode dormant socket activates on demand.
- **PRO-03** incompatible provider protocol is reported, not crashed.
- **PRO-04** Chronos absent -> runtime alignment NOT_EVALUATED.
- **PRO-05** active provider job is not killed by idle lifecycle.

## Installed distribution

Every documented agent/CLI example runs against installed artifact, not only unit tests.
