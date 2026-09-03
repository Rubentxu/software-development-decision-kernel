# SPEC-042 — Engineering Assurance Pack

**Status:** Proposed

## Purpose

Reusable engineering assessment for SDDK workflows.

## Manifest sketch

```toml
[pack]
id = "sddk-pack-engineering-assurance"
version = "0.1.0"
schema_version = 2
compatibility = ">=1.37"
risk = "low"
consequence = "creates"
category = "domain"

[dependencies]
requires = ["sddk-core"]
integrates_with = ["sddk-pack-sdd", "sddk-pack-uat", "sddk-pack-incident", "sddk-pack-security"]

[provides]
capabilities = [
  "engineering.assess",
  "architecture.review",
  "systems.review",
  "performance.review",
  "concurrency.review",
  "representation.review",
  "verification.plan",
  "engineering.profile.resolve"
]
event_schemas = [
  "engineering.assessment.requested",
  "engineering.assessment.completed",
  "engineering.finding.recorded",
  "engineering.finding.resolved",
  "engineering.obligation.declared",
  "engineering.obligation.verified",
  "engineering.obligation.violated",
  "engineering.evidence.linked"
]
view_types = ["engineering-assurance-report", "engineering-invariant-map"]
```

## Model

### EngineeringAssessment

```yaml
assessment_id: ea-...
subject:
  project_id: ...
  revision: ...
  scope: [...]
goal: ...
dimensions: [...]
profile_ids: [...]
status: planned|running|completed|blocked
```

### AssuranceObligation

```yaml
obligation_id: eao-...
statement: ...
scope: [...]
source:
  kind: policy|adr|change_contract|profile|human
required_evidence: ...
severity_on_violation: ...
status: pending|verified|violated|waived
```

### EngineeringFinding

```yaml
finding_id: eaf-...
assessment_id: ...
rule_id: ...
dimension: ...
severity: critical|high|medium|low|info
claim: ...
scope: ...
evidence_refs: [...]
obligation_ref: ...
fingerprint: sha256:...
disposition: open|accepted|remediated|waived|false_positive
```

### AssuranceVerdict

```yaml
assessment_id: ...
verdict: PASS|PASS_WITH_WARNINGS|FAIL|INCONCLUSIVE
blocking_findings: [...]
unsatisfied_obligations: [...]
evidence_complete: true|false
policy_hash: ...
```

## Default adjudication

```text
missing required evidence → INCONCLUSIVE
blocking violated obligation → FAIL
blocking open finding → FAIL
only warnings → PASS_WITH_WARNINGS
all required obligations satisfied → PASS
```

## Composition

- **SDD Adaptive:** SHAPE can request planning; CONVERGE can request relevant reviews; ChangeContract stays SDD authority.
- **UAT:** link technical evidence to defects/retests, never replace human acceptance.
- **Incident:** analyze invariant/resource/concurrency failures.
- **Security:** security ontology remains in Security pack.

## Non-goals

Code mutation, generic debt replacement, language-specific kernel types.
