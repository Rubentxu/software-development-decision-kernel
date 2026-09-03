# SPEC-044 — Assurance Evidence Contract

**Status:** Proposed

## Evidence record

```yaml
evidence_id: eve-...
assessment_id: ea-...
kind: source|test|compiler|static-analysis|benchmark|property|fuzz|formal|graph|human|receipt
producer:
  capability: ...
  provider_id: ...
subject:
  project_id: ...
  scope: [...]
artifact_ref: artifact://...
content_hash: sha256:...
source_revision: ...
observed_at: ...
stale: false
```

## Quality rules

1. Blocking findings require non-prose evidence.
2. Source evidence records revision.
3. Benchmarks identify workload and metric.
4. Analyzer evidence records tool/version/config when available.
5. Human evidence references review/approval receipt.
6. Private chain-of-thought is never evidence.
7. Stale required evidence cannot satisfy an obligation.

## Fingerprint

Use stable semantic inputs:

```text
dimension + rule_id + normalized scope + claim class
```

Avoid volatile wording.

## Verdict algorithm

```text
if required evidence missing/stale: INCONCLUSIVE
elif blocking obligation violated: FAIL
elif blocking finding open: FAIL
elif warning finding open: PASS_WITH_WARNINGS
else: PASS
```

## Active Graph

```text
Assessment --evaluates--> Subject
Assessment --uses_profile--> Profile
Finding --violates--> Obligation
Finding --supported_by--> Evidence
Obligation --verified_by--> Evidence
Assessment --produced--> Verdict
```

Projection only.
