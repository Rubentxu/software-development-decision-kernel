# Debt Documentation

This directory contains the canonical contracts for the durable debt remediation framework (ADR-0047).

## Files

- **[SEVERITY.md](./SEVERITY.md)** — Severity taxonomy (`critical | high | medium | low`). Intrinsic technical impact, independent of scheduling.
- **[PRIORITY.md](./PRIORITY.md)** — Priority taxonomy (`P0 | P1 | P2 | P3`). Remediation scheduling, distinct from UAT priority namespace.
- **[debt-report.schema.json](./debt-report.schema.json)** (v1.0.0, draft-07) — JSON Schema for the per-cycle debt report.
- **[INCIDENCE-TEMPLATE.md](./INCIDENCE-TEMPLATE.md)** — Template for `INC-NNN-{slug}.md` cross-cycle records.
- **[INC-DEBT-023](./INC-DEBT-023-lints-advisory-no-expansion-cycle.md)** — Lints advisory sin ciclo de expansión programado (low/P3, open; cycle conformance-closeout-2026-09-13).
- **[INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH](./INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH.md)** — Un commit de metadata derivada (`MANIFEST.sha256`) no tiene camino admisible de push sin quemar un bump (medium/P2, open; cycle config-model-v1-2026-09-17).

## Source of truth

- [ADR-0047 — Remediación durable y priorizada de deuda técnica](../adr/ADR-0047-durable-debt-remediation.md)

## Status

- cycle-7a: ratified status, severity+priority taxonomies published (this directory).
- Cycle-7a + 7b: runtime contracts live.

## LOC policy

The project's LOC budget policy is documented in [ADR-0048](../adr/ADR-0048-loc-budget-policy-reformulation.md). The policy uses **total-module-sum budgets** (implementation + boilerplate + test fixtures) rather than per-file targets. Per-file targets are deprecated as of cycle-10.
