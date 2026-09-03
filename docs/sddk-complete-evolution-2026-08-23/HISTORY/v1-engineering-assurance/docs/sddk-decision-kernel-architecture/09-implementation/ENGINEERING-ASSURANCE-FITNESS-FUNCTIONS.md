# Engineering Assurance — Proposed Architecture Fitness Functions

These extend the current ARCH001..ARCH015 direction without changing existing rule semantics.

## ARCH016 — Assurance remains pack-owned

Kernel/domain crates MUST NOT define Engineering Assurance bounded-context types such as:

```text
EngineeringAssessment
EngineeringFinding
AssuranceObligation
EngineeringProfile
```

Allowed: generic capability/event/evidence/pack contracts.

## ARCH017 — Technology neutrality

Kernel and generic workflow runtime MUST NOT branch on:

```text
rust
go
cpp
typescript
jvm
```

Technology selection belongs to profiles/providers.

## ARCH018 — Skills have no execution authority

A skill may influence an agent prompt but MUST NOT itself grant filesystem, Git, network or deployment effects.

Static validation checks that capability grants remain the only effect authority.

## ARCH019 — Read-only review by default

Engineering Assurance v1 review capabilities are side-effect free.

Any remediation must leave the assessment workflow and request a separately governed mutation capability.

## ARCH020 — Blocking findings require evidence

A gate-compatible `high`/`critical` finding without valid evidence refs is invalid assessment output.

Expected outcome: `INCONCLUSIVE`, not `FAIL` or `PASS`.

## ARCH021 — Profile isolation

Adding/removing a technology profile MUST NOT require:

- new WorkflowIR operator;
- new kernel event primitive;
- new cycle state;
- kernel enum variant for the language.

## ARCH022 — Deterministic verdict authority

Final assurance verdict must be produced by deterministic aggregation over normalized findings, obligations, evidence and policy.

Agent provider output may propose but not directly persist the authoritative verdict.

## ARCH023 — No duplicated analyzer execution

If a current SDD verifier/debt analyzer has already produced valid evidence for the same source revision/config, a bridge SHOULD reuse that evidence rather than rerun it.

Initial enforcement: warning/metrics. Ratchet only after stable evidence identity exists.

## ARCH024 — Source evidence freshness

Required source-derived evidence whose source revision is stale cannot satisfy an obligation.

## ARCH025 — Conditional systems depth

Representation, zero-copy, unsafe, formal verification and deep concurrency checks MUST NOT be mandatory for projects/scopes where profile signals do not activate them.

This prevents the Rust reference profile from turning systems-programming techniques into universal dogma.

## Ratchet

```text
document
→ measure
→ fixture
→ warning
→ baseline exception
→ enforce
```

Do not turn all proposed rules into errors in EA-0.
