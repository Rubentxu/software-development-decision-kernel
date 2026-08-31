# Severity Taxonomy

The intrinsic technical impact of a debt finding, independent of scheduling.

## critical
**Definition**: Blocks release OR causes data loss OR breaches security boundary.
**Examples**: SQL injection, RCE in production, unhandled panic on hot path, data corruption.
**Escalates when**: Found in any release-blocking gate.
**When to consider remediation**: As soon as detected; do not let this age without a durable record.

## high
**Definition**: Degrades core functionality without workaround.
**Examples**: Major feature broken, performance regression > 50%, broken contract.
**Escalates when**: No workaround exists or workaround degrades other paths.
**When to consider remediation**: Within the current planning horizon; surface in triage.

## medium
**Definition**: Degrades non-core functionality; workaround exists.
**Examples**: Sub-optimal UX, edge case bug, missing test coverage.
**Escalates when**: Workaround becomes unavailable or scope expands.
**When to consider remediation**: Schedule alongside adjacent work or in next minor cleanup pass.

## low
**Definition**: Cosmetic, structural, or speculative debt.
**Examples**: Naming inconsistency, unused import, dead branch, over-abstraction.
**Escalates when**: Accumulates into maintainability problems.
**When to consider remediation**: Opportunistic; revisit when the affected area is touched.

> **Note**: Severity is independent from scheduling priority. The `When to consider remediation` lines above are informational hints only; priority (`P0..P3`, see [PRIORITY.md](./PRIORITY.md)) is a separate axis derived from rules, not from severity bands. A `critical` finding may be assigned any priority depending on context, exposure, and capacity.

> See [ADR-0047](./../adr/ADR-0047-durable-debt-remediation.md) for the framework rationale.
