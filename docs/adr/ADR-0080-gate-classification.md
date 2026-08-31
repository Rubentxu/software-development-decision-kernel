# ADR-0080 - Gate Classification Taxonomy

**Status:** proposed
**Type:** decision
**Created:** 2026-08-31
**Created in cycle:** recover-forward-lifecycle
**Supersedes:** none
**Suppressed:** none
**Renamed from:** none
**Amends:** none

---

## Context

Wave-1 budget gates (`gate-uat`, `gate-budget`, `gate-debt-verification`,
`gate-delivery-quality`, `gate-release-clean`) ship without a classification
taxonomy. When a gate fails, the system has no structured way to communicate
whether the failure is `RecoverForward` (retry), `FailClosed` (block), or
`Advisory` (warn). This gaps the recovery-actionability requirements from the
recover-forward lifecycle.

The [[REQ-Gate-Classification-Discriminator]] and
[[REQ-Recovery-Action-Contract]] requirements define the contract:
`GateKind ∈ {Security, Process, Mixed}` and `RecoveryAction ∈
{RecoverForward, FailClosed, Advisory}`. A closed registry at
`gates/classifications.toml` sources the authoritative classification.

---

## Decision

GateKind taxonomy:

| Variant | Meaning |
|---------|---------|
| `Security` | Gate failure blocks all forward progress; unrecoverable by design. |
| `Process` | Gate failure is a workflow process issue; recoverable via `cycle.replan`. |
| `Mixed` | Gate has both Security and Process characteristics; requires explicit resolution. |

RecoveryAction taxonomy:

| Variant | Meaning |
|---------|---------|
| `RecoverForward` | Retry the failed step via `sddk cycle replan`. |
| `FailClosed` | Stop and require human intervention. |
| `Advisory` | Warn but allow forward progress. |

Wave-1 budget gates default to `class = Process` and `recovery_action =
RecoverForward`.

A typed `RecoveryHint` (RFC 9457 shape) carries `{recovery_command, hint}`
for each classified gate. The hint is surfaced to the operator on failure so
the recovery path is actionable without digging through documentation.

The closed registry at `gates/classifications.toml` is the single source of
truth. The file is validated at lint time (SDDK033) for:
- `class` field present and valid (`security | process | mixed`)
- `recovery_action` field valid when present
- `waiver_expiry_days ≤ 30` when `waiver_authority` is set

---

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Inline classification on `GateDef` YAML | Inflates the YAML schema and couples gate definition to classification policy. |
| Separate `sddk-gate-classifier` crate | Over-classification; the taxonomy belongs in `models/` alongside `Severity`. |
| Dynamic lookup at runtime from a database | Breaks the zero-intrusion rule; a TOML file on disk is agent-readable. |
| Security gates default to `FailClosed` without a registry | Requires explicit classification to avoid silent security regressions. |

---

## Consequences

- `GateKind` and `RecoveryAction` enums land in `sddk-domain/src/models/gate_classification.rs`.
- `gates/classifications.toml` ships Wave-1 gates as `Process`/`RecoverForward`.
- The CLI lint rule SDDK033 validates the registry at `sddk lint` time.
- Operators receive a `RecoveryHint` with a concrete recovery command on gate failure.
- Existing gate YAML files are unaffected; classification is additive.

---

## References

- [[REQ-Gate-Classification-Discriminator]]
- [[REQ-Recovery-Action-Contract]]
- [[REQ-Process-Gate-Recoverable-Default]]
- ADR-0047 (Severity 2-variant preservation)
