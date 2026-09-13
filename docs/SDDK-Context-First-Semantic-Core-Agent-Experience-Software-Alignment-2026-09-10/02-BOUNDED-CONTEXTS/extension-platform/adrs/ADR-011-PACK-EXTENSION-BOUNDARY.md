# ADR-011 — Packs extend the kernel through namespaced contracts

**Status:** Proposed

## Decision

Core knows no UAT/Incident/Security-specific domain semantics. Packs register versioned contributions through SDK extension points:

- workflow/target/task providers;
- schema providers;
- projection contributors;
- context contributors;
- evidence validators;
- policy rules;
- query/explanation contributors.

Extension kinds use qualified names such as `uat.scenario` or `incident.root_cause`.

Packs cannot write concrete storage tables or mutate canonical projections directly.

## 2026-09-10 amendment

Packs may contribute AlignmentLens, WorkbookDefinition, analyzers, metrics and suggestion rules. A lens cannot grant capability, mutate projections directly or impose itself on a project without configuration/policy.
