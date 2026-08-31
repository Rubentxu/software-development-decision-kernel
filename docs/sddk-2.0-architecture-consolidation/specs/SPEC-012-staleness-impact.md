# SPEC-012 — Staleness, Impact and Drift Propagation

**Status:** Proposed

## 1. Goal

Generalize the UAT staleness concept across architecture, documentation, tests, evidence and decisions.

## 2. Staleness states

Recommended universal states:

- `fresh`;
- `possibly_stale`;
- `stale`;
- `invalidated`;
- `unknown`.

## 3. Derivation

A fact/view/decision may become stale when a version-bound dependency changes.

Examples:

- code changed after UAT acceptance;
- component changed after ADR review;
- requirement changed after test design;
- CLI surface changed after README example;
- source artifact changed after agent finding;
- benchmark environment changed after performance claim.

## 4. Relationship-driven propagation

Relations such as `verifies`, `governs`, `derived_from`, `documents`, `supports` MAY define deterministic propagation rules.

The initial implementation SHOULD be conservative: mark `possibly_stale` and require explicit revalidation before marking `stale` when semantic invalidation is uncertain.

## 5. Impact queries

Target queries:

```text
sddk stale
sddk impact <commit|artifact|requirement>
sddk graph why-stale <entity>
```

Output MUST include the causal path responsible for staleness.

## 6. Release integration

Release policy may fail closed on stale critical acceptance/evidence while treating low-risk documentation staleness as advisory. Severity is policy, not hard-coded graph logic.
