# `sddk-sdd` Workflow Family

## Purpose
Preserve the original strength of SDDK while moving SDD semantics out of the generic kernel and allowing both reference and adaptive strategies.

## Strategies

### `sdd.reference` / A-full baseline
Preserves the current explicit phase-heavy path for traceability and benchmarking:

```text
Explore → Propose → Spec + Design → Tasks → Build
→ Verify → Debt Verify → Archive/Integrate
```

Legacy `A-min`, `A-lite`, `A-full`, `B-direct` remain compatibility presets during migration.

### `sdd.adaptive`
Compact invariant-driven strategy:

```text
SHAPE → BUILD ⇄ CONVERGE → INTEGRATE
```

See `SDD-ADAPTIVE-WORKFLOW.md` and SPEC-038.

## Common guarantees
Both strategies must preserve:
- clear intent/scope;
- testable acceptance;
- relevant architecture/security constraints;
- implementation/evidence traceability;
- independent verification;
- governed effects/release.

## Migration rule
Do not delete the reference workflow initially. Run reference/adaptive comparisons through Workflow Laboratory and promote adaptive only after agreed non-inferiority thresholds.
