# SPEC-014 — Quality Gates, Entropy and Golden Dataset

**Status:** Proposed

## 1. Architecture entropy

Introduce `sddk dev entropy` as a multidimensional architecture health report.

Metrics SHOULD include:

- LOC and file/module size;
- cyclomatic/cognitive complexity where available;
- public API surface;
- dependency fan-in/fan-out;
- crate/module coupling;
- responsibilities/signals per module;
- Git churn;
- test size and fixture complexity;
- build time hotspot;
- duplication indicators.

LOC alone MUST NOT be a fail gate.

## 2. Known self-dogfood target

The existing large UAT CLI/domain modules are ideal initial entropy fixtures. The goal is not arbitrary size reduction; it is restoring bounded responsibilities and dependency direction.

## 3. Architecture lint

Introduce explicit rules such as:

- `ARCH001 engine_must_not_depend_on_storage`;
- `ARCH002 domain_must_not_depend_on_adapters`;
- `ARCH003 cli_must_not_own_persistence_logic`;
- `ARCH004 packs_must_declare_dependencies`;
- `ARCH005 reactive_behaviors_must_not_execute_governed_effects_directly`.

## 4. Golden dataset

Grow the existing debt-verification golden dataset from the initial handful of cases to **30–50** balanced cases across:

- architecture/coupling;
- duplication;
- security;
- testing;
- documentation drift;
- overengineering;
- UX/UAT;
- agent behavior;
- subtle code smells;
- adversarial hidden mutations.

Include multiple languages such as Rust, TypeScript, Python, Go and Java.

## 5. Metrics

Track:

- precision;
- recall;
- F1;
- calibration where confidence is emitted;
- false-positive severity;
- inter-rater agreement for human labels;
- model/version stability;
- cost per true positive;
- latency.

Initial target floor can retain precision > 0.8 and recall > 0.7, but promotion decisions should consider false-positive severity and cost, not only F1.

## 6. Ratchets

New gates SHOULD start as advisory, establish a baseline, then ratchet only when the repository demonstrates stable compliance.
