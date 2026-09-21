# SPEC-045 — Systems Review Workflow

**Status:** Proposed

## Goal

Provide a reusable workflow for deep engineering review inspired by high-assurance systems practice while remaining language-neutral and dynamically scoped.

## Entry capability

```text
systems.review
```

Input:

```yaml
subject:
  project_id: ...
  scope: ["..."]
goal: ...
risk: low|medium|high|critical
known_constraints: []
requested_dimensions: []
```

## Workflow shape

Compile to existing Workflow Pattern Algebra primitives:

```text
ProfileResolve
    ↓
ConstraintAndInvariantDiscovery
    ↓
Choice(risk/scope)
    ├─ architecture
    ├─ control-flow/resources
    ├─ concurrency
    ├─ representation
    ├─ performance
    └─ verification
          ↓
        Parallel/Map
          ↓
          Join
          ↓
EvidenceValidation
          ↓
DeterministicAdjudication
```

No new kernel operator is required.

## Dynamic dimension selection

Cheap deterministic signals run first.

Examples:

| Signal | Enable dimension |
|---|---|
| concurrency primitives / async runtime | concurrency |
| parser/binary/FFI/unsafe surface | representation |
| explicit latency/throughput goal | performance |
| architecture boundary change | architecture |
| high-risk state transition | invariants + verification |
| none of the above | generic architecture/invariant review |

The Supervisor may propose extra dimensions. Runtime policy validates budgets and capabilities.

## Systems reasoning procedure

Every selected reviewer follows:

1. establish measurable constraints;
2. identify invariants;
3. map trust and architectural boundaries;
4. trace one representative operation end-to-end;
5. identify ownership/liveness and blocking points;
6. account for concurrency, queues and backpressure where relevant;
7. identify copies/allocations/layout assumptions only on relevant hot paths;
8. select proportional verification evidence;
9. emit normalized findings, never unstructured verdict-only prose.

## Netstack-derived heuristics

Use as heuristics, not universal mandates:

- keep core logic small and easy to reason about;
- isolate platform/runtime concerns behind explicit boundaries;
- make invalid states unrepresentable where practical;
- avoid introducing execution-model complexity without a demonstrated need;
- distinguish fixed native layouts from variable protocol representations;
- use compiler/type-system guarantees as part of verification;
- escalate from testing to fuzzing/formal methods according to risk.

## Risk-depth policy

### Low
- architecture/invariants;
- existing deterministic checks;
- no specialist fan-out unless signals demand it.

### Medium
- representative-path trace;
- relevant specialist dimensions;
- evidence completeness gate.

### High/Critical
- parallel independent reviewers where valuable;
- adversarial or human review if policy requires;
- stronger runtime/property/fuzz/formal evidence when applicable;
- `INCONCLUSIVE` blocks the consuming gate.

## Output

```yaml
assessment_ref: ea-...
profiles: [...]
constraints: [...]
obligations: [...]
findings: [...]
evidence_refs: [...]
verification_plan: [...]
verdict: PASS|PASS_WITH_WARNINGS|FAIL|INCONCLUSIVE
```

## Remediation

The review workflow does not mutate code. A consuming workflow may transform accepted findings into WorkUnits or remediation proposals and invoke governed mutation capabilities.
