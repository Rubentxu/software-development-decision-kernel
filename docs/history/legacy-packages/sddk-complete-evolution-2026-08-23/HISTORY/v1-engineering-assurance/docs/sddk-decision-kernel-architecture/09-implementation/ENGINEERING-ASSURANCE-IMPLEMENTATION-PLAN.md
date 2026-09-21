# Engineering Assurance — Implementation Plan

## Architectural rule

Build the thinnest possible vertical slice on existing SDDK primitives. Do not create kernel types or new WorkflowIR operators for Engineering Assurance.

## Target layout

```text
packs/
  sddk-pack-engineering-assurance/
    manifest.toml
    profiles/
      engineering.systems.v1.yaml
      engineering.rust.v1.yaml
    schemas/
      assessment.schema.json
      finding.schema.json
      obligation.schema.json
      evidence.schema.json
      verdict.schema.json
    fixtures/
      pass/
      fail/
      inconclusive/
    workflows/
      systems-review.yaml

crates/
  sddk-pack-engineering-assurance/       # only when deterministic code is needed
    src/
      model.rs
      verdict.rs
      profile.rs
      events.rs
      projection.rs

skills/
  systems-reasoning/
  rust-systems-reasoning/

agents/
  engineering-assurance-reviewer.md      # optional logical provider
```

Do not create the Rust crate before a deterministic implementation is actually needed. A manifest + schemas + fixtures is acceptable for EA-0.

## Work packages

### WP1 — Schema first

Implement JSON/TOML/YAML schemas and fixtures before runtime logic.

Acceptance:

- unknown fields rejected where appropriate;
- stable enum sets;
- deterministic fixture serialization;
- source revision mandatory for source evidence.

### WP2 — Verdict evaluator

Pure function:

```text
AssessmentInputs + Policy → AssuranceVerdict
```

No filesystem/network/LLM dependency.

Test matrix:

- missing required evidence → INCONCLUSIVE;
- blocking violation → FAIL;
- warning only → PASS_WITH_WARNINGS;
- complete/no finding → PASS;
- scoped waiver → recomputed result;
- stale evidence → INCONCLUSIVE.

### WP3 — Profile resolver

Start deterministic:

```text
Cargo.toml → rust
go.mod → go
package.json + tsconfig → typescript
pom.xml/build.gradle → jvm
```

Support explicit override. Persist resolution rationale/evidence.

### WP4 — Provider composition

Register capability providers independently:

```text
architecture.review
  - logical architecture agent
  - CogniCode/graph deterministic route
  - human route

systems.review
  - systems reviewer agent

verification.plan
  - reviewer agent
  - deterministic profile rules

engineering.profile.resolve
  - deterministic resolver only
```

A capability may aggregate multiple providers through normal workflow operators.

### WP5 — Rust profile

Reuse existing `rust-patterns`; do not copy it.

Add only assurance deltas:

- invariant/type modeling;
- unsafe proof boundary;
- async/concurrency boundary analysis;
- representation/layout/zero-copy decision gate;
- targeted verification ladder.

Tool adapters are optional and capability/evidence based.

### WP6 — SDD bridge

Current SDD verify/debt remains operational.

Bridge normalized results:

```text
current verifier output
      ↓
normalization
      ↓
EngineeringFinding/Evidence
```

Do not rerun the same analyzer just to satisfy a new schema.

For `sdd-adaptive`:

```text
ChangeContract.verification.obligations
      ↕
AssuranceObligation
```

Mapping belongs to the SDD integration layer, not kernel.

### WP7 — Event/projection

Append pack-owned events using canonical registry validation. Add projection only after event contract is stable.

### WP8 — Dynamic review

Use existing algebra:

```text
Task(profile.resolve)
→ Task(plan)
→ Choice
→ Map/Parallel(review)
→ Join
→ Task(evidence.validate)
→ Gate(verdict)
```

No generated shell or arbitrary orchestration code.

## Migration / compatibility

No migration of historical cycles is required for v1.

Optional later backfill may project existing verify/debt reports into historical assessments, but must mark provenance as `imported`.

## Testing

### Contract tests
- pack manifest validation;
- schema fixtures;
- capability resolution;
- event validation.

### Deterministic unit tests
- profile resolution;
- finding fingerprints;
- evidence staleness;
- verdict aggregation.

### Replay tests
- same events → same projection/verdict.

### Integration
- SDD A-full compatibility bridge;
- SDD Adaptive assurance node;
- Incident consumer example;
- Rust profile self-review of SDDK.

### Negative tests
- blocking finding with prose-only evidence rejected;
- language-specific type in kernel fails architecture fitness check;
- review capability attempting code mutation denied;
- stale required evidence cannot PASS.

## Success metrics

- evidence completeness rate;
- findings with source/test/analyzer evidence;
- duplicate analyzer executions avoided;
- first-pass accepted changes;
- escaped defects attributable to reviewed dimensions;
- false-positive/waiver rate;
- tokens/time/cost per accepted assessment;
- assessment reuse across packs;
- number of language profiles without kernel changes.
