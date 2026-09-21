# SPEC-042 — Engineering Assurance Pack

**Status:** Proposed

## Purpose

Provide reusable, language-neutral engineering assessment capabilities to any SDDK workflow without adding domain-specific rules to the kernel.

## Pack identity

```toml
[pack]
id = "sddk-pack-engineering-assurance"
version = "0.1.0"
schema_version = 2
compatibility = ">=1.37"
risk = "low"
consequence = "creates"
category = "domain"
description = "Evidence-backed architecture, systems, performance and verification assurance"

[dependencies]
requires = ["sddk-core"]
integrates_with = [
  "sddk-pack-sdd",
  "sddk-pack-uat",
  "sddk-pack-incident",
  "sddk-pack-security"
]
conflicts_with = []

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
  "engineering.assessment.planned",
  "engineering.assessment.completed",
  "engineering.finding.recorded",
  "engineering.finding.resolved",
  "engineering.obligation.declared",
  "engineering.obligation.verified",
  "engineering.obligation.violated",
  "engineering.evidence.linked"
]
view_types = [
  "engineering-assurance-report",
  "engineering-invariant-map",
  "engineering-hot-path-trace"
]
```

## Bounded-context model

### EngineeringAssessment

```yaml
assessment_id: ea-...
subject:
  project_id: ...
  scope: ["crates/foo/**"]
goal: "review architecture and concurrency"
dimensions:
  - architecture
  - concurrency
profile_ids:
  - engineering.rust.v1
risk:
  class: medium
requested_capabilities:
  - architecture.review
  - concurrency.review
status: planned|running|completed|blocked
```

### EngineeringFinding

```yaml
finding_id: eaf-...
assessment_id: ea-...
dimension: architecture
severity: critical|high|medium|low|info
claim: "application core constructs a concrete SQLite adapter"
scope:
  paths: ["..."]
evidence_refs: ["artifact://...", "source://..."]
obligation_ref: eao-...
disposition: open|accepted|remediated|waived|false_positive
fingerprint: sha256:...
```

### AssuranceObligation

```yaml
obligation_id: eao-...
statement: "application core does not construct infrastructure adapters"
scope: ["..."]
source:
  kind: policy|adr|change_contract|profile|human
required_evidence:
  any_of:
    - static-analysis
    - architecture-query
severity_on_violation: high
status: pending|verified|violated|waived
```

### AssuranceVerdict

```yaml
assessment_id: ea-...
verdict: PASS|PASS_WITH_WARNINGS|FAIL|INCONCLUSIVE
blocking_findings: []
unsatisfied_obligations: []
evidence_complete: true
policy_hash: sha256:...
```

## Capability semantics

| Capability | Default effect | Output |
|---|---|---|
| `engineering.assess` | read-only | assessment plan/result |
| `architecture.review` | read-only | normalized findings |
| `systems.review` | read-only | invariant/control-flow/resource findings |
| `performance.review` | read-only | budget/hot-path findings |
| `concurrency.review` | read-only | concurrency/liveness findings |
| `representation.review` | read-only | parsing/layout/representation findings |
| `verification.plan` | read-only | evidence obligations + recommended checks |
| `engineering.profile.resolve` | deterministic read-only | applicable profile set |

No capability in v1 directly mutates project source.

## Event rules

- Pack events use canonical Event Ledger envelopes.
- Event payloads are versioned and validated before append.
- `assessment.completed` references the deterministic verdict artifact.
- Graph/Cockpit state is rebuilt from events; neither becomes authority.

## Policy

Default policy:

- `critical/high` unsatisfied blocking finding → `FAIL`;
- required evidence absent → `INCONCLUSIVE`;
- only non-blocking findings → `PASS_WITH_WARNINGS`;
- no blocking findings + all required obligations satisfied → `PASS`.

Pack consumers MAY tighten policy but MUST NOT reinterpret `INCONCLUSIVE` as `PASS`.

## Composition

### SDD
Use during SHAPE for architecture/verification planning and CONVERGE for adaptive review.

### UAT
Use for technical evidence behind acceptance failures, not as a substitute for human acceptance.

### Incident
Use to analyze invariant violations, concurrency/resource hazards and remediation safety.

### Security
Integrate findings/evidence while security-specific ontology remains in the security pack.

## Non-goals

- code mutation;
- replacing debt lifecycle;
- replacing SDD requirements;
- replacing security threat modeling;
- language-specific kernel types.
